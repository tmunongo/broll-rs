package main

import (
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"broll-rs/pkg/config"
	"broll-rs/pkg/db"
	"broll-rs/pkg/routes"
)

type customTransport struct {
	transport http.RoundTripper
}

func (c *customTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	req.Header.Set("User-Agent", "broll-harness/1.0")
	return c.transport.RoundTrip(req)
}

func main() {
	cfg := config.FromEnv()
	log.Printf("Downloads dir: %s", cfg.DownloadsDir)

	if err := os.MkdirAll(cfg.DownloadsDir, 0755); err != nil {
		log.Fatalf("Failed to create downloads directory: %v", err)
	}

	gormDB, err := db.InitDB(cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("Failed to initialize database: %v", err)
	}
	log.Printf("Database ready: %s", cfg.DatabaseURL)

	httpClient := &http.Client{
		Timeout: 10 * time.Minute,
		Transport: &customTransport{
			transport: http.DefaultTransport,
		},
	}

	app := &routes.App{
		DB:     gormDB,
		Config: cfg,
		HTTP:   httpClient,
	}

	router := routes.NewRouter(app)

	addr := fmt.Sprintf("0.0.0.0:%d", cfg.Port)
	log.Printf("Listening on http://%s", addr)

	if err := http.ListenAndServe(addr, router); err != nil {
		log.Fatalf("HTTP server failed: %v", err)
	}
}
