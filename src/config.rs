use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub pexels_api_key: String,
    pub pixabay_api_key: String,
    pub downloads_dir: PathBuf,
    pub database_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        Self {
            pexels_api_key: std::env::var("PEXELS_API_KEY").unwrap_or_default(),
            pixabay_api_key: std::env::var("PIXABAY_API_KEY").unwrap_or_default(),
            downloads_dir: std::env::var("DOWNLOADS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("downloads")),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://library.db".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8000),
        }
    }
}
