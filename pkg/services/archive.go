package services

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/url"
	"strconv"
	"strings"

	"broll-rs/pkg/models"
)

const (
	ArchiveSearchURL = "https://archive.org/advancedsearch.php"
	ArchiveBaseURL   = "https://archive.org"
)

type ArchiveResponse struct {
	Response ArchiveInner `json:"response"`
}

type ArchiveInner struct {
	Docs []ArchiveDoc `json:"docs"`
}

type ArchiveDoc struct {
	Identifier string          `json:"identifier"`
	Title      json.RawMessage `json:"title"`
	Runtime    json.RawMessage `json:"runtime"`
}

func ArchiveSearch(client *http.Client, query string, rows int) []models.VideoResult {
	results, err := DoArchiveSearch(client, ArchiveSearchURL, query, rows)
	if err != nil {
		log.Printf("Archive search failed: %v", err)
		return []models.VideoResult{}
	}
	return results
}

func DoArchiveSearch(client *http.Client, searchURL, query string, rows int) ([]models.VideoResult, error) {
	reqURL, err := url.Parse(searchURL)
	if err != nil {
		return nil, err
	}

	q := reqURL.Query()
	q.Set("q", fmt.Sprintf("%s AND mediatype:movies", query))
	q.Add("fl[]", "identifier")
	q.Add("fl[]", "title")
	q.Add("fl[]", "runtime")
	q.Set("rows", strconv.Itoa(rows))
	q.Set("output", "json")
	q.Add("sort[]", "downloads desc")
	reqURL.RawQuery = q.Encode()

	resp, err := client.Get(reqURL.String())
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	var aResp ArchiveResponse
	if err := json.NewDecoder(resp.Body).Decode(&aResp); err != nil {
		return nil, err
	}

	results := make([]models.VideoResult, 0, len(aResp.Response.Docs))
	for _, doc := range aResp.Response.Docs {
		id := doc.Identifier
		title := parseArchiveTitle(doc.Title, id)
		duration := parseArchiveRuntime(doc.Runtime)

		thumb := fmt.Sprintf("%s/services/img/%s", ArchiveBaseURL, id)
		prev := fmt.Sprintf("%s/embed/%s", ArchiveBaseURL, id)
		dl := fmt.Sprintf("%s/download/%s", ArchiveBaseURL, id)
		license := "Public Domain / Open License"
		source := "archive"

		results = append(results, models.VideoResult{
			ID:          fmt.Sprintf("archive_%s", id),
			Title:       title,
			Source:      source,
			Duration:    duration,
			Thumbnail:   &thumb,
			PreviewURL:  &prev,
			DownloadURL: &dl,
			License:     &license,
		})
	}

	return results, nil
}

func parseArchiveTitle(raw json.RawMessage, defaultID string) string {
	if len(raw) == 0 {
		return defaultID
	}
	var s string
	if err := json.Unmarshal(raw, &s); err == nil && s != "" {
		return s
	}
	var arr []string
	if err := json.Unmarshal(raw, &arr); err == nil && len(arr) > 0 && arr[0] != "" {
		return arr[0]
	}
	return defaultID
}

func parseArchiveRuntime(raw json.RawMessage) *float64 {
	if len(raw) == 0 {
		return nil
	}
	var s string
	if err := json.Unmarshal(raw, &s); err != nil || s == "" {
		return nil
	}
	return ParseRuntime(s)
}

func ParseRuntime(s string) *float64 {
	parts := strings.Split(s, ":")
	switch len(parts) {
	case 3:
		h, err1 := strconv.ParseFloat(parts[0], 64)
		m, err2 := strconv.ParseFloat(parts[1], 64)
		sec, err3 := strconv.ParseFloat(parts[2], 64)
		if err1 != nil || err2 != nil || err3 != nil {
			return nil
		}
		val := h*3600.0 + m*60.0 + sec
		return &val
	case 2:
		m, err1 := strconv.ParseFloat(parts[0], 64)
		sec, err2 := strconv.ParseFloat(parts[1], 64)
		if err1 != nil || err2 != nil {
			return nil
		}
		val := m*60.0 + sec
		return &val
	default:
		return nil
	}
}
