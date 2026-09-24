package routes_test

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"

	"broll-rs/pkg/config"
	"broll-rs/pkg/db"
	"broll-rs/pkg/models"
	"broll-rs/pkg/routes"
)

func buildTestApp(t *testing.T) http.Handler {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, err := db.InitDB(dbURL)
	if err != nil {
		t.Fatalf("InitDB failed: %v", err)
	}

	cfg := config.Config{
		PexelsAPIKey:  "",
		PixabayAPIKey: "",
		DownloadsDir:  filepath.Join(os.TempDir(), "broll-test-routes"),
		DatabaseURL:   dbURL,
		Port:          8000,
	}

	app := &routes.App{
		DB:     gormDB,
		Config: cfg,
		HTTP:   &http.Client{},
	}

	return routes.NewRouter(app)
}

func executeRequest(handler http.Handler, req *http.Request) *httptest.ResponseRecorder {
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)
	return rr
}

func parseJSON(t *testing.T, rr *httptest.ResponseRecorder, v interface{}) {
	if err := json.NewDecoder(rr.Body).Decode(v); err != nil {
		t.Fatalf("failed to decode JSON body: %v, body string: %s", err, rr.Body.String())
	}
}

func TestIndexReturnsHTML(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("GET", "/", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rr.Code)
	}
	if ct := rr.Header().Get("Content-Type"); ct != "text/html; charset=utf-8" {
		t.Errorf("expected text/html; charset=utf-8, got %s", ct)
	}
}

func TestListProjectsEmpty(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("GET", "/api/projects", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rr.Code)
	}
	var projects []models.Project
	parseJSON(t, rr, &projects)
	if len(projects) != 0 {
		t.Errorf("expected empty array, got %d items", len(projects))
	}
}

func TestCreateProjectSuccess(t *testing.T) {
	app := buildTestApp(t)
	body, _ := json.Marshal(map[string]string{"name": "Nature Docs"})
	req, _ := http.NewRequest("POST", "/api/projects", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(app, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}
	var proj models.Project
	parseJSON(t, rr, &proj)
	if proj.Name != "Nature Docs" {
		t.Errorf("expected name 'Nature Docs', got '%s'", proj.Name)
	}
	if proj.Slug != "nature-docs" {
		t.Errorf("expected slug 'nature-docs', got '%s'", proj.Slug)
	}
}

func TestCreateProjectEmptyNameReturns400(t *testing.T) {
	app := buildTestApp(t)
	body, _ := json.Marshal(map[string]string{"name": "   "})
	req, _ := http.NewRequest("POST", "/api/projects", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(app, req)

	if rr.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rr.Code)
	}
}

func TestCreateProjectSpecialCharsOnlyReturns400(t *testing.T) {
	app := buildTestApp(t)
	body, _ := json.Marshal(map[string]string{"name": "!!!"})
	req, _ := http.NewRequest("POST", "/api/projects", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(app, req)

	if rr.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rr.Code)
	}
}

func TestDeleteProjectNotFound(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("DELETE", "/api/projects/nonexistent-id", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", rr.Code)
	}
}

func TestCreateAndDeleteProject(t *testing.T) {
	app := buildTestApp(t)

	// Create
	body, _ := json.Marshal(map[string]string{"name": "Temp Project"})
	req, _ := http.NewRequest("POST", "/api/projects", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(app, req)
	if rr.Code != http.StatusOK {
		t.Fatalf("create failed with status %d", rr.Code)
	}

	var proj models.Project
	parseJSON(t, rr, &proj)
	id := proj.ID

	// Delete
	reqDel, _ := http.NewRequest("DELETE", "/api/projects/"+id, nil)
	rrDel := executeRequest(app, reqDel)
	if rrDel.Code != http.StatusOK {
		t.Fatalf("delete failed with status %d", rrDel.Code)
	}

	var delResp map[string]string
	parseJSON(t, rrDel, &delResp)
	if delResp["deleted"] != id {
		t.Errorf("expected deleted id %s, got %s", id, delResp["deleted"])
	}
}

func TestCreateDuplicateProjectReturns400(t *testing.T) {
	app := buildTestApp(t)
	body, _ := json.Marshal(map[string]string{"name": "Unique Project"})

	req1, _ := http.NewRequest("POST", "/api/projects", bytes.NewReader(body))
	req1.Header.Set("Content-Type", "application/json")
	rr1 := executeRequest(app, req1)
	if rr1.Code != http.StatusOK {
		t.Fatalf("first create failed: %d", rr1.Code)
	}

	req2, _ := http.NewRequest("POST", "/api/projects", bytes.NewReader(body))
	req2.Header.Set("Content-Type", "application/json")
	rr2 := executeRequest(app, req2)
	if rr2.Code != http.StatusBadRequest {
		t.Errorf("expected 400 on duplicate, got %d", rr2.Code)
	}
}

func TestListLibraryEmpty(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("GET", "/api/library", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rr.Code)
	}
	var vids []models.LibraryVideo
	parseJSON(t, rr, &vids)
	if len(vids) != 0 {
		t.Errorf("expected empty array, got %d items", len(vids))
	}
}

