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

    let filtering = !app.filter_query.is_empty();
    let filtered_rows: Vec<(usize, &crate::app::EnvVarRow)> = app
        .env_var_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| app.filter_matches(&r.key))
        .collect();

    let rows: Vec<Row> = filtered_rows
        .iter()
        .enumerate()
        .map(|(vis_i, (orig_i, row))| {
            let selected = if filtering {
                vis_i == app.filter_selected
            } else {
                *orig_i == app.env_var_selected
            } && focused;
            if let Some(val) = &row.vault_value {
                let key_style = if selected {
                    style_selected()
                } else {
                    style_normal()
                };
                let val_style = if selected {
                    style_selected()
                } else {
                    style_normal()
                };

                Row::new(vec![
                    Cell::from(row.key.clone()).style(key_style),
                    Cell::from(val.clone()).style(val_style),
                ])
                .height(1)
            } else {
                // Not in vault — single dim row
                let key_style = if selected {
                    style_selected()
                } else {
                    style_normal()
                };
                let val_style = if selected {
                    style_selected()
                } else {
                    style_dim()
                };
                Row::new(vec![
                    Cell::from(row.key.clone()).style(key_style),
                    Cell::from("[NOT IN VAULT]").style(val_style),
                ])
                .height(1)
            }
        })
        .collect();

    let title = if filtering {
        format!(
            " Env Vars ({}/{}) ",
            filtered_rows.len(),
            app.env_var_rows.len()
        )
    } else {
        format!(" Env Vars ({}) ", app.env_var_rows.len())
    };

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
    if filtering {
        if !filtered_rows.is_empty() {
            state.select(Some(app.filter_selected));
        }
    } else if !app.env_var_rows.is_empty() {
        state.select(Some(app.env_var_selected));
    }
    f.render_stateful_widget(table, area, &mut state);
}
