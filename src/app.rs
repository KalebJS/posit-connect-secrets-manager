use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashSet;
use tokio::sync::mpsc;

use crate::api::client::ConnectClient;
use crate::api::types::{ContentItem, EnvVar};
use crate::config::Config;
use crate::error::AppError;
use crate::ui::theme::Palette;
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

#[derive(Debug)]
pub enum EditorTarget {
    /// Edit the value of a vault key from the Vault page or Env Vars page.
    VaultEntry(String),
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
    pub vault_value: Option<String>,
}

#[derive(Debug)]
pub struct AddVarPopup {
    pub guid: String,
    pub query: String,
    pub selected: usize,
}

// ---------------------------------------------------------------------------
// Async events (background tasks → UI thread)
// ---------------------------------------------------------------------------

pub enum AppEvent {
    ProjectsFetched(Vec<ContentItem>),
    EnvVarsFetched {
        guid: String,
        vars: Vec<EnvVar>,
    },
    EnvVarsFetchError {
        guid: String,
        error: String,
    },
    SyncComplete {
        _guid: String,
        result: Result<(), String>,
    },
    EnvVarPatched {
        _guid: String,
        result: Result<(), String>,
    },
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
    /// None = cursor on project row; Some(n) = cursor on nth var of selected project
    pub project_var_selected: Option<usize>,

    // Sync confirmation modal: None = hidden; Some(names) = awaiting confirmation
    pub sync_confirm: Option<Vec<String>>,

    // Add-var autocomplete popup
    pub add_var_popup: Option<AddVarPopup>,

    // Set by key handlers when an external editor should be opened for a vault value.
    // Consumed and cleared by the main event loop before the next draw.
    pub open_editor_for: Option<EditorTarget>,

    // Theme / palette (built from config at startup)
    pub palette: Palette,

    // Env var list
    pub env_var_selected: usize,
    /// Key shown in "projects using this var" popup; None = hidden
    pub env_var_detail: Option<String>,

    // Vault
    pub vault_selected: usize,
    pub vault_editing: Option<usize>,
    pub vault_edit_buffer: String,
    pub vault_edit_field: VaultField,

    // Settings
    pub settings_selected: usize,
    pub settings_editing: bool,
    pub settings_edit_buffer: String,

    // Filter
    pub filter_query: String,
    pub filter_editing: bool,
    pub filter_selected: usize,

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

        let palette = Palette::new(config.theme.clone());

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
            project_var_selected: None,
            sync_confirm: None,
            add_var_popup: None,
            open_editor_for: None,
            palette,
            env_var_selected: 0,
            env_var_detail: None,
            vault_selected: 0,
            vault_editing: None,
            vault_edit_buffer: String::new(),
            vault_edit_field: VaultField::Value,
            settings_selected: 0,
            settings_editing: false,
            settings_edit_buffer: String::new(),
            filter_query: String::new(),
            filter_editing: false,
            filter_selected: 0,
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

    /// Returns `(guid, vars_to_send)` for every project that passes the whitelist/blacklist
    /// filters and has at least one var remaining after filtering.
    ///
    /// This is the single source of truth for sync filtering; both `trigger_sync` (modal preview)
    /// and `execute_sync` (actual HTTP calls) derive from it.
    pub fn compute_sync_payloads(&self) -> Vec<(String, Vec<EnvVar>)> {
        self.projects
            .iter()
            .filter(|p| self.config.included_projects.contains(&p.guid))
            .filter(|p| !p.env_vars.is_empty())
            .filter_map(|p| {
                let excluded = self
                    .config
                    .excluded_vars
                    .get(&p.guid)
                    .cloned()
                    .unwrap_or_default();
                // Only include vars that have a corresponding vault value.
                // Sending a var with no value (even as a name-only entry) causes Connect to
                // clear that variable, so non-vault vars must be omitted entirely.
                let merged: Vec<EnvVar> = p
                    .env_vars
                    .iter()
                    .filter(|v| !excluded.contains(&v.name))
                    .filter_map(|v| {
                        self.vault.get(&v.name).map(|vault_val| EnvVar {
                            name: v.name.clone(),
                            value: Some(vault_val.to_string()),
                        })
                    })
                    .collect();
                if merged.is_empty() {
                    None
                } else {
                    Some((p.guid.clone(), merged))
                }
            })
            .collect()
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

        let payloads = self.compute_sync_payloads();
        if payloads.is_empty() {
            self.set_status(
                "No projects selected for sync. Press x on a project to include it.".into(),
                StatusLevel::Info,
            );
            return;
        }

        // Build display names for the confirmation modal from the same set that will be synced
        let will_sync: Vec<String> = payloads
            .iter()
            .filter_map(|(guid, _)| {
                self.projects
                    .iter()
                    .find(|p| &p.guid == guid)
                    .map(|p| p.title.clone().unwrap_or_else(|| p.name.clone()))
            })
            .collect();

        self.sync_confirm = Some(will_sync);
    }

