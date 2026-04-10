use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};

// ═══════════════════════════════════════════════════════════════
// LIQUID CHROME — Silver base with violet tints
//
// Strategy: Use Color::Rgb unconditionally (like Hermes forces truecolor).
// We set COLORTERM=truecolor at startup in main.rs.
// Fallback: Color::Indexed for ANSI escape contexts.
// ═══════════════════════════════════════════════════════════════

pub const CHROME: [Color; 12] = [
    Color::Rgb(230, 230, 240),  //  0: bright silver
    Color::Rgb(210, 210, 225),  //  1: silver
    Color::Rgb(190, 185, 215),  //  2: silver-lavender
    Color::Rgb(175, 160, 230),  //  3: lavender
    Color::Rgb(155, 130, 240),  //  4: chrome light violet
    Color::Rgb(139, 92, 246),   //  5: core violet (PRIMARY)
    Color::Rgb(110, 70, 220),   //  6: deep mid
    Color::Rgb(85, 50, 190),    //  7: deep violet
    Color::Rgb(65, 35, 150),    //  8: dark purple
    Color::Rgb(45, 25, 100),    //  9: shadow
    Color::Rgb(30, 15, 60),     // 10: deep shadow
    Color::Rgb(15, 10, 35),     // 11: void
];

// Named aliases
pub const VIOLET: Color = Color::Rgb(139, 92, 246);
pub const VIOLET_BRIGHT: Color = Color::Rgb(155, 130, 240);
pub const VIOLET_DIM: Color = Color::Rgb(85, 50, 190);
pub const VIOLET_MUTED: Color = Color::Rgb(45, 25, 100);
pub const LAVENDER: Color = Color::Rgb(175, 160, 230);
pub const CHROME_HIGHLIGHT: Color = Color::Rgb(230, 230, 240);
pub const CHROME_SHADOW: Color = Color::Rgb(65, 35, 150);

// Layer colors
pub const CODE_COLOR: Color = VIOLET;
pub const EXEC_COLOR: Color = Color::Rgb(245, 158, 11);
pub const KNOWLEDGE_COLOR: Color = Color::Rgb(34, 197, 94);
pub const PROJECT_COLOR: Color = Color::Rgb(244, 63, 94);

// UI base
pub const BG: Color = Color::Rgb(15, 10, 35);
pub const FG: Color = Color::Rgb(230, 230, 235);
pub const DIM: Color = Color::Rgb(120, 120, 135);
pub const ERROR: Color = Color::Rgb(239, 68, 68);
pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
pub const WARNING: Color = Color::Rgb(234, 179, 8);

// ═══════════════════════════════════════════════════════════════
// STYLES
// ═══════════════════════════════════════════════════════════════

pub fn user_style() -> Style { Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD) }
pub fn assistant_style() -> Style { Style::default().fg(FG) }
pub fn tool_style() -> Style { Style::default().fg(DIM) }
pub fn error_style() -> Style { Style::default().fg(ERROR).add_modifier(Modifier::BOLD) }
pub fn status_bar_style() -> Style { Style::default().fg(CHROME[0]).bg(CHROME[7]) }
pub fn border_style() -> Style { Style::default().fg(CHROME[7]) }
pub fn active_border_style() -> Style { Style::default().fg(CHROME[4]) }
pub fn graph_update_style() -> Style { Style::default().fg(CHROME[3]) }
pub fn header_style() -> Style { Style::default().fg(CHROME[0]).bg(CHROME[8]) }

pub fn confidence_color(conf: f64) -> Color {
    if conf >= 0.7 { SUCCESS } else if conf >= 0.4 { WARNING } else { ERROR }
}

// ═══════════════════════════════════════════════════════════════
// ANSI 256-color (for setup wizard, banner — outside ratatui)
// These use 256-color because they print via raw println!
// ═══════════════════════════════════════════════════════════════

pub const ANSI_CHROME: [&str; 12] = [
    "\x1b[38;5;252m", "\x1b[38;5;251m", "\x1b[38;5;189m", "\x1b[38;5;183m",
    "\x1b[38;5;147m", "\x1b[38;5;135m", "\x1b[38;5;98m",  "\x1b[38;5;97m",
    "\x1b[38;5;55m",  "\x1b[38;5;54m",  "\x1b[38;5;53m",  "\x1b[38;5;17m",
];

pub const ANSI_VIOLET: &str = "\x1b[38;5;135m";
pub const ANSI_VIOLET_BRIGHT: &str = "\x1b[38;5;141m";
pub const ANSI_VIOLET_DIM: &str = "\x1b[38;5;97m";
pub const ANSI_LAVENDER: &str = "\x1b[38;5;183m";
pub const ANSI_HIGHLIGHT: &str = "\x1b[38;5;252m";
pub const ANSI_DIM: &str = "\x1b[90m";
pub const ANSI_WHITE: &str = "\x1b[37m";
pub const ANSI_BOLD: &str = "\x1b[1m";
pub const ANSI_RESET: &str = "\x1b[0m";

pub fn chrome_gradient_ansi(text: &str) -> String {
    format!("\x1b[38;5;135m{text}\x1b[0m")
}
