package services

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"

	"broll-rs/pkg/config"
	"broll-rs/pkg/models"
	"gorm.io/gorm"
)

func RunDownload(db *gorm.DB, cfg config.Config, client *http.Client, req models.DownloadRequest) {
	SetStatus(db, req.ID, "downloading")

	dir := ResolveDir(db, cfg, req.ProjectID)
	if err := os.MkdirAll(dir, 0755); err != nil {
		log.Printf("Cannot create download dir %s: %v", dir, err)
		SetStatus(db, req.ID, "error")
		return
	}

	var path string
	var err error

	switch req.Source {
	case "youtube":
		path, err = DownloadYTDLP(req.DownloadURL, dir, req.ID, cfg)
	case "archive":
		path, err = DownloadArchive(client, req.DownloadURL, dir, req.ID)
	default:
		path, err = DownloadDirect(client, req.DownloadURL, dir, req.ID)
	}

	if err != nil {
		errMsg := fmt.Sprintf("Download failed for %s: %v", req.ID, err)
		log.Printf("%s", errMsg)
		_ = os.WriteFile("download_error.txt", []byte(errMsg), 0644)
		SetStatus(db, req.ID, "error")
		return
	}

	db.Model(&models.LibraryVideo{}).
		Where("id = ?", req.ID).
		Updates(map[string]interface{}{
			"status":   "complete",
			"filepath": path,
		})
	log.Printf("Download complete: %s", path)
}

func SetStatus(db *gorm.DB, id, status string) {
	db.Model(&models.LibraryVideo{}).
		Where("id = ?", id).
		Update("status", status)
}

func ResolveDir(db *gorm.DB, cfg config.Config, projectID *string) string {
	if projectID != nil && *projectID != "" {
		var proj models.Project
		if err := db.Where("id = ?", *projectID).First(&proj).Error; err == nil {
			return filepath.Join(cfg.DownloadsDir, proj.Slug)
		}
	}
	return cfg.DownloadsDir
}

func DownloadYTDLP(urlStr, dir, id string, cfg config.Config) (string, error) {
	safeID := strings.ReplaceAll(id, "/", "_")
	template := filepath.Join(dir, fmt.Sprintf("%s.%%(ext)s", safeID))

	args := []string{
		urlStr,
		"-o",
		template,
		"--no-playlist",
		"--quiet",
		"--merge-output-format",
		"mp4",
		"-f",
		"bestvideo[height<=1080]+bestaudio/best[height<=1080]/best",
	}

	if cfg.YouTubeCookiesBrowser != nil && *cfg.YouTubeCookiesBrowser != "" {
		args = append(args, "--cookies-from-browser", *cfg.YouTubeCookiesBrowser)
	}

	cmd := exec.Command("yt-dlp", args...)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr

	if err := cmd.Run(); err != nil {
		return "", fmt.Errorf("yt-dlp error: %w, stderr: %s", err, stderr.String())
	}

	foundPath, found := FindFileWithPrefix(dir, safeID)
	if !found {
		return "", fmt.Errorf("yt-dlp finished but output file not found")
	}

	return foundPath, nil
}

func DownloadDirect(client *http.Client, urlStr, dir, id string) (string, error) {
	ext := ExtFromURL(urlStr)
	safeID := strings.ReplaceAll(id, "/", "_")
	dest := filepath.Join(dir, fmt.Sprintf("%s.%s", safeID, ext))

	if err := StreamToFile(client, urlStr, dest); err != nil {
		return "", err
	}
	return dest, nil
}

func DownloadArchive(client *http.Client, baseURL, dir, id string) (string, error) {
	resolved := ResolveArchiveURL(client, baseURL)
	if resolved == "" {
		resolved = baseURL
	}
	return DownloadDirect(client, resolved, dir, id)
}

func ResolveArchiveURL(client *http.Client, baseURL string) string {
	trimmed := strings.TrimRight(baseURL, "/")
	parts := strings.Split(trimmed, "/")
	if len(parts) == 0 {
		return ""
	}
	identifier := parts[len(parts)-1]
	metaURL := fmt.Sprintf("https://archive.org/metadata/%s", identifier)

	resp, err := client.Get(metaURL)
	if err != nil || resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return ""
	}
	defer resp.Body.Close()

	var meta struct {
		Files []struct {
			Name string      `json:"name"`
			Size interface{} `json:"size"`
		} `json:"files"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&meta); err != nil {
		return ""
	}

	var bestName string
	var maxSize uint64

	for _, f := range meta.Files {
		if strings.HasSuffix(f.Name, ".mp4") {
			var size uint64
			switch v := f.Size.(type) {
			case string:
				size, _ = strconv.ParseUint(v, 10, 64)
			case float64:
				size = uint64(v)
			}

			if size >= maxSize {
				maxSize = size
				bestName = f.Name
			}
		}
	}

	if bestName != "" {
		return fmt.Sprintf("https://archive.org/download/%s/%s", identifier, bestName)
	}

	return ""
}

func StreamToFile(client *http.Client, urlStr, dest string) error {
	resp, err := client.Get(urlStr)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	out, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, resp.Body)
	return err
}

func FindFileWithPrefix(dir, prefix string) (string, bool) {
	entries, err := os.ReadDir(dir)
	if err != nil {
		return "", false
	}
	for _, entry := range entries {
		if strings.HasPrefix(entry.Name(), prefix) {
			return filepath.Join(dir, entry.Name()), true
		}
	}
	return "", false
}

func ExtFromURL(urlStr string) string {
	parts := strings.Split(urlStr, "?")
	path := parts[0]
	dotIdx := strings.LastIndex(path, ".")
	if dotIdx != -1 && dotIdx < len(path)-1 {
		ext := path[dotIdx+1:]
		if len(ext) <= 4 {
			return ext
		}
	}
	return "mp4"
}
