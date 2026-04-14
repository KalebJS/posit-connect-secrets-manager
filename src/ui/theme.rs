use ratatui::style::{Color, Modifier, Style};

// Palette
pub const COLOR_BG: Color = Color::Rgb(18, 18, 24);
pub const COLOR_PRIMARY: Color = Color::Rgb(135, 206, 235); // sky blue
pub const COLOR_ACCENT: Color = Color::Rgb(255, 140, 0);    // orange
pub const COLOR_DIM: Color = Color::Rgb(80, 100, 120);      // muted blue-gray
pub const COLOR_ERROR: Color = Color::Rgb(220, 80, 80);
pub const COLOR_SUCCESS: Color = Color::Rgb(100, 200, 120);
pub const COLOR_BORDER: Color = COLOR_PRIMARY;
pub const COLOR_SELECTED_FG: Color = Color::Rgb(18, 18, 24);

pub fn style_normal() -> Style {
    Style::default().fg(COLOR_PRIMARY).bg(COLOR_BG)
}

pub fn style_selected() -> Style {
    Style::default()
        .fg(COLOR_SELECTED_FG)
        .bg(COLOR_ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn style_dim() -> Style {
    Style::default().fg(COLOR_DIM).bg(COLOR_BG)
}

pub fn style_border() -> Style {
    Style::default().fg(COLOR_BORDER)
}

pub fn style_accent() -> Style {
    Style::default().fg(COLOR_ACCENT).bg(COLOR_BG)
}

pub fn style_error() -> Style {
    Style::default().fg(COLOR_ERROR).bg(COLOR_BG)
}

pub fn style_success() -> Style {
    Style::default().fg(COLOR_SUCCESS).bg(COLOR_BG)
}

pub fn style_header() -> Style {
    Style::default()
        .fg(COLOR_ACCENT)
        .bg(COLOR_BG)
        .add_modifier(Modifier::BOLD)
}
