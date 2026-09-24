# B-Roll Studio

A fast, local research workbench for finding, previewing, and archiving royalty-free video clips.  
Search Pexels · Pixabay · Internet Archive · YouTube from one interface.  
Organise downloads into **projects** — each project gets its own subdirectory.

---

## Quick Start (Using Make)

We provide a `Makefile` for streamlined local development.

```bash
# Prerequisites
# 1. Install Go 1.22+ (https://go.dev/dl/)
# 2. Install ffmpeg & python3 via your OS package manager:
#    macOS: brew install ffmpeg python3
#    Debian/Ubuntu: apt install ffmpeg python3
#    Windows: winget install ffmpeg python3

# Clone & enter directory
git clone https://github.com/tmunongo/broll-rs.git
cd broll-rs

# Setup API keys (Pexels, Pixabay)
cp .env.example .env # Or create a .env file and populate it

# Build & run locally
make dev
```

Open http://localhost:8000

---

## Setup & Code Quality Checks

To ensure your code meets the quality standards before committing, you can install the pre-commit git hooks:
```bash
make install-hooks
```
You can also manually run the tests and checks via:
```bash
make check   # Runs go vet & gofmt checks
make test    # Runs go test
make precommit # Runs both
```

---

## Docker & Raspberry Pi (Multi-Arch)

The included `Dockerfile` is optimized to run natively on both `linux/amd64` (Standard PCs) and `linux/arm64` (Raspberry Pi 4/5, Apple Silicon). 
A GitHub Actions workflow is included (`.github/workflows/docker.yml`) that automatically builds and pushes these multi-arch images to `ghcr.io` whenever a new semantic version tag is pushed (e.g., `v1.0.0`).

### Running via Docker Compose
```bash
docker-compose up --build
```

---

## Projects

Projects are named collections. When you click **Save** on any clip, a modal  
asks which project to assign it to (or none). The file lands in:

```
downloads/                     ← no project
downloads/my-documentary/      ← project "My Documentary"
downloads/ocean-b-roll/        ← project "Ocean B-Roll"
```

Slugs are auto-generated from the project name (lowercase, hyphenated).  
The left sidebar lets you filter the library by project.

---

## Environment Variables

| Variable         | Default              | Description              |
|------------------|----------------------|--------------------------|
| `PEXELS_API_KEY` | _(empty)_            | Pexels API key           |
| `PIXABAY_API_KEY`| _(empty)_            | Pixabay API key          |
| `DATABASE_URL`   | `sqlite://library.db`| SQLite path              |
| `DOWNLOADS_DIR`  | `./downloads`        | Root downloads directory |
| `PORT`           | `8000`               | Listening port           |

API keys from pexels.com/api and pixabay.com/api/docs (both free, unlimited).

---

## API

```
GET  /api/search?q=...&sources=pexels,pixabay,archive,youtube
POST /api/download                  { id, title, source, download_url, project_id? }
GET  /api/download/status/:id
GET  /api/library?project_id=...
DELETE /api/library/:id
PATCH  /api/library/:id/tags        { tags: "..." }
GET  /api/library/file/:id
GET  /api/projects
POST /api/projects                  { name: "..." }
DELETE /api/projects/:id
```

---

## Architecture

```
Browser (Alpine.js)
    │
    ▼
Go Server (chi + GORM — single binary)
    ├── GET /api/search ──→  Parallel goroutines across:
    │                          net/http → Pexels API
    │                          net/http → Pixabay API
    │                          net/http → Archive.org
    │                          exec.Command → yt-dlp --dump-json
    │
    ├── POST /api/download ──→  goroutine (non-blocking)
    │                              └── yt-dlp subprocess  (YouTube)
    │                              └── net/http stream    (others)
    │                              └── writes to downloads/{project_slug}/
    │
    ├── /api/library ──→  SQLite via GORM
    └── /api/projects ──→  SQLite via GORM
```

---

## Source Structure

```
├── main.go                 Server entry point
├── pkg/
│   ├── config/             Env var configuration
│   ├── db/                 GORM SQLite initialization & migrations
│   ├── models/             Data models & slugify utility
│   ├── routes/             HTTP router (Chi) & API endpoints
│   └── services/           Pexels, Pixabay, Archive, YouTube, Downloader
templates/
└── index.html              Embedded at compile time (go:embed)
```
