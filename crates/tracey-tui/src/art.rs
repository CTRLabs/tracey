//! Braille Unicode ASCII art for Tracey.
//! Causal graph emblem — depicts interconnected nodes forming a DAG.
//! Technique: same as Hermes Agent's HERMES_CADUCEUS (braille chars U+2800-U+28FF).

/// Full causal graph emblem (15 lines × ~30 cols)
/// Depicts: a branching DAG with nodes and edges, representing causal connections.
/// Applied with per-line silver→violet gradient.
pub const CAUSAL_GRAPH_EMBLEM: [&str; 15] = [
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣤⣴⣶⣶⣦⣤⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠀⠀⠀⠀⣠⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣄⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠀⠀⣠⣾⡿⠋⠀⠀⣿⣿⡟⢿⣿⠀⠀⠙⢿⣷⣄⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠀⣴⣿⠟⠀⠀⣠⣾⣿⠟⠀⠀⠻⣿⣷⣄⠀⠻⣿⣦⠀⠀⠀⠀⠀",
    "⠀⠀⣼⣿⠃⠀⢀⣾⣿⡿⠁⠀⢀⡀⠀⠈⢿⣿⣷⡀⠘⣿⣧⠀⠀⠀⠀",
    "⠀⢸⣿⡇⠀⢠⣿⣿⠟⠀⠀⣴⣿⣿⣦⠀⠀⠻⣿⣿⡄⠈⣿⡇⠀⠀⠀",
    "⠀⣿⣿⠀⢀⣿⣿⠃⠀⢀⣾⡿⠁⠈⢿⣷⡀⠀⠘⣿⣿⡀⢸⣿⠀⠀⠀",
    "⠀⣿⣿⠀⣸⣿⡟⠀⠀⣾⡿⠁⢀⡀⠈⢿⣷⠀⠀⢻⣿⣇⢸⣿⠀⠀⠀",
    "⠀⢸⣿⡄⠸⣿⣧⠀⢸⣿⡇⠀⣿⣿⠀⢸⣿⡇⠀⣼⣿⠇⣸⡿⠀⠀⠀",
    "⠀⠀⢿⣷⡀⠻⣿⣧⡈⢿⣷⡀⠙⠋⢀⣾⡿⢁⣼⣿⠟⢀⣿⠃⠀⠀⠀",
    "⠀⠀⠈⢿⣷⣄⠙⢿⣿⣦⡙⢿⣶⣶⡿⢋⣴⣿⡿⠋⣠⣿⠏⠀⠀⠀⠀",
    "⠀⠀⠀⠀⠙⢿⣷⣄⠉⠻⣿⣶⣬⣭⣶⣿⠟⠉⣠⣾⡿⠋⠀⠀⠀⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠈⠻⣿⣦⣄⡉⠛⠛⢉⣠⣴⣿⠟⠁⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠿⣿⣿⠿⠛⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
];

/// Compact causal graph art (5 lines) for narrow terminals / setup wizard
pub const CAUSAL_GRAPH_COMPACT: [&str; 5] = [
    "⠀⠀⠀⣀⣤⣶⣿⣿⣶⣤⣀⠀⠀⠀",
    "⠀⣠⣾⡿⠋⣿⡟⢿⣿⠙⢿⣷⣄⠀",
    "⣼⣿⠃⢀⣾⠟⠀⠀⠻⣷⡀⠘⣿⣧",
    "⠸⣿⣧⡈⢿⣷⣶⣶⣾⡿⢁⣼⣿⠇",
    "⠀⠀⠉⠛⠿⣶⣤⣤⣶⠿⠛⠉⠀⠀",
];

/// Gradient colors for the emblem (256-color, silver → violet)
pub const EMBLEM_COLORS: [&str; 15] = [
    "\x1b[38;5;252m",  // bright silver
    "\x1b[38;5;252m",  // silver
    "\x1b[38;5;251m",  // silver
    "\x1b[38;5;250m",  // light silver
    "\x1b[38;5;189m",  // silver-lavender
    "\x1b[38;5;183m",  // lavender
    "\x1b[38;5;183m",  // lavender
    "\x1b[38;5;147m",  // light violet
    "\x1b[38;5;141m",  // light violet
    "\x1b[38;5;135m",  // violet
    "\x1b[38;5;135m",  // violet
    "\x1b[38;5;98m",   // medium purple
    "\x1b[38;5;97m",   // dark violet
    "\x1b[38;5;55m",   // deep purple
    "\x1b[38;5;55m",   // deep purple
];

pub const EMBLEM_COMPACT_COLORS: [&str; 5] = [
    "\x1b[38;5;252m",  // silver
    "\x1b[38;5;189m",  // silver-lavender
    "\x1b[38;5;183m",  // lavender
    "\x1b[38;5;141m",  // light violet
    "\x1b[38;5;135m",  // violet
];

/// Print the full emblem with gradient colors
pub fn print_emblem() {
    for (i, line) in CAUSAL_GRAPH_EMBLEM.iter().enumerate() {
        println!("{}{line}\x1b[0m", EMBLEM_COLORS[i]);
    }
}

/// Print the compact emblem
pub fn print_emblem_compact() {
    for (i, line) in CAUSAL_GRAPH_COMPACT.iter().enumerate() {
        println!("{}{line}\x1b[0m", EMBLEM_COMPACT_COLORS[i]);
    }
}

/// Animated causal graph spinner frames for the TUI
pub const GRAPH_TRACE_FRAMES: [&str; 8] = [
    "◉",
    "◉──",
    "◉────▸",
    "◉────▸ ◉",
    "◉────▸ ◉──",
    "◉────▸ ◉────▸",
    "◉────▸ ◉────▸ ◉",
    "◉────▸ ◉────▸ ◉──▸",
];

/// Pulsing node frames (for tool calls)
pub const PULSE_FRAMES: [&str; 6] = ["◇", "◈", "◆", "●", "◆", "◈"];

/// Spreading verification frames
pub const VERIFY_FRAMES: [&str; 6] = ["○", "◎", "◉", "●", "◉", "◎"];

/// Edge animation frames (for editing)
pub const EDGE_FRAMES: [&str; 4] = ["──▸", "╌╌▸", "──▸", "━━▸"];
