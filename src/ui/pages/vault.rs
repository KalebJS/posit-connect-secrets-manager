use crate::app::{App, VaultField};
use crate::ui::theme::*;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
    Frame,
};

/// Splits `s` into chunks of at most `width` characters for display.
fn wrap_at(s: &str, width: usize) -> Vec<String> {
    if width < 4 || s.is_empty() {
        return vec![s.to_string()];
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return vec![s.to_string()];
    }
    chars.chunks(width).map(|c| c.iter().collect()).collect()
}

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

    // Compute approximate value-column width for wrapping.
    // Layout: 2 border chars + 1 column-spacing = 3 overhead; value col is ~60% of the rest.
    let val_col_width = (area.width.saturating_sub(3) as usize * 60 / 100).max(10);

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
                // Editing row: single-line, show inline cursor
                match app.vault_edit_field {
                    VaultField::Key => Row::new(vec![
                        Cell::from(format!("{}█", app.vault_edit_buffer)).style(style_selected()),
                        Cell::from(v.clone()).style(style_normal()),
                    ])
                    .height(1),
                    VaultField::Value => Row::new(vec![
                        Cell::from(k.clone()).style(style_normal()),
                        Cell::from(format!("{}█", app.vault_edit_buffer)).style(style_selected()),
                    ])
                    .height(1),
                }
            } else {
                let key_style = if is_selected {
                    style_selected()
                } else {
                    style_normal()
                };
                let val_style = if is_selected {
                    style_selected()
                } else {
                    style_normal()
                };

                let wrapped = wrap_at(v, val_col_width);
                let height = wrapped.len() as u16;
                let val_text = Text::from(
                    wrapped
                        .iter()
                        .map(|l| Line::from(Span::styled(l.clone(), val_style)))
                        .collect::<Vec<_>>(),
                );
                Row::new(vec![
                    Cell::from(k.clone()).style(key_style),
                    Cell::from(val_text),
                ])
                .height(height)
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
