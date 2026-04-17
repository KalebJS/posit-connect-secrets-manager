use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    #[default]
    Inherit,
    OneDark,
    #[serde(rename = "sky-orange")]
    SkyOrange,
}

pub struct Palette {
    variant: ThemeVariant,
}

impl Palette {
    pub fn new(variant: ThemeVariant) -> Self {
        Self { variant }
    }

    pub fn style_normal(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => Style::default(),
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(171, 178, 191))
                .bg(Color::Rgb(40, 44, 52)),
            ThemeVariant::SkyOrange => Style::default().fg(Color::White).bg(Color::Rgb(18, 18, 24)),
        }
    }

    pub fn style_selected(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            }
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(40, 44, 52))
                .bg(Color::Rgb(229, 192, 123))
                .add_modifier(Modifier::BOLD),
            ThemeVariant::SkyOrange => Style::default()
                .fg(Color::Rgb(18, 18, 24))
                .bg(Color::Rgb(255, 140, 0))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn style_dim(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => Style::default().add_modifier(Modifier::DIM),
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(92, 99, 112))
                .bg(Color::Rgb(40, 44, 52)),
            ThemeVariant::SkyOrange => Style::default()
                .fg(Color::Rgb(80, 100, 120))
                .bg(Color::Rgb(18, 18, 24)),
        }
    }

    pub fn style_accent(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => Style::default().add_modifier(Modifier::BOLD),
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(229, 192, 123)),
            ThemeVariant::SkyOrange => Style::default().fg(Color::Rgb(255, 140, 0)),
        }
    }

    pub fn style_border(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => Style::default().add_modifier(Modifier::DIM),
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(97, 175, 239)),
            ThemeVariant::SkyOrange => Style::default().fg(Color::Rgb(135, 206, 235)),
        }
    }

    pub fn style_error(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => {
                Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            }
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(224, 108, 117)),
            ThemeVariant::SkyOrange => Style::default()
                .fg(Color::Rgb(220, 80, 80))
                .bg(Color::Rgb(18, 18, 24)),
        }
    }

    pub fn style_success(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => Style::default().add_modifier(Modifier::BOLD),
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(152, 195, 121)),
            ThemeVariant::SkyOrange => Style::default()
                .fg(Color::Rgb(100, 200, 120))
                .bg(Color::Rgb(18, 18, 24)),
        }
    }

    pub fn style_header(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => Style::default().add_modifier(Modifier::BOLD),
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(229, 192, 123))
                .add_modifier(Modifier::BOLD),
            ThemeVariant::SkyOrange => Style::default()
                .fg(Color::Rgb(255, 140, 0))
                .bg(Color::Rgb(18, 18, 24))
                .add_modifier(Modifier::BOLD),
        }
    }

    /// Style applied to Block widgets for background fill.
    pub fn block_bg(&self) -> Style {
        match self.variant {
            ThemeVariant::Inherit => Style::default(),
            ThemeVariant::OneDark => Style::default().bg(Color::Rgb(40, 44, 52)),
            ThemeVariant::SkyOrange => Style::default().bg(Color::Rgb(18, 18, 24)),
        }
    }
}
