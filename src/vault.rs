use crate::error::AppError;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

pub struct Vault {
    pub path: PathBuf,
    pub entries: IndexMap<String, String>,
    pub dirty: bool,
}

impl Vault {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        let entries = if path.exists() {
            let contents = std::fs::read_to_string(&path).map_err(AppError::Io)?;
            if contents.trim().is_empty() {
                IndexMap::new()
            } else {
                serde_json::from_str(&contents)?
            }
        } else {
            IndexMap::new()
        };
        let mut vault = Self {
            path,
            entries,
            dirty: false,
        };
        vault.sort();
        Ok(vault)
    }

    pub fn load_empty() -> Self {
        Self {
            path: PathBuf::new(),
            entries: IndexMap::new(),
            dirty: false,
        }
    }

    pub fn save(&mut self) -> Result<(), AppError> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
        }
        let contents = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.path, contents).map_err(AppError::Io)?;
        self.dirty = false;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    pub fn remove_at(&mut self, idx: usize) {
        if let Some(key) = self.entries.keys().nth(idx).cloned() {
            self.entries.shift_remove(&key);
            self.dirty = true;
        }
    }

    pub fn sort(&mut self) {
        let mut pairs: Vec<(String, String)> = self.entries.drain(..).collect();
        pairs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        self.entries.extend(pairs);
    }
}
