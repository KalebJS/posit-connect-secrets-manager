pub mod pages;
pub mod sidebar;
pub mod status_bar;
pub mod theme;

use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
    Frame,
};
use theme::COLOR_BG;
use crate::app::{App, Page};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(COLOR_BG)), area);

    // Split: [body] / [status bar]
    let main_chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    // Split body: [sidebar] | [content]
    let body_chunks = Layout::horizontal([
        Constraint::Length(22),
        Constraint::Min(0),
    ])
    .split(main_chunks[0]);

    sidebar::render(f, app, body_chunks[0]);

    match app.page {
        Page::ProjectList => pages::project_list::render(f, app, body_chunks[1]),
        Page::EnvVarList => pages::env_var_list::render(f, app, body_chunks[1]),
        Page::Vault => pages::vault::render(f, app, body_chunks[1]),
        Page::Settings => pages::settings::render(f, app, body_chunks[1]),
    }

    status_bar::render(f, app, main_chunks[1]);
}
