use anyhow::Result;
use crossterm::{
    event::EventStream,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::time::{interval, Duration};

mod api;
mod app;
mod config;
mod error;
mod ui;
mod vault;

use app::{App, EditorTarget};

/// RAII guard — restores the terminal even on panic
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut app = App::new()?;
    app.check_auto_refresh();

    let mut event_stream = EventStream::new();
    let mut tick = interval(Duration::from_millis(250));

    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        tokio::select! {
            maybe_event = event_stream.next() => {
                if let Some(Ok(event)) = maybe_event {
                    app.handle_crossterm_event(event);
                }
            }
            maybe_msg = app.rx.recv() => {
                if let Some(msg) = maybe_msg {
                    app.handle_app_event(msg);
                }
            }
            _ = tick.tick() => {
                app.on_tick();
            }
        }

        // Open external editor if a key handler requested it
        if let Some(target) = app.open_editor_for.take() {
            // Restore terminal so the editor can use the full screen
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);

            match target {
                EditorTarget::VaultEntry(key) => {
                    let current = app.vault.get(&key).unwrap_or("").to_string();

                    let tmp_path = std::env::temp_dir()
                        .join(format!("posit-secrets-{}.tmp", std::process::id()));
                    if std::fs::write(&tmp_path, &current).is_ok() {
                        let editor = std::env::var("EDITOR")
                            .or_else(|_| std::env::var("VISUAL"))
                            .unwrap_or_else(|_| "vi".to_string());
                        let _ = std::process::Command::new(&editor).arg(&tmp_path).status();
                        if let Ok(new_value) = std::fs::read_to_string(&tmp_path) {
                            // Strip a single trailing newline that editors typically append
                            let new_value = new_value
                                .strip_suffix('\n')
                                .unwrap_or(&new_value)
                                .to_string();
                            app.vault.entries.insert(key, new_value);
                            app.vault.dirty = true;
                            let _ = app.vault.save();
                            app.rebuild_env_var_rows();
                        }
                        let _ = std::fs::remove_file(&tmp_path);
                    }
                }
                EditorTarget::ProjectVar { guid, var_name } => {
                    let current = app.vault.get(&var_name).unwrap_or("").to_string();

                    let tmp_path = std::env::temp_dir()
                        .join(format!("posit-secrets-{}.tmp", std::process::id()));
                    if std::fs::write(&tmp_path, &current).is_ok() {
                        let editor = std::env::var("EDITOR")
                            .or_else(|_| std::env::var("VISUAL"))
                            .unwrap_or_else(|_| "vi".to_string());
                        let _ = std::process::Command::new(&editor).arg(&tmp_path).status();
                        if let Ok(new_value) = std::fs::read_to_string(&tmp_path) {
                            // Strip a single trailing newline that editors typically append
                            let new_value = new_value
                                .strip_suffix('\n')
                                .unwrap_or(&new_value)
                                .to_string();
                            app.project_var_confirm = Some(app::ProjectVarConfirm {
                                guid,
                                var_name,
                                new_value,
                            });
                        }
                        let _ = std::fs::remove_file(&tmp_path);
                    }
                }
            }

            // Re-initialize the terminal
            let _ = enable_raw_mode();
            let _ = execute!(io::stdout(), EnterAlternateScreen);
            terminal.clear()?;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
