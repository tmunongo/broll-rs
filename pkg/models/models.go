package models

import (
	"strings"
	"unicode"
)

type VideoResult struct {
	ID          string   `json:"id"`
	Title       string   `json:"title"`
	Source      string   `json:"source"`
	Duration    *float64 `json:"duration"`
	Thumbnail   *string  `json:"thumbnail"`
	PreviewURL  *string  `json:"preview_url"`
	DownloadURL *string  `json:"download_url"`
	License     *string  `json:"license"`
	Width       *uint32  `json:"width"`
	Height      *uint32  `json:"height"`
}

type DownloadRequest struct {
	ID          string   `json:"id"`
	Title       string   `json:"title"`
	Source      string   `json:"source"`
	DownloadURL string   `json:"download_url"`
	Thumbnail   *string  `json:"thumbnail"`
	Duration    *float64 `json:"duration"`
	ProjectID   *string  `json:"project_id"`
}

type LibraryVideo struct {
	ID          string   `gorm:"primaryKey" json:"id"`
	Title       string   `gorm:"not null" json:"title"`
	Source      string   `gorm:"not null" json:"source"`
	Filepath    *string  `gorm:"column:filepath" json:"filepath"`
	Duration    *float64 `gorm:"column:duration" json:"duration"`
	Tags        *string  `gorm:"column:tags;default:''" json:"tags"`
	Thumbnail   *string  `gorm:"column:thumbnail" json:"thumbnail"`
	OriginalURL *string  `gorm:"column:original_url" json:"original_url"`
	Status      string   `gorm:"not null;default:'pending'" json:"status"`
	ProjectID   *string  `gorm:"column:project_id" json:"project_id"`
	ProjectName *string  `gorm:"->;-:migration;column:project_name" json:"project_name"`
	CreatedAt   string   `gorm:"not null" json:"created_at"`
}

func (LibraryVideo) TableName() string {
	return "videos"
}

type Project struct {
	ID        string `gorm:"primaryKey" json:"id"`
	Name      string `gorm:"not null;unique" json:"name"`
	Slug      string `gorm:"not null;unique" json:"slug"`
	CreatedAt string `gorm:"not null" json:"created_at"`
}

func (Project) TableName() string {
	return "projects"
}

type CreateProjectRequest struct {
	Name string `json:"name"`
}

type SearchParams struct {
	Q       string  `json:"q"`
	Sources *string `json:"sources"`
}

type LibraryParams struct {
	ProjectID *string `json:"project_id"`
}

type TagUpdate struct {
	Tags string `json:"tags"`
}

type StatusResponse struct {
	ID       string  `json:"id"`
	Status   string  `json:"status"`
	Filepath *string `json:"filepath"`
}

func Slugify(s string) string {
	s = strings.TrimSpace(strings.ToLower(s))
	var sb strings.Builder
	for _, r := range s {
		if unicode.IsLetter(r) || unicode.IsNumber(r) || r == '-' {
			sb.WriteRune(r)
		} else {
			sb.WriteRune('-')
		}
	}
	parts := strings.Split(sb.String(), "-")
	var filtered []string
	for _, p := range parts {
		if p != "" {
			filtered = append(filtered, p)
		}
	}
	return strings.Join(filtered, "-")
}
