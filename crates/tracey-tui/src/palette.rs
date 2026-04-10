//! Terminal color capability detection and adaptive color selection.
//! Ported from Codex's terminal_palette.rs (Apache 2.0).
//! Detects TrueColor vs 256-color vs basic and returns the best Color.

use ratatui::style::Color;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorLevel {
    TrueColor,
    Ansi256,
    Basic,
}

static COLOR_LEVEL: OnceLock<ColorLevel> = OnceLock::new();

pub fn color_level() -> ColorLevel {
    *COLOR_LEVEL.get_or_init(|| {
        match supports_color::on_cached(supports_color::Stream::Stdout) {
            Some(level) if level.has_16m => ColorLevel::TrueColor,
            Some(level) if level.has_256 => ColorLevel::Ansi256,
            _ => ColorLevel::Basic,
        }
    })
}

/// Return the best Color for an RGB target based on terminal capability.
pub fn best(r: u8, g: u8, b: u8) -> Color {
    match color_level() {
        ColorLevel::TrueColor => Color::Rgb(r, g, b),
        ColorLevel::Ansi256 => Color::Indexed(closest_256(r, g, b)),
        ColorLevel::Basic => Color::Magenta, // fallback
    }
}

/// Find the closest xterm-256 color index for an RGB value.
fn closest_256(r: u8, g: u8, b: u8) -> u8 {
    // Check grayscale ramp first (232-255)
    let gray_avg = (r as u16 + g as u16 + b as u16) / 3;
    if r.abs_diff(g) < 10 && g.abs_diff(b) < 10 && gray_avg > 8 && gray_avg < 238 {
        return (232 + ((gray_avg - 8) * 24 / 230).min(23)) as u8;
    }

    // Map to 6x6x6 color cube (16-231)
    let ri = color_cube_index(r);
    let gi = color_cube_index(g);
    let bi = color_cube_index(b);
    16 + 36 * ri + 6 * gi + bi
}

fn color_cube_index(v: u8) -> u8 {
    match v {
        0..=47 => 0,
        48..=115 => 1,
        116..=155 => 2,
        156..=195 => 3,
        196..=235 => 4,
        _ => 5,
    }
}

// Pre-computed best colors for our palette
pub fn violet() -> Color { best(139, 92, 246) }
pub fn silver() -> Color { best(210, 210, 225) }
pub fn lavender() -> Color { best(175, 160, 230) }
pub fn deep_violet() -> Color { best(85, 50, 190) }
pub fn dark_purple() -> Color { best(55, 30, 120) }
pub fn chrome_highlight() -> Color { best(230, 230, 240) }
