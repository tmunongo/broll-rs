package models_test

import (
	"encoding/json"
	"strings"
	"testing"

	"broll-rs/pkg/models"
)

func TestSlugifyBasic(t *testing.T) {
	if got := models.Slugify("Hello World"); got != "hello-world" {
		t.Errorf("expected 'hello-world', got '%s'", got)
	}
}

func TestSlugifySpecialChars(t *testing.T) {
	if got := models.Slugify("B-Roll: Nature & Wildlife!"); got != "b-roll-nature-wildlife" {
		t.Errorf("expected 'b-roll-nature-wildlife', got '%s'", got)
	}
}

func TestSlugifyLeadingTrailingWhitespace(t *testing.T) {
	if got := models.Slugify("  my project  "); got != "my-project" {
		t.Errorf("expected 'my-project', got '%s'", got)
	}
}

func TestSlugifyMultipleSpaces(t *testing.T) {
	if got := models.Slugify("a   b"); got != "a-b" {
		t.Errorf("expected 'a-b', got '%s'", got)
	}
}

func TestSlugifyAlreadySlug(t *testing.T) {
	if got := models.Slugify("already-slug"); got != "already-slug" {
		t.Errorf("expected 'already-slug', got '%s'", got)
	}
}

func TestSlugifyEmpty(t *testing.T) {
	if got := models.Slugify(""); got != "" {
		t.Errorf("expected '', got '%s'", got)
	}
}

func TestSlugifyOnlySpecialChars(t *testing.T) {
	if got := models.Slugify("!!!"); got != "" {
		t.Errorf("expected '', got '%s'", got)
	}
}

func TestSlugifyUnicode(t *testing.T) {
	got := models.Slugify("café")
	if got == "" {
		t.Errorf("expected non-empty slug for 'café', got '%s'", got)
	}
}

func TestVideoResultSerializes(t *testing.T) {
	duration := 30.5
	thumb := "https://example.com/thumb.jpg"
	preview := "https://example.com/preview.mp4"
	download := "https://example.com/video.mp4"
	license := "CC0"
	var width uint32 = 1920
	var height uint32 = 1080

	v := models.VideoResult{
		ID:          "test_1",
		Title:       "Test Video",
		Source:      "pexels",
		Duration:    &duration,
		Thumbnail:   &thumb,
		PreviewURL:  &preview,
		DownloadURL: &download,
		License:     &license,
		Width:       &width,
		Height:      &height,
	}

	data, err := json.Marshal(v)
	if err != nil {
		t.Fatalf("failed to marshal VideoResult: %v", err)
	}

	str := string(data)
	if !strings.Contains(str, `"id":"test_1"`) {
		t.Errorf("expected string to contain id, got %s", str)
	}
	if !strings.Contains(str, `"source":"pexels"`) {
		t.Errorf("expected string to contain source, got %s", str)
	}
	if !strings.Contains(str, `"duration":30.5`) {
		t.Errorf("expected string to contain duration, got %s", str)
	}
}

func TestVideoResultDeserializes(t *testing.T) {
	raw := `{
		"id": "archive_foo",
		"title": "Foo",
		"source": "archive",
		"duration": null,
		"thumbnail": null,
		"preview_url": null,
		"download_url": null,
		"license": null,
		"width": null,
		"height": null
	}`
	var v models.VideoResult
	if err := json.Unmarshal([]byte(raw), &v); err != nil {
		t.Fatalf("failed to unmarshal VideoResult: %v", err)
	}
	if v.ID != "archive_foo" || v.Source != "archive" || v.Duration != nil {
		t.Errorf("unexpected VideoResult content: %+v", v)
	}
}

func TestDownloadRequestDeserializes(t *testing.T) {
	raw := `{
		"id":"vid1","title":"A","source":"pexels",
		"download_url":"http://x.com/a.mp4","thumbnail":null,
		"duration":null,"project_id":null
	}`
	var req models.DownloadRequest
	if err := json.Unmarshal([]byte(raw), &req); err != nil {
		t.Fatalf("failed to unmarshal DownloadRequest: %v", err)
	}
	if req.ID != "vid1" || req.Source != "pexels" || req.ProjectID != nil {
		t.Errorf("unexpected DownloadRequest content: %+v", req)
	}
}

func TestTagUpdateDeserializes(t *testing.T) {
	raw := `{"tags":"nature,wildlife"}`
	var tag models.TagUpdate
	if err := json.Unmarshal([]byte(raw), &tag); err != nil {
		t.Fatalf("failed to unmarshal TagUpdate: %v", err)
	}
	if tag.Tags != "nature,wildlife" {
		t.Errorf("expected 'nature,wildlife', got '%s'", tag.Tags)
	}
}

func TestSearchParamsDeserializes(t *testing.T) {
	raw := `{"q":"nature","sources":"pexels,pixabay"}`
	var params models.SearchParams
	if err := json.Unmarshal([]byte(raw), &params); err != nil {
		t.Fatalf("failed to unmarshal SearchParams: %v", err)
	}
	if params.Q != "nature" || params.Sources == nil || *params.Sources != "pexels,pixabay" {
		t.Errorf("unexpected SearchParams content: %+v", params)
	}
}

func TestLibraryParamsOptionalProject(t *testing.T) {
	raw := `{}`
	var params models.LibraryParams
	if err := json.Unmarshal([]byte(raw), &params); err != nil {
		t.Fatalf("failed to unmarshal LibraryParams: %v", err)
	}
	if params.ProjectID != nil {
		t.Errorf("expected ProjectID nil, got %v", params.ProjectID)
	}
}

func TestStatusResponseSerializes(t *testing.T) {
	fp := "/downloads/id1.mp4"
	s := models.StatusResponse{
		ID:       "id1",
		Status:   "complete",
		Filepath: &fp,
	}
	data, err := json.Marshal(s)
	if err != nil {
		t.Fatalf("failed to marshal StatusResponse: %v", err)
	}
	str := string(data)
	if !strings.Contains(str, `"status":"complete"`) {
		t.Errorf("expected status complete in JSON, got %s", str)
	}
}
