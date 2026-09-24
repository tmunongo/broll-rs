package routes

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"broll-rs/pkg/config"
	"broll-rs/pkg/models"
	"broll-rs/pkg/services"
	"broll-rs/templates"
	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/google/uuid"
	"gorm.io/gorm"
)

type App struct {
	DB     *gorm.DB
	Config config.Config
	HTTP   *http.Client
}

func NewRouter(app *App) http.Handler {
	r := chi.NewRouter()
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)

	// Static / UI
	r.Get("/", app.handleIndex)
	r.Get("/app.css", app.handleAppCSS)
	r.Get("/app.js", app.handleAppJS)

	// API
	r.Route("/api", func(r chi.Router) {
		r.Get("/search", app.handleSearch)

		r.Post("/download", app.handleStartDownload)
		r.Get("/download/status/{id}", app.handleDownloadStatus)
		r.Post("/download/retry/{id}", app.handleDownloadRetry)

		r.Get("/library", app.handleListLibrary)
		r.Delete("/library/{id}", app.handleDeleteLibrary)
		r.Patch("/library/{id}/tags", app.handleUpdateTags)
		r.Get("/library/file/{id}", app.handleServeFile)

		r.Get("/projects", app.handleListProjects)
		r.Post("/projects", app.handleCreateProject)
		r.Delete("/projects/{id}", app.handleDeleteProject)
	})

	return r
}

func jsonResponse(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
}

func jsonError(w http.ResponseWriter, status int, msg string) {
	jsonResponse(w, status, map[string]string{"error": msg})
}

func (app *App) handleIndex(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	_, _ = w.Write([]byte(templates.IndexHTML))
}

func (app *App) handleAppCSS(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/css; charset=utf-8")
	_, _ = w.Write([]byte(templates.AppCSS))
}

func (app *App) handleAppJS(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/javascript; charset=utf-8")
	_, _ = w.Write([]byte(templates.AppJS))
}

func (app *App) handleSearch(w http.ResponseWriter, r *http.Request) {
	q := strings.TrimSpace(r.URL.Query().Get("q"))
	sources := r.URL.Query().Get("sources")
	if sources == "" {
		sources = "pexels,pixabay,archive,youtube"
	}

	var wg sync.WaitGroup
	var mu sync.Mutex
	combined := []models.VideoResult{}

	if strings.Contains(sources, "pexels") {
		wg.Add(1)
		go func() {
			defer wg.Done()
			res := services.PexelsSearch(app.HTTP, app.Config.PexelsAPIKey, q, 8)
			mu.Lock()
			combined = append(combined, res...)
			mu.Unlock()
		}()
	}

	if strings.Contains(sources, "pixabay") {
		wg.Add(1)
		go func() {
			defer wg.Done()
			res := services.PixabaySearch(app.HTTP, app.Config.PixabayAPIKey, q, 8)
			mu.Lock()
			combined = append(combined, res...)
			mu.Unlock()
		}()
	}

	if strings.Contains(sources, "archive") {
		wg.Add(1)
		go func() {
			defer wg.Done()
			res := services.ArchiveSearch(app.HTTP, q, 6)
			mu.Lock()
			combined = append(combined, res...)
			mu.Unlock()
		}()
	}

	if strings.Contains(sources, "youtube") {
		wg.Add(1)
		go func() {
			defer wg.Done()
			res := services.YouTubeSearch(q, 8, app.Config.YouTubeCookiesBrowser)
			mu.Lock()
			combined = append(combined, res...)
			mu.Unlock()
		}()
	}

	wg.Wait()
	jsonResponse(w, http.StatusOK, combined)
}

func (app *App) getLibraryVideoByID(id string) (*models.LibraryVideo, error) {
	var rec models.LibraryVideo
	err := app.DB.Table("videos v").
		Select("v.*, p.name as project_name").
		Joins("LEFT JOIN projects p ON v.project_id = p.id").
		Where("v.id = ?", id).
		Scan(&rec).Error
	if err != nil || rec.ID == "" {
		return nil, gorm.ErrRecordNotFound
	}
	return &rec, nil
}

