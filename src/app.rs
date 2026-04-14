use std::collections::HashSet;
use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::api::client::ConnectClient;
use crate::api::types::{ContentItem, EnvVar};
use crate::config::Config;
use crate::error::AppError;
use crate::vault::Vault;

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    ProjectList,
    EnvVarList,
    Vault,
    Settings,
}

impl Page {
    pub fn index(&self) -> usize {
        match self {
            Page::ProjectList => 0,
            Page::EnvVarList => 1,
            Page::Vault => 2,
            Page::Settings => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Page::ProjectList,
            1 => Page::EnvVarList,
            2 => Page::Vault,
            3 => Page::Settings,
            _ => Page::ProjectList,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Page::ProjectList => "Projects",
            Page::EnvVarList => "Env Vars",
            Page::Vault => "Vault",
            Page::Settings => "Settings",
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum LoadState {
    Idle,
    Loading,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VaultField {
    Key,
    Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusLevel {
    Info,
    Success,
    Error,
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ProjectEntry {
    pub guid: String,
    pub name: String,
    pub title: Option<String>,
    pub env_vars: Vec<EnvVar>,
    pub load_state: LoadState,
}

#[derive(Debug, Clone)]
pub struct EnvVarRow {
    pub key: String,
    pub project_name: String,
    pub vault_value: Option<String>,
}

// ---------------------------------------------------------------------------
// Async events (background tasks → UI thread)
// ---------------------------------------------------------------------------

pub enum AppEvent {
    ProjectsFetched(Vec<ContentItem>),
    EnvVarsFetched { guid: String, vars: Vec<EnvVar> },
    SyncComplete { _guid: String, result: Result<(), String> },
    FetchError(String),
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct App {
    pub page: Page,
    pub projects: Vec<ProjectEntry>,
    pub env_var_rows: Vec<EnvVarRow>,
    pub vault: Vault,
    pub config: Config,
    pub should_quit: bool,

    /// true = sidebar has focus; false = main content has focus
    pub sidebar_focused: bool,

    // Project list
    pub project_list_selected: usize,
    pub project_expanded: HashSet<String>,

    // Env var list
    pub env_var_selected: usize,

    // Vault
    pub vault_selected: usize,
    pub vault_editing: Option<usize>,
    pub vault_edit_buffer: String,
    pub vault_edit_field: VaultField,

    // Settings
    pub settings_selected: usize,
    pub settings_editing: bool,
    pub settings_edit_buffer: String,

    // Async
    pub tx: mpsc::Sender<AppEvent>,
    pub rx: mpsc::Receiver<AppEvent>,

    /// (message, level, ticks_remaining)
    pub status_message: Option<(String, StatusLevel, u8)>,
    pub load_state: LoadState,
    pub pending_fetches: usize,
    pub spinner_frame: usize,
}

impl App {
    pub fn new() -> Result<Self, AppError> {
        let (tx, rx) = mpsc::channel(128);
        let config = Config::load()?;

        let vault = if !config.vault_path.is_empty() {
            Vault::load(&config.vault_path)?
        } else {
            Vault::load_empty()
        };

        Ok(Self {
            page: Page::ProjectList,
            projects: Vec::new(),
            env_var_rows: Vec::new(),
            vault,
            config,
            should_quit: false,
            sidebar_focused: true,
            project_list_selected: 0,
            project_expanded: HashSet::new(),
            env_var_selected: 0,
            vault_selected: 0,
            vault_editing: None,
            vault_edit_buffer: String::new(),
            vault_edit_field: VaultField::Value,
            settings_selected: 0,
            settings_editing: false,
            settings_edit_buffer: String::new(),
            tx,
            rx,
            status_message: None,
            load_state: LoadState::Idle,
            pending_fetches: 0,
            spinner_frame: 0,
        })
    }

    // -----------------------------------------------------------------------
    // Tick (called every ~250ms)
    // -----------------------------------------------------------------------

    pub fn on_tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();

        if let Some((_, _, ref mut ticks)) = self.status_message {
            if *ticks > 0 {
                *ticks -= 1;
            } else {
                self.status_message = None;
            }
        }
    }

    pub fn spinner(&self) -> &str {
        SPINNER_FRAMES[self.spinner_frame]
    }

    // -----------------------------------------------------------------------
    // Status bar
    // -----------------------------------------------------------------------

    pub fn set_status(&mut self, message: String, level: StatusLevel) {
        // 16 ticks × 250ms = 4 seconds display time
        self.status_message = Some((message, level, 16));
    }

    // -----------------------------------------------------------------------
    // Auto-refresh on startup
    // -----------------------------------------------------------------------

    pub fn check_auto_refresh(&mut self) {
        if self.config.server_url.is_empty() || self.config.api_key.is_empty() {
            return;
        }
        let stale = match &self.config.last_refresh {
            None => true,
            Some(ts) => chrono::DateTime::parse_from_rfc3339(ts)
                .map(|dt| {
                    Utc::now()
                        .signed_duration_since(dt.with_timezone(&Utc))
                        .num_hours()
                        >= 24
                })
                .unwrap_or(true),
        };
        if stale {
            self.trigger_fetch();
        }
    }

    // -----------------------------------------------------------------------
    // Background task launchers
    // -----------------------------------------------------------------------

    pub fn trigger_fetch(&mut self) {
        if self.config.server_url.is_empty() || self.config.api_key.is_empty() {
            self.set_status(
                "Configure server URL and API key in Settings (Tab → ↓↓↓ → Enter)".into(),
                StatusLevel::Error,
            );
            return;
        }
        self.load_state = LoadState::Loading;
        let client = ConnectClient::new(&self.config.server_url, &self.config.api_key);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            match client.list_content().await {
                Ok(items) => {
                    let _ = tx.send(AppEvent::ProjectsFetched(items)).await;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::FetchError(e.to_string())).await;
                }
            }
        });
    }

    pub fn trigger_sync(&mut self) {
        if self.config.server_url.is_empty() || self.config.api_key.is_empty() {
            self.set_status(
                "Configure server URL and API key in Settings first".into(),
                StatusLevel::Error,
            );
            return;
        }
        if self.projects.is_empty() {
            self.set_status(
                "No projects loaded. Press Ctrl+P to refresh.".into(),
                StatusLevel::Error,
            );
            return;
        }

        let mut synced = 0usize;
        for project in &self.projects {
            if project.env_vars.is_empty() {
                continue;
            }
            // Safe merge: keep all current vars, overlay vault values for matching keys
            let mut merged = project.env_vars.clone();
            for var in merged.iter_mut() {
                if let Some(v) = self.vault.get(&var.name) {
                    var.value = Some(v.to_string());
                }
            }

            let guid = project.guid.clone();
            let client = ConnectClient::new(&self.config.server_url, &self.config.api_key);
            let tx = self.tx.clone();
            tokio::spawn(async move {
                match client.set_env_vars(&guid, &merged).await {
                    Ok(()) => {
                        let _ = tx
                            .send(AppEvent::SyncComplete { _guid: guid, result: Ok(()) })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(AppEvent::SyncComplete {
                                _guid: guid,
                                result: Err(e.to_string()),
                            })
                            .await;
                    }
                }
            });
            synced += 1;
        }

        if synced > 0 {
            self.set_status(
                format!("Syncing {} project(s) to Connect…", synced),
                StatusLevel::Info,
            );
        } else {
            self.set_status("No env vars to sync (projects have no vars).".into(), StatusLevel::Info);
        }
    }

    // -----------------------------------------------------------------------
    // Background event handler
    // -----------------------------------------------------------------------

    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ProjectsFetched(items) => {
                self.projects.clear();
                self.pending_fetches = items.len();

                // Collect spawn params before mutating self.projects
                let to_fetch: Vec<(String, String, String, Option<String>)> = items
                    .iter()
                    .map(|i| {
                        (
                            i.guid.clone(),
                            i.name.clone(),
                            self.config.server_url.clone(),
                            Some(self.config.api_key.clone()),
                        )
                    })
                    .collect();

                for item in items {
                    self.projects.push(ProjectEntry {
                        guid: item.guid,
                        name: item.name,
                        title: item.title,
                        env_vars: Vec::new(),
                        load_state: LoadState::Loading,
                    });
                }

                for (guid, _name, server_url, api_key) in to_fetch {
                    let api_key = api_key.unwrap_or_default();
                    let client = ConnectClient::new(&server_url, &api_key);
                    let tx = self.tx.clone();
                    let guid_clone = guid.clone();
                    tokio::spawn(async move {
                        match client.get_env_vars(&guid_clone).await {
                            Ok(vars) => {
                                let _ = tx
                                    .send(AppEvent::EnvVarsFetched { guid: guid_clone, vars })
                                    .await;
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::FetchError(e.to_string())).await;
                            }
                        }
                    });
                }

                if self.pending_fetches == 0 {
                    self.load_state = LoadState::Idle;
                }

                self.config.last_refresh = Some(Utc::now().to_rfc3339());
                let _ = self.config.save();
            }

            AppEvent::EnvVarsFetched { guid, vars } => {
                if let Some(project) = self.projects.iter_mut().find(|p| p.guid == guid) {
                    project.env_vars = vars;
                    project.load_state = LoadState::Idle;
                }
                if self.pending_fetches > 0 {
                    self.pending_fetches -= 1;
                }
                if self.pending_fetches == 0 {
                    self.load_state = LoadState::Idle;
                    self.set_status("Projects loaded successfully".into(), StatusLevel::Success);
                }
                self.rebuild_env_var_rows();
            }

            AppEvent::SyncComplete { _guid, result } => match result {
                Ok(()) => {
                    self.set_status("Sync complete!".into(), StatusLevel::Success);
                }
                Err(e) => {
                    self.set_status(format!("Sync failed: {}", e), StatusLevel::Error);
                }
            },

            AppEvent::FetchError(e) => {
                self.load_state = LoadState::Error(e.clone());
                if self.pending_fetches > 0 {
                    self.pending_fetches -= 1;
                }
                self.set_status(format!("Error: {}", e), StatusLevel::Error);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Derived state
    // -----------------------------------------------------------------------

    pub fn rebuild_env_var_rows(&mut self) {
        self.env_var_rows.clear();
        for project in &self.projects {
            for var in &project.env_vars {
                let vault_value = self.vault.get(&var.name).map(|s| s.to_string());
                self.env_var_rows.push(EnvVarRow {
                    key: var.name.clone(),
                    project_name: project
                        .title
                        .clone()
                        .unwrap_or_else(|| project.name.clone()),
                    vault_value,
                });
            }
        }
        // Clamp selections
        if self.env_var_selected >= self.env_var_rows.len() && !self.env_var_rows.is_empty() {
            self.env_var_selected = self.env_var_rows.len() - 1;
        }
    }

    // -----------------------------------------------------------------------
    // Input handling
    // -----------------------------------------------------------------------

    pub fn handle_crossterm_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            self.handle_key(key);
        }
        // Resize is handled automatically by ratatui
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('p') => {
                    self.trigger_fetch();
                    return;
                }
                KeyCode::Char('u') => {
                    self.trigger_sync();
                    return;
                }
                _ => {}
            }
        }

