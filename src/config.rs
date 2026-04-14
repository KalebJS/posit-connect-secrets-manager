use crate::error::AppError;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub server_url: String,
    pub api_key: String,
    pub vault_path: String,
    /// RFC3339 timestamp of last successful project refresh
    pub last_refresh: Option<String>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        let mut path = config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("posit-secrets");
        path.push("config.toml");
        path
    }

    pub fn load() -> Result<Self, AppError> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(&path).map_err(AppError::Io)?;
        toml::from_str(&contents).map_err(|e| AppError::Toml(e.to_string()))
    }

    pub fn save(&self) -> Result<(), AppError> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(AppError::Io)?;
        }
        let contents = toml::to_string_pretty(self).map_err(|e| AppError::Toml(e.to_string()))?;
        std::fs::write(&path, contents).map_err(AppError::Io)?;
        Ok(())
    }
}
