# B-Roll Harness (Rust/Axum)

A fast, local research workbench for finding, previewing, and archiving royalty-free video clips.  
Search Pexels · Pixabay · Internet Archive · YouTube from one interface.  
Organise downloads into **projects** — each project gets its own subdirectory.

---

## Quick Start

```bash
# Prerequisites
brew install rust ffmpeg     # macOS
pip install yt-dlp

# Build & run
export PEXELS_API_KEY=your_key
export PIXABAY_API_KEY=your_key
cargo run --release
```

Open http://localhost:8000

---

## Docker

```bash
PEXELS_API_KEY=xxx PIXABAY_API_KEY=yyy docker-compose up --build
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
