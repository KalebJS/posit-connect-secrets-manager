use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = !app.sidebar_focused;
    let border_style = if focused {
        app.palette.style_accent()
    } else {
        app.palette.style_border()
    };

    let outer_block = Block::default()
        .title(Span::styled(" Settings ", app.palette.style_header()))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(app.palette.block_bg());

    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(inner);

    // Field definitions: (label, value, masked)
    let fields: &[(&str, &str, bool)] = &[
        ("Server URL", &app.config.server_url, false),
        ("API Key", &app.config.api_key, true),
        ("Vault File Path", &app.config.vault_path, false),
    ];

    for (i, (label, value, masked)) in fields.iter().enumerate() {
        let is_selected = i == app.settings_selected && focused;
        let is_editing = app.settings_editing && is_selected;

        let display = if is_editing {
            format!("{}█", app.settings_edit_buffer)
        } else if *masked && !value.is_empty() {
            "●".repeat(value.len().min(32))
        } else {
            (*value).to_string()
        };

        let (label_style, value_style, field_border) = if is_selected {
            (
                app.palette.style_accent(),
                if is_editing {
                    app.palette.style_selected()
                } else {
                    app.palette.style_normal()
                },
                app.palette.style_accent(),
            )
        } else {
            (
                app.palette.style_dim(),
                app.palette.style_normal(),
                app.palette.style_border(),
            )
        };

        let block = Block::default()
            .title(Span::styled(format!(" {} ", label), label_style))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(field_border)
            .style(Style::default());

        let para = Paragraph::new(Line::from(Span::styled(
            format!(" {}", display),
            value_style,
        )))
        .block(block);

        f.render_widget(para, chunks[i]);
    }

    // Last refresh (read-only)
    let refresh_val = app.config.last_refresh.as_deref().unwrap_or("Never");
    let refresh_block = Block::default()
        .title(Span::styled(" Last Refresh ", app.palette.style_dim()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(app.palette.style_dim())
        .style(Style::default());
    let refresh_para = Paragraph::new(Line::from(Span::styled(
        format!(" {}", refresh_val),
        app.palette.style_dim(),
    )))
    .block(refresh_block);
    f.render_widget(refresh_para, chunks[3]);
}
