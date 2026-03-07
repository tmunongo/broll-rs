use sqlx::SqlitePool;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    slug       TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS videos (
    id           TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    source       TEXT NOT NULL,
    filepath     TEXT,
    duration     REAL,
    tags         TEXT DEFAULT '',
    thumbnail    TEXT,
    original_url TEXT,
    status       TEXT NOT NULL DEFAULT 'pending',
    project_id   TEXT REFERENCES projects(id) ON DELETE SET NULL,
    created_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_videos_project ON videos(project_id);
CREATE INDEX IF NOT EXISTS idx_videos_status  ON videos(status);
"#;

pub async fn init_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    // Create the file if it doesn't exist
    let path = database_url.trim_start_matches("sqlite://");
    if path != ":memory:" && !std::path::Path::new(path).exists() {
        std::fs::File::create(path)?;
    }

    let pool = SqlitePool::connect(database_url).await?;

    // Run each statement separately (sqlx doesn't support multi-statement in one call)
    for stmt in SCHEMA.split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(&pool).await?;
        }
    }

    Ok(pool)
}
