package db_test

import (
	"testing"

	"broll-rs/pkg/db"
	"broll-rs/pkg/models"
)

func TestInitPoolCreatesTables(t *testing.T) {
	gormDB, err := db.InitDB("sqlite::memory:")
	if err != nil {
		t.Fatalf("InitDB failed: %v", err)
	}

	p := models.Project{
		ID:        "p1",
		Name:      "Test",
		Slug:      "test",
		CreatedAt: "2024-01-01",
	}
	if err := gormDB.Create(&p).Error; err != nil {
		t.Fatalf("insert into projects failed: %v", err)
	}

	var count int64
	if err := gormDB.Model(&models.Project{}).Count(&count).Error; err != nil {
		t.Fatalf("query count failed: %v", err)
	}
	if count != 1 {
		t.Errorf("expected count 1, got %d", count)
	}
}

func TestInitPoolCreatesVideosTable(t *testing.T) {
	gormDB, err := db.InitDB("sqlite::memory:")
	if err != nil {
		t.Fatalf("InitDB failed: %v", err)
	}

	v := models.LibraryVideo{
		ID:        "v1",
		Title:     "Vid",
		Source:    "src",
		Status:    "pending",
		CreatedAt: "2024-01-01",
	}
	if err := gormDB.Create(&v).Error; err != nil {
		t.Fatalf("insert into videos failed: %v", err)
	}

	var count int64
	if err := gormDB.Model(&models.LibraryVideo{}).Count(&count).Error; err != nil {
		t.Fatalf("query count failed: %v", err)
	}
	if count != 1 {
		t.Errorf("expected count 1, got %d", count)
	}
}

func TestInitPoolIsIdempotent(t *testing.T) {
	dbURL := "file:memdb_idempotent?mode=memory&cache=shared"
	gormDB1, err := db.InitDB(dbURL)
	if err != nil {
		t.Fatalf("InitDB 1 failed: %v", err)
	}

	_, err = db.InitDB(dbURL)
	if err != nil {
		t.Fatalf("InitDB 2 failed: %v", err)
	}

	var count int64
	if err := gormDB1.Model(&models.Project{}).Count(&count).Error; err != nil {
		t.Fatalf("query count failed: %v", err)
	}
	if count != 0 {
		t.Errorf("expected count 0, got %d", count)
	}
}
