//! ASCII art and animation frames. Only uses chars that render in all terminals.

/// Figlet "slant" font TRACEY — only uses /\-_| characters (no block chars)
pub const FIGLET_TRACEY: &str = r#"
  ______
 /_  __/______ _________  __  __
  / / / ___/ _` / ___/ _ \/ / / /
 / / / /  / /_/ / /__/  __/ /_/ /
/_/ /_/   \__,_/\___/\___/\__, /
                          /____/
"#;

/// Compact TRACEY for narrow terminals
pub const TRACEY_COMPACT: &str = "T R A C E Y";

/// Graph trace frames (thinking animation)
pub const TRACE_FRAMES: [&str; 8] = [
    "◉",
    "◉───",
    "◉─────▸",
    "◉─────▸ ◉",
    "◉─────▸ ◉───",
    "◉─────▸ ◉─────▸",
    "◉─────▸ ◉─────▸ ◉",
    "◉─────▸ ◉─────▸ ◉──▸",
];

/// Tool call pulse
pub const PULSE_FRAMES: [&str; 6] = ["◇", "◈", "◆", "●", "◆", "◈"];

/// Verification spreading
pub const VERIFY_FRAMES: [&str; 6] = ["○", "◎", "◉", "●", "◉", "◎"];
