use crate::app::{App, Page};
use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.sidebar_focused;
    let border_style = if focused {
        app.palette.style_accent()
    } else {
        app.palette.style_border()
    };

    let pages = [
        Page::ProjectList,
        Page::EnvVarList,
        Page::Vault,
        Page::Settings,
    ];

    let items: Vec<ListItem> = pages
        .iter()
        .map(|page| {
            let is_current = *page == app.page;
            let arrow = if is_current && focused {
                "▶"
            } else if is_current {
                "·"
            } else {
                " "
            };
            let label = format!(" {} {}", arrow, page.label());
            let style = if is_current {
                app.palette.style_selected()
            } else {
                app.palette.style_normal()
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let block = Block::default()
        .title(Span::styled(" POSIT SECRETS ", app.palette.style_header()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(app.palette.block_bg());

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
