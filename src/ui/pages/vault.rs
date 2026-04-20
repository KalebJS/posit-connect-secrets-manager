use crate::app::{App, VaultField};
use crate::ui::mask_value;
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
        app.palette.style_accent()
    } else {
        app.palette.style_border()
    };

    let header = Row::new(vec![
        Cell::from("Key").style(app.palette.style_header()),
        Cell::from("Value").style(app.palette.style_header()),
    ])
    .height(1)
    .bottom_margin(1);

    let editing_idx = app.vault_editing;
    let filtering = !app.filter_query.is_empty();

    let filtered_entries: Vec<(usize, &String, &String)> = app
        .vault
        .entries
        .iter()
        .enumerate()
        .filter(|(_, (k, _))| app.filter_matches(k))
        .map(|(i, (k, v))| (i, k, v))
        .collect();

    let rows: Vec<Row> = if filtering {
        filtered_entries
            .iter()
            .enumerate()
            .map(|(vis_i, (orig_i, k, v))| {
                let is_selected = vis_i == app.filter_selected && focused;
                let is_editing = editing_idx == Some(*orig_i);
                vault_row(is_selected, is_editing, k, v, app)
            })
            .collect()
    } else {
        app.vault
            .entries
            .iter()
            .enumerate()
            .map(|(i, (k, v))| {
                let is_selected = i == app.vault_selected && focused;
                let is_editing = editing_idx == Some(i);
                vault_row(is_selected, is_editing, k, v, app)
            })
            .collect()
    };
    let dirty = if app.vault.dirty { " ●" } else { "" };
    let title = if filtering {
        format!(
            " Vault ({}/{} entries){} ",
            filtered_entries.len(),
            app.vault.entries.len(),
            dirty
        )
    } else {
        format!(" Vault ({} entries){} ", app.vault.entries.len(), dirty)
    };

    let block = Block::default()
        .title(Span::styled(title, app.palette.style_header()))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(app.palette.block_bg());

    let widths = [Constraint::Percentage(40), Constraint::Percentage(60)];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .style(Style::default())
        .column_spacing(1);

    let mut state = TableState::default();
    if filtering {
        if !filtered_entries.is_empty() {
            state.select(Some(app.filter_selected));
        }
    } else if !app.vault.entries.is_empty() {
        state.select(Some(app.vault_selected));
    }
    f.render_stateful_widget(table, area, &mut state);
}

fn vault_row<'a>(
    is_selected: bool,
    is_editing: bool,
    k: &'a str,
    v: &'a str,
    app: &'a App,
) -> Row<'a> {
    if is_editing {
        match app.vault_edit_field {
            VaultField::Key => Row::new(vec![
                Cell::from(format!("{}█", app.vault_edit_buffer))
                    .style(app.palette.style_selected()),
                Cell::from(mask_value(v)).style(app.palette.style_normal()),
            ])
            .height(1),
            VaultField::Value => Row::new(vec![
                Cell::from(k.to_string()).style(app.palette.style_normal()),
                Cell::from(format!("{}█", app.vault_edit_buffer))
                    .style(app.palette.style_selected()),
            ])
            .height(1),
        }
    } else {
        let key_style = if is_selected {
            app.palette.style_selected()
        } else {
            app.palette.style_normal()
        };
        let val_style = if is_selected {
            app.palette.style_selected()
        } else {
            app.palette.style_normal()
        };
        Row::new(vec![
            Cell::from(k.to_string()).style(key_style),
            Cell::from(mask_value(v)).style(val_style),
        ])
        .height(1)
    }
}
