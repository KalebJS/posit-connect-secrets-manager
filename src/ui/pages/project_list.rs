use crate::app::{App, LoadState};
use crate::ui::theme::*;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
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
        let is_whitelisted = app.config.included_projects.contains(&project.guid);
        let expand_icon = if is_expanded { "▾" } else { "▸" };
        let display_name = project.title.as_deref().unwrap_or(&project.name);

        let loading_badge = match &project.load_state {
            LoadState::Loading => format!(" {}", app.spinner()),
            LoadState::Error(_) => " [!]".to_string(),
            LoadState::Idle => String::new(),
        };

        let guid_short: String = project.guid.chars().take(8).collect();
        let sync_marker = if is_whitelisted { "[✓]" } else { "[ ]" };
        let label = format!(
            "  {} {} {}  {}{}",
            expand_icon, sync_marker, display_name, guid_short, loading_badge
        );

        // Cursor is on this project row (not on a var sub-row)
        let cursor_on_project = is_selected && app.project_var_selected.is_none();
        let style = if cursor_on_project && focused {
            style_selected()
        } else if is_whitelisted {
            style_normal()
        } else {
            style_dim()
        };
        items.push(ListItem::new(Line::from(Span::styled(label, style))));

        if is_expanded {
            if project.env_vars.is_empty() && matches!(project.load_state, LoadState::Idle) {
                items.push(ListItem::new(Line::from(Span::styled(
                    "      (no env vars)",
                    style_dim(),
                ))));
            }
            let excl_vars = app.config.excluded_vars.get(&project.guid);
            for (var_idx, var) in project.env_vars.iter().enumerate() {
                let is_var_excluded = excl_vars.is_some_and(|ev| ev.contains(&var.name));
                let cursor_on_var =
                    is_selected && focused && app.project_var_selected == Some(var_idx);

                let in_vault = app.vault.get(&var.name).is_some();
                let (dot, suffix, base_style) = if is_var_excluded {
                    ("[x]", "  [EXCLUDED FROM SYNC]".to_string(), style_dim())
                } else if in_vault {
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
                let style = if cursor_on_var {
                    style_selected()
                } else {
                    base_style
                };
                items.push(ListItem::new(Line::from(Span::styled(line, style))));
            }
        }
    }

    let title = format!(" Projects ({}) ", app.projects.len());

    let block = Block::default()
        .title(Span::styled(title, style_header()))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(COLOR_BG));

    // Compute the flat list index of the highlighted row (project or var)
    let mut flat_selected = 0usize;
    for (idx, project) in app.projects.iter().enumerate() {
        if idx == app.project_list_selected {
            // Add offset for sub-selected var
            if let Some(var_idx) = app.project_var_selected {
                flat_selected += 1 + var_idx; // project row + var offset
            }
            break;
        }
        flat_selected += 1; // the project row itself
        if app.project_expanded.contains(&project.guid) {
            let sub_rows =
                if project.env_vars.is_empty() && matches!(project.load_state, LoadState::Idle) {
                    1 // "(no env vars)" placeholder
                } else {
                    project.env_vars.len()
                };
            flat_selected += sub_rows;
        }
    }

    let list = List::new(items).block(block);
    let mut state = ListState::default();
    if !app.projects.is_empty() {
        state.select(Some(flat_selected));
    }
    f.render_stateful_widget(list, area, &mut state);
}
