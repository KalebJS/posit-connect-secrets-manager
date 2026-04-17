pub mod pages;
pub mod sidebar;
pub mod status_bar;
pub mod theme;

use crate::app::{App, Page};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use theme::*;

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(COLOR_BG)), area);

    // Split: [body] / [status bar]
    let main_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    // Split body: [sidebar] | [content]
    let body_chunks =
        Layout::horizontal([Constraint::Length(22), Constraint::Min(0)]).split(main_chunks[0]);

    sidebar::render(f, app, body_chunks[0]);

    match app.page {
        Page::ProjectList => pages::project_list::render(f, app, body_chunks[1]),
        Page::EnvVarList => pages::env_var_list::render(f, app, body_chunks[1]),
        Page::Vault => pages::vault::render(f, app, body_chunks[1]),
        Page::Settings => pages::settings::render(f, app, body_chunks[1]),
    }

    status_bar::render(f, app, main_chunks[1]);

    // Render sync confirmation modal on top if active
    if let Some(names) = &app.sync_confirm.clone() {
        render_sync_modal(f, area, names);
    }
}

fn render_sync_modal(f: &mut Frame, area: Rect, names: &[String]) {
    const POPUP_W: u16 = 52;
    // Header + blank + count line + blank + up to 10 project lines + blank + footer + blank
    let visible = names.len().min(10) as u16;
    let popup_h = 6 + visible;

    let x = area.x + area.width.saturating_sub(POPUP_W) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect {
        x,
        y,
        width: POPUP_W.min(area.width),
        height: popup_h.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(" Confirm Sync ", style_header()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(style_accent())
        .style(Style::default().bg(COLOR_BG));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  Syncing vault values to {} project(s):", names.len()),
        style_normal(),
    )));
    lines.push(Line::from(""));
    for name in names.iter().take(10) {
        lines.push(Line::from(Span::styled(
            format!("    • {}", name),
            style_normal(),
        )));
    }
    if names.len() > 10 {
        lines.push(Line::from(Span::styled(
            format!("    … and {} more", names.len() - 10),
            style_dim(),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter/y: Confirm    Esc/n: Cancel",
        style_dim(),
    )));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup_area);
}