func (app *App) handleStartDownload(w http.ResponseWriter, r *http.Request) {
	var req models.DownloadRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		jsonError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	existing, _ := app.getLibraryVideoByID(req.ID)
	if existing != nil && existing.Status == "complete" {
		jsonResponse(w, http.StatusOK, existing)
		return
	}

	now := time.Now().UTC().Format(time.RFC3339)

	if existing != nil {
		app.DB.Model(&models.LibraryVideo{}).
			Where("id = ?", req.ID).
			Updates(map[string]interface{}{
				"status":     "pending",
				"project_id": req.ProjectID,
			})
	} else {
		newVid := models.LibraryVideo{
			ID:          req.ID,
			Title:       req.Title,
			Source:      req.Source,
			Thumbnail:   req.Thumbnail,
			OriginalURL: &req.DownloadURL,
			Duration:    req.Duration,
			Status:      "pending",
			ProjectID:   req.ProjectID,
			CreatedAt:   now,
		}
		if err := app.DB.Create(&newVid).Error; err != nil {
			jsonError(w, http.StatusInternalServerError, err.Error())
			return
		}
	}

	record, err := app.getLibraryVideoByID(req.ID)
	if err != nil {
		jsonError(w, http.StatusInternalServerError, "Failed to retrieve record")
		return
	}

	go services.RunDownload(app.DB, app.Config, app.HTTP, req)

	jsonResponse(w, http.StatusOK, record)
}

func (app *App) handleDownloadRetry(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	record, err := app.getLibraryVideoByID(id)
	if err != nil {
		jsonError(w, http.StatusNotFound, fmt.Sprintf("No record for id=%s", id))
		return
	}

	if record.Status != "error" {
		jsonError(w, http.StatusBadRequest, fmt.Sprintf("Can only retry failed downloads (current status: %s)", record.Status))
		return
	}

	if record.OriginalURL == nil || *record.OriginalURL == "" {
		jsonError(w, http.StatusBadRequest, "No original URL stored — cannot retry")
		return
	}

	app.DB.Model(&models.LibraryVideo{}).
		Where("id = ?", id).
		Update("status", "pending")

	updated, err := app.getLibraryVideoByID(id)
	if err != nil {
		jsonError(w, http.StatusInternalServerError, "Failed to retrieve updated record")
		return
	}

	req := models.DownloadRequest{
		ID:          record.ID,
		Title:       record.Title,
		Source:      record.Source,
		DownloadURL: *record.OriginalURL,
		Thumbnail:   record.Thumbnail,
		Duration:    record.Duration,
		ProjectID:   record.ProjectID,
	}

	go services.RunDownload(app.DB, app.Config, app.HTTP, req)

	jsonResponse(w, http.StatusOK, updated)
}

func (app *App) handleDownloadStatus(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var vid models.LibraryVideo
	err := app.DB.Select("id, status, filepath").Where("id = ?", id).First(&vid).Error
	if err != nil {
		jsonError(w, http.StatusNotFound, fmt.Sprintf("No download record for id=%s", id))
		return
	}

	resp := models.StatusResponse{
		ID:       vid.ID,
		Status:   vid.Status,
		Filepath: vid.Filepath,
	}
	jsonResponse(w, http.StatusOK, resp)
}

