use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};

// ═══════════════════════════════════════════════════════════════
// LIQUID CHROME VIOLET — 12-stop metallic gradient
// Non-linear: bright peak → gradual mid → compressed shadows
// Creates the signature "polished violet metal" look
// ═══════════════════════════════════════════════════════════════

pub const CHROME: [Color; 12] = [
    Color::Rgb(245, 242, 255),  //  0: near-white peak (highlight)
    Color::Rgb(230, 220, 255),  //  1: bright highlight
    Color::Rgb(210, 190, 255),  //  2: chrome light
    Color::Rgb(180, 160, 245),  //  3: light violet
    Color::Rgb(155, 130, 240),  //  4: mid bright
    Color::Rgb(139, 92, 246),   //  5: core violet (primary)
    Color::Rgb(110, 70, 220),   //  6: deep mid
    Color::Rgb(85, 50, 190),    //  7: deep violet
    Color::Rgb(65, 35, 150),    //  8: dark
    Color::Rgb(45, 25, 100),    //  9: shadow start
    Color::Rgb(30, 15, 60),     // 10: deep shadow
    Color::Rgb(15, 8, 35),      // 11: void
];

// ANSI versions of the chrome stops (for non-ratatui contexts)
pub const ANSI_CHROME: [&str; 12] = [
    "\x1b[38;2;245;242;255m",
    "\x1b[38;2;230;220;255m",
    "\x1b[38;2;210;190;255m",
    "\x1b[38;2;180;160;245m",
    "\x1b[38;2;155;130;240m",
    "\x1b[38;2;139;92;246m",
    "\x1b[38;2;110;70;220m",
    "\x1b[38;2;85;50;190m",
    "\x1b[38;2;65;35;150m",
    "\x1b[38;2;45;25;100m",
    "\x1b[38;2;30;15;60m",
    "\x1b[38;2;15;8;35m",
];

// Primary aliases (for backward compat and quick access)
pub const VIOLET: Color = Color::Rgb(139, 92, 246);          // chrome[5]
pub const VIOLET_BRIGHT: Color = Color::Rgb(180, 160, 245);  // chrome[3]
pub const VIOLET_DIM: Color = Color::Rgb(85, 50, 190);       // chrome[7]
pub const VIOLET_MUTED: Color = Color::Rgb(45, 25, 100);     // chrome[9]
pub const LAVENDER: Color = Color::Rgb(210, 190, 255);        // chrome[2]
pub const CHROME_HIGHLIGHT: Color = Color::Rgb(245, 242, 255); // chrome[0]
pub const CHROME_SHADOW: Color = Color::Rgb(30, 15, 60);      // chrome[10]

// Layer colors for causal graph
pub const CODE_COLOR: Color = VIOLET;
pub const EXEC_COLOR: Color = Color::Rgb(245, 158, 11);      // amber
pub const KNOWLEDGE_COLOR: Color = Color::Rgb(16, 185, 129);  // emerald
pub const PROJECT_COLOR: Color = Color::Rgb(244, 63, 94);     // rose

// UI colors
pub const BG: Color = Color::Rgb(10, 14, 39);                 // deep blue-black
pub const FG: Color = Color::Rgb(229, 229, 234);              // soft white
pub const DIM: Color = Color::Rgb(100, 100, 115);             // muted
pub const ERROR: Color = Color::Rgb(239, 68, 68);
pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
pub const WARNING: Color = Color::Rgb(234, 179, 8);

// ═══════════════════════════════════════════════════════════════
// GRADIENT FUNCTIONS
// ═══════════════════════════════════════════════════════════════

/// Render text as a liquid chrome gradient (ratatui Spans).
/// Non-linear distribution: bright peak in first 30%, gradual falloff.
pub fn chrome_gradient_spans(text: &str) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return vec![];
    }

    chars.iter().enumerate().map(|(i, ch)| {
        let t = i as f64 / len.max(1) as f64;
        let color = chrome_interpolate(t);
        Span::styled(ch.to_string(), Style::default().fg(color))
    }).collect()
}

