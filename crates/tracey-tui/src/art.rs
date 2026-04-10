//! Animation frames for the TUI (ratatui context).
//! Uses only box-drawing + symbols that render cleanly.

/// Thinking spinner — causal graph traces right
pub const GRAPH_TRACE_FRAMES: [&str; 8] = [
    "◉",
    "◉───",
    "◉─────▸",
    "◉─────▸ ◉",
    "◉─────▸ ◉───",
    "◉─────▸ ◉─────▸",
    "◉─────▸ ◉─────▸ ◉",
    "◉─────▸ ◉─────▸ ◉──▸",
];

/// Tool execution — pulsing node
pub const PULSE_FRAMES: [&str; 6] = ["◇", "◈", "◆", "●", "◆", "◈"];

/// Verification — spreading
pub const VERIFY_FRAMES: [&str; 6] = ["○", "◎", "◉", "●", "◉", "◎"];

/// Edge animation
pub const EDGE_FRAMES: [&str; 4] = ["───▸", "╌╌╌▸", "───▸", "━━━▸"];
