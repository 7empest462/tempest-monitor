use ratatui::style::{Color, Modifier, Style};

// ── Btop-inspired dark color palette ──────────────────────────────────────────

/// Muted text for less-important info
pub const FG_MUTED: Color = Color::Rgb(108, 112, 124);

/// Primary accent — cyan
pub const ACCENT: Color = Color::Cyan;

/// Secondary accent — magenta
pub const ACCENT2: Color = Color::Magenta;

/// Title text color
pub const TITLE_FG: Color = Color::Rgb(198, 208, 245);

/// Border color
pub const BORDER: Color = Color::Rgb(88, 91, 112);

/// Header bar background
pub const HEADER_BG: Color = Color::Rgb(30, 30, 46);

/// Footer bar background
pub const FOOTER_BG: Color = Color::Rgb(30, 30, 46);

// ── Gradient stops for usage percentage (green → yellow → red) ───────────────

const GRADIENT: [(f64, Color); 5] = [
    (0.0, Color::Rgb(166, 227, 161)),   // green
    (25.0, Color::Rgb(148, 226, 213)),  // teal
    (50.0, Color::Rgb(249, 226, 175)),  // yellow
    (75.0, Color::Rgb(250, 179, 135)),  // peach/orange
    (100.0, Color::Rgb(243, 139, 168)), // red
];

/// Returns a color along the green→yellow→red gradient based on usage 0–100.
pub fn usage_color(percent: f64) -> Color {
    let pct = percent.clamp(0.0, 100.0);

    // Find the two gradient stops we sit between
    for i in 0..GRADIENT.len() - 1 {
        let (lo_pct, lo_col) = GRADIENT[i];
        let (hi_pct, hi_col) = GRADIENT[i + 1];

        if pct <= hi_pct {
            let t = (pct - lo_pct) / (hi_pct - lo_pct);
            return lerp_color(lo_col, hi_col, t);
        }
    }
    GRADIENT.last().unwrap().1
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

// ── Pre-built styles ─────────────────────────────────────────────────────────

pub fn style_header() -> Style {
    Style::default()
        .fg(TITLE_FG)
        .bg(HEADER_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn style_footer() -> Style {
    Style::default().fg(FG_MUTED).bg(FOOTER_BG)
}

pub fn style_title() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn style_border() -> Style {
    Style::default().fg(BORDER)
}

pub fn style_selected() -> Style {
    Style::default()
        .fg(Color::Rgb(30, 30, 46))
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn style_muted() -> Style {
    Style::default().fg(FG_MUTED)
}

pub fn style_tab_active() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn style_table_header() -> Style {
    Style::default()
        .fg(ACCENT2)
        .add_modifier(Modifier::BOLD)
}

pub fn style_root_badge() -> Style {
    Style::default()
        .fg(Color::Rgb(243, 139, 168)) // red
        .add_modifier(Modifier::BOLD)
}

/// Style a gauge based on current usage percentage.
pub fn style_gauge(percent: f64) -> Style {
    Style::default().fg(usage_color(percent))
}
