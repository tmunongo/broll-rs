package config

import (
	"os"
	"path/filepath"
	"strconv"

	"github.com/joho/godotenv"
)

type Config struct {
	PexelsAPIKey          string
	PixabayAPIKey         string
	DownloadsDir          string
	DatabaseURL           string
	Port                  int
	YouTubeCookiesBrowser *string
}

func FromEnv() Config {
	_ = godotenv.Load()

	port := 8000
	if pStr := os.Getenv("PORT"); pStr != "" {
		if p, err := strconv.Atoi(pStr); err == nil && p > 0 {
			port = p
		}
	}

	downloadsDir := os.Getenv("DOWNLOADS_DIR")
	if downloadsDir == "" {
		downloadsDir = "downloads"
	}
	downloadsDir = filepath.Clean(downloadsDir)

	dbURL := os.Getenv("DATABASE_URL")
	if dbURL == "" {
		dbURL = "sqlite://library.db"
	}

	var ytCookies *string
	if c := os.Getenv("YOUTUBE_COOKIES_BROWSER"); c != "" {
		ytCookies = &c
	}

	return Config{
		PexelsAPIKey:          os.Getenv("PEXELS_API_KEY"),
		PixabayAPIKey:         os.Getenv("PIXABAY_API_KEY"),
		DownloadsDir:          downloadsDir,
		DatabaseURL:           dbURL,
		Port:                  port,
		YouTubeCookiesBrowser: ytCookies,
	}
}
