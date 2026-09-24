package services_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"broll-rs/pkg/services"
)

func TestTitlecaseBasic(t *testing.T) {
	if got := services.Titlecase("hello world"); got != "Hello world" {
		t.Errorf("expected 'Hello world', got '%s'", got)
	}
}

func TestTitlecaseAlreadyUpper(t *testing.T) {
	if got := services.Titlecase("Hello"); got != "Hello" {
		t.Errorf("expected 'Hello', got '%s'", got)
	}
}

func TestTitlecaseEmpty(t *testing.T) {
	if got := services.Titlecase(""); got != "" {
		t.Errorf("expected '', got '%s'", got)
	}
}

func TestTitlecaseSingleChar(t *testing.T) {
	if got := services.Titlecase("x"); got != "X" {
		t.Errorf("expected 'X', got '%s'", got)
	}
}

func TestBestPrefersLarge(t *testing.T) {
	w, h := uint32(1920), uint32(1080)
	wM, hM := uint32(1280), uint32(720)
	vids := services.PixabayVideos{
		Large: &services.PixabayFile{
			URL:    "large.mp4",
			Width:  &w,
			Height: &h,
		},
		Medium: &services.PixabayFile{
			URL:    "medium.mp4",
			Width:  &wM,
			Height: &hM,
		},
	}
	best := vids.Best()
	if best == nil || best.URL != "large.mp4" {
		t.Errorf("expected large.mp4, got %v", best)
	}
}

func TestBestFallsBackToMedium(t *testing.T) {
	vids := services.PixabayVideos{
		Medium: &services.PixabayFile{URL: "medium.mp4"},
	}
	best := vids.Best()
	if best == nil || best.URL != "medium.mp4" {
		t.Errorf("expected medium.mp4, got %v", best)
	}
}

func TestBestFallsBackToTiny(t *testing.T) {
	vids := services.PixabayVideos{
		Tiny: &services.PixabayFile{URL: "tiny.mp4"},
	}
	best := vids.Best()
	if best == nil || best.URL != "tiny.mp4" {
		t.Errorf("expected tiny.mp4, got %v", best)
	}
}

func TestBestReturnsNilWhenAllNil(t *testing.T) {
	vids := services.PixabayVideos{}
	if best := vids.Best(); best != nil {
		t.Errorf("expected nil best, got %v", best)
	}
}

func TestPixabaySearchReturnsEmptyWhenKeyIsEmpty(t *testing.T) {
	client := &http.Client{}
	results := services.PixabaySearch(client, "", "nature", 5)
	if len(results) != 0 {
		t.Errorf("expected empty results, got %d", len(results))
	}
}

func TestPixabaySearchParsesResponseCorrectly(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		resp := map[string]interface{}{
			"hits": []map[string]interface{}{
				{
					"id":           12345,
					"duration":     15.5,
					"userImageURL": "thumb_url",
					"videos": map[string]interface{}{
						"large": map[string]interface{}{"url": "link_large", "width": 1920, "height": 1080},
					},
				},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer server.Close()

	client := server.Client()
	results, err := services.DoPixabaySearch(client, server.URL, "apikey", "nature", 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	r := results[0]
	if r.ID != "pixabay_12345" {
		t.Errorf("expected ID pixabay_12345, got %s", r.ID)
	}
	if r.Title != "Nature — Pixabay #12345" {
		t.Errorf("expected title 'Nature — Pixabay #12345', got '%s'", r.Title)
	}
	if r.Source != "pixabay" {
		t.Errorf("expected source pixabay, got %s", r.Source)
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
