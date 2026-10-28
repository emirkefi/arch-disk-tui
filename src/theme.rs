use ratatui::style::Color;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    ArchIceBlue,
    Catppuccin,
    Gruvbox,
    Mono,
    NoColor,
}

impl ThemeName {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "catppuccin" | "mocha" => Self::Catppuccin,
            "gruvbox" => Self::Gruvbox,
            "mono" | "monochrome" => Self::Mono,
            "nocolor" | "no_color" | "none" | "plain" => Self::NoColor,
            _ => Self::ArchIceBlue,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::ArchIceBlue => Self::Catppuccin,
            Self::Catppuccin => Self::Gruvbox,
            Self::Gruvbox => Self::Mono,
            Self::Mono => Self::ArchIceBlue,
            Self::NoColor => Self::NoColor,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ArchIceBlue => "Arch Ice Blue",
            Self::Catppuccin => "Catppuccin",
            Self::Gruvbox => "Gruvbox",
            Self::Mono => "Mono",
            Self::NoColor => "NO_COLOR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconMode {
    Auto,
    Nerd,
    Ascii,
    None,
}

impl IconMode {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "nerd" | "nerdfont" | "nerd-font" | "true" | "on" => Self::Nerd,
            "ascii" | "plain" => Self::Ascii,
            "none" | "off" | "false" => Self::None,
            _ => Self::Auto,
        }
    }

    pub fn use_nerd_icons(self) -> bool {
        match self {
            Self::Nerd => true,
            Self::Ascii | Self::None => false,
            Self::Auto => {
                if std::env::var_os("NO_COLOR").is_some() {
                    return false;
                }
                if let Ok(val) = std::env::var("NERD_FONTS") {
                    if val == "0" || val.eq_ignore_ascii_case("false") {
                        return false;
                    }
                    if val == "1" || val.eq_ignore_ascii_case("true") {
                        return true;
                    }
                }
                let term = std::env::var("TERM").unwrap_or_default();
                if term == "linux" || term == "dumb" || term.is_empty() {
                    return false;
                }
                true
            }
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub name: ThemeName,
    pub no_color: bool,
    pub bg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub accent: Color,
    pub secondary: Color,
    pub text: Color,
    pub text_dim: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub match_fg: Color,
    pub match_bg: Color,
    pub gauge_ok: Color,
    pub gauge_warn: Color,
    pub gauge_crit: Color,
    pub bar_empty: Color,
    // Category colors
    pub dir_color: Color,
    pub media_color: Color,
    pub archive_color: Color,
    pub code_color: Color,
    pub cache_color: Color,
    pub doc_color: Color,
    pub bin_color: Color,
    pub other_color: Color,
    pub palette: [Color; 7],
}

impl Theme {
    pub fn from_name(name: ThemeName) -> Self {
        if std::env::var_os("NO_COLOR").is_some() || name == ThemeName::NoColor {
            return Self::no_color();
        }
        match name {
            ThemeName::ArchIceBlue => Self::arch_ice_blue(),
            ThemeName::Catppuccin => Self::catppuccin(),
            ThemeName::Gruvbox => Self::gruvbox(),
            ThemeName::Mono => Self::mono(),
            ThemeName::NoColor => Self::no_color(),
        }
    }

    pub fn arch_ice_blue() -> Self {
        Self {
            name: ThemeName::ArchIceBlue,
            no_color: false,
            bg: Color::Rgb(15, 17, 26),
            border: Color::Rgb(60, 70, 95),
            border_focused: Color::Rgb(125, 207, 255),
            accent: Color::Rgb(125, 207, 255),
            secondary: Color::Rgb(180, 130, 255),
            text: Color::White,
            text_dim: Color::Rgb(120, 130, 155),
            highlight_bg: Color::Rgb(40, 50, 75),
            highlight_fg: Color::White,
            match_fg: Color::Rgb(15, 17, 26),
            match_bg: Color::Rgb(255, 198, 109),
            gauge_ok: Color::Rgb(120, 230, 160),
            gauge_warn: Color::Rgb(255, 198, 109),
            gauge_crit: Color::Rgb(255, 95, 95),
            bar_empty: Color::Rgb(45, 52, 70),
            dir_color: Color::Rgb(125, 207, 255),     // Arch Ice Blue
            media_color: Color::Rgb(255, 120, 180),   // Neon Rose
            archive_color: Color::Rgb(255, 198, 109), // Amber Gold
            code_color: Color::Rgb(120, 230, 160),    // Mint Green
            cache_color: Color::Rgb(255, 100, 100),   // Crimson Warning
            doc_color: Color::Rgb(180, 130, 255),     // Cyber Violet
            bin_color: Color::Rgb(100, 220, 240),     // Cyan Glow
            other_color: Color::Rgb(180, 190, 210),
            palette: [
                Color::Rgb(125, 207, 255),
                Color::Rgb(120, 230, 160),
                Color::Rgb(255, 198, 109),
                Color::Rgb(255, 120, 180),
                Color::Rgb(180, 130, 255),
                Color::Rgb(255, 100, 100),
                Color::Rgb(100, 220, 240),
            ],
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            name: ThemeName::Catppuccin,
            no_color: false,
            bg: Color::Rgb(30, 30, 46),
            border: Color::Rgb(88, 91, 112),
            border_focused: Color::Rgb(137, 180, 250),
            accent: Color::Rgb(137, 180, 250),
            secondary: Color::Rgb(203, 166, 247),
            text: Color::Rgb(205, 214, 244),
            text_dim: Color::Rgb(147, 153, 178),
            highlight_bg: Color::Rgb(69, 71, 90),
            highlight_fg: Color::Rgb(205, 214, 244),
            match_fg: Color::Rgb(30, 30, 46),
            match_bg: Color::Rgb(249, 226, 175),
            gauge_ok: Color::Rgb(166, 227, 161),
            gauge_warn: Color::Rgb(249, 226, 175),
            gauge_crit: Color::Rgb(243, 139, 168),
            bar_empty: Color::Rgb(49, 50, 68),
            dir_color: Color::Rgb(137, 180, 250),     // Blue
            media_color: Color::Rgb(245, 194, 231),   // Pink
            archive_color: Color::Rgb(250, 179, 135), // Peach
            code_color: Color::Rgb(166, 227, 161),    // Green
            cache_color: Color::Rgb(243, 139, 168),   // Red
            doc_color: Color::Rgb(203, 166, 247),     // Mauve
            bin_color: Color::Rgb(148, 226, 213),     // Teal
            other_color: Color::Rgb(186, 194, 222),
            palette: [
                Color::Rgb(137, 180, 250),
                Color::Rgb(166, 227, 161),
                Color::Rgb(250, 179, 135),
                Color::Rgb(245, 194, 231),
                Color::Rgb(203, 166, 247),
                Color::Rgb(243, 139, 168),
                Color::Rgb(148, 226, 213),
            ],
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: ThemeName::Gruvbox,
            no_color: false,
            bg: Color::Rgb(40, 40, 40),
            border: Color::Rgb(102, 92, 84),
            border_focused: Color::Rgb(250, 189, 47),
            accent: Color::Rgb(131, 165, 152),
            secondary: Color::Rgb(211, 134, 155),
            text: Color::Rgb(235, 219, 178),
            text_dim: Color::Rgb(146, 131, 116),
            highlight_bg: Color::Rgb(60, 56, 54),
            highlight_fg: Color::Rgb(251, 241, 199),
            match_fg: Color::Rgb(40, 40, 40),
            match_bg: Color::Rgb(250, 189, 47),
            gauge_ok: Color::Rgb(184, 187, 38),
            gauge_warn: Color::Rgb(250, 189, 47),
            gauge_crit: Color::Rgb(251, 73, 52),
            bar_empty: Color::Rgb(60, 56, 54),
            dir_color: Color::Rgb(131, 165, 152),     // Blue
            media_color: Color::Rgb(211, 134, 155),   // Purple
            archive_color: Color::Rgb(254, 128, 25),  // Orange
            code_color: Color::Rgb(184, 187, 38),     // Green
            cache_color: Color::Rgb(251, 73, 52),     // Red
            doc_color: Color::Rgb(250, 189, 47),     // Yellow
            bin_color: Color::Rgb(142, 192, 124),     // Aqua
            other_color: Color::Rgb(213, 196, 161),
            palette: [
                Color::Rgb(131, 165, 152),
                Color::Rgb(184, 187, 38),
                Color::Rgb(250, 189, 47),
                Color::Rgb(254, 128, 25),
                Color::Rgb(211, 134, 155),
                Color::Rgb(251, 73, 52),
                Color::Rgb(142, 192, 124),
            ],
        }
    }

    pub fn mono() -> Self {
        Self {
            name: ThemeName::Mono,
            no_color: false,
            bg: Color::Reset,
            border: Color::DarkGray,
            border_focused: Color::White,
            accent: Color::White,
            secondary: Color::Gray,
            text: Color::White,
            text_dim: Color::DarkGray,
            highlight_bg: Color::Rgb(55, 55, 55),
            highlight_fg: Color::White,
            match_fg: Color::Black,
            match_bg: Color::White,
            gauge_ok: Color::White,
            gauge_warn: Color::Gray,
            gauge_crit: Color::White,
            bar_empty: Color::DarkGray,
            dir_color: Color::White,
            media_color: Color::Gray,
            archive_color: Color::Gray,
            code_color: Color::White,
            cache_color: Color::DarkGray,
            doc_color: Color::Gray,
            bin_color: Color::White,
            other_color: Color::Gray,
            palette: [
                Color::White,
                Color::Gray,
                Color::White,
                Color::Gray,
                Color::White,
                Color::Gray,
                Color::White,
            ],
        }
    }

    pub fn no_color() -> Self {
        Self {
            name: ThemeName::NoColor,
            no_color: true,
            bg: Color::Reset,
            border: Color::Reset,
            border_focused: Color::Reset,
            accent: Color::Reset,
            secondary: Color::Reset,
            text: Color::Reset,
            text_dim: Color::Reset,
            highlight_bg: Color::Reset,
            highlight_fg: Color::Reset,
            match_fg: Color::Reset,
            match_bg: Color::Reset,
            gauge_ok: Color::Reset,
            gauge_warn: Color::Reset,
            gauge_crit: Color::Reset,
            bar_empty: Color::Reset,
            dir_color: Color::Reset,
            media_color: Color::Reset,
            archive_color: Color::Reset,
            code_color: Color::Reset,
            cache_color: Color::Reset,
            doc_color: Color::Reset,
            bin_color: Color::Reset,
            other_color: Color::Reset,
            palette: [Color::Reset; 7],
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub theme: ThemeName,
    pub icons: IconMode,
    pub one_file_system: bool,
    pub exclude: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeName::ArchIceBlue,
            icons: IconMode::Auto,
            one_file_system: false,
            exclude: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let mut cfg = Self::default();
        let mut candidates = Vec::new();

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            candidates.push(PathBuf::from(xdg).join("arch-disk-tui/config.toml"));
        } else if let Ok(home) = std::env::var("HOME") {
            candidates.push(PathBuf::from(home).join(".config/arch-disk-tui/config.toml"));
        }
        candidates.push(PathBuf::from("config.toml"));

        for path in candidates {
            if let Ok(content) = std::fs::read_to_string(&path) {
                cfg.parse_toml(&content);
                break;
            }
        }

        if std::env::var_os("NO_COLOR").is_some() {
            cfg.theme = ThemeName::NoColor;
        }

        cfg
    }

    pub fn parse_toml(&mut self, content: &str) {
        for raw_line in content.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('[') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim().trim_matches('"').trim_matches('\'');
                match key {
                    "theme" => self.theme = ThemeName::from_str(val),
                    "icons" => self.icons = IconMode::from_str(val),
                    "one_file_system" => {
                        self.one_file_system = val.eq_ignore_ascii_case("true") || val == "1";
                    }
                    "exclude" => {
                        if val.starts_with('[') && val.ends_with(']') {
                            let inner = &val[1..val.len() - 1];
                            self.exclude = inner
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_toml_parsing() {
        let mut cfg = AppConfig::default();
        cfg.parse_toml(
            r#"
            # Sample configuration
            theme = "catppuccin"
            icons = "ascii"
            one_file_system = true
            exclude = ["node_modules", ".git", "target"]
            "#,
        );
        assert_eq!(cfg.theme, ThemeName::Catppuccin);
        assert_eq!(cfg.icons, IconMode::Ascii);
        assert!(cfg.one_file_system);
        assert_eq!(cfg.exclude, vec!["node_modules", ".git", "target"]);
    }
}