        // Tab toggles sidebar/content focus (except when editing)
        let editing = self.vault_editing.is_some() || self.settings_editing;
        if key.code == KeyCode::Tab && !editing {
            self.sidebar_focused = !self.sidebar_focused;
            return;
        }

        if self.sidebar_focused {
            self.handle_sidebar_key(key);
        } else {
            self.handle_content_key(key);
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        let current = self.page.index();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if current > 0 {
                    self.page = Page::from_index(current - 1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if current < 3 {
                    self.page = Page::from_index(current + 1);
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                self.sidebar_focused = false;
            }
            _ => {}
        }
    }

    fn handle_content_key(&mut self, key: KeyEvent) {
        match self.page.clone() {
            Page::ProjectList => self.handle_project_list_key(key),
            Page::EnvVarList => self.handle_env_var_list_key(key),
            Page::Vault => self.handle_vault_key(key),
            Page::Settings => self.handle_settings_key(key),
        }
    }

    fn handle_project_list_key(&mut self, key: KeyEvent) {
        let count = self.projects.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if count > 0 && self.project_list_selected > 0 {
                    self.project_list_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.project_list_selected + 1 < count {
                    self.project_list_selected += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(project) = self.projects.get(self.project_list_selected) {
                    let guid = project.guid.clone();
                    if self.project_expanded.contains(&guid) {
                        self.project_expanded.remove(&guid);
                    } else {
                        self.project_expanded.insert(guid);
                    }
                }
            }
            KeyCode::Left | KeyCode::Esc => {
                self.sidebar_focused = true;
            }
            _ => {}
        }
    }

    fn handle_env_var_list_key(&mut self, key: KeyEvent) {
        let count = self.env_var_rows.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if count > 0 && self.env_var_selected > 0 {
                    self.env_var_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.env_var_selected + 1 < count {
                    self.env_var_selected += 1;
                }
            }
            KeyCode::Left | KeyCode::Esc => {
                self.sidebar_focused = true;
            }
            _ => {}
        }
    }

