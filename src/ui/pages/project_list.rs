use crate::app::{App, LoadState};
use crate::ui::mask_value;
use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = !app.sidebar_focused;
    let border_style = if focused {
        app.palette.style_accent()
    } else {
        app.palette.style_border()
    };

    let mut items: Vec<ListItem> = Vec::new();
    if app.projects.is_empty() {
        let msg = match &app.load_state {
            LoadState::Loading => format!("  {} Fetching projects…", app.spinner()),
            LoadState::Error(e) => format!("  ✗ Error: {}", e),
            LoadState::Idle => "  No projects loaded. Press Ctrl+P to refresh.".to_string(),
        };
        items.push(ListItem::new(Line::from(Span::styled(
            msg,
            app.palette.style_dim(),
        ))));
    }

    let filtering = !app.filter_query.is_empty();
    let mut filter_visible_idx = 0usize;

    for (flat_idx, project) in app.projects.iter().enumerate() {
        if filtering {
            let display_name = project.title.as_deref().unwrap_or(&project.name);
            if !app.filter_matches(display_name) {
                continue;
            }
        }
        let is_selected = if filtering {
            let matched = filter_visible_idx == app.filter_selected;
            filter_visible_idx += 1;
            matched
        } else {
            flat_idx == app.project_list_selected
        };
        let _ = flat_idx; // suppress unused warning when filtering
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
            app.palette.style_selected()
        } else if is_whitelisted {
            app.palette.style_normal()
        } else {
            app.palette.style_dim()
        };
        items.push(ListItem::new(Line::from(Span::styled(label, style))));

        if is_expanded {
            let url = format!(
                "      {} {}/connect/#/apps/{}",
                "🔗",
                app.config.server_url.trim_end_matches('/'),
                project.guid
            );
            items.push(ListItem::new(Line::from(Span::styled(
                url,
                app.palette.style_dim(),
            ))));

            if project.env_vars.is_empty() && matches!(project.load_state, LoadState::Idle) {
                items.push(ListItem::new(Line::from(Span::styled(
                    "      (no env vars)",
                    app.palette.style_dim(),
                ))));
            }
            let excl_vars = app.config.excluded_vars.get(&project.guid);
            for (var_idx, var) in project.env_vars.iter().enumerate() {
                let is_var_excluded = excl_vars.is_some_and(|ev| ev.contains(&var.name));
                let cursor_on_var =
                    is_selected && focused && app.project_var_selected == Some(var_idx);

                let in_vault = app.vault.get(&var.name).is_some();
                let (dot, suffix, base_style) = if !is_whitelisted {
                    // Project won't sync — dim all vars regardless of vault status
                    let val_hint = if in_vault {
                        let val = app.vault.get(&var.name).unwrap_or("");
                        format!(" = {}", mask_value(val))
                    } else {
                        "  [NOT IN VAULT]".to_string()
                    };
                    (
                        if in_vault { "●" } else { "○" },
                        val_hint,
                        app.palette.style_dim(),
                    )
                } else if is_var_excluded {
                    (
                        "○",
                        "  [EXCLUDED FROM SYNC]".to_string(),
                        app.palette.style_dim(),
                    )
                } else if in_vault {
                    let val = app.vault.get(&var.name).unwrap_or("");
                    (
                        "●",
                        format!(" = {}", mask_value(val)),
                        app.palette.style_success(),
                    )
                } else {
                    ("○", "  [NOT IN VAULT]".to_string(), app.palette.style_dim())
                };

                let line = format!("      {} {}{}", dot, var.name, suffix);
                let style = if cursor_on_var {
                    app.palette.style_selected()
                } else {
                    base_style
                };
                items.push(ListItem::new(Line::from(Span::styled(line, style))));
            }
        }
    }

    let title = format!(" Projects ({}) ", app.projects.len());

    let block = Block::default()
        .title(Span::styled(title, app.palette.style_header()))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(app.palette.block_bg());

    // Compute the flat list index of the highlighted row (project or var)
    let mut flat_selected = 0usize;
    if filtering {
        // Walk only visible (filtered) projects; filter_selected is position among them
        let mut vis = 0usize;
        for project in app.projects.iter() {
            let display_name = project.title.as_deref().unwrap_or(&project.name);
            if !app.filter_matches(display_name) {
                continue;
            }
            if vis == app.filter_selected {
                break;
            }
            flat_selected += 1; // project row
            if app.project_expanded.contains(&project.guid) {
                let sub_rows = if project.env_vars.is_empty()
                    && matches!(project.load_state, LoadState::Idle)
                {
                    2
                } else {
                    1 + project.env_vars.len()
                };
                flat_selected += sub_rows;
            }
            vis += 1;
        }
    } else {
        for (idx, project) in app.projects.iter().enumerate() {
            if idx == app.project_list_selected {
                // Add offset for sub-selected var (url row always precedes vars)
                if let Some(var_idx) = app.project_var_selected {
                    flat_selected += 1 + 1 + var_idx; // project row + url row + var offset
                }
                break;
            }
            flat_selected += 1; // the project row itself
            if app.project_expanded.contains(&project.guid) {
                let sub_rows = if project.env_vars.is_empty()
                    && matches!(project.load_state, LoadState::Idle)
                {
                    2 // url row + "(no env vars)" placeholder
                } else {
                    1 + project.env_vars.len() // url row + vars
                };
                flat_selected += sub_rows;
            }
        }
    }

    let list = List::new(items).block(block);
    let mut state = ListState::default();
    if !app.projects.is_empty() {
        state.select(Some(flat_selected));
    }
    f.render_stateful_widget(list, area, &mut state);
}
