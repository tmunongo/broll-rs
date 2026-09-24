package services

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/url"
	"strconv"
	"unicode"

	"broll-rs/pkg/models"
)

type PixabayResponse struct {
	Hits []PixabayHit `json:"hits"`
}

type PixabayHit struct {
	ID           uint64        `json:"id"`
	Duration     *float64      `json:"duration"`
	UserImageURL *string       `json:"userImageURL"`
	Videos       PixabayVideos `json:"videos"`
}

type PixabayVideos struct {
	Large  *PixabayFile `json:"large"`
	Medium *PixabayFile `json:"medium"`
	Small  *PixabayFile `json:"small"`
	Tiny   *PixabayFile `json:"tiny"`
}

type PixabayFile struct {
	URL    string  `json:"url"`
	Width  *uint32 `json:"width"`
	Height *uint32 `json:"height"`
}

func (v PixabayVideos) Best() *PixabayFile {
	if v.Large != nil {
		return v.Large
	}
	if v.Medium != nil {
		return v.Medium
	}
	if v.Small != nil {
		return v.Small
	}
	return v.Tiny
}

func Titlecase(s string) string {
	if s == "" {
		return ""
	}
	runes := []rune(s)
	runes[0] = unicode.ToUpper(runes[0])
	return string(runes)
}

func PixabaySearch(client *http.Client, apiKey, query string, perPage int) []models.VideoResult {
	if apiKey == "" {
		return []models.VideoResult{}
	}
	results, err := DoPixabaySearch(client, "https://pixabay.com/api/videos/", apiKey, query, perPage)
	if err != nil {
		log.Printf("Pixabay search failed: %v", err)
		return []models.VideoResult{}
	}
	return results
}

func DoPixabaySearch(client *http.Client, searchURL, apiKey, query string, perPage int) ([]models.VideoResult, error) {
	reqURL, err := url.Parse(searchURL)
	if err != nil {
		return nil, err
	}

	q := reqURL.Query()
	q.Set("key", apiKey)
	q.Set("q", query)
	q.Set("per_page", strconv.Itoa(perPage))
	q.Set("video_type", "film")
	reqURL.RawQuery = q.Encode()

	resp, err := client.Get(reqURL.String())
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	var pResp PixabayResponse
	if err := json.NewDecoder(resp.Body).Decode(&pResp); err != nil {
		return nil, err
	}

	results := make([]models.VideoResult, 0, len(pResp.Hits))
	for _, hit := range pResp.Hits {
		best := hit.Videos.Best()
		if best == nil {
			continue
		}

		title := fmt.Sprintf("%s — Pixabay #%d", Titlecase(query), hit.ID)
		license := "Pixabay License (free commercial use)"
		source := "pixabay"
		prevURL := best.URL
		dlURL := best.URL

		results = append(results, models.VideoResult{
			ID:          fmt.Sprintf("pixabay_%d", hit.ID),
			Title:       title,
			Source:      source,
			Duration:    hit.Duration,
			Thumbnail:   hit.UserImageURL,
			PreviewURL:  &prevURL,
			DownloadURL: &dlURL,
			License:     &license,
			Width:       best.Width,
			Height:      best.Height,
		})
	}

	return results, nil
}