    fn handle_vault_key(&mut self, key: KeyEvent) {
        if self.vault_editing.is_some() {
            match key.code {
                KeyCode::Esc => {
                    // Cancel: if editing a new empty-key entry, remove it
                    if self.vault_edit_field == VaultField::Key {
                        if let Some(idx) = self.vault_editing {
                            let empty_key = self
                                .vault
                                .entries
                                .keys()
                                .nth(idx)
                                .filter(|k| k.is_empty())
                                .is_some();
                            if empty_key {
                                self.vault.remove_at(idx);
                                if self.vault_selected > 0
                                    && self.vault_selected >= self.vault.entries.len()
                                {
                                    self.vault_selected -= 1;
                                }
                            }
                        }
                    }
                    self.vault_editing = None;
                    self.vault_edit_buffer.clear();
                }
                KeyCode::Enter => self.commit_vault_edit(),
                KeyCode::Backspace => {
                    self.vault_edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.vault_edit_buffer.push(c);
                }
                _ => {}
            }
        } else {
            let entries_len = self.vault.entries.len();
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if entries_len > 0 && self.vault_selected > 0 {
                        self.vault_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.vault_selected + 1 < entries_len {
                        self.vault_selected += 1;
                    }
                }
                KeyCode::Char('e') | KeyCode::Enter => {
                    if entries_len > 0 {
                        let value = self
                            .vault
                            .entries
                            .values()
                            .nth(self.vault_selected)
                            .cloned()
                            .unwrap_or_default();
                        self.vault_edit_buffer = value;
                        self.vault_edit_field = VaultField::Value;
                        self.vault_editing = Some(self.vault_selected);
                    }
                }
                KeyCode::Char('n') => {
                    // Insert new blank entry at end, start editing the key
                    let new_idx = entries_len;
                    self.vault.entries.insert(String::new(), String::new());
                    self.vault_selected = new_idx;
                    self.vault_edit_buffer.clear();
                    self.vault_edit_field = VaultField::Key;
                    self.vault_editing = Some(new_idx);
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if entries_len > 0 {
                        self.vault.remove_at(self.vault_selected);
                        let new_len = self.vault.entries.len();
                        if self.vault_selected >= new_len && self.vault_selected > 0 {
                            self.vault_selected -= 1;
                        }
                        let _ = self.vault.save();
                        self.rebuild_env_var_rows();
                    }
                }
                KeyCode::Left | KeyCode::Esc => {
                    self.sidebar_focused = true;
                }
                _ => {}
            }
        }
    }

    fn commit_vault_edit(&mut self) {
        let Some(idx) = self.vault_editing else {
            return;
        };
        let new_value = self.vault_edit_buffer.clone();

        match self.vault_edit_field {
            VaultField::Value => {
                if let Some(key) = self.vault.entries.keys().nth(idx).cloned() {
                    self.vault.entries.insert(key, new_value);
                    self.vault.dirty = true;
                }
                self.vault_editing = None;
                self.vault_edit_buffer.clear();
                let _ = self.vault.save();
                self.rebuild_env_var_rows();
            }
            VaultField::Key => {
                // Replace the empty-key placeholder with the typed key, then edit value
                let keys: Vec<String> = self.vault.entries.keys().cloned().collect();
                if let Some(old_key) = keys.get(idx).cloned() {
                    let old_val = self.vault.entries.get(&old_key).cloned().unwrap_or_default();
                    self.vault.entries.shift_remove(&old_key);

                    if !new_value.is_empty() {
                        self.vault.entries.insert(new_value.clone(), old_val);
                        self.vault.dirty = true;
                        // Find new index and start editing value
                        let new_idx = self
                            .vault
                            .entries
                            .keys()
                            .position(|k| k == &new_value)
                            .unwrap_or(0);
                        self.vault_selected = new_idx;
                        self.vault_edit_buffer.clear();
                        self.vault_edit_field = VaultField::Value;
                        self.vault_editing = Some(new_idx);
                        // Don't clear editing — continue to value step
                        return;
                    } else {
                        // Empty key entered — abort
                        self.vault.dirty = true;
                    }
                }
                self.vault_editing = None;
                self.vault_edit_buffer.clear();
                let _ = self.vault.save();
                self.rebuild_env_var_rows();
            }
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        const FIELD_COUNT: usize = 3; // server_url, api_key, vault_path

        if self.settings_editing {
            match key.code {
                KeyCode::Esc => {
                    self.settings_editing = false;
                    self.settings_edit_buffer.clear();
                }
                KeyCode::Enter => self.commit_settings_edit(),
                KeyCode::Backspace => {
                    self.settings_edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.settings_edit_buffer.push(c);
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.settings_selected > 0 {
                        self.settings_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.settings_selected + 1 < FIELD_COUNT {
                        self.settings_selected += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    let current = match self.settings_selected {
                        0 => self.config.server_url.clone(),
                        1 => self.config.api_key.clone(),
                        2 => self.config.vault_path.clone(),
                        _ => String::new(),
                    };
                    self.settings_edit_buffer = current;
                    self.settings_editing = true;
                }
                KeyCode::Left | KeyCode::Esc => {
                    self.sidebar_focused = true;
                }
                _ => {}
            }
        }
    }

    fn commit_settings_edit(&mut self) {
        let value = self.settings_edit_buffer.clone();
        match self.settings_selected {
            0 => self.config.server_url = value,
            1 => self.config.api_key = value,
            2 => {
                self.config.vault_path = value.clone();
                if !value.is_empty() {
                    match Vault::load(&value) {
                        Ok(v) => {
                            self.vault = v;
                            self.rebuild_env_var_rows();
                        }
                        Err(e) => {
                            self.set_status(
                                format!("Could not load vault: {}", e),
                                StatusLevel::Error,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        let _ = self.config.save();
        self.settings_editing = false;
        self.settings_edit_buffer.clear();
        self.set_status("Settings saved".into(), StatusLevel::Success);
    }
}
