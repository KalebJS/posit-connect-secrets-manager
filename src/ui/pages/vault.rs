use crate::app::{App, VaultField};
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
        Cell::from("Key").style(style_header()),
        Cell::from("Value").style(style_header()),
    ])
    .height(1)
    .bottom_margin(1);

    let editing_idx = app.vault_editing;

    let rows: Vec<Row> = app
        .vault
        .entries
        .iter()
        .enumerate()
        .map(|(i, (k, v))| {
            let is_selected = i == app.vault_selected && focused;
            let is_editing = editing_idx == Some(i);

            if is_editing {
                match app.vault_edit_field {
                    VaultField::Key => Row::new(vec![
                        Cell::from(format!("{}█", app.vault_edit_buffer)).style(style_selected()),
                        Cell::from(v.clone()).style(style_normal()),
                    ]),
                    VaultField::Value => Row::new(vec![
                        Cell::from(k.clone()).style(style_normal()),
                        Cell::from(format!("{}█", app.vault_edit_buffer)).style(style_selected()),
                    ]),
                }
            } else if is_selected {
                Row::new(vec![
                    Cell::from(k.clone()).style(style_selected()),
                    Cell::from(v.clone()).style(style_selected()),
                ])
            } else {
                Row::new(vec![
                    Cell::from(k.clone()).style(style_normal()),
                    Cell::from(v.clone()).style(style_normal()),
                ])
            }
        })
        .collect();

    let dirty = if app.vault.dirty { " ●" } else { "" };
    let title = format!(" Vault ({} entries){} ", app.vault.entries.len(), dirty);

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
    if !app.vault.entries.is_empty() {
        state.select(Some(app.vault_selected));
    }
    f.render_stateful_widget(table, area, &mut state);
}
