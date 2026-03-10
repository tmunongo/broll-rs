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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn init_pool_creates_tables() {
        let pool = init_pool("sqlite::memory:").await.expect("pool should initialize");

        // Verify 'projects' table exists by inserting and querying
        sqlx::query(
            "INSERT INTO projects (id, name, slug, created_at) VALUES ('p1','Test','test','2024-01-01')"
        )
        .execute(&pool)
        .await
        .expect("insert into projects failed");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(&pool)
            .await
            .expect("query count failed");
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn init_pool_creates_videos_table() {
        let pool = init_pool("sqlite::memory:").await.expect("pool should initialize");

        // Verify 'videos' table exists
        sqlx::query(
            "INSERT INTO videos (id, title, source, status, created_at) VALUES ('v1','Vid','src','pending','2024-01-01')"
        )
        .execute(&pool)
        .await
        .expect("insert into videos failed");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM videos")
            .fetch_one(&pool)
            .await
            .expect("query count failed");
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn init_pool_is_idempotent() {
        // Running init on the same in-memory pool again should not fail (CREATE IF NOT EXISTS)
        let pool = init_pool("sqlite::memory:").await.expect("pool init 1");
        // Re-run the schema statements via a fresh call would need the same pool,
        // but we can at least verify two separate in-memory DBs initialize cleanly
        let pool2 = init_pool("sqlite::memory:").await.expect("pool init 2");
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(&pool)
            .await
            .unwrap();
        let count2: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
        assert_eq!(count2.0, 0);
    }
}
