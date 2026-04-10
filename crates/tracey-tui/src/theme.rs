//! Tracey theme — all colors go through palette::best() for terminal compatibility.

use ratatui::style::{Color, Modifier, Style};
use crate::palette;

// Chrome palette — computed once via palette detection
pub fn chrome(i: usize) -> Color {
    match i {
        0 => palette::chrome_highlight(),
        1 => palette::silver(),
        2 => palette::lavender(),
        3 => palette::lavender(),
        4 => palette::violet(),
        5 => palette::violet(),
        6 => palette::deep_violet(),
        7 => palette::deep_violet(),
        8 => palette::dark_purple(),
        9 => palette::dark_purple(),
        _ => palette::violet(),
    }
}

// Named colors
pub fn violet() -> Color { palette::violet() }
pub fn silver() -> Color { palette::silver() }
pub fn lavender() -> Color { palette::lavender() }
pub fn dim_color() -> Color { palette::best(120, 120, 135) }
pub fn error_color() -> Color { palette::best(239, 68, 68) }
pub fn success_color() -> Color { palette::best(34, 197, 94) }
pub fn warning_color() -> Color { palette::best(234, 179, 8) }
pub fn fg_color() -> Color { palette::best(230, 230, 235) }

// Styles
pub fn user_style() -> Style { Style::default().fg(lavender()).add_modifier(Modifier::BOLD) }
pub fn assistant_style() -> Style { Style::default().fg(fg_color()) }
pub fn tool_style() -> Style { Style::default().fg(dim_color()) }
pub fn error_style() -> Style { Style::default().fg(error_color()).add_modifier(Modifier::BOLD) }
pub fn border_style() -> Style { Style::default().fg(palette::deep_violet()) }
pub fn active_border_style() -> Style { Style::default().fg(violet()) }
pub fn graph_update_style() -> Style { Style::default().fg(lavender()) }
pub fn status_bar_style() -> Style { Style::default().fg(silver()).bg(palette::dark_purple()) }
pub fn header_style() -> Style { Style::default().fg(silver()).bg(palette::dark_purple()) }

// ANSI escape codes for non-ratatui (setup wizard, banner)
pub const ANSI_VIOLET: &str = "\x1b[35m";
pub const ANSI_BRIGHT_VIOLET: &str = "\x1b[1;95m";
pub const ANSI_CYAN: &str = "\x1b[36m";
pub const ANSI_DIM: &str = "\x1b[90m";
pub const ANSI_WHITE: &str = "\x1b[1;37m";
pub const ANSI_GREEN: &str = "\x1b[32m";
pub const ANSI_RED: &str = "\x1b[31m";
pub const ANSI_YELLOW: &str = "\x1b[33m";
pub const ANSI_BOLD: &str = "\x1b[1m";
pub const ANSI_RESET: &str = "\x1b[0m";

// Backward-compat constants (used by app.rs, welcome.rs, etc.)
// These are computed lazily via palette
pub const CHROME: [Color; 12] = [
    Color::Magenta, Color::Magenta, Color::Magenta, Color::Magenta,
    Color::Magenta, Color::Magenta, Color::Magenta, Color::Magenta,
    Color::Magenta, Color::Magenta, Color::Magenta, Color::Magenta,
]; // Dummy — actual rendering uses chrome(i) function

pub const DIM: Color = Color::DarkGray;
pub const FG: Color = Color::White;
pub const ERROR: Color = Color::Red;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const VIOLET: Color = Color::Magenta;
pub const VIOLET_BRIGHT: Color = Color::LightMagenta;
pub const VIOLET_DIM: Color = Color::DarkGray;
pub const LAVENDER: Color = Color::LightMagenta;
pub const CHROME_HIGHLIGHT: Color = Color::White;
pub const CHROME_SHADOW: Color = Color::DarkGray;
pub const VIOLET_MUTED: Color = Color::DarkGray;

pub const ANSI_CHROME: [&str; 12] = [
    "\x1b[37m", "\x1b[37m", "\x1b[95m", "\x1b[95m",
    "\x1b[35m", "\x1b[35m", "\x1b[90m", "\x1b[90m",
    "\x1b[90m", "\x1b[90m", "\x1b[90m", "\x1b[90m",
];

// Confidence color
pub fn confidence_color(conf: f64) -> Color {
    if conf >= 0.7 { success_color() }
    else if conf >= 0.4 { warning_color() }
    else { error_color() }
}
