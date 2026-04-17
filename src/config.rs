use crate::error::AppError;
use crate::ui::theme::ThemeVariant;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub server_url: String,
    pub api_key: String,
    pub vault_path: String,
    /// RFC3339 timestamp of last successful project refresh
    pub last_refresh: Option<String>,
    /// Whitelist: GUIDs of projects to include in sync. Empty = nothing syncs.
    #[serde(default)]
    pub included_projects: Vec<String>,
    /// Blacklist: per-GUID list of env var names to skip during sync.
    #[serde(default)]
    pub excluded_vars: HashMap<String, Vec<String>>,
    /// Color theme: "inherit" (default), "onedark", or "sky-orange"
    #[serde(default)]
    pub theme: ThemeVariant,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_without_exclusion_fields_gets_empty_defaults() {
        // Simulates loading a config written before excluded_vars/included_projects existed.
        let toml_str = r#"
server_url = "https://connect.example.com"
api_key = "abc123"
vault_path = "/home/user/.vault.json"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.included_projects.is_empty());
        assert!(config.excluded_vars.is_empty());
    }

    #[test]
    fn config_roundtrip_preserves_included_projects() {
        let mut config = Config::default();
        config.server_url = "https://connect.example.com".into();
        config.included_projects = vec!["guid-a".into(), "guid-b".into()];

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(restored.included_projects, config.included_projects);
    }

    #[test]
    fn config_roundtrip_preserves_excluded_vars() {
        let mut config = Config::default();
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["SECRET".into(), "TOKEN".into()]);

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();

        let excl = restored.excluded_vars.get("guid-a").unwrap();
        assert_eq!(excl.len(), 2);
        assert!(excl.contains(&"SECRET".to_string()));
        assert!(excl.contains(&"TOKEN".to_string()));
    }

    #[test]
    fn config_toml_with_exclusions_parsed_correctly() {
        let toml_str = r#"
server_url = "https://connect.example.com"
api_key = "key"
vault_path = ""
included_projects = ["guid-a", "guid-c"]

[excluded_vars]
guid-a = ["FOO", "BAR"]
guid-c = ["SKIP_ME"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.included_projects, vec!["guid-a", "guid-c"]);
        let a = config.excluded_vars.get("guid-a").unwrap();
        assert!(a.contains(&"FOO".to_string()));
        assert!(a.contains(&"BAR".to_string()));
        let c = config.excluded_vars.get("guid-c").unwrap();
        assert!(c.contains(&"SKIP_ME".to_string()));
    }

    #[test]
    fn config_with_empty_exclusion_lists_roundtrips_cleanly() {
        let config = Config::default(); // all fields empty/default
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();
        assert!(restored.included_projects.is_empty());
        assert!(restored.excluded_vars.is_empty());
    }

    #[test]
    fn excluded_vars_for_unknown_guid_does_not_error() {
        let toml_str = r#"
server_url = ""
api_key = ""
vault_path = ""

[excluded_vars]
nonexistent-guid = ["FOO"]
"#;
        // Should parse without error even if the GUID doesn't match any project
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.excluded_vars.contains_key("nonexistent-guid"));
    }
}
