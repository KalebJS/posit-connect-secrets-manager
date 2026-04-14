use crate::app::{App, LoadState};
use crate::ui::theme::*;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = !app.sidebar_focused;
    let border_style = if focused {
        style_accent()
    } else {
        style_border()
    };

    let mut items: Vec<ListItem> = Vec::new();
    if app.projects.is_empty() {
        let msg = match &app.load_state {
            LoadState::Loading => format!("  {} Fetching projects…", app.spinner()),
            LoadState::Error(e) => format!("  ✗ Error: {}", e),
            LoadState::Idle => "  No projects loaded. Press Ctrl+P to refresh.".to_string(),
        };
        items.push(ListItem::new(Line::from(Span::styled(msg, style_dim()))));
    }

    for (flat_idx, project) in app.projects.iter().enumerate() {
        let is_selected = flat_idx == app.project_list_selected;
        let is_expanded = app.project_expanded.contains(&project.guid);
        let expand_icon = if is_expanded { "▾" } else { "▸" };
        let display_name = project.title.as_deref().unwrap_or(&project.name);

        let loading_badge = match &project.load_state {
            LoadState::Loading => format!(" {}", app.spinner()),
            LoadState::Error(_) => " [!]".to_string(),
            LoadState::Idle => String::new(),
        };

        let guid_short: String = project.guid.chars().take(8).collect();
        let label = format!(
            "  {} {}  {}{}",
            expand_icon, display_name, guid_short, loading_badge
        );

        let style = if is_selected && focused {
            style_selected()
        } else {
            style_normal()
        };
        items.push(ListItem::new(Line::from(Span::styled(label, style))));

        if is_expanded {
            if project.env_vars.is_empty() && matches!(project.load_state, LoadState::Idle) {
                items.push(ListItem::new(Line::from(Span::styled(
                    "      (no env vars)",
                    style_dim(),
                ))));
            }
            for var in &project.env_vars {
                let in_vault = app.vault.get(&var.name).is_some();
                let (dot, suffix, style) = if in_vault {
                    let val = app.vault.get(&var.name).unwrap_or("");
                    let truncated = if val.len() > 28 {
                        format!("{}…", &val[..28])
                    } else {
                        val.to_string()
                    };
                    (
                        "●",
                        format!(" = {}", truncated),
                        Style::default().fg(COLOR_SUCCESS).bg(COLOR_BG),
                    )
                } else {
                    ("○", "  [NOT IN VAULT]".to_string(), style_dim())
                };
                let line = format!("      {} {}{}", dot, var.name, suffix);
                items.push(ListItem::new(Line::from(Span::styled(line, style))));
            }
        }
    }

    let title = format!(" Projects ({}) ", app.projects.len());
    let hint = if focused {
        " [Enter: expand  ←/Esc: sidebar] "
    } else {
        ""
    };

    let block = Block::default()
        .title(Span::styled(title, style_header()))
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(hint, style_dim()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(COLOR_BG));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
