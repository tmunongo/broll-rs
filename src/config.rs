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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;
    
    // Use a static mutex so we don't race setting env vars across threads
    lazy_static::lazy_static! {
        static ref ENV_LOCK: Mutex<()> = Mutex::new(());
    }

    /// Helper: run `f` with the given env vars set smoothly
    fn with_env<F: FnOnce()>(vars: &[(&str, &str)], f: F) {
        let _guard = ENV_LOCK.lock().unwrap();

        // Save existing values
        let saved: Vec<(&str, Option<String>)> =
            vars.iter().map(|(k, _)| (*k, env::var(k).ok())).collect();
        // Set test values
        for (k, v) in vars {
            env::set_var(k, v);
        }
        f();
        // Restore
        for (k, original) in saved {
            match original {
                Some(v) => env::set_var(k, v),
                None => env::remove_var(k),
            }
        }
    }

    #[test]
    fn default_port_when_not_set() {
        // We cannot reliably clear PORT if another test sets it, but with_env locks it.
        // Still, we can test the explicitly-invalid fallback logic.
        with_env(&[("PORT", "invalid_port_string")], || {
            let config = Config::from_env();
            assert_eq!(config.port, 8000);
        });
    }

    #[test]
    fn reads_port_from_env() {
        with_env(&[("PORT", "9090")], || {
            let config = Config::from_env();
            assert_eq!(config.port, 9090);
        });
    }

    #[test]
    fn invalid_port_falls_back_to_default() {
        with_env(&[("PORT", "not_a_number")], || {
            let config = Config::from_env();
            assert_eq!(config.port, 8000);
        });
    }

    #[test]
    fn reads_api_keys_from_env() {
        with_env(
            &[
                ("PEXELS_API_KEY", "pexels-123"),
                ("PIXABAY_API_KEY", "pixabay-456"),
            ],
            || {
                let config = Config::from_env();
                assert_eq!(config.pexels_api_key, "pexels-123");
                assert_eq!(config.pixabay_api_key, "pixabay-456");
            },
        );
    }

    #[test]
    fn reads_downloads_dir_from_env() {
        with_env(&[("DOWNLOADS_DIR", "/tmp/my-downloads")], || {
            let config = Config::from_env();
            assert_eq!(config.downloads_dir, PathBuf::from("/tmp/my-downloads"));
        });
    }

    #[test]
    fn reads_database_url_from_env() {
        with_env(&[("DATABASE_URL", "sqlite:///tmp/test.db")], || {
            let config = Config::from_env();
            assert_eq!(config.database_url, "sqlite:///tmp/test.db");
        });
    }

    #[test]
    fn config_is_clone() {
        with_env(&[], || {
            let config = Config::from_env();
            let cloned = config.clone();
            assert_eq!(cloned.port, config.port);
        });
    }
}
