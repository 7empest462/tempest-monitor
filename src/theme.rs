use std::sync::RwLock;
use ratatui::style::{Color, Modifier, Style};

pub struct ThemeConfig {
    pub fg_muted: Color,
    pub accent: Color,
    pub accent2: Color,
    pub title_fg: Color,
    pub border: Color,
    pub header_bg: Color,
    pub footer_bg: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub gradient: [(f64, Color); 5],
}

impl ThemeConfig {
    pub const fn dark() -> Self {
        Self {
            fg_muted: Color::Rgb(108, 112, 124),
            accent: Color::Cyan,
            accent2: Color::Magenta,
            title_fg: Color::Rgb(198, 208, 245),
            border: Color::Rgb(88, 91, 112),
            header_bg: Color::Rgb(30, 30, 46),
            footer_bg: Color::Rgb(30, 30, 46),
            selected_bg: Color::Cyan,
            selected_fg: Color::Rgb(30, 30, 46),
            gradient: [
                (0.0, Color::Rgb(166, 227, 161)),   // green
                (25.0, Color::Rgb(148, 226, 213)),  // teal
                (50.0, Color::Rgb(249, 226, 175)),  // yellow
                (75.0, Color::Rgb(250, 179, 135)),  // peach/orange
                (100.0, Color::Rgb(243, 139, 168)), // red
            ],
        }
    }

    pub const fn light() -> Self {
        Self {
            fg_muted: Color::Rgb(100, 110, 120),
            accent: Color::Rgb(30, 100, 200), // blue
            accent2: Color::Rgb(150, 50, 150), // purple
            title_fg: Color::Rgb(40, 40, 50),
            border: Color::Rgb(200, 200, 200),
            header_bg: Color::Rgb(240, 242, 245),
            footer_bg: Color::Rgb(240, 242, 245),
            selected_bg: Color::Rgb(30, 100, 200),
            selected_fg: Color::White,
            gradient: [
                (0.0, Color::Rgb(60, 160, 80)),
                (25.0, Color::Rgb(50, 140, 150)),
                (50.0, Color::Rgb(180, 150, 50)),
                (75.0, Color::Rgb(200, 100, 40)),
                (100.0, Color::Rgb(200, 60, 80)),
            ],
        }
    }

    pub const fn catppuccin() -> Self {
        Self::dark() // Catppuccin Mocha is the default dark
    }

    pub const fn nord() -> Self {
        Self {
            fg_muted: Color::Rgb(110, 120, 135),
            accent: Color::Rgb(136, 192, 208), // frost blue
            accent2: Color::Rgb(180, 142, 173), // aurora purple
            title_fg: Color::Rgb(216, 222, 233),
            border: Color::Rgb(76, 86, 106),
            header_bg: Color::Rgb(46, 52, 64),
            footer_bg: Color::Rgb(46, 52, 64),
            selected_bg: Color::Rgb(136, 192, 208),
            selected_fg: Color::Rgb(46, 52, 64),
            gradient: [
                (0.0, Color::Rgb(163, 190, 140)), // green
                (25.0, Color::Rgb(143, 188, 187)), // teal
                (50.0, Color::Rgb(235, 203, 139)), // yellow
                (75.0, Color::Rgb(208, 135, 112)), // orange
                (100.0, Color::Rgb(191, 97, 106)), // red
            ],
        }
    }

    pub const fn dracula() -> Self {
        Self {
            fg_muted: Color::Rgb(98, 114, 164),    // comment / purple-blue
            accent: Color::Rgb(139, 233, 253),      // cyan
            accent2: Color::Rgb(255, 121, 198),     // pink
            title_fg: Color::Rgb(248, 248, 242),    // fg
            border: Color::Rgb(68, 71, 90),         // selection / border
            header_bg: Color::Rgb(40, 42, 54),       // background
            footer_bg: Color::Rgb(40, 42, 54),
            selected_bg: Color::Rgb(189, 147, 249), // purple
            selected_fg: Color::Rgb(40, 42, 54),
            gradient: [
                (0.0, Color::Rgb(80, 250, 123)),    // green
                (25.0, Color::Rgb(139, 233, 253)),  // cyan
                (50.0, Color::Rgb(241, 250, 140)),  // yellow
                (75.0, Color::Rgb(255, 184, 108)),  // orange
                (100.0, Color::Rgb(255, 85, 85)),   // red
            ],
        }
    }

    pub const fn gruvbox() -> Self {
        Self {
            fg_muted: Color::Rgb(146, 131, 116),    // gray
            accent: Color::Rgb(250, 189, 47),       // bright yellow
            accent2: Color::Rgb(254, 128, 25),      // bright orange
            title_fg: Color::Rgb(235, 219, 178),    // light1
            border: Color::Rgb(80, 73, 69),         // dark2
            header_bg: Color::Rgb(40, 40, 40),       // dark0
            footer_bg: Color::Rgb(40, 40, 40),
            selected_bg: Color::Rgb(250, 189, 47),  // yellow
            selected_fg: Color::Rgb(40, 40, 40),
            gradient: [
                (0.0, Color::Rgb(184, 187, 38)),    // green
                (25.0, Color::Rgb(142, 192, 124)),  // aqua
                (50.0, Color::Rgb(250, 189, 47)),   // yellow
                (75.0, Color::Rgb(254, 128, 25)),   // orange
                (100.0, Color::Rgb(251, 73, 52)),   // red
            ],
        }
    }

