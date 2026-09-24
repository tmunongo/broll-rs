package services

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/url"
	"sort"
	"strconv"

	"broll-rs/pkg/models"
)

type PexelsResponse struct {
	Videos []PexelsVideo `json:"videos"`
}

type PexelsVideo struct {
	ID            uint64          `json:"id"`
	Duration      *float64        `json:"duration"`
	User          *PexelsUser     `json:"user"`
	VideoFiles    []PexelsFile    `json:"video_files"`
	VideoPictures []PexelsPicture `json:"video_pictures"`
}

type PexelsUser struct {
	Name string `json:"name"`
}

type PexelsFile struct {
	Link   string  `json:"link"`
	Width  *uint32 `json:"width"`
	Height *uint32 `json:"height"`
}

type PexelsPicture struct {
	Picture string `json:"picture"`
}

func PexelsSearch(client *http.Client, apiKey, query string, perPage int) []models.VideoResult {
	if apiKey == "" {
		return []models.VideoResult{}
	}
	results, err := DoPexelsSearch(client, "https://api.pexels.com/videos/search", apiKey, query, perPage)
	if err != nil {
		log.Printf("Pexels search failed: %v", err)
		return []models.VideoResult{}
	}
	return results
}

func DoPexelsSearch(client *http.Client, searchURL, apiKey, query string, perPage int) ([]models.VideoResult, error) {
	reqURL, err := url.Parse(searchURL)
	if err != nil {
		return nil, err
	}

	q := reqURL.Query()
	q.Set("query", query)
	q.Set("per_page", strconv.Itoa(perPage))
	q.Set("size", "medium")
	reqURL.RawQuery = q.Encode()

	req, err := http.NewRequest("GET", reqURL.String(), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", apiKey)

	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	var pResp PexelsResponse
	if err := json.NewDecoder(resp.Body).Decode(&pResp); err != nil {
		return nil, err
	}

	results := make([]models.VideoResult, 0, len(pResp.Videos))
	for _, v := range pResp.Videos {
		files := make([]PexelsFile, len(v.VideoFiles))
		copy(files, v.VideoFiles)

		sort.Slice(files, func(i, j int) bool {
			wI, wJ := uint32(0), uint32(0)
			if files[i].Width != nil {
				wI = *files[i].Width
			}
			if files[j].Width != nil {
				wJ = *files[j].Width
			}
			return wI > wJ
		})

		var best PexelsFile
		found := false
		for _, f := range files {
			w := uint32(0)
			if f.Width != nil {
				w = *f.Width
			}
			if w <= 1920 {
				best = f
				found = true
				break
			}
		}
		if !found {
			best = PexelsFile{Link: ""}
		}

		var thumb *string
		if len(v.VideoPictures) > 0 {
			t := v.VideoPictures[0].Picture
			thumb = &t
		}

		author := ""
		if v.User != nil {
			author = v.User.Name
		}

		title := fmt.Sprintf("%s — %s", author, query)
		source := "pexels"
		cc0 := "CC0"

		var prevURL, dlURL *string
		if best.Link != "" {
			l := best.Link
			prevURL = &l
			dlURL = &l
		}

		results = append(results, models.VideoResult{
			ID:          fmt.Sprintf("pexels_%d", v.ID),
			Title:       title,
			Source:      source,
			Duration:    v.Duration,
			Thumbnail:   thumb,
			PreviewURL:  prevURL,
			DownloadURL: dlURL,
			License:     &cc0,
			Width:       best.Width,
			Height:      best.Height,
		})
	}

	return results, nil
}
