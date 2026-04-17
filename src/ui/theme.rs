use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    #[default]
    OneDark,
    OneLight,
}

impl ThemeVariant {
    pub fn next(&self) -> Self {
        match self {
            Self::OneDark => Self::OneLight,
            Self::OneLight => Self::OneDark,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::OneDark => "onedark",
            Self::OneLight => "onelight",
        }
    }
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
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(171, 178, 191))
                .bg(Color::Rgb(40, 44, 52)),
            ThemeVariant::OneLight => Style::default()
                .fg(Color::Rgb(56, 58, 66))
                .bg(Color::Rgb(250, 250, 250)),
        }
    }

    pub fn style_selected(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(40, 44, 52))
                .bg(Color::Rgb(229, 192, 123))
                .add_modifier(Modifier::BOLD),
            ThemeVariant::OneLight => Style::default()
                .fg(Color::Rgb(250, 250, 250))
                .bg(Color::Rgb(64, 120, 242))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn style_dim(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(92, 99, 112))
                .bg(Color::Rgb(40, 44, 52)),
            ThemeVariant::OneLight => Style::default()
                .fg(Color::Rgb(160, 161, 167))
                .bg(Color::Rgb(250, 250, 250)),
        }
    }

    pub fn style_accent(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(229, 192, 123)),
            ThemeVariant::OneLight => Style::default().fg(Color::Rgb(152, 104, 1)),
        }
    }

    pub fn style_border(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(97, 175, 239)),
            ThemeVariant::OneLight => Style::default().fg(Color::Rgb(129, 162, 190)),
        }
    }

    pub fn style_error(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(224, 108, 117)),
            ThemeVariant::OneLight => Style::default().fg(Color::Rgb(199, 57, 65)),
        }
    }

    pub fn style_success(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default().fg(Color::Rgb(152, 195, 121)),
            ThemeVariant::OneLight => Style::default().fg(Color::Rgb(80, 161, 79)),
        }
    }

    pub fn style_header(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default()
                .fg(Color::Rgb(229, 192, 123))
                .add_modifier(Modifier::BOLD),
            ThemeVariant::OneLight => Style::default()
                .fg(Color::Rgb(152, 104, 1))
                .bg(Color::Rgb(250, 250, 250))
                .add_modifier(Modifier::BOLD),
        }
    }

    /// Style applied to Block widgets for background fill.
    pub fn block_bg(&self) -> Style {
        match self.variant {
            ThemeVariant::OneDark => Style::default().bg(Color::Rgb(40, 44, 52)),
            ThemeVariant::OneLight => Style::default().bg(Color::Rgb(250, 250, 250)),
        }
    }
}
