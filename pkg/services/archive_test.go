package services_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"broll-rs/pkg/services"
)

func TestParseRuntimeHMS(t *testing.T) {
	got := services.ParseRuntime("1:02:03")
	expected := 1.0*3600.0 + 2.0*60.0 + 3.0
	if got == nil || *got != expected {
		t.Errorf("expected %f, got %v", expected, got)
	}
}

func TestParseRuntimeMS(t *testing.T) {
	got := services.ParseRuntime("5:30")
	expected := 5.0*60.0 + 30.0
	if got == nil || *got != expected {
		t.Errorf("expected %f, got %v", expected, got)
	}
}

func TestParseRuntimeSingleSegmentIsNil(t *testing.T) {
	if got := services.ParseRuntime("120"); got != nil {
		t.Errorf("expected nil, got %v", got)
	}
}

func TestParseRuntimeInvalidIsNil(t *testing.T) {
	if got := services.ParseRuntime("abc:def"); got != nil {
		t.Errorf("expected nil, got %v", got)
	}
}

func TestParseRuntimeEmptyIsNil(t *testing.T) {
	if got := services.ParseRuntime(""); got != nil {
		t.Errorf("expected nil, got %v", got)
	}
}

func TestParseRuntimeZero(t *testing.T) {
	got := services.ParseRuntime("0:00")
	if got == nil || *got != 0.0 {
		t.Errorf("expected 0.0, got %v", got)
	}
}

func TestArchiveSearchReturnsEmptyOnAPIError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer server.Close()

	client := server.Client()
	_, err := services.DoArchiveSearch(client, server.URL, "test_query", 1)
	if err == nil {
		t.Error("expected error on HTTP 500, got nil")
	}
}

func TestDoArchiveSearchParsesResponse(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		resp := map[string]interface{}{
			"response": map[string]interface{}{
				"docs": []map[string]interface{}{
					{
						"identifier": "test-vid-1",
						"title":      "Test Video 1",
						"runtime":    "1:30",
					},
					{
						"identifier": "test-vid-2",
						"title":      []string{"Test Video 2 array"},
						"runtime":    "0:30.5",
					},
				},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer server.Close()

	client := server.Client()
	results, err := services.DoArchiveSearch(client, server.URL, "test", 10)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(results))
	}

	r1 := results[0]
	if r1.Title != "Test Video 1" || r1.Duration == nil || *r1.Duration != 90.0 {
		t.Errorf("unexpected content for result 0: %+v", r1)
	}

	r2 := results[1]
	if r2.Title != "Test Video 2 array" || r2.Duration == nil || *r2.Duration != 30.5 {
		t.Errorf("unexpected content for result 1: %+v", r2)
	}
}

func TestArchiveSearchTimeoutReturnsEmpty(t *testing.T) {
	client := &http.Client{
		Timeout: 1 * time.Millisecond,
	}
	results := services.ArchiveSearch(client, "rust programming", 2)
	if len(results) != 0 {
		t.Errorf("expected empty results on timeout, got %d", len(results))
	}
}