func (app *App) handleListLibrary(w http.ResponseWriter, r *http.Request) {
	projectID := r.URL.Query().Get("project_id")

	var videos []models.LibraryVideo
	query := app.DB.Table("videos v").
		Select("v.*, p.name as project_name").
		Joins("LEFT JOIN projects p ON v.project_id = p.id")

	if projectID != "" {
		query = query.Where("v.project_id = ?", projectID)
	}

	if err := query.Order("v.created_at DESC").Scan(&videos).Error; err != nil {
		jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	if videos == nil {
		videos = []models.LibraryVideo{}
	}

	jsonResponse(w, http.StatusOK, videos)
}

func (app *App) handleDeleteLibrary(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var vid models.LibraryVideo
	if err := app.DB.Where("id = ?", id).First(&vid).Error; err != nil {
		jsonError(w, http.StatusNotFound, fmt.Sprintf("id=%s", id))
		return
	}

	if vid.Filepath != nil && *vid.Filepath != "" {
		if _, err := os.Stat(*vid.Filepath); err == nil {
			os.Remove(*vid.Filepath)
		}
	}

	if err := app.DB.Delete(&vid).Error; err != nil {
		jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	jsonResponse(w, http.StatusOK, map[string]string{"deleted": id})
}

func (app *App) handleUpdateTags(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var body models.TagUpdate
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		jsonError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	res := app.DB.Model(&models.LibraryVideo{}).
		Where("id = ?", id).
		Update("tags", body.Tags)

	if res.Error != nil {
		jsonError(w, http.StatusInternalServerError, res.Error.Error())
		return
	}
	if res.RowsAffected == 0 {
		jsonError(w, http.StatusNotFound, fmt.Sprintf("id=%s", id))
		return
	}

	jsonResponse(w, http.StatusOK, map[string]string{"id": id, "tags": body.Tags})
}

func (app *App) handleServeFile(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var vid models.LibraryVideo
	if err := app.DB.Where("id = ?", id).First(&vid).Error; err != nil || vid.Filepath == nil || *vid.Filepath == "" {
		jsonError(w, http.StatusNotFound, "File not found")
		return
	}

	fp := *vid.Filepath
	if _, err := os.Stat(fp); os.IsNotExist(err) {
		jsonError(w, http.StatusNotFound, "File missing from disk")
		return
	}

	filename := filepath.Base(fp)
	w.Header().Set("Content-Type", "video/mp4")
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=\"%s\"", filename))
	http.ServeFile(w, r, fp)
}

func (app *App) handleListProjects(w http.ResponseWriter, r *http.Request) {
	var projects []models.Project
	if err := app.DB.Order("created_at DESC").Find(&projects).Error; err != nil {
		jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}
	if projects == nil {
		projects = []models.Project{}
	}
	jsonResponse(w, http.StatusOK, projects)
}

func (app *App) handleCreateProject(w http.ResponseWriter, r *http.Request) {
	var req models.CreateProjectRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		jsonError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	name := strings.TrimSpace(req.Name)
	if name == "" {
		jsonError(w, http.StatusBadRequest, "Project name cannot be empty")
		return
	}

	slug := models.Slugify(name)
	if slug == "" {
		jsonError(w, http.StatusBadRequest, "Project name produces an empty slug")
		return
	}

	id := uuid.New().String()
	now := time.Now().UTC().Format(time.RFC3339)

	proj := models.Project{
		ID:        id,
		Name:      name,
		Slug:      slug,
		CreatedAt: now,
	}

	if err := app.DB.Create(&proj).Error; err != nil {
		if strings.Contains(err.Error(), "UNIQUE") || strings.Contains(err.Error(), "unique") {
			jsonError(w, http.StatusBadRequest, fmt.Sprintf("Project '%s' already exists", name))
			return
		}
		jsonError(w, http.StatusInternalServerError, err.Error())
		return
	}

	jsonResponse(w, http.StatusOK, proj)
}

func (app *App) handleDeleteProject(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	res := app.DB.Delete(&models.Project{}, "id = ?", id)
	if res.Error != nil {
		jsonError(w, http.StatusInternalServerError, res.Error.Error())
		return
	}
	if res.RowsAffected == 0 {
		jsonError(w, http.StatusNotFound, fmt.Sprintf("Project id=%s", id))
		return
	}
	jsonResponse(w, http.StatusOK, map[string]string{"deleted": id})
}