/// Render text as liquid chrome ANSI string (for non-ratatui contexts).
pub fn chrome_gradient_ansi(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return String::new();
    }

    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let t = i as f64 / len.max(1) as f64;
        let (r, g, b) = chrome_rgb(t);
        result.push_str(&format!("\x1b[38;2;{r};{g};{b}m{ch}"));
    }
    result.push_str(ANSI_RESET);
    result
}

/// Interpolate through the chrome gradient.
/// t = 0.0 → highlight (bright), t = 1.0 → shadow (dark)
/// Uses a non-linear curve: bright peak clustered at t=0-0.3
fn chrome_interpolate(t: f64) -> Color {
    let (r, g, b) = chrome_rgb(t);
    Color::Rgb(r, g, b)
}

/// Get RGB values for a position in the chrome gradient.
/// Non-linear: uses a quadratic ease-in for the "metallic valley" effect.
fn chrome_rgb(t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);

    // Non-linear mapping: accelerate into shadows
    let t_curved = t * t * 0.7 + t * 0.3; // Quadratic ease with linear blend

    // Interpolate between chrome stops
    let idx_f = t_curved * 11.0;
    let idx_low = (idx_f as usize).min(10);
    let idx_high = (idx_low + 1).min(11);
    let frac = idx_f - idx_low as f64;

    let Color::Rgb(r1, g1, b1) = CHROME[idx_low] else { return (139, 92, 246) };
    let Color::Rgb(r2, g2, b2) = CHROME[idx_high] else { return (139, 92, 246) };

    let r = (r1 as f64 * (1.0 - frac) + r2 as f64 * frac) as u8;
    let g = (g1 as f64 * (1.0 - frac) + g2 as f64 * frac) as u8;
    let b = (b1 as f64 * (1.0 - frac) + b2 as f64 * frac) as u8;

    (r, g, b)
}

/// Confidence → color (green=high, yellow=mid, red=low)
pub fn confidence_color(conf: f64) -> Color {
    if conf >= 0.7 {
        SUCCESS
    } else if conf >= 0.4 {
        WARNING
    } else {
        ERROR
    }
}

// ═══════════════════════════════════════════════════════════════
// PRE-BUILT STYLES
// ═══════════════════════════════════════════════════════════════

pub fn user_style() -> Style {
    Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD) // chrome light
}

pub fn assistant_style() -> Style {
    Style::default().fg(FG)
}

pub fn tool_style() -> Style {
    Style::default().fg(DIM)
}

pub fn error_style() -> Style {
    Style::default().fg(ERROR).add_modifier(Modifier::BOLD)
}

pub fn status_bar_style() -> Style {
    Style::default().fg(CHROME[0]).bg(CHROME[7]) // highlight on deep violet
}

pub fn border_style() -> Style {
    Style::default().fg(CHROME[7]) // deep violet borders
}

pub fn active_border_style() -> Style {
    Style::default().fg(CHROME[4]) // mid-bright when active
}

pub fn graph_update_style() -> Style {
    Style::default().fg(CHROME[2]) // chrome light
}

pub fn header_style() -> Style {
    Style::default().fg(CHROME[0]).bg(CHROME[8]) // highlight on dark
}

// ═══════════════════════════════════════════════════════════════
// ANSI ESCAPE CODES (for non-ratatui contexts)
// ═══════════════════════════════════════════════════════════════

pub const ANSI_VIOLET: &str = "\x1b[38;2;139;92;246m";
pub const ANSI_VIOLET_BRIGHT: &str = "\x1b[38;2;180;160;245m";
pub const ANSI_VIOLET_DIM: &str = "\x1b[38;2;85;50;190m";
pub const ANSI_LAVENDER: &str = "\x1b[38;2;210;190;255m";
pub const ANSI_HIGHLIGHT: &str = "\x1b[38;2;245;242;255m";
pub const ANSI_DIM: &str = "\x1b[38;2;100;100;115m";
pub const ANSI_WHITE: &str = "\x1b[38;2;229;229;234m";
pub const ANSI_BOLD: &str = "\x1b[1m";
pub const ANSI_RESET: &str = "\x1b[0m";
