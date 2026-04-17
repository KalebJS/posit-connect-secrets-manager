use super::theme::*;
use crate::app::{App, LoadState, Page, StatusLevel};
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
        let hints = if app.sidebar_focused {
            "Tab:Content  ↑↓:Nav  Ctrl+P:Refresh  Ctrl+C:Quit"
        } else {
            match app.page {
                Page::ProjectList => "Tab:Sidebar  ↑↓:Nav  Enter/Space:Expand  x:Toggle  a:AddVar  d:Del  ←/Esc:Sidebar  Ctrl+P:Refresh  Ctrl+C:Quit",
                Page::EnvVarList  => "Tab:Sidebar  ↑↓:Nav  e/E:Editor  ←/Esc:Sidebar  Ctrl+C:Quit",
                Page::Vault if app.vault_editing.is_some() => "Enter:Save  Esc:Cancel",
                Page::Vault       => "Tab:Sidebar  ↑↓:Nav  e/Enter:Edit  E:Editor  n:New  d:Del  ←/Esc:Sidebar  Ctrl+U:Sync  Ctrl+C:Quit",
                Page::Settings if app.settings_editing => "Enter:Confirm  Esc:Cancel",
                Page::Settings    => "Tab:Sidebar  ↑↓:Nav  Enter/e:Edit  ←/Esc:Sidebar  Ctrl+C:Quit",
            }
        };
        Line::from(vec![
            Span::styled(spinner, style_accent()),
            Span::styled(hints, style_dim()),
        ])
    };

    let widget = Paragraph::new(line).style(Style::default().bg(COLOR_BG));
    f.render_widget(widget, area);
}
