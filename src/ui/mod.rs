pub mod pages;
pub mod sidebar;
pub mod status_bar;
pub mod theme;

use crate::app::{App, Page};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Split: [body] / [status bar]
    let main_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    // Split body: [sidebar] | [content]
    let body_chunks =
        Layout::horizontal([Constraint::Length(22), Constraint::Min(0)]).split(main_chunks[0]);

    sidebar::render(f, app, body_chunks[0]);

    let show_filter = !app.sidebar_focused && (app.filter_editing || !app.filter_query.is_empty());
    let (page_area, filter_area) = if show_filter {
        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(body_chunks[1]);
        (chunks[0], Some(chunks[1]))
    } else {
        (body_chunks[1], None)
    };

    match app.page {
        Page::ProjectList => pages::project_list::render(f, app, page_area),
        Page::EnvVarList => pages::env_var_list::render(f, app, page_area),
        Page::Vault => pages::vault::render(f, app, page_area),
        Page::Settings => pages::settings::render(f, app, page_area),
    }

    if let Some(area) = filter_area {
        render_filter_bar(f, app, area);
    }

    status_bar::render(f, app, main_chunks[1]);

    // Render sync confirmation modal on top if active
    if let Some(names) = &app.sync_confirm.clone() {
        render_sync_modal(f, app, area, names);
    }

    // Render add-var popup on top if active
    if app.add_var_popup.is_some() {
        render_add_var_popup(f, app, area);
    }

    // Render env var detail popup on top if active
    if app.env_var_detail.is_some() {
        render_env_var_detail_popup(f, app, area);
    }
}

fn render_add_var_popup(f: &mut Frame, app: &App, area: Rect) {
    let Some(popup) = &app.add_var_popup else {
        return;
    };
    let project_name = app
        .projects
        .iter()
        .find(|p| p.guid == popup.guid)
        .map(|p| p.title.as_deref().unwrap_or(&p.name))
        .unwrap_or("Project");
    let suggestions = app.add_var_suggestions();

    const POPUP_W: u16 = 56;
    let visible = suggestions.len().min(8) as u16;
    let popup_h = (3 + visible).max(5);
    let x = area.x + area.width.saturating_sub(POPUP_W) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect {
        x,
        y,
        width: POPUP_W.min(area.width),
        height: popup_h.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let title = format!(" Add Env Var → {} ", project_name);
    let block = Block::default()
        .title(Span::styled(title, app.palette.style_header()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(app.palette.style_accent())
        .style(app.palette.block_bg());
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    let input = Paragraph::new(Span::styled(
        format!(" > {}█", popup.query),
        app.palette.style_normal(),
    ));
    f.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = suggestions
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let style = if i == popup.selected {
                app.palette.style_selected()
            } else {
                app.palette.style_normal()
            };
            ListItem::new(Line::from(Span::styled(format!("  {}", key), style)))
        })
        .collect();
    let list = List::new(items);
    let mut state = ListState::default();
    if !suggestions.is_empty() {
        state.select(Some(popup.selected));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);
}

fn render_filter_bar(f: &mut Frame, app: &App, area: Rect) {
    let cursor = if app.filter_editing { "█" } else { "" };
    let label = format!(" / {}{}", app.filter_query, cursor);
    let style = if app.filter_editing {
        app.palette.style_accent()
    } else {
        app.palette.style_dim()
    };
    let para = Paragraph::new(Span::styled(label, style));
    f.render_widget(para, area);
}

fn render_sync_modal(f: &mut Frame, app: &App, area: Rect, names: &[String]) {
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
        .title(Span::styled(" Confirm Sync ", app.palette.style_header()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(app.palette.style_accent())
        .style(app.palette.block_bg());

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  Syncing vault values to {} project(s):", names.len()),
        app.palette.style_normal(),
    )));
    lines.push(Line::from(""));
    for name in names.iter().take(10) {
        lines.push(Line::from(Span::styled(
            format!("    • {}", name),
            app.palette.style_normal(),
        )));
    }
    if names.len() > 10 {
        lines.push(Line::from(Span::styled(
            format!("    … and {} more", names.len() - 10),
            app.palette.style_dim(),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter/y: Confirm    Esc/n: Cancel",
        app.palette.style_dim(),
    )));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup_area);
}

fn render_env_var_detail_popup(f: &mut Frame, app: &App, area: Rect) {
    let Some(key) = &app.env_var_detail else {
        return;
    };

    // Find all projects that have this env var
    let matches: Vec<(String, bool)> = app
        .projects
        .iter()
        .filter(|p| p.env_vars.iter().any(|v| &v.name == key))
        .map(|p| {
            let name = p.title.as_deref().unwrap_or(&p.name).to_string();
            let included = app.config.included_projects.contains(&p.guid);
            (name, included)
        })
        .collect();

    const POPUP_W: u16 = 58;
    let body_lines = matches.len().max(1) as u16;
    let popup_h = (4 + body_lines).min(16);

    let x = area.x + area.width.saturating_sub(POPUP_W) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect {
        x,
        y,
        width: POPUP_W.min(area.width),
        height: popup_h.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let title = format!(" Projects using: {} ", key);
    let block = Block::default()
        .title(Span::styled(title, app.palette.style_header()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(app.palette.style_accent())
        .style(app.palette.block_bg());

    let max_body = (popup_h as usize).saturating_sub(3);
    let mut lines: Vec<Line> = Vec::new();

    if matches.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no projects have this var)",
            app.palette.style_dim(),
        )));
    } else {
        for (name, included) in matches.iter().take(max_body.saturating_sub(1)) {
            let marker = if *included { "[✓]" } else { "[ ]" };
            let style = if *included {
                app.palette.style_normal()
            } else {
                app.palette.style_dim()
            };
            lines.push(Line::from(Span::styled(
                format!("  {} {}", marker, name),
                style,
            )));
        }
        if matches.len() > max_body.saturating_sub(1) {
            lines.push(Line::from(Span::styled(
                format!(
                    "  … and {} more",
                    matches.len() - max_body.saturating_sub(1)
                ),
                app.palette.style_dim(),
            )));
        }
    }

    lines.push(Line::from(Span::styled(
        "  any key to close",
        app.palette.style_dim(),
    )));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup_area);
}
