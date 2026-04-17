use crate::app::App;
use crate::ui::theme::*;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = !app.sidebar_focused;
    let border_style = if focused {
        style_accent()
    } else {
        style_border()
    };

    let header = Row::new(vec![
        Cell::from("Env Var Key").style(style_header()),
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
            let vault_display = row
                .vault_value
                .as_deref()
                .map(|v| if v.len() > 40 { &v[..40] } else { v })
                .unwrap_or("[NOT IN VAULT]");
            if selected {
                Row::new(vec![
                    Cell::from(row.key.clone()).style(style_selected()),
                    Cell::from(vault_display).style(style_selected()),
                ])
            } else {
                let val_style = if row.vault_value.is_some() {
                    style_normal()
                } else {
                    style_dim()
                };
                Row::new(vec![
                    Cell::from(row.key.clone()).style(style_normal()),
                    Cell::from(vault_display).style(val_style),
                ])
            }
        })
        .collect();

    let title = format!(" Env Vars ({}) ", app.env_var_rows.len());

    let block = Block::default()
        .title(Span::styled(title, style_header()))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(COLOR_BG));

    let widths = [Constraint::Percentage(40), Constraint::Percentage(60)];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .style(style_normal())
        .column_spacing(1);

    let mut state = TableState::default();
    if !app.env_var_rows.is_empty() {
        state.select(Some(app.env_var_selected));
    }
    f.render_stateful_widget(table, area, &mut state);
}
