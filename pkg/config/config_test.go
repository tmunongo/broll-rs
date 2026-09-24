package config_test

import (
	"os"
	"path/filepath"
	"sync"
	"testing"

	"broll-rs/pkg/config"
)

var envLock sync.Mutex

func withEnv(vars map[string]string, f func()) {
	envLock.Lock()
	defer envLock.Unlock()

	saved := make(map[string]*string)
	for k := range vars {
		if val, ok := os.LookupEnv(k); ok {
			saved[k] = &val
		} else {
			saved[k] = nil
		}
	}

	for k, v := range vars {
		os.Setenv(k, v)
	}

	defer func() {
		for k, orig := range saved {
			if orig != nil {
				os.Setenv(k, *orig)
			} else {
				os.Unsetenv(k)
			}
		}
	}()

	f()
}

func TestDefaultPortWhenNotSet(t *testing.T) {
	withEnv(map[string]string{"PORT": "invalid_port_string"}, func() {
		cfg := config.FromEnv()
		if cfg.Port != 8000 {
			t.Errorf("expected port 8000, got %d", cfg.Port)
		}
	})
}

func TestReadsPortFromEnv(t *testing.T) {
	withEnv(map[string]string{"PORT": "9090"}, func() {
		cfg := config.FromEnv()
		if cfg.Port != 9090 {
			t.Errorf("expected port 9090, got %d", cfg.Port)
		}
	})
}

func TestInvalidPortFallsBackToDefault(t *testing.T) {
	withEnv(map[string]string{"PORT": "not_a_number"}, func() {
		cfg := config.FromEnv()
		if cfg.Port != 8000 {
			t.Errorf("expected port 8000, got %d", cfg.Port)
		}
	})
}

func TestReadsAPIKeysFromEnv(t *testing.T) {
	withEnv(map[string]string{
		"PEXELS_API_KEY":  "pexels-123",
		"PIXABAY_API_KEY": "pixabay-456",
	}, func() {
		cfg := config.FromEnv()
		if cfg.PexelsAPIKey != "pexels-123" {
			t.Errorf("expected Pexels key pexels-123, got %s", cfg.PexelsAPIKey)
		}
		if cfg.PixabayAPIKey != "pixabay-456" {
			t.Errorf("expected Pixabay key pixabay-456, got %s", cfg.PixabayAPIKey)
		}
	})
}

func TestReadsDownloadsDirFromEnv(t *testing.T) {
	withEnv(map[string]string{"DOWNLOADS_DIR": "/tmp/my-downloads"}, func() {
		cfg := config.FromEnv()
		expected := filepath.Clean("/tmp/my-downloads")
		if cfg.DownloadsDir != expected {
			t.Errorf("expected downloads dir %s, got %s", expected, cfg.DownloadsDir)
		}
	})
}

func TestReadsDatabaseURLFromEnv(t *testing.T) {
	withEnv(map[string]string{"DATABASE_URL": "sqlite:///tmp/test.db"}, func() {
		cfg := config.FromEnv()
		if cfg.DatabaseURL != "sqlite:///tmp/test.db" {
			t.Errorf("expected DB url sqlite:///tmp/test.db, got %s", cfg.DatabaseURL)
		}
	})
}

func TestConfigCopy(t *testing.T) {
	withEnv(map[string]string{}, func() {
		cfg := config.FromEnv()
		cloned := cfg
		if cloned.Port != cfg.Port {
			t.Errorf("expected port %d, got %d", cfg.Port, cloned.Port)
		}
	})
}
