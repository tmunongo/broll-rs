# B-Roll Studio

A fast, local research workbench for finding, previewing, and archiving royalty-free video clips.  
Search Pexels · Pixabay · Internet Archive · YouTube from one interface.  
Organise downloads into **projects** — each project gets its own subdirectory.

---

## Quick Start (Using Make)

We provide a `Makefile` for streamlined local development and versioning.

```bash
# Prerequisites
# 1. Install Rust via rustup (https://rustup.rs/):
#    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
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
make check   # Runs clippy and rustfmt
make test    # Runs cargo test
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

### Releasing a new version via Makefile
When you're ready to deploy a new version to GHCR, use the release commands:
```bash
make version-patch   # Bumps v0.1.0 -> v0.1.1
make version-minor   # Bumps v0.1.0 -> v0.2.0
make version-major   # Bumps v0.1.0 -> v1.0.0
```
This automatically updates `Cargo.toml`, creates a git commit, and tags the release. Just run `git push && git push --tags` afterwards to trigger the GitHub Actions pipeline.

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
| `RUST_LOG`       | `broll_rs=info`      | Log level                |

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
Axum (thin control layer — single binary)
    ├── GET /api/search ──→  tokio::join! across:
    │                          reqwest → Pexels API
    │                          reqwest → Pixabay API
    │                          reqwest → Archive.org
    │                          tokio::process → yt-dlp --dump-json
    │
    ├── POST /api/download ──→  tokio::spawn (non-blocking)
    │                              └── yt-dlp subprocess  (YouTube)
    │                              └── reqwest stream     (others)
    │                              └── writes to downloads/{project_slug}/
    │
    ├── /api/library ──→  SQLite via sqlx
    └── /api/projects ──→  SQLite via sqlx
```

---

## Source Structure

```
src/
├── main.rs                 Router assembly + startup
├── config.rs               Env var config
├── db.rs                   SQLite pool + schema
├── error.rs                AppError + AppResult
├── models.rs               All types (VideoResult, Project, ...)
├── state.rs                AppState (pool, config, http client)
├── routes/
│   ├── search.rs           Fan-out search handler
│   ├── download.rs         Queue + status
│   ├── library.rs          List, delete, tag, serve file
│   └── projects.rs         CRUD
└── services/
    ├── pexels.rs
    ├── pixabay.rs
    ├── archive.rs
    ├── youtube.rs          yt-dlp subprocess adapter
    └── downloader.rs       Background download pipeline
templates/
└── index.html              Embedded at compile time (include_str!)
```
