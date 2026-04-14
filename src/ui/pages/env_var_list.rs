use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
    Frame,
};
use crate::app::App;
use crate::ui::theme::*;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = !app.sidebar_focused;
    let border_style = if focused { style_accent() } else { style_border() };

    let header = Row::new(vec![
        Cell::from("Env Var Key").style(style_header()),
        Cell::from("Project").style(style_header()),
        Cell::from("Vault Value").style(style_header()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .env_var_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let selected = i == app.env_var_selected && focused;
            if selected {
                let vault_display = row
                    .vault_value
                    .as_deref()
                    .map(|v| if v.len() > 40 { &v[..40] } else { v })
                    .unwrap_or("[NOT IN VAULT]");
                Row::new(vec![
                    Cell::from(row.key.clone()).style(style_selected()),
                    Cell::from(row.project_name.clone()).style(style_selected()),
                    Cell::from(vault_display).style(style_selected()),
                ])
            } else {
                let val_style = if row.vault_value.is_some() {
                    style_normal()
                } else {
                    style_dim()
                };
                let vault_display = row
                    .vault_value
                    .as_deref()
                    .map(|v| if v.len() > 40 { &v[..40] } else { v })
                    .unwrap_or("[NOT IN VAULT]");
                Row::new(vec![
                    Cell::from(row.key.clone()).style(style_accent()),
                    Cell::from(row.project_name.clone()).style(style_dim()),
                    Cell::from(vault_display).style(val_style),
                ])
            }
        })
        .collect();

    let title = format!(" Env Vars ({}) ", app.env_var_rows.len());
    let hint = if focused { " [↑↓: navigate  ←/Esc: sidebar] " } else { "" };

    let block = Block::default()
        .title(Span::styled(title, style_header()))
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(hint, style_dim()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(COLOR_BG));

    let widths = [
        Constraint::Percentage(35),
        Constraint::Percentage(30),
        Constraint::Percentage(35),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .style(style_normal())
        .column_spacing(1);

    f.render_widget(table, area);
}