func TestDeleteLibraryItemNotFound(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("DELETE", "/api/library/nonexistent-id", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", rr.Code)
	}
}

func TestUpdateTagsNotFound(t *testing.T) {
	app := buildTestApp(t)
	body, _ := json.Marshal(map[string]string{"tags": "nature"})
	req, _ := http.NewRequest("PATCH", "/api/library/nonexistent-id/tags", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(app, req)

	if rr.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", rr.Code)
	}
}

func TestServeFileNotFound(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("GET", "/api/library/file/nonexistent-id", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", rr.Code)
	}
}

func TestDownloadStatusNotFound(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("GET", "/api/download/status/nonexistent-id", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", rr.Code)
	}
}

func TestStartDownloadCreatesRecord(t *testing.T) {
	app := buildTestApp(t)
	body, _ := json.Marshal(models.DownloadRequest{
		ID:          "vid-abc-123",
		Title:       "Sample Video",
		Source:      "pexels",
		DownloadURL: "https://example.com/video.mp4",
	})
	req, _ := http.NewRequest("POST", "/api/download", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(app, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var rec models.LibraryVideo
	parseJSON(t, rr, &rec)
	if rec.ID != "vid-abc-123" {
		t.Errorf("expected ID vid-abc-123, got %s", rec.ID)
	}
	if rec.Status != "pending" {
		t.Errorf("expected status pending, got %s", rec.Status)
	}
}

func TestDownloadStatusAfterStart(t *testing.T) {
	app := buildTestApp(t)
	body, _ := json.Marshal(models.DownloadRequest{
		ID:          "vid-status-test",
		Title:       "Status Test Video",
		Source:      "archive",
		DownloadURL: "https://example.com/video.mp4",
	})
	req, _ := http.NewRequest("POST", "/api/download", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rrPost := executeRequest(app, req)
	if rrPost.Code != http.StatusOK {
		t.Fatalf("start download failed with %d", rrPost.Code)
	}

	reqStatus, _ := http.NewRequest("GET", "/api/download/status/vid-status-test", nil)
	rrStatus := executeRequest(app, reqStatus)
	if rrStatus.Code != http.StatusOK {
		t.Fatalf("status request failed with %d", rrStatus.Code)
	}

	var statusResp models.StatusResponse
	parseJSON(t, rrStatus, &statusResp)
	if statusResp.ID != "vid-status-test" {
		t.Errorf("expected ID vid-status-test, got %s", statusResp.ID)
	}
	if statusResp.Status == "" {
		t.Error("expected non-empty status")
	}
}

func TestStartDownloadExistingCompleteReturnsExisting(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, err := db.InitDB(dbURL)
	if err != nil {
		t.Fatalf("InitDB failed: %v", err)
	}

	doneVid := models.LibraryVideo{
		ID:        "done-vid",
		Title:     "Done",
		Source:    "pexels",
		Status:    "complete",
		CreatedAt: "2024-01-01",
	}
	gormDB.Create(&doneVid)

	app := &routes.App{
		DB:     gormDB,
		Config: config.Config{DownloadsDir: "/tmp"},
		HTTP:   &http.Client{},
	}
	router := routes.NewRouter(app)

	body, _ := json.Marshal(models.DownloadRequest{
		ID:          "done-vid",
		Title:       "Done",
		Source:      "pexels",
		DownloadURL: "https://example.com/done.mp4",
	})
	req, _ := http.NewRequest("POST", "/api/download", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(router, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var rec models.LibraryVideo
	parseJSON(t, rr, &rec)
	if rec.Status != "complete" {
		t.Errorf("expected status complete, got %s", rec.Status)
	}
}

func TestSearchWithNoRealKeysReturnsEmptyOrResults(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("GET", "/api/search?q=nature&sources=pexels,pixabay", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rr.Code)
	}
	var results []models.VideoResult
	parseJSON(t, rr, &results)
}

func TestListLibraryWithSeedData(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, _ := db.InitDB(dbURL)

	v := models.LibraryVideo{
		ID:        "v1",
		Title:     "A",
		Source:    "pexels",
		Status:    "complete",
		CreatedAt: "2024-01-01",
	}
	gormDB.Create(&v)

	app := &routes.App{DB: gormDB, Config: config.Config{}, HTTP: &http.Client{}}
	router := routes.NewRouter(app)

	req, _ := http.NewRequest("GET", "/api/library", nil)
	rr := executeRequest(router, req)
	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var vids []models.LibraryVideo
	parseJSON(t, rr, &vids)
	if len(vids) != 1 || vids[0].ID != "v1" {
		t.Errorf("unexpected library content: %+v", vids)
	}
}

func TestUpdateTagsSuccess(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, _ := db.InitDB(dbURL)

	v := models.LibraryVideo{
		ID:        "v2",
		Title:     "B",
		Source:    "archive",
		Status:    "pending",
		CreatedAt: "2024-01-01",
	}
	gormDB.Create(&v)

	app := &routes.App{DB: gormDB, Config: config.Config{}, HTTP: &http.Client{}}
	router := routes.NewRouter(app)

	body, _ := json.Marshal(map[string]string{"tags": "nature,wildlife"})
	req, _ := http.NewRequest("PATCH", "/api/library/v2/tags", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rr := executeRequest(router, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var res map[string]string
	parseJSON(t, rr, &res)
	if res["tags"] != "nature,wildlife" {
		t.Errorf("expected tags 'nature,wildlife', got '%s'", res["tags"])
	}
}

func TestDeleteLibraryItemSuccess(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, _ := db.InitDB(dbURL)

	v := models.LibraryVideo{
		ID:        "v3",
		Title:     "C",
		Source:    "pixabay",
		Status:    "complete",
		CreatedAt: "2024-01-01",
	}
	gormDB.Create(&v)

	app := &routes.App{DB: gormDB, Config: config.Config{}, HTTP: &http.Client{}}
	router := routes.NewRouter(app)

	req, _ := http.NewRequest("DELETE", "/api/library/v3", nil)
	rr := executeRequest(router, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var res map[string]string
	parseJSON(t, rr, &res)
	if res["deleted"] != "v3" {
		t.Errorf("expected deleted v3, got %s", res["deleted"])
	}
}

func TestListLibraryFilteredByProject(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, _ := db.InitDB(dbURL)

	proj := models.Project{
		ID:        "proj1",
		Name:      "NatDocs",
		Slug:      "nat-docs",
		CreatedAt: "2024-01-01",
	}
	gormDB.Create(&proj)

	pid := "proj1"
	v4 := models.LibraryVideo{
		ID:        "v4",
		Title:     "D",
		Source:    "pexels",
		Status:    "complete",
		ProjectID: &pid,
		CreatedAt: "2024-01-02",
	}
	v5 := models.LibraryVideo{
		ID:        "v5",
		Title:     "E",
		Source:    "pexels",
		Status:    "pending",
		CreatedAt: "2024-01-03",
	}
	gormDB.Create(&v4)
	gormDB.Create(&v5)

	app := &routes.App{DB: gormDB, Config: config.Config{}, HTTP: &http.Client{}}
	router := routes.NewRouter(app)

	req, _ := http.NewRequest("GET", "/api/library?project_id=proj1", nil)
	rr := executeRequest(router, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var vids []models.LibraryVideo
	parseJSON(t, rr, &vids)
	if len(vids) != 1 || vids[0].ID != "v4" {
		t.Errorf("expected only v4, got %+v", vids)
	}
}

func TestRetryDownloadNotFound(t *testing.T) {
	app := buildTestApp(t)
	req, _ := http.NewRequest("POST", "/api/download/retry/nonexistent-id", nil)
	rr := executeRequest(app, req)

	if rr.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", rr.Code)
	}
}

func TestRetryDownloadRejectsNonErrorStatus(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, _ := db.InitDB(dbURL)

	origURL := "https://example.com/v.mp4"
	v := models.LibraryVideo{
		ID:          "ok-vid",
		Title:       "OK",
		Source:      "pexels",
		Status:      "complete",
		OriginalURL: &origURL,
		CreatedAt:   "2024-01-01",
	}
	gormDB.Create(&v)

	app := &routes.App{DB: gormDB, Config: config.Config{}, HTTP: &http.Client{}}
	router := routes.NewRouter(app)

	req, _ := http.NewRequest("POST", "/api/download/retry/ok-vid", nil)
	rr := executeRequest(router, req)

	if rr.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rr.Code)
	}
}

func TestRetryDownloadResetsErrorToPending(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, _ := db.InitDB(dbURL)

	origURL := "https://example.com/v.mp4"
	v := models.LibraryVideo{
		ID:          "fail-vid",
		Title:       "Failed",
		Source:      "pexels",
		Status:      "error",
		OriginalURL: &origURL,
		CreatedAt:   "2024-01-01",
	}
	gormDB.Create(&v)

	app := &routes.App{DB: gormDB, Config: config.Config{}, HTTP: &http.Client{}}
	router := routes.NewRouter(app)

	req, _ := http.NewRequest("POST", "/api/download/retry/fail-vid", nil)
	rr := executeRequest(router, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var rec models.LibraryVideo
	parseJSON(t, rr, &rec)
	if rec.ID != "fail-vid" || rec.Status != "pending" {
		t.Errorf("unexpected record after retry: %+v", rec)
	}
}

func TestRetryDownloadNoOriginalURLReturns400(t *testing.T) {
	dbURL := fmt.Sprintf("file:mem_%s?mode=memory&cache=shared", t.Name())
	gormDB, _ := db.InitDB(dbURL)

	v := models.LibraryVideo{
		ID:        "no-url-vid",
		Title:     "No URL",
		Source:    "pexels",
		Status:    "error",
		CreatedAt: "2024-01-01",
	}
	gormDB.Create(&v)

	app := &routes.App{DB: gormDB, Config: config.Config{}, HTTP: &http.Client{}}
	router := routes.NewRouter(app)

	req, _ := http.NewRequest("POST", "/api/download/retry/no-url-vid", nil)
	rr := executeRequest(router, req)

	if rr.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rr.Code)
	}
}
