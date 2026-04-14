use super::theme::*;
use crate::app::{App, LoadState, StatusLevel};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let line = if let Some((msg, level, _)) = &app.status_message {
        let style = match level {
            StatusLevel::Info => style_normal(),
            StatusLevel::Success => style_success(),
            StatusLevel::Error => style_error(),
        };
        Line::from(Span::styled(format!("  {}", msg), style))
    } else {
        let spinner = match &app.load_state {
            LoadState::Loading => format!("{} ", app.spinner()),
            _ => "  ".to_string(),
        };
        Line::from(vec![
            Span::styled(spinner, style_accent()),
            Span::styled(
                "Tab:Focus  ↑↓:Nav  Enter:Select  e:Edit  n:New  d:Del  Ctrl+P:Refresh  Ctrl+U:Sync  Ctrl+C:Quit",
                style_dim(),
            ),
        ])
    };

    let widget = Paragraph::new(line).style(Style::default().bg(COLOR_BG));
    f.render_widget(widget, area);
}
