package services_test

import (
	"testing"

	"broll-rs/pkg/services"
)

func TestParseYtDlpOutputSuccess(t *testing.T) {
	jsonStr := `{"id":"123","title":"Test Video","duration":120.5,"thumbnail":"http://thumb","webpage_url":"http://watch"}`
	results := services.ParseYtDlpOutput(jsonStr)
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	r := results[0]
	if r.ID != "youtube_123" {
		t.Errorf("expected ID youtube_123, got %s", r.ID)
	}
	if r.Title != "Test Video" {
		t.Errorf("expected title 'Test Video', got '%s'", r.Title)
	}
	if r.Duration == nil || *r.Duration != 120.5 {
		t.Errorf("expected duration 120.5, got %v", r.Duration)
	}
	if r.Thumbnail == nil || *r.Thumbnail != "http://thumb" {
		t.Errorf("expected thumbnail http://thumb, got %v", r.Thumbnail)
	}
	if r.DownloadURL == nil || *r.DownloadURL != "http://watch" {
		t.Errorf("expected download_url http://watch, got %v", r.DownloadURL)
	}
}

func TestParseYtDlpOutputMissingOptionalFields(t *testing.T) {
	jsonStr := `{"id":"abc"}`
	results := services.ParseYtDlpOutput(jsonStr)
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	r := results[0]
	if r.ID != "youtube_abc" {
		t.Errorf("expected ID youtube_abc, got %s", r.ID)
	}
	if r.Title != "Untitled" {
		t.Errorf("expected title 'Untitled', got '%s'", r.Title)
	}
	if r.Duration != nil {
		t.Errorf("expected nil duration, got %v", r.Duration)
	}
	if r.Thumbnail != nil {
		t.Errorf("expected nil thumbnail, got %v", r.Thumbnail)
	}
	expectedURL := "https://www.youtube.com/watch?v=abc"
	if r.DownloadURL == nil || *r.DownloadURL != expectedURL {
		t.Errorf("expected download_url %s, got %v", expectedURL, r.DownloadURL)
	}
}

func TestParseYtDlpOutputWithThumbnailsArray(t *testing.T) {
	jsonStr := `{"id":"xyz","thumbnails":[{"url":"t1"},{"url":"t2"},{"url":"t3"}]}`
	results := services.ParseYtDlpOutput(jsonStr)
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	// len=3, mid=1 => t2
	r := results[0]
	if r.Thumbnail == nil || *r.Thumbnail != "t2" {
		t.Errorf("expected thumbnail t2, got %v", r.Thumbnail)
	}
}

func TestParseYtDlpOutputIgnoresInvalidJSON(t *testing.T) {
	jsonStr := "invalid\n{\"id\":\"ok\"}\nnot_json"
	results := services.ParseYtDlpOutput(jsonStr)
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	if results[0].ID != "youtube_ok" {
		t.Errorf("expected ID youtube_ok, got %s", results[0].ID)
	}
}

func TestYouTubeSearchHandlesCommandFailureGracefully(t *testing.T) {
	// Call YouTubeSearch with dummy query; even if command fails, returns slice gracefully
	res := services.YouTubeSearch("test", 1, nil)
	_ = res
}