    pub fn execute_sync(&mut self) {
        let payloads = self.compute_sync_payloads();
        let count = payloads.len();

        for (guid, merged) in payloads {
            let client = ConnectClient::new(&self.config.server_url, &self.config.api_key);
            let tx = self.tx.clone();
            tokio::spawn(async move {
                match client.set_env_vars(&guid, &merged).await {
                    Ok(()) => {
                        let _ = tx
                            .send(AppEvent::SyncComplete {
                                _guid: guid,
                                result: Ok(()),
                            })
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
        }

        if count > 0 {
            self.set_status(
                format!("Syncing {} project(s) to Connect…", count),
                StatusLevel::Info,
            );
        } else {
            self.set_status(
                "No env vars to sync (projects have no vars).".into(),
                StatusLevel::Info,
            );
        }
    }

    pub fn trigger_delete_var(&mut self) {
        let Some(var_idx) = self.project_var_selected else {
            return;
        };
        let Some(project) = self.projects.get(self.project_list_selected) else {
            return;
        };
        if self.config.server_url.is_empty() || self.config.api_key.is_empty() {
            self.set_status(
                "Configure server URL and API key in Settings first".into(),
                StatusLevel::Error,
            );
            return;
        }
        let var_name = match project.env_vars.get(var_idx) {
            Some(v) => v.name.clone(),
            None => return,
        };
        let guid = project.guid.clone();
        // Build PATCH: all vars except the deleted one, vault overlays applied where available
        let payload: Vec<EnvVar> = project
            .env_vars
            .iter()
            .filter(|v| v.name != var_name)
            .map(|v| EnvVar {
                name: v.name.clone(),
                value: self
                    .vault
                    .get(&v.name)
                    .map(|s| s.to_string())
                    .or(v.value.clone()),
            })
            .collect();
        // Optimistic local update
        if let Some(p) = self.projects.iter_mut().find(|p| p.guid == guid) {
            p.env_vars.retain(|v| v.name != var_name);
            let len = p.env_vars.len();
            self.project_var_selected = if len == 0 {
                None
            } else {
                Some(var_idx.min(len - 1))
            };
        }
        self.rebuild_env_var_rows();
        let client = ConnectClient::new(&self.config.server_url, &self.config.api_key);
        let tx = self.tx.clone();
        let gc = guid.clone();
        tokio::spawn(async move {
            let result = client
                .set_env_vars(&gc, &payload)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AppEvent::EnvVarPatched { _guid: gc, result }).await;
        });
        self.set_status(
            format!("Deleting {} from project…", var_name),
            StatusLevel::Info,
        );
    }

    // -----------------------------------------------------------------------
    // Background event handler
    // -----------------------------------------------------------------------

    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ProjectsFetched(items) => {
                let items: Vec<ContentItem> = items
                    .into_iter()
                    .filter(|i| matches!(i.app_role.as_deref(), Some("owner") | Some("editor")))
                    .collect();
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
                self.projects.sort_by(|a, b| {
                    let a_name = a.title.as_deref().unwrap_or(&a.name);
                    let b_name = b.title.as_deref().unwrap_or(&b.name);
                    a_name.to_lowercase().cmp(&b_name.to_lowercase())
                });

                for (guid, _name, server_url, api_key) in to_fetch {
                    let api_key = api_key.unwrap_or_default();
                    let client = ConnectClient::new(&server_url, &api_key);
                    let tx = self.tx.clone();
                    let guid_clone = guid.clone();
                    tokio::spawn(async move {
                        match client.get_env_vars(&guid_clone).await {
                            Ok(vars) => {
                                let _ = tx
                                    .send(AppEvent::EnvVarsFetched {
                                        guid: guid_clone,
                                        vars,
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(AppEvent::EnvVarsFetchError {
                                        guid: guid_clone,
                                        error: e.to_string(),
                                    })
                                    .await;
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

            AppEvent::EnvVarsFetchError { guid, error } => {
                if let Some(project) = self.projects.iter_mut().find(|p| p.guid == guid) {
                    project.load_state = LoadState::Error(error.clone());
                }
                if self.pending_fetches > 0 {
                    self.pending_fetches -= 1;
                }
                if self.pending_fetches == 0 {
                    self.load_state = LoadState::Idle;
                }
                self.set_status(
                    format!("Error loading env vars: {}", error),
                    StatusLevel::Error,
                );
            }

            AppEvent::SyncComplete { _guid, result } => match result {
                Ok(()) => {
                    self.set_status("Sync complete!".into(), StatusLevel::Success);
                }
                Err(e) => {
                    self.set_status(format!("Sync failed: {}", e), StatusLevel::Error);
                }
            },

            AppEvent::EnvVarPatched { _guid: _, result } => match result {
                Ok(()) => {
                    self.set_status("Change applied to project".into(), StatusLevel::Success);
                }
                Err(e) => {
                    self.set_status(format!("Patch failed: {}", e), StatusLevel::Error);
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
        let mut map: std::collections::BTreeMap<String, Option<String>> =
            std::collections::BTreeMap::new();
        for project in self
            .projects
            .iter()
            .filter(|p| self.config.included_projects.contains(&p.guid))
        {
            for var in &project.env_vars {
                map.entry(var.name.clone())
                    .or_insert_with(|| self.vault.get(&var.name).map(|s| s.to_string()));
            }
        }
        self.env_var_rows = map
            .into_iter()
            .map(|(key, vault_value)| EnvVarRow { key, vault_value })
            .collect();
        // Clamp selection
        if self.env_var_selected >= self.env_var_rows.len() && !self.env_var_rows.is_empty() {
            self.env_var_selected = self.env_var_rows.len() - 1;
        }
    }

    // -----------------------------------------------------------------------
    // Filter helpers
    // -----------------------------------------------------------------------

    pub fn filter_matches(&self, text: &str) -> bool {
        if self.filter_query.is_empty() {
            return true;
        }
        match regex::Regex::new(&format!("(?i){}", self.filter_query)) {
            Ok(re) => re.is_match(text),
            Err(_) => text
                .to_lowercase()
                .contains(&self.filter_query.to_lowercase()),
        }
    }

    pub fn filtered_count(&self) -> usize {
        match self.page {
            Page::ProjectList => self
                .projects
                .iter()
                .filter(|p| self.filter_matches(p.title.as_deref().unwrap_or(&p.name)))
                .count(),
            Page::EnvVarList => self
                .env_var_rows
                .iter()
                .filter(|r| self.filter_matches(&r.key))
                .count(),
            Page::Vault => self
                .vault
                .entries
                .keys()
                .filter(|k| self.filter_matches(k))
                .count(),
            Page::Settings => 0,
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filter_editing = false;
            }
            KeyCode::Backspace => {
                self.filter_query.pop();
                self.filter_selected = 0;
            }
            KeyCode::Char(c) => {
                self.filter_query.push(c);
                self.filter_selected = 0;
            }
            _ => {}
        }
    }

    /// Returns vault keys matching the popup query that are not already on the project.
    pub fn add_var_suggestions(&self) -> Vec<String> {
        let Some(popup) = &self.add_var_popup else {
            return Vec::new();
        };
        let q = popup.query.to_lowercase();
        let existing: std::collections::HashSet<&str> = self
            .projects
            .iter()
            .find(|p| p.guid == popup.guid)
            .map(|p| p.env_vars.iter().map(|v| v.name.as_str()).collect())
            .unwrap_or_default();
        self.vault
            .entries
            .keys()
            .filter(|k| !existing.contains(k.as_str()))
            .filter(|k| q.is_empty() || k.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    pub fn trigger_add_var(&mut self) {
        let Some(project) = self.projects.get(self.project_list_selected) else {
            return;
        };
        self.add_var_popup = Some(AddVarPopup {
            guid: project.guid.clone(),
            query: String::new(),
            selected: 0,
        });
    }

    pub fn commit_add_var(&mut self) {
        let suggestions = self.add_var_suggestions();
        let (guid, selected_key, vault_value) = if let Some(popup) = &self.add_var_popup {
            let key = match suggestions.get(popup.selected) {
                Some(k) => k.clone(),
                None => return,
            };
            let val = match self.vault.get(&key) {
                Some(v) => v.to_string(),
                None => return,
            };
            (popup.guid.clone(), key, val)
        } else {
            return;
        };
        self.add_var_popup = None;

        // Build PATCH: existing vars (vault overlays) + new var
        let payload: Vec<EnvVar> =
            if let Some(project) = self.projects.iter().find(|p| p.guid == guid) {
                let mut vars: Vec<EnvVar> = project
                    .env_vars
                    .iter()
                    .map(|v| EnvVar {
                        name: v.name.clone(),
                        value: self
                            .vault
                            .get(&v.name)
                            .map(|s| s.to_string())
                            .or(v.value.clone()),
                    })
                    .collect();
                vars.push(EnvVar {
                    name: selected_key.clone(),
                    value: Some(vault_value),
                });
                vars
            } else {
                return;
            };

        // Optimistic local update
        if let Some(p) = self.projects.iter_mut().find(|p| p.guid == guid) {
            p.env_vars.push(EnvVar {
                name: selected_key.clone(),
                value: None,
            });
        }
        self.rebuild_env_var_rows();

        if self.config.server_url.is_empty() || self.config.api_key.is_empty() {
            self.set_status(
                "Configure server URL and API key in Settings first".into(),
                StatusLevel::Error,
            );
            return;
        }

        let client = ConnectClient::new(&self.config.server_url, &self.config.api_key);
        let tx = self.tx.clone();
        let gc = guid.clone();
        tokio::spawn(async move {
            let result = client
                .set_env_vars(&gc, &payload)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(AppEvent::EnvVarPatched { _guid: gc, result }).await;
        });
        self.set_status(
            format!("Adding {} to project…", selected_key),
            StatusLevel::Info,
        );
    }

    fn handle_add_var_popup_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.add_var_popup = None;
            }
            KeyCode::Enter => {
                self.commit_add_var();
            }
            KeyCode::Up => {
                if let Some(p) = &mut self.add_var_popup {
                    if p.selected > 0 {
                        p.selected -= 1;
                    }
                }
            }
            KeyCode::Down => {
                let count = self.add_var_suggestions().len();
                if let Some(p) = &mut self.add_var_popup {
                    if p.selected + 1 < count {
                        p.selected += 1;
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(p) = &mut self.add_var_popup {
                    p.query.pop();
                    p.selected = 0;
                }
            }
            KeyCode::Char(c) => {
                if let Some(p) = &mut self.add_var_popup {
                    p.query.push(c);
                    p.selected = 0;
                }
            }
            _ => {}
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
        // Confirmation modal intercepts all input
        if self.sync_confirm.is_some() {
            match key.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.sync_confirm = None;
                    self.execute_sync();
                }
                KeyCode::Esc | KeyCode::Char('n') => {
                    self.sync_confirm = None;
                }
                _ => {}
            }
            return;
        }

        // Add-var popup intercepts all input
        if self.add_var_popup.is_some() {
            self.handle_add_var_popup_key(key);
            return;
        }

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
                    self.filter_query.clear();
                    self.filter_editing = false;
                    self.filter_selected = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if current < 3 {
                    self.page = Page::from_index(current + 1);
                    self.filter_query.clear();
                    self.filter_editing = false;
                    self.filter_selected = 0;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.sidebar_focused = false;
            }
            _ => {}
        }
    }

    fn handle_content_key(&mut self, key: KeyEvent) {
        // Filter editing intercepts all input
        if self.filter_editing {
            self.handle_filter_key(key);
            return;
        }

        // f opens filter; F clears it — not while editing an entry
        let in_edit_mode = self.vault_editing.is_some() || self.settings_editing;
        if !in_edit_mode {
            match key.code {
                KeyCode::Char('f') => {
                    self.filter_editing = true;
                    return;
                }
                KeyCode::Char('F') => {
                    self.filter_query.clear();
                    self.filter_selected = 0;
                    return;
                }
                _ => {}
            }
        }

        // When filter active, j/k navigate the filtered list
        if !self.filter_query.is_empty() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.filter_selected > 0 {
                        self.filter_selected -= 1;
                    }
                    return;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let count = self.filtered_count();
                    if self.filter_selected + 1 < count {
                        self.filter_selected += 1;
                    }
                    return;
                }
                _ => {}
            }
        }

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
                if count == 0 {
                    return;
                }
                if let Some(var_idx) = self.project_var_selected {
                    if var_idx > 0 {
                        self.project_var_selected = Some(var_idx - 1);
                    } else {
                        self.project_var_selected = None;
                    }
                } else if self.project_list_selected > 0 {
                    self.project_list_selected -= 1;
                    // If prev project is expanded with vars, land on its last var
                    let prev = &self.projects[self.project_list_selected];
                    if self.project_expanded.contains(&prev.guid) && !prev.env_vars.is_empty() {
                        self.project_var_selected = Some(prev.env_vars.len() - 1);
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if count == 0 {
                    return;
                }
                if let Some(var_idx) = self.project_var_selected {
                    let var_count = self.projects[self.project_list_selected].env_vars.len();
                    if var_idx + 1 < var_count {
                        self.project_var_selected = Some(var_idx + 1);
                    } else if self.project_list_selected + 1 < count {
                        self.project_list_selected += 1;
                        self.project_var_selected = None;
                    }
                } else {
                    let project = &self.projects[self.project_list_selected];
                    if self.project_expanded.contains(&project.guid) && !project.env_vars.is_empty()
                    {
                        self.project_var_selected = Some(0);
                    } else if self.project_list_selected + 1 < count {
                        self.project_list_selected += 1;
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Only toggle expand when cursor is on the project row
                if self.project_var_selected.is_none() {
                    if let Some(project) = self.projects.get(self.project_list_selected) {
                        let guid = project.guid.clone();
                        if self.project_expanded.contains(&guid) {
                            self.project_expanded.remove(&guid);
                            self.project_var_selected = None;
                        } else {
                            self.project_expanded.insert(guid);
                        }
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(project) = self.projects.get(self.project_list_selected) {
                    let guid = project.guid.clone();
                    if let Some(var_idx) = self.project_var_selected {
                        // Toggle var blacklist for this project
                        if let Some(var) = project.env_vars.get(var_idx) {
                            let var_name = var.name.clone();
                            let entry = self.config.excluded_vars.entry(guid).or_default();
                            if let Some(pos) = entry.iter().position(|v| v == &var_name) {
                                entry.remove(pos);
                            } else {
                                entry.push(var_name);
                            }
                        }
                    } else {
                        // Toggle project whitelist
                        if let Some(pos) = self
                            .config
                            .included_projects
                            .iter()
                            .position(|g| g == &guid)
                        {
                            self.config.included_projects.remove(pos);
                        } else {
                            self.config.included_projects.push(guid);
                        }
                    }
                    let _ = self.config.save();
                }
            }
            KeyCode::Char('d') => {
                if self.project_var_selected.is_some() {
                    self.trigger_delete_var();
                }
            }
            KeyCode::Char('a') => {
                self.trigger_add_var();
            }
            KeyCode::Left | KeyCode::Esc => {
                self.sidebar_focused = true;
            }
            _ => {}
        }
    }

    fn handle_env_var_list_key(&mut self, key: KeyEvent) {
        // Any key closes the detail popup when it's open
        if self.env_var_detail.is_some() {
            self.env_var_detail = None;
            return;
        }
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
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(row) = self.env_var_rows.get(self.env_var_selected) {
                    self.env_var_detail = Some(row.key.clone());
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if let Some(row) = self.env_var_rows.get(self.env_var_selected) {
                    self.open_editor_for = Some(EditorTarget::VaultEntry(row.key.clone()));
                }
            }
            KeyCode::Left | KeyCode::Esc | KeyCode::Char('h') => {
                self.sidebar_focused = true;
            }
            _ => {}
        }
    }

    /// Resolves the currently selected vault index, accounting for active filter.
    /// Returns None if vault is empty or filter has no matches.
    fn effective_vault_index(&self) -> Option<usize> {
        if self.filter_query.is_empty() {
            if self.vault.entries.is_empty() {
                None
            } else {
                Some(self.vault_selected)
            }
        } else {
            self.vault
                .entries
                .keys()
                .enumerate()
                .filter(|(_, k)| self.filter_matches(k))
                .nth(self.filter_selected)
                .map(|(orig_i, _)| orig_i)
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
                    if let Some(idx) = self.effective_vault_index() {
                        let value = self
                            .vault
                            .entries
                            .values()
                            .nth(idx)
                            .cloned()
                            .unwrap_or_default();
                        self.vault_edit_buffer = value;
                        self.vault_edit_field = VaultField::Value;
                        self.vault_editing = Some(idx);
                    }
                }
                KeyCode::Char('E') => {
                    if let Some(idx) = self.effective_vault_index() {
                        if let Some(key) = self.vault.entries.keys().nth(idx).cloned() {
                            self.open_editor_for = Some(EditorTarget::VaultEntry(key));
                        }
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
                    if let Some(idx) = self.effective_vault_index() {
                        self.vault.remove_at(idx);
                        let new_len = self.vault.entries.len();
                        if self.filter_query.is_empty() {
                            if self.vault_selected >= new_len && self.vault_selected > 0 {
                                self.vault_selected -= 1;
                            }
                        } else {
                            let new_filtered = self.filtered_count();
                            if self.filter_selected >= new_filtered && self.filter_selected > 0 {
                                self.filter_selected -= 1;
                            }
                        }
                        let _ = self.vault.save();
                        self.rebuild_env_var_rows();
                    }
                }
                KeyCode::Left | KeyCode::Esc | KeyCode::Char('h') => {
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
                    let old_val = self
                        .vault
                        .entries
                        .get(&old_key)
                        .cloned()
                        .unwrap_or_default();
                    self.vault.entries.shift_remove(&old_key);

                    if !new_value.is_empty() {
                        self.vault.entries.insert(new_value.clone(), old_val);
                        self.vault.dirty = true;
                        self.vault.sort();
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
                KeyCode::Left | KeyCode::Esc | KeyCode::Char('h') => {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::EnvVar;
    use crate::config::Config;
    use crate::vault::Vault;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn env_var(name: &str) -> EnvVar {
        EnvVar {
            name: name.to_string(),
            value: None,
        }
    }

    fn project(guid: &str, name: &str, vars: Vec<EnvVar>) -> ProjectEntry {
        ProjectEntry {
            guid: guid.to_string(),
            name: name.to_string(),
            title: None,
            env_vars: vars,
            load_state: LoadState::Idle,
        }
    }

    fn project_with_title(guid: &str, name: &str, title: &str, vars: Vec<EnvVar>) -> ProjectEntry {
        ProjectEntry {
            guid: guid.to_string(),
            name: name.to_string(),
            title: Some(title.to_string()),
            env_vars: vars,
            load_state: LoadState::Idle,
        }
    }

    fn base_config() -> Config {
        Config {
            server_url: "http://connect.test".into(),
            api_key: "key".into(),
            vault_path: String::new(),
            last_refresh: None,
            included_projects: Vec::new(),
            excluded_vars: HashMap::new(),
            theme: crate::ui::theme::ThemeVariant::Inherit,
        }
    }

    fn make_app(config: Config, vault: Vault, projects: Vec<ProjectEntry>) -> App {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let palette = crate::ui::theme::Palette::new(config.theme.clone());
        App {
            page: Page::ProjectList,
            projects,
            env_var_rows: Vec::new(),
            vault,
            config,
            should_quit: false,
            sidebar_focused: false,
            project_list_selected: 0,
            project_expanded: HashSet::new(),
            project_var_selected: None,
            sync_confirm: None,
            add_var_popup: None,
            open_editor_for: None,
            palette,
            env_var_selected: 0,
            env_var_detail: None,
            vault_selected: 0,
            vault_editing: None,
            vault_edit_buffer: String::new(),
            vault_edit_field: VaultField::Value,
            settings_selected: 0,
            settings_editing: false,
            settings_edit_buffer: String::new(),
            filter_query: String::new(),
            filter_editing: false,
            filter_selected: 0,
            tx,
            rx,
            status_message: None,
            load_state: LoadState::Idle,
            pending_fetches: 0,
            spinner_frame: 0,
        }
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_crossterm_event(crossterm::event::Event::Key(KeyEvent::new(
            code,
            KeyModifiers::NONE,
        )));
    }

    // Two expanded projects: proj-a (2 vars), proj-b (1 var)
    fn expanded_two_project_app() -> App {
        let config = base_config();
        let projects = vec![
            project("guid-a", "proj-a", vec![env_var("VAR1"), env_var("VAR2")]),
            project("guid-b", "proj-b", vec![env_var("OTHER")]),
        ];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.project_expanded.insert("guid-a".into());
        app
    }

    // -----------------------------------------------------------------------
    // compute_sync_payloads — whitelist enforcement
    // -----------------------------------------------------------------------

    #[test]
    fn empty_whitelist_produces_no_payloads() {
        // No projects added to included_projects → nothing syncs
        let config = base_config();
        let projects = vec![project("guid-a", "proj-a", vec![env_var("FOO")])];
        let app = make_app(config, Vault::load_empty(), projects);
        assert!(app.compute_sync_payloads().is_empty());
    }

    #[test]
    fn non_whitelisted_project_excluded_from_payloads() {
        let mut config = base_config();
        config.included_projects = vec!["guid-b".into()]; // only guid-b whitelisted
        let projects = vec![
            project("guid-a", "proj-a", vec![env_var("FOO")]),
            project("guid-b", "proj-b", vec![env_var("BAR")]),
        ];
        let mut vault = Vault::load_empty();
        vault.entries.insert("BAR".into(), "v".into());
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].0, "guid-b");
        assert!(!payloads.iter().any(|(g, _)| g == "guid-a"));
    }

    #[test]
    fn whitelisted_project_with_vars_included_in_payloads() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![project("guid-a", "proj-a", vec![env_var("FOO")])];
        let mut vault = Vault::load_empty();
        vault.entries.insert("FOO".into(), "bar".into());
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].0, "guid-a");
        assert_eq!(payloads[0].1[0].name, "FOO");
    }

    #[test]
    fn whitelisted_project_with_no_vars_excluded_from_payloads() {
        // Even if whitelisted, a project with no env vars must not appear
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![project("guid-a", "proj-a", vec![])];
        let app = make_app(config, Vault::load_empty(), projects);
        assert!(app.compute_sync_payloads().is_empty());
    }

    #[test]
    fn all_three_projects_mixed_whitelist() {
        // guid-a and guid-c whitelisted; guid-b not
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into(), "guid-c".into()];
        let projects = vec![
            project("guid-a", "proj-a", vec![env_var("X")]),
            project("guid-b", "proj-b", vec![env_var("Y")]),
            project("guid-c", "proj-c", vec![env_var("Z")]),
        ];
        let mut vault = Vault::load_empty();
        vault.entries.insert("X".into(), "xval".into());
        vault.entries.insert("Z".into(), "zval".into());
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads.len(), 2);
        let guids: Vec<&str> = payloads.iter().map(|(g, _)| g.as_str()).collect();
        assert!(guids.contains(&"guid-a"));
        assert!(!guids.contains(&"guid-b"));
        assert!(guids.contains(&"guid-c"));
    }

    // -----------------------------------------------------------------------
    // compute_sync_payloads — var blacklist enforcement
    // -----------------------------------------------------------------------

    #[test]
    fn blacklisted_var_absent_from_payload() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["SECRET".into()]);
        let projects = vec![project(
            "guid-a",
            "proj-a",
            vec![env_var("PUBLIC"), env_var("SECRET")],
        )];
        let mut vault = Vault::load_empty();
        vault.entries.insert("PUBLIC".into(), "pubval".into());
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads.len(), 1);
        let vars = &payloads[0].1;
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "PUBLIC");
        assert!(!vars.iter().any(|v| v.name == "SECRET"));
    }

    #[test]
    fn non_blacklisted_var_present_in_payload() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["SKIP".into()]);
        let projects = vec![project(
            "guid-a",
            "proj-a",
            vec![env_var("KEEP"), env_var("SKIP")],
        )];
        let mut vault = Vault::load_empty();
        vault.entries.insert("KEEP".into(), "keepval".into());
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        let vars = &payloads[0].1;
        assert!(vars.iter().any(|v| v.name == "KEEP"));
    }

    #[test]
    fn all_vars_blacklisted_project_omitted_entirely() {
        // Prevents an accidental empty PATCH that would delete all env vars on Connect.
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["FOO".into(), "BAR".into()]);
        let projects = vec![project(
            "guid-a",
            "proj-a",
            vec![env_var("FOO"), env_var("BAR")],
        )];
        let app = make_app(config, Vault::load_empty(), projects);
        assert!(
            app.compute_sync_payloads().is_empty(),
            "project with all vars blacklisted must not appear — an empty PATCH would delete all env vars"
        );
    }

    #[test]
    fn blacklist_is_scoped_per_project_guid() {
        // SECRET is blacklisted for guid-a only; guid-b's SECRET must still sync.
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into(), "guid-b".into()];
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["SECRET".into()]);
        let projects = vec![
            project(
                "guid-a",
                "proj-a",
                vec![env_var("PUBLIC"), env_var("SECRET")],
            ),
            project("guid-b", "proj-b", vec![env_var("SECRET")]),
        ];
        let mut vault = Vault::load_empty();
        vault.entries.insert("PUBLIC".into(), "pubval".into());
        vault.entries.insert("SECRET".into(), "secval".into());
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads.len(), 2);

        let a = payloads.iter().find(|(g, _)| g == "guid-a").unwrap();
        assert_eq!(a.1.len(), 1, "guid-a must only have PUBLIC");
        assert_eq!(a.1[0].name, "PUBLIC");

        let b = payloads.iter().find(|(g, _)| g == "guid-b").unwrap();
        assert_eq!(b.1.len(), 1, "guid-b's SECRET must not be filtered");
        assert_eq!(b.1[0].name, "SECRET");
    }

    #[test]
    fn empty_excluded_vars_map_does_not_affect_sync() {
        // excluded_vars exists in config but has no entry for this project
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        config
            .excluded_vars
            .insert("guid-other".into(), vec!["X".into()]);
        let projects = vec![project("guid-a", "proj-a", vec![env_var("FOO")])];
        let mut vault = Vault::load_empty();
        vault.entries.insert("FOO".into(), "fooval".into());
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].1.len(), 1);
    }

    // -----------------------------------------------------------------------
    // compute_sync_payloads — vault value overlay
    // -----------------------------------------------------------------------

    #[test]
    fn vault_value_overlaid_for_included_var() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let mut vault = Vault::load_empty();
        vault.entries.insert("DB_PASS".into(), "s3cr3t".into());
        let projects = vec![project("guid-a", "proj-a", vec![env_var("DB_PASS")])];
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads[0].1[0].value.as_deref(), Some("s3cr3t"));
    }

    #[test]
    fn vault_value_for_var_not_on_project_is_not_injected() {
        // Vault has a key that the project doesn't have — must not appear in payload.
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let mut vault = Vault::load_empty();
        vault.entries.insert("FOO".into(), "fooval".into());
        vault.entries.insert("EXTRA".into(), "value".into());
        // Project only has FOO; EXTRA is vault-only and must not be injected
        let projects = vec![project("guid-a", "proj-a", vec![env_var("FOO")])];
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        assert_eq!(payloads[0].1.len(), 1);
        assert_eq!(payloads[0].1[0].name, "FOO");
        assert!(!payloads[0].1.iter().any(|v| v.name == "EXTRA"));
    }

    #[test]
    fn blacklisted_var_vault_value_not_sent() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["DB_PASS".into()]);
        let mut vault = Vault::load_empty();
        vault.entries.insert("DB_PASS".into(), "s3cr3t".into());
        vault.entries.insert("OTHER".into(), "fine".into());
        let projects = vec![project(
            "guid-a",
            "proj-a",
            vec![env_var("DB_PASS"), env_var("OTHER")],
        )];
        let app = make_app(config, vault, projects);
        let payloads = app.compute_sync_payloads();
        let vars = &payloads[0].1;
        assert!(
            !vars.iter().any(|v| v.name == "DB_PASS"),
            "blacklisted var must not appear"
        );
        assert_eq!(
            vars[0].value.as_deref(),
            Some("fine"),
            "OTHER's vault value must be applied"
        );
    }

    #[test]
    fn var_without_vault_entry_excluded_from_payload() {
        // Non-vault vars are omitted from the PATCH to avoid clearing their values on Connect.
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![project("guid-a", "proj-a", vec![env_var("FOO")])];
        let app = make_app(config, Vault::load_empty(), projects);
        assert!(app.compute_sync_payloads().is_empty());
    }

    // -----------------------------------------------------------------------
    // trigger_sync — modal preview
    // -----------------------------------------------------------------------

    #[test]
    fn trigger_sync_empty_whitelist_sets_status_not_modal() {
        let config = base_config();
        let projects = vec![project("guid-a", "proj-a", vec![env_var("X")])];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.trigger_sync();
        assert!(
            app.sync_confirm.is_none(),
            "no modal when whitelist is empty"
        );
        assert!(
            app.status_message.is_some(),
            "status message should explain"
        );
    }

    #[test]
    fn trigger_sync_whitelisted_project_with_vars_shows_modal() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![project("guid-a", "proj-a", vec![env_var("X")])];
        let mut vault = Vault::load_empty();
        vault.entries.insert("X".into(), "xval".into());
        let mut app = make_app(config, vault, projects);
        app.trigger_sync();
        let names = app.sync_confirm.as_ref().expect("modal must be shown");
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "proj-a");
    }

    #[test]
    fn trigger_sync_uses_title_over_name_in_modal() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let p = project_with_title("guid-a", "proj-a", "My Pretty App", vec![env_var("X")]);
        let mut vault = Vault::load_empty();
        vault.entries.insert("X".into(), "xval".into());
        let mut app = make_app(config, vault, vec![p]);
        app.trigger_sync();
        let names = app.sync_confirm.as_ref().unwrap();
        assert_eq!(names[0], "My Pretty App");
    }

    #[test]
    fn trigger_sync_whitelisted_but_no_vars_no_modal() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![project("guid-a", "proj-a", vec![])];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.trigger_sync();
        assert!(app.sync_confirm.is_none());
        assert!(app.status_message.is_some());
    }

    #[test]
    fn trigger_sync_whitelisted_all_vars_blacklisted_no_modal() {
        // After blacklisting removes all vars, no modal — equivalent to nothing to sync.
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["FOO".into()]);
        let projects = vec![project("guid-a", "proj-a", vec![env_var("FOO")])];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.trigger_sync();
        assert!(app.sync_confirm.is_none());
    }

    #[test]
    fn trigger_sync_modal_lists_only_whitelisted_projects() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()]; // only a
        let projects = vec![
            project("guid-a", "proj-a", vec![env_var("X")]),
            project("guid-b", "proj-b", vec![env_var("Y")]),
        ];
        let mut vault = Vault::load_empty();
        vault.entries.insert("X".into(), "xval".into());
        let mut app = make_app(config, vault, projects);
        app.trigger_sync();
        let names = app.sync_confirm.as_ref().unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "proj-a");
        assert!(!names.iter().any(|n| n == "proj-b"));
    }

    // -----------------------------------------------------------------------
    // Navigation — project_list_selected and project_var_selected
    // -----------------------------------------------------------------------

    #[test]
    fn nav_down_from_project_row_enters_first_var_when_expanded() {
        let mut app = expanded_two_project_app();
        assert_eq!(app.project_list_selected, 0);
        assert!(app.project_var_selected.is_none());
        press(&mut app, KeyCode::Down);
        assert_eq!(app.project_list_selected, 0, "must stay on same project");
        assert_eq!(app.project_var_selected, Some(0));
    }

    #[test]
    fn nav_down_advances_through_vars() {
        let mut app = expanded_two_project_app();
        app.project_var_selected = Some(0);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.project_list_selected, 0);
        assert_eq!(app.project_var_selected, Some(1));
    }

    #[test]
    fn nav_down_from_last_var_moves_to_next_project() {
        let mut app = expanded_two_project_app();
        app.project_var_selected = Some(1); // proj-a has 2 vars; this is the last
        press(&mut app, KeyCode::Down);
        assert_eq!(app.project_list_selected, 1, "must move to proj-b");
        assert!(app.project_var_selected.is_none());
    }

    #[test]
    fn nav_down_does_not_advance_past_last_project() {
        let mut app = expanded_two_project_app();
        app.project_list_selected = 1;
        press(&mut app, KeyCode::Down);
        assert_eq!(app.project_list_selected, 1);
    }

    #[test]
    fn nav_up_from_first_var_returns_to_project_row() {
        let mut app = expanded_two_project_app();
        app.project_var_selected = Some(0);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.project_list_selected, 0);
        assert!(app.project_var_selected.is_none());
    }

    #[test]
    fn nav_up_within_vars() {
        let mut app = expanded_two_project_app();
        app.project_var_selected = Some(1);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.project_var_selected, Some(0));
    }

    #[test]
    fn nav_up_from_project_row_enters_prev_projects_last_var() {
        let mut app = expanded_two_project_app();
        app.project_list_selected = 1;
        app.project_var_selected = None;
        press(&mut app, KeyCode::Up);
        // proj-a is expanded with 2 vars; should land on last (index 1)
        assert_eq!(app.project_list_selected, 0);
        assert_eq!(app.project_var_selected, Some(1));
    }

    #[test]
    fn nav_up_does_not_go_before_first_project() {
        let app_config = base_config();
        let projects = vec![project("guid-a", "proj-a", vec![])];
        let mut app = make_app(app_config, Vault::load_empty(), projects);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.project_list_selected, 0);
    }

    #[test]
    fn nav_down_collapsed_project_skips_vars() {
        // proj-a is NOT expanded; pressing down must jump straight to proj-b
        let app_config = base_config();
        let projects = vec![
            project("guid-a", "proj-a", vec![env_var("X")]),
            project("guid-b", "proj-b", vec![env_var("Y")]),
        ];
        let mut app = make_app(app_config, Vault::load_empty(), projects);
        // project_expanded is empty — proj-a is collapsed
        press(&mut app, KeyCode::Down);
        assert_eq!(app.project_list_selected, 1);
        assert!(app.project_var_selected.is_none());
    }

    // -----------------------------------------------------------------------
    // x-key — whitelist and blacklist toggling
    // -----------------------------------------------------------------------

    #[test]
    fn x_on_project_row_adds_to_whitelist() {
        let config = base_config();
        let projects = vec![project("guid-a", "proj-a", vec![])];
        let mut app = make_app(config, Vault::load_empty(), projects);
        press(&mut app, KeyCode::Char('x'));
        assert!(
            app.config.included_projects.contains(&"guid-a".to_string()),
            "guid-a must be in whitelist after x"
        );
    }

    #[test]
    fn x_on_whitelisted_project_removes_it() {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![project("guid-a", "proj-a", vec![])];
        let mut app = make_app(config, Vault::load_empty(), projects);
        press(&mut app, KeyCode::Char('x'));
        assert!(
            app.config.included_projects.is_empty(),
            "guid-a must be removed from whitelist"
        );
    }

    #[test]
    fn x_toggle_is_idempotent_addremove() {
        let config = base_config();
        let projects = vec![project("guid-a", "proj-a", vec![])];
        let mut app = make_app(config, Vault::load_empty(), projects);
        press(&mut app, KeyCode::Char('x')); // add
        press(&mut app, KeyCode::Char('x')); // remove
        assert!(app.config.included_projects.is_empty());
    }

    #[test]
    fn x_only_affects_selected_project() {
        let config = base_config();
        let projects = vec![
            project("guid-a", "proj-a", vec![]),
            project("guid-b", "proj-b", vec![]),
        ];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.project_list_selected = 1; // cursor on proj-b
        press(&mut app, KeyCode::Char('x'));
        assert!(!app.config.included_projects.contains(&"guid-a".to_string()));
        assert!(app.config.included_projects.contains(&"guid-b".to_string()));
    }

    #[test]
    fn x_on_var_row_adds_to_blacklist() {
        let config = base_config();
        let projects = vec![project(
            "guid-a",
            "proj-a",
            vec![env_var("FOO"), env_var("BAR")],
        )];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.project_expanded.insert("guid-a".into());
        app.project_var_selected = Some(0); // cursor on FOO
        press(&mut app, KeyCode::Char('x'));
        let excl = app
            .config
            .excluded_vars
            .get("guid-a")
            .expect("entry must exist");
        assert!(excl.contains(&"FOO".to_string()));
        assert!(
            !excl.contains(&"BAR".to_string()),
            "BAR must not be affected"
        );
    }

    #[test]
    fn x_on_blacklisted_var_removes_it() {
        let mut config = base_config();
        config
            .excluded_vars
            .insert("guid-a".into(), vec!["FOO".into()]);
        let projects = vec![project("guid-a", "proj-a", vec![env_var("FOO")])];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.project_expanded.insert("guid-a".into());
        app.project_var_selected = Some(0);
        press(&mut app, KeyCode::Char('x'));
        let excl = app.config.excluded_vars.get("guid-a");
        assert!(
            excl.map_or(true, |v| !v.contains(&"FOO".to_string())),
            "FOO must be removed from blacklist"
        );
    }

    #[test]
    fn x_on_var_does_not_affect_other_projects_blacklist() {
        let config = base_config();
        let projects = vec![
            project("guid-a", "proj-a", vec![env_var("SECRET")]),
            project("guid-b", "proj-b", vec![env_var("SECRET")]),
        ];
        let mut app = make_app(config, Vault::load_empty(), projects);
        app.project_list_selected = 0;
        app.project_expanded.insert("guid-a".into());
        app.project_var_selected = Some(0); // SECRET in proj-a
        press(&mut app, KeyCode::Char('x'));
        // guid-b's SECRET must not be blacklisted
        assert!(app
            .config
            .excluded_vars
            .get("guid-b")
            .map_or(true, |v| !v.contains(&"SECRET".to_string())));
    }

    // -----------------------------------------------------------------------
    // Modal key handling
    // -----------------------------------------------------------------------

    #[test]
    fn modal_enter_clears_sync_confirm() {
        let config = base_config();
        let mut app = make_app(config, Vault::load_empty(), vec![]);
        app.sync_confirm = Some(vec!["proj-a".into()]);
        press(&mut app, KeyCode::Enter);
        assert!(app.sync_confirm.is_none());
    }

    #[test]
    fn modal_y_clears_sync_confirm() {
        let config = base_config();
        let mut app = make_app(config, Vault::load_empty(), vec![]);
        app.sync_confirm = Some(vec!["proj-a".into()]);
        press(&mut app, KeyCode::Char('y'));
        assert!(app.sync_confirm.is_none());
    }

    #[test]
    fn modal_esc_cancels_without_sync() {
        let config = base_config();
        let mut app = make_app(config, Vault::load_empty(), vec![]);
        app.sync_confirm = Some(vec!["proj-a".into()]);
        press(&mut app, KeyCode::Esc);
        assert!(app.sync_confirm.is_none());
    }

    #[test]
    fn modal_n_cancels_without_sync() {
        let config = base_config();
        let mut app = make_app(config, Vault::load_empty(), vec![]);
        app.sync_confirm = Some(vec!["proj-a".into()]);
        press(&mut app, KeyCode::Char('n'));
        assert!(app.sync_confirm.is_none());
    }

    #[test]
    fn modal_other_keys_do_not_dismiss_it() {
        let config = base_config();
        let mut app = make_app(config, Vault::load_empty(), vec![]);
        app.sync_confirm = Some(vec!["proj-a".into()]);
        press(&mut app, KeyCode::Char('q'));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Tab);
        assert!(app.sync_confirm.is_some(), "modal must remain open");
    }

    #[test]
    fn ctrl_u_without_modal_open_does_not_call_execute_sync_directly() {
        // When modal is shown, Ctrl+U should not re-open another modal on top.
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![project("guid-a", "proj-a", vec![env_var("X")])];
        let mut vault = Vault::load_empty();
        vault.entries.insert("X".into(), "xval".into());
        let mut app = make_app(config, vault, projects);
        // First Ctrl+U opens the modal
        app.trigger_sync();
        assert!(app.sync_confirm.is_some());
        // A second Ctrl+U while modal is open must be swallowed by modal handler, not re-trigger
        let ctrl_u =
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        app.handle_crossterm_event(ctrl_u);
        // 'u' is not y/Enter/Esc/n so modal stays open
        assert!(
            app.sync_confirm.is_some(),
            "modal must remain; Ctrl+U must not stack"
        );
    }

    // -----------------------------------------------------------------------
    // Filter — filter_matches
    // -----------------------------------------------------------------------

    #[test]
    fn filter_matches_empty_query_always_true() {
        let app = make_app(base_config(), Vault::load_empty(), vec![]);
        assert!(app.filter_matches("anything"));
        assert!(app.filter_matches(""));
    }

    #[test]
    fn filter_matches_literal_case_insensitive() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        app.filter_query = "jwt".into();
        assert!(app.filter_matches("JWT_SECRET"));
        assert!(app.filter_matches("jwt_secret"));
        assert!(app.filter_matches("MY_JWT_KEY"));
        assert!(!app.filter_matches("DB_PASS"));
    }

    #[test]
    fn filter_matches_regex_pattern() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        app.filter_query = "^DB_".into();
        assert!(app.filter_matches("DB_HOST"));
        assert!(app.filter_matches("DB_PASS"));
        assert!(!app.filter_matches("MY_DB_PASS"));
    }

    #[test]
    fn filter_matches_invalid_regex_falls_back_to_substring() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        app.filter_query = "[unclosed".into(); // invalid regex
        assert!(app.filter_matches("[unclosed bracket"));
        assert!(!app.filter_matches("something else"));
    }

    // -----------------------------------------------------------------------
    // Filter — filtered_count
    // -----------------------------------------------------------------------

    #[test]
    fn filtered_count_projects_no_filter() {
        let projects = vec![project("a", "alpha", vec![]), project("b", "beta", vec![])];
        let app = make_app(base_config(), Vault::load_empty(), projects);
        assert_eq!(app.filtered_count(), 2);
    }

    #[test]
    fn filtered_count_projects_with_filter() {
        let projects = vec![
            project("a", "alpha", vec![]),
            project("b", "beta", vec![]),
            project("c", "gamma", vec![]),
        ];
        let mut app = make_app(base_config(), Vault::load_empty(), projects);
        app.filter_query = "al".into();
        assert_eq!(app.filtered_count(), 1);
    }

    #[test]
    fn filtered_count_env_vars_with_filter() {
        let config = base_config();
        let mut app = make_app(config, Vault::load_empty(), vec![]);
        app.env_var_rows = vec![
            EnvVarRow {
                key: "DB_HOST".into(),
                vault_value: None,
            },
            EnvVarRow {
                key: "DB_PASS".into(),
                vault_value: None,
            },
            EnvVarRow {
                key: "JWT_SECRET".into(),
                vault_value: None,
            },
        ];
        app.page = Page::EnvVarList;
        app.filter_query = "DB".into();
        assert_eq!(app.filtered_count(), 2);
    }

    #[test]
    fn filtered_count_vault_with_filter() {
        let mut vault = Vault::load_empty();
        vault.entries.insert("DB_HOST".into(), "localhost".into());
        vault.entries.insert("JWT_SECRET".into(), "s3cr3t".into());
        let mut app = make_app(base_config(), vault, vec![]);
        app.page = Page::Vault;
        app.filter_query = "JWT".into();
        assert_eq!(app.filtered_count(), 1);
    }

    // -----------------------------------------------------------------------
    // Filter — key handling
    // -----------------------------------------------------------------------

    #[test]
    fn f_key_opens_filter_input() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        assert!(!app.filter_editing);
        press(&mut app, KeyCode::Char('f'));
        assert!(app.filter_editing);
    }

    #[test]
    fn typing_while_filter_editing_appends_to_query() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        press(&mut app, KeyCode::Char('f')); // open filter
        press(&mut app, KeyCode::Char('j')); // 'j' goes to query, not navigation
        press(&mut app, KeyCode::Char('w'));
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(app.filter_query, "jwt");
        assert_eq!(app.filter_selected, 0);
    }

    #[test]
    fn enter_closes_filter_input_keeps_query() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        press(&mut app, KeyCode::Char('f'));
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('b'));
        press(&mut app, KeyCode::Enter);
        assert!(!app.filter_editing);
        assert_eq!(app.filter_query, "db");
    }

    #[test]
    fn esc_closes_filter_input_keeps_query() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        press(&mut app, KeyCode::Char('f'));
        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Esc);
        assert!(!app.filter_editing);
        assert_eq!(app.filter_query, "x");
    }

    #[test]
    fn backspace_removes_last_char_and_resets_selection() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        press(&mut app, KeyCode::Char('f'));
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('b'));
        app.filter_selected = 3;
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.filter_query, "a");
        assert_eq!(app.filter_selected, 0);
    }

    #[test]
    fn shift_f_clears_filter() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        app.filter_query = "some_query".into();
        app.filter_selected = 2;
        press(&mut app, KeyCode::Char('F'));
        assert_eq!(app.filter_query, "");
        assert_eq!(app.filter_selected, 0);
        assert!(!app.filter_editing);
    }

    #[test]
    fn jk_navigate_filtered_list_when_filter_active() {
        let projects = vec![
            project("a", "alpha", vec![]),
            project("b", "bravo", vec![]),
            project("c", "gamma", vec![]),
        ];
        let mut app = make_app(base_config(), Vault::load_empty(), projects);
        // "lph" matches only "alpha" → 1 result; use "^[ab]" to match alpha+bravo → 2 results
        app.filter_query = "^[ab]".into(); // matches "alpha" and "bravo" (2 of 3)
        assert_eq!(app.filtered_count(), 2);
        assert_eq!(app.filter_selected, 0);

        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.filter_selected, 1);

        press(&mut app, KeyCode::Char('j')); // already at end of 2 matches
        assert_eq!(app.filter_selected, 1);

        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.filter_selected, 0);

        press(&mut app, KeyCode::Char('k')); // already at top
        assert_eq!(app.filter_selected, 0);
    }

    #[test]
    fn page_switch_clears_filter() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        app.filter_query = "jwt".into();
        app.filter_selected = 1;
        app.filter_editing = false;
        app.sidebar_focused = true;
        press(&mut app, KeyCode::Char('j')); // switch to next page in sidebar
        assert_eq!(app.filter_query, "");
        assert_eq!(app.filter_selected, 0);
    }

    #[test]
    fn f_second_press_opens_editing_to_show_current_filter() {
        let mut app = make_app(base_config(), Vault::load_empty(), vec![]);
        // Type a filter and close it
        press(&mut app, KeyCode::Char('f'));
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('b'));
        press(&mut app, KeyCode::Enter); // close — filter_editing = false, query = "db"
        assert!(!app.filter_editing);
        assert_eq!(app.filter_query, "db");
        // Press f again — should re-open editing so user can see/edit the query
        press(&mut app, KeyCode::Char('f'));
        assert!(app.filter_editing);
        assert_eq!(app.filter_query, "db"); // query preserved
    }

    // -----------------------------------------------------------------------
    // Theme config
    // -----------------------------------------------------------------------

    #[test]
    fn theme_defaults_to_inherit_when_absent() {
        let toml = r#"
server_url = ""
api_key = ""
vault_path = ""
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(
            config.theme,
            crate::ui::theme::ThemeVariant::Inherit
        ));
    }

    #[test]
    fn theme_onedark_parses_from_config() {
        let toml = r#"
server_url = ""
api_key = ""
vault_path = ""
theme = "onedark"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(
            config.theme,
            crate::ui::theme::ThemeVariant::OneDark
        ));
    }

    #[test]
    fn theme_sky_orange_parses_from_config() {
        let toml = r#"
server_url = ""
api_key = ""
vault_path = ""
theme = "sky-orange"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(
            config.theme,
            crate::ui::theme::ThemeVariant::SkyOrange
        ));
    }

    #[test]
    fn theme_roundtrips_through_toml() {
        let mut config = base_config();
        config.theme = crate::ui::theme::ThemeVariant::OneDark;
        let serialized = toml::to_string_pretty(&config).unwrap();
        let restored: Config = toml::from_str(&serialized).unwrap();
        assert!(matches!(
            restored.theme,
            crate::ui::theme::ThemeVariant::OneDark
        ));
    }

    // -----------------------------------------------------------------------
    // env_var_detail popup
    // -----------------------------------------------------------------------

    fn env_var_list_app() -> App {
        let mut config = base_config();
        config.included_projects = vec!["guid-a".into()];
        let projects = vec![
            project("guid-a", "proj-a", vec![env_var("FOO"), env_var("BAR")]),
            project("guid-b", "proj-b", vec![env_var("FOO")]),
        ];
        let mut vault = Vault::load_empty();
        vault.entries.insert("FOO".into(), "secret".into());
        let mut app = make_app(config, vault, projects);
        app.page = Page::EnvVarList;
        app.sidebar_focused = false;
        app.rebuild_env_var_rows();
        app
    }

    #[test]
    fn space_opens_env_var_detail_popup() {
        let mut app = env_var_list_app();
        assert!(app.env_var_detail.is_none());
        press(&mut app, KeyCode::Char(' '));
        assert!(app.env_var_detail.is_some());
    }

    #[test]
    fn enter_opens_env_var_detail_popup() {
        let mut app = env_var_list_app();
        press(&mut app, KeyCode::Enter);
        assert!(app.env_var_detail.is_some());
    }

    #[test]
    fn popup_key_contains_selected_row_key() {
        let mut app = env_var_list_app();
        let expected_key = app.env_var_rows[app.env_var_selected].key.clone();
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.env_var_detail.as_deref(), Some(expected_key.as_str()));
    }

    #[test]
    fn any_key_closes_env_var_detail_popup() {
        let mut app = env_var_list_app();
        press(&mut app, KeyCode::Char(' ')); // open
        assert!(app.env_var_detail.is_some());
        press(&mut app, KeyCode::Char('x')); // any key
        assert!(app.env_var_detail.is_none());
    }

    #[test]
    fn esc_closes_env_var_detail_popup_not_sidebar() {
        let mut app = env_var_list_app();
        press(&mut app, KeyCode::Char(' ')); // open
        press(&mut app, KeyCode::Esc); // close popup, NOT go to sidebar
        assert!(app.env_var_detail.is_none());
        // sidebar focus should NOT have changed — popup intercepted the key
        assert!(!app.sidebar_focused);
    }

    #[test]
    fn nav_still_works_when_popup_closed() {
        let mut app = env_var_list_app();
        let initial = app.env_var_selected;
        // Open and immediately close popup
        press(&mut app, KeyCode::Char(' '));
        press(&mut app, KeyCode::Char('x'));
        // Nav should still work
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.env_var_selected, initial + 1);
    }

    #[test]
    fn nav_blocked_while_popup_open() {
        let mut app = env_var_list_app();
        let initial = app.env_var_selected;
        press(&mut app, KeyCode::Char(' ')); // open popup
        press(&mut app, KeyCode::Char('j')); // this closes popup, not navigates
                                             // selection must be unchanged — j closed the popup
        assert_eq!(app.env_var_selected, initial);
        assert!(app.env_var_detail.is_none());
    }
}
