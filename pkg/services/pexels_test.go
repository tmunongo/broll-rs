package services_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"broll-rs/pkg/services"
)

func TestPexelsSearchReturnsEmptyWhenKeyIsEmpty(t *testing.T) {
	client := &http.Client{}
	results := services.PexelsSearch(client, "", "nature", 5)
	if len(results) != 0 {
		t.Errorf("expected empty results, got %d", len(results))
	}
}

func TestPexelsSearchReturnsEmptyOnHTTPError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer server.Close()

	client := server.Client()
	_, err := services.DoPexelsSearch(client, server.URL, "valid_key", "test", 1)
	if err == nil {
		t.Error("expected error on HTTP 500, got nil")
	}
}

func TestPexelsSearchParsesResponseCorrectly(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		resp := map[string]interface{}{
			"videos": []map[string]interface{}{
				{
					"id":       12345,
					"duration": 15.5,
					"user":     map[string]interface{}{"name": "John Doe"},
					"video_files": []map[string]interface{}{
						{"link": "link_small", "width": 800, "height": 600},
						{"link": "link_large", "width": 1920, "height": 1080},
					},
					"video_pictures": []map[string]interface{}{
						{"picture": "thumb_url"},
					},
				},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer server.Close()

	client := server.Client()
	results, err := services.DoPexelsSearch(client, server.URL, "apikey", "nature", 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	r := results[0]
	if r.ID != "pexels_12345" {
		t.Errorf("expected ID pexels_12345, got %s", r.ID)
	}
	if r.Title != "John Doe — nature" {
		t.Errorf("expected title 'John Doe — nature', got '%s'", r.Title)
	}
	if r.Source != "pexels" {
		t.Errorf("expected source pexels, got %s", r.Source)
	}
	if r.Duration == nil || *r.Duration != 15.5 {
		t.Errorf("expected duration 15.5, got %v", r.Duration)
	}
	if r.Thumbnail == nil || *r.Thumbnail != "thumb_url" {
		t.Errorf("expected thumbnail thumb_url, got %v", r.Thumbnail)
	}
	if r.DownloadURL == nil || *r.DownloadURL != "link_large" {
		t.Errorf("expected download_url link_large, got %v", r.DownloadURL)
	}
}