    pub const fn tokyo_night() -> Self {
        Self {
            fg_muted: Color::Rgb(86, 95, 137),      // comment
            accent: Color::Rgb(122, 162, 247),      // blue
            accent2: Color::Rgb(187, 154, 247),     // purple
            title_fg: Color::Rgb(154, 165, 206),    // fg
            border: Color::Rgb(65, 72, 104),        // border
            header_bg: Color::Rgb(26, 27, 38),       // background
            footer_bg: Color::Rgb(26, 27, 38),
            selected_bg: Color::Rgb(122, 162, 247), // blue
            selected_fg: Color::Rgb(26, 27, 38),
            gradient: [
                (0.0, Color::Rgb(158, 206, 106)),   // green
                (25.0, Color::Rgb(122, 162, 247)),  // blue
                (50.0, Color::Rgb(224, 175, 104)),  // yellow
                (75.0, Color::Rgb(255, 158, 100)),  // orange
                (100.0, Color::Rgb(247, 118, 142)), // red
            ],
        }
    }
}

static CURRENT_THEME: RwLock<ThemeConfig> = RwLock::new(ThemeConfig::dark());

pub fn set_theme(name: &str) {
    let mut theme = CURRENT_THEME.write().unwrap();
    *theme = match name.to_lowercase().replace("-", "_").as_str() {
        "light" => ThemeConfig::light(),
        "nord" => ThemeConfig::nord(),
        "catppuccin" => ThemeConfig::catppuccin(),
        "dracula" => ThemeConfig::dracula(),
        "gruvbox" => ThemeConfig::gruvbox(),
        "tokyo_night" | "tokyonight" => ThemeConfig::tokyo_night(),
        _ => ThemeConfig::dark(),
    };
}

pub fn fg_muted() -> Color { CURRENT_THEME.read().unwrap().fg_muted }
pub fn accent() -> Color { CURRENT_THEME.read().unwrap().accent }
pub fn accent2() -> Color { CURRENT_THEME.read().unwrap().accent2 }
pub fn title_fg() -> Color { CURRENT_THEME.read().unwrap().title_fg }
pub fn border() -> Color { CURRENT_THEME.read().unwrap().border }
pub fn header_bg() -> Color { CURRENT_THEME.read().unwrap().header_bg }
pub fn footer_bg() -> Color { CURRENT_THEME.read().unwrap().footer_bg }
pub fn selected_bg() -> Color { CURRENT_THEME.read().unwrap().selected_bg }
pub fn selected_fg() -> Color { CURRENT_THEME.read().unwrap().selected_fg }

pub fn usage_color(percent: f64) -> Color {
    let pct = percent.clamp(0.0, 100.0);
    let theme = CURRENT_THEME.read().unwrap();
    let grad = &theme.gradient;

    for i in 0..grad.len() - 1 {
        let (lo_pct, lo_col) = grad[i];
        let (hi_pct, hi_col) = grad[i + 1];

        if pct <= hi_pct {
            let t = (pct - lo_pct) / (hi_pct - lo_pct);
            return lerp_color(lo_col, hi_col, t);
        }
    }
    grad.last().unwrap().1
}

fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) {
        Color::Rgb(
            lerp_u8(r1, r2, t),
            lerp_u8(g1, g2, t),
            lerp_u8(b1, b2, t),
        )
    } else {
        b
    }
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let result = a as f64 + (b as f64 - a as f64) * t;
    result.round().clamp(0.0, 255.0) as u8
}

pub fn style_header() -> Style {
    Style::default()
        .fg(title_fg())
        .bg(header_bg())
        .add_modifier(Modifier::BOLD)
}

pub fn style_footer() -> Style {
    Style::default().fg(fg_muted()).bg(footer_bg())
}

pub fn style_title() -> Style {
    Style::default()
        .fg(accent())
        .add_modifier(Modifier::BOLD)
}

pub fn style_border() -> Style {
    Style::default().fg(border())
}

pub fn style_selected() -> Style {
    Style::default()
        .fg(selected_fg())
        .bg(selected_bg())
        .add_modifier(Modifier::BOLD)
}

pub fn style_muted() -> Style {
    Style::default().fg(fg_muted())
}

pub fn style_tab_active() -> Style {
    Style::default()
        .fg(accent())
        .add_modifier(Modifier::BOLD)
}

pub fn style_table_header() -> Style {
    Style::default()
        .fg(accent2())
        .add_modifier(Modifier::BOLD)
}

pub fn style_root_badge() -> Style {
    Style::default()
        .fg(Color::Rgb(243, 139, 168)) // red
        .add_modifier(Modifier::BOLD)
}

pub fn style_gauge(percent: f64) -> Style {
    Style::default().fg(usage_color(percent))
}
