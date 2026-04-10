use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};

// ═══════════════════════════════════════════════════════════════
// LIQUID CHROME — 256-color indexed palette
// Uses Color::Indexed(N) which works in ALL terminals including tmux
// Color::Rgb does NOT work in tmux (renders as green/wrong color)
// ═══════════════════════════════════════════════════════════════

pub const CHROME: [Color; 12] = [
    Color::Indexed(252),  //  0: bright silver
    Color::Indexed(251),  //  1: silver
    Color::Indexed(189),  //  2: silver-lavender
    Color::Indexed(183),  //  3: lavender
    Color::Indexed(147),  //  4: chrome light violet
    Color::Indexed(135),  //  5: core violet (PRIMARY)
    Color::Indexed(98),   //  6: deep mid
    Color::Indexed(97),   //  7: deep violet
    Color::Indexed(55),   //  8: dark purple
    Color::Indexed(54),   //  9: shadow
    Color::Indexed(53),   // 10: deep shadow
    Color::Indexed(17),   // 11: near-black blue
];

// ANSI 256-color escape codes (for non-ratatui contexts)
pub const ANSI_CHROME: [&str; 12] = [
    "\x1b[38;5;252m", "\x1b[38;5;251m", "\x1b[38;5;189m", "\x1b[38;5;183m",
    "\x1b[38;5;147m", "\x1b[38;5;135m", "\x1b[38;5;98m",  "\x1b[38;5;97m",
    "\x1b[38;5;55m",  "\x1b[38;5;54m",  "\x1b[38;5;53m",  "\x1b[38;5;17m",
];

// Named aliases
pub const VIOLET: Color = Color::Indexed(135);          // chrome[5]
pub const VIOLET_BRIGHT: Color = Color::Indexed(141);   // between chrome[3] and [4]
pub const VIOLET_DIM: Color = Color::Indexed(97);       // chrome[7]
pub const VIOLET_MUTED: Color = Color::Indexed(54);     // chrome[9]
pub const LAVENDER: Color = Color::Indexed(183);         // chrome[3]
pub const CHROME_HIGHLIGHT: Color = Color::Indexed(252); // chrome[0]
pub const CHROME_SHADOW: Color = Color::Indexed(55);     // chrome[8]

// Layer colors for causal graph
pub const CODE_COLOR: Color = VIOLET;
pub const EXEC_COLOR: Color = Color::Indexed(214);     // amber/orange
pub const KNOWLEDGE_COLOR: Color = Color::Indexed(35);  // emerald green
pub const PROJECT_COLOR: Color = Color::Indexed(204);   // rose/pink

// UI base colors
pub const BG: Color = Color::Indexed(17);               // deep navy
pub const FG: Color = Color::Indexed(253);              // near-white
pub const DIM: Color = Color::Indexed(245);             // gray
pub const ERROR: Color = Color::Indexed(196);           // red
pub const SUCCESS: Color = Color::Indexed(35);          // green
pub const WARNING: Color = Color::Indexed(220);         // yellow

// ═══════════════════════════════════════════════════════════════
// GRADIENT FUNCTIONS
// ═══════════════════════════════════════════════════════════════

/// Render text as chrome gradient ANSI string (for non-ratatui).
pub fn chrome_gradient_ansi(text: &str) -> String {
    // Per-line only — per-character breaks block chars
    format!("\x1b[38;5;135m{text}\x1b[0m")
}

/// Confidence → color
pub fn confidence_color(conf: f64) -> Color {
    if conf >= 0.7 { SUCCESS }
    else if conf >= 0.4 { WARNING }
    else { ERROR }
}

// ═══════════════════════════════════════════════════════════════
// PRE-BUILT STYLES
// ═══════════════════════════════════════════════════════════════

pub fn user_style() -> Style {
    Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD)
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
    Style::default().fg(CHROME[0]).bg(CHROME[7])
}

pub fn border_style() -> Style {
    Style::default().fg(CHROME[7])
}

pub fn active_border_style() -> Style {
    Style::default().fg(CHROME[4])
}

pub fn graph_update_style() -> Style {
    Style::default().fg(CHROME[3])
}

pub fn header_style() -> Style {
    Style::default().fg(CHROME[0]).bg(CHROME[8])
}

// ANSI escape codes (for logo, banner, etc)
pub const ANSI_VIOLET: &str = "\x1b[38;5;135m";
pub const ANSI_VIOLET_BRIGHT: &str = "\x1b[38;5;141m";
pub const ANSI_VIOLET_DIM: &str = "\x1b[38;5;97m";
pub const ANSI_LAVENDER: &str = "\x1b[38;5;183m";
pub const ANSI_HIGHLIGHT: &str = "\x1b[38;5;252m";
pub const ANSI_DIM: &str = "\x1b[90m";
pub const ANSI_WHITE: &str = "\x1b[37m";
pub const ANSI_BOLD: &str = "\x1b[1m";
pub const ANSI_RESET: &str = "\x1b[0m";
