package services

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"os/exec"
	"strings"

	"broll-rs/pkg/models"
)

type YtInfo struct {
	ID         string    `json:"id"`
	Title      *string   `json:"title"`
	Duration   *float64  `json:"duration"`
	Thumbnail  *string   `json:"thumbnail"`
	WebpageURL *string   `json:"webpage_url"`
	Thumbnails []YtThumb `json:"thumbnails"`
}

type YtThumb struct {
	URL string `json:"url"`
}

func YouTubeSearch(query string, count int, browserCookies *string) []models.VideoResult {
	results, err := DoYouTubeSearch(query, count, browserCookies)
	if err != nil {
		log.Printf("YouTube search failed: %v", err)
		return []models.VideoResult{}
	}
	return results
}

func DoYouTubeSearch(query string, count int, browserCookies *string) ([]models.VideoResult, error) {
	searchTerm := fmt.Sprintf("ytsearch%d:%s", count, query)
	args := []string{
		searchTerm,
		"--dump-json",
		"--no-playlist",
		"--skip-download",
	}

	if browserCookies != nil && *browserCookies != "" {
		args = append(args, "--cookies-from-browser", *browserCookies)
	}

	cmd := exec.Command("yt-dlp", args...)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("yt-dlp failed: %w, stderr: %s", err, stderr.String())
	}

	return ParseYtDlpOutput(stdout.String()), nil
}

func ParseYtDlpOutput(stdout string) []models.VideoResult {
	var results []models.VideoResult
	scanner := bufio.NewScanner(strings.NewReader(stdout))

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}

		var info YtInfo
		if err := json.Unmarshal([]byte(line), &info); err != nil || info.ID == "" {
			continue
		}

		var thumb *string
		if len(info.Thumbnails) > 0 {
			mid := len(info.Thumbnails) / 2
			t := info.Thumbnails[mid].URL
			thumb = &t
		} else if info.Thumbnail != nil {
			thumb = info.Thumbnail
		}

		url := fmt.Sprintf("https://www.youtube.com/watch?v=%s", info.ID)
		if info.WebpageURL != nil && *info.WebpageURL != "" {
			url = *info.WebpageURL
		}

		title := "Untitled"
		if info.Title != nil && *info.Title != "" {
			title = *info.Title
		}

		prevURL := fmt.Sprintf("https://www.youtube.com/embed/%s", info.ID)
		dlURL := url
		license := "YouTube (check individual video license)"
		source := "youtube"

		results = append(results, models.VideoResult{
			ID:          fmt.Sprintf("youtube_%s", info.ID),
			Title:       title,
			Source:      source,
			Duration:    info.Duration,
			Thumbnail:   thumb,
			PreviewURL:  &prevURL,
			DownloadURL: &dlURL,
			License:     &license,
		})
	}

	return results
}
