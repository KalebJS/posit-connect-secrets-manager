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

use app::App;

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

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
