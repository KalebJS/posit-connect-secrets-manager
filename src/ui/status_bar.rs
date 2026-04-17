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
            StatusLevel::Info => app.palette.style_normal(),
            StatusLevel::Success => app.palette.style_success(),
            StatusLevel::Error => app.palette.style_error(),
        };
        Line::from(Span::styled(format!("  {}", msg), style))
    } else {
        let spinner = match &app.load_state {
            LoadState::Loading => format!("{} ", app.spinner()),
            _ => "  ".to_string(),
        };
        let hints = if app.sidebar_focused {
            "Tab:Content  j/k:Nav  g/G:Top/Bot  q:Quit"
        } else {
            match app.page {
                Page::ProjectList => "j/k:Nav  Enter:Expand  x:Toggle  a:Add  d:Del  g/G:Top/Bot  f/:Filter  h:Back  Ctrl+P:Refresh  q:Quit",
                Page::EnvVarList  => "j/k:Nav  Enter:Detail  e:Editor  g/G:Top/Bot  f/:Filter  h:Back  q:Quit",
                Page::Vault if app.vault_editing.is_some() => "Enter:Save  Esc:Cancel",
                Page::Vault       => "j/k:Nav  e/Enter:Edit  E:ExtEditor  n:New  d:Del  g/G:Top/Bot  f/:Filter  Ctrl+U:Sync  q:Quit",
                Page::Settings if app.settings_editing => "Enter:Confirm  Esc:Cancel",
                Page::Settings    => "j/k:Nav  Enter/e:Edit  g/G:Top/Bot  h:Back  q:Quit",
            }
        };
        Line::from(vec![
            Span::styled(spinner, app.palette.style_accent()),
            Span::styled(hints, app.palette.style_dim()),
        ])
    };

    let widget = Paragraph::new(line).style(Style::default());
    f.render_widget(widget, area);
}
