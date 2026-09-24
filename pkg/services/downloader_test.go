package services_test

import (
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"

	"broll-rs/pkg/config"
	"broll-rs/pkg/db"
	"broll-rs/pkg/models"
	"broll-rs/pkg/services"
)

func TestExtFromSimpleURL(t *testing.T) {
	if got := services.ExtFromURL("https://example.com/video.mp4"); got != "mp4" {
		t.Errorf("expected 'mp4', got '%s'", got)
	}
}

func TestExtFromURLWithQueryString(t *testing.T) {
	if got := services.ExtFromURL("https://example.com/video.webm?token=abc"); got != "webm" {
		t.Errorf("expected 'webm', got '%s'", got)
	}
}

func TestExtFromURLNoExtension(t *testing.T) {
	if got := services.ExtFromURL("https://example.com/video"); got != "mp4" {
		t.Errorf("expected 'mp4', got '%s'", got)
	}
}

func TestExtFromURLLongExtensionFallsBack(t *testing.T) {
	if got := services.ExtFromURL("https://example.com/video.download"); got != "mp4" {
		t.Errorf("expected 'mp4', got '%s'", got)
	}
}

func TestExtFromURLMov(t *testing.T) {
	if got := services.ExtFromURL("https://cdn.example.com/clip.mov"); got != "mov" {
		t.Errorf("expected 'mov', got '%s'", got)
	}
}

func TestFindFileWithPrefixFindsFile(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "broll-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	filePath := filepath.Join(tmpDir, "abc123.mp4")
	if err := os.WriteFile(filePath, []byte("data"), 0644); err != nil {
		t.Fatalf("failed to create file: %v", err)
	}

	path, found := services.FindFileWithPrefix(tmpDir, "abc123")
	if !found {
		t.Fatal("expected file to be found")
	}
	if filepath.Base(path) != "abc123.mp4" {
		t.Errorf("expected file abc123.mp4, got %s", filepath.Base(path))
	}
}

func TestFindFileWithPrefixReturnsNoneWhenNoMatch(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "broll-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	filePath := filepath.Join(tmpDir, "xyz999.mp4")
	if err := os.WriteFile(filePath, []byte("data"), 0644); err != nil {
		t.Fatalf("failed to create file: %v", err)
	}

	_, found := services.FindFileWithPrefix(tmpDir, "abc123")
	if found {
		t.Error("expected file not to be found")
	}
}

func TestFindFileWithPrefixEmptyDir(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "broll-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	_, found := services.FindFileWithPrefix(tmpDir, "abc")
	if found {
		t.Error("expected file not to be found")
	}
}

func TestDownloadDirectStreamsToFile(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("fake video bytes"))
	}))
	defer server.Close()

	tmpDir, err := os.MkdirTemp("", "broll-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	client := server.Client()
	url := server.URL + "/video.mp4"

	path, err := services.DownloadDirect(client, url, tmpDir, "test_id")
	if err != nil {
		t.Fatalf("DownloadDirect failed: %v", err)
	}

	contents, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read downloaded file: %v", err)
	}
	if string(contents) != "fake video bytes" {
		t.Errorf("expected 'fake video bytes', got '%s'", string(contents))
	}
}

func TestDownloadDirectFailsOn404(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotFound)
	}))
	defer server.Close()

	tmpDir, err := os.MkdirTemp("", "broll-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	client := server.Client()
	url := server.URL + "/missing.mp4"

	_, err = services.DownloadDirect(client, url, tmpDir, "test_id")
	if err == nil {
		t.Error("expected error on 404, got nil")
	}
}

func TestResolveDirWithNoProjectReturnsBase(t *testing.T) {
	gormDB, err := db.InitDB("sqlite::memory:")
	if err != nil {
		t.Fatalf("InitDB failed: %v", err)
	}
	cfg := config.Config{
		DownloadsDir: "/tmp/broll-test",
	}

	dir := services.ResolveDir(gormDB, cfg, nil)
	if dir != "/tmp/broll-test" {
		t.Errorf("expected /tmp/broll-test, got %s", dir)
	}
}

func TestResolveDirWithUnknownProjectReturnsBase(t *testing.T) {
	gormDB, err := db.InitDB("sqlite::memory:")
	if err != nil {
		t.Fatalf("InitDB failed: %v", err)
	}
	cfg := config.Config{
		DownloadsDir: "/tmp/broll-test",
	}

	pid := "unknown-id"
	dir := services.ResolveDir(gormDB, cfg, &pid)
	if dir != "/tmp/broll-test" {
		t.Errorf("expected /tmp/broll-test, got %s", dir)
	}
}

func TestResolveDirWithKnownProjectReturnsSlugDir(t *testing.T) {
	gormDB, err := db.InitDB("sqlite::memory:")
	if err != nil {
		t.Fatalf("InitDB failed: %v", err)
	}
	proj := models.Project{
		ID:        "p1",
		Name:      "Nature Docs",
		Slug:      "nature-docs",
		CreatedAt: "2024-01-01",
	}
	if err := gormDB.Create(&proj).Error; err != nil {
		t.Fatalf("failed to insert project: %v", err)
	}

	cfg := config.Config{
		DownloadsDir: "/tmp/broll-test",
	}

	pid := "p1"
	dir := services.ResolveDir(gormDB, cfg, &pid)
	expected := filepath.Join("/tmp/broll-test", "nature-docs")
	if dir != expected {
		t.Errorf("expected %s, got %s", expected, dir)
	}
}
