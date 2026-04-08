use ratatui::style::{Color, Modifier, Style};

// Violet palette — unclaimed in the coding agent space
pub const VIOLET: Color = Color::Rgb(139, 92, 246);       // #8B5CF6 — primary
pub const VIOLET_BRIGHT: Color = Color::Rgb(167, 139, 250); // #A78BFA — highlights
pub const VIOLET_DIM: Color = Color::Rgb(109, 40, 217);   // #6D28D9 — borders
pub const VIOLET_MUTED: Color = Color::Rgb(76, 29, 149);  // #4C1D95 — backgrounds
pub const LAVENDER: Color = Color::Rgb(196, 181, 253);     // #C4B5FD — accent

// Layer colors for causal graph
pub const CODE_COLOR: Color = VIOLET;                       // code = violet
pub const EXEC_COLOR: Color = Color::Rgb(245, 158, 11);    // exec = amber
pub const KNOWLEDGE_COLOR: Color = Color::Rgb(16, 185, 129); // knowledge = emerald
pub const PROJECT_COLOR: Color = Color::Rgb(244, 63, 94);   // project = rose

// UI colors
pub const BG: Color = Color::Rgb(15, 15, 20);              // near-black
pub const FG: Color = Color::Rgb(229, 229, 234);           // soft white
pub const DIM: Color = Color::Rgb(100, 100, 115);          // muted text
pub const ERROR: Color = Color::Rgb(239, 68, 68);          // red
pub const SUCCESS: Color = Color::Rgb(34, 197, 94);        // green
pub const WARNING: Color = Color::Rgb(234, 179, 8);        // yellow

// Pre-built styles
pub fn user_style() -> Style {
    Style::default().fg(VIOLET_BRIGHT).add_modifier(Modifier::BOLD)
}

pub fn assistant_style() -> Style {
    Style::default().fg(FG)
}

pub fn tool_style() -> Style {
    Style::default().fg(DIM)
}

pub fn error_style() -> Style {
    Style::default().fg(ERROR)
}

pub fn status_bar_style() -> Style {
    Style::default().fg(Color::Black).bg(VIOLET)
}

pub fn border_style() -> Style {
    Style::default().fg(VIOLET_DIM)
}

pub fn active_border_style() -> Style {
    Style::default().fg(VIOLET)
}

pub fn graph_update_style() -> Style {
    Style::default().fg(LAVENDER)
}

// ANSI escape codes for non-ratatui contexts (logo, oneshot mode)
pub const ANSI_VIOLET: &str = "\x1b[38;2;139;92;246m";
pub const ANSI_VIOLET_BRIGHT: &str = "\x1b[38;2;167;139;250m";
pub const ANSI_VIOLET_DIM: &str = "\x1b[38;2;109;40;217m";
pub const ANSI_LAVENDER: &str = "\x1b[38;2;196;181;253m";
pub const ANSI_DIM: &str = "\x1b[38;2;100;100;115m";
pub const ANSI_WHITE: &str = "\x1b[38;2;229;229;234m";
pub const ANSI_BOLD: &str = "\x1b[1m";
pub const ANSI_RESET: &str = "\x1b[0m";
