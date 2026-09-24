package db

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"broll-rs/pkg/models"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
)

func InitDB(databaseURL string) (*gorm.DB, error) {
	dsn := databaseURL
	if strings.HasPrefix(dsn, "sqlite://") {
		dsn = strings.TrimPrefix(dsn, "sqlite://")
	} else if strings.HasPrefix(dsn, "sqlite:") {
		dsn = strings.TrimPrefix(dsn, "sqlite:")
	}

	if dsn == "memory:" || dsn == ":memory:" || strings.HasPrefix(dsn, ":memory:") {
		dsn = ":memory:"
	}

	if dsn != ":memory:" && !strings.Contains(dsn, "mode=memory") && !strings.HasPrefix(dsn, "file:") {
		filePath := dsn
		if idx := strings.Index(filePath, "?"); idx != -1 {
			filePath = filePath[:idx]
		}
		dir := filepath.Dir(filePath)
		if dir != "." && dir != "" {
			if err := os.MkdirAll(dir, 0755); err != nil {
				return nil, fmt.Errorf("failed to create db directory: %w", err)
			}
		}
		if _, err := os.Stat(filePath); os.IsNotExist(err) {
			file, err := os.Create(filePath)
			if err != nil {
				return nil, fmt.Errorf("failed to create db file: %w", err)
			}
			file.Close()
		}
	}

	dbDSN := dsn
	if dbDSN == "memory:" {
		dbDSN = ":memory:"
	}

	gormDB, err := gorm.Open(sqlite.Open(dbDSN), &gorm.Config{
		Logger: logger.Default.LogMode(logger.Silent),
	})
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	if err := gormDB.Exec("PRAGMA foreign_keys = ON;").Error; err != nil {
		return nil, fmt.Errorf("failed to enable foreign keys: %w", err)
	}

	if err := gormDB.AutoMigrate(&models.Project{}, &models.LibraryVideo{}); err != nil {
		return nil, fmt.Errorf("failed to auto migrate schema: %w", err)
	}

	if err := gormDB.Exec("CREATE INDEX IF NOT EXISTS idx_videos_project ON videos(project_id);").Error; err != nil {
		return nil, fmt.Errorf("failed to create index idx_videos_project: %w", err)
	}

	if err := gormDB.Exec("CREATE INDEX IF NOT EXISTS idx_videos_status ON videos(status);").Error; err != nil {
		return nil, fmt.Errorf("failed to create index idx_videos_status: %w", err)
	}

	return gormDB, nil
}
