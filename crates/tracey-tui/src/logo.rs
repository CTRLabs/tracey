use crate::theme::*;
use std::time::Duration;

pub const LOGO_TEXT: &str = r#"
  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗
  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝
     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝
     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝
     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║
     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝"#;

pub const CAUSAL_GRAPH: &str = r#"      ◉──╌╌──▸ ◉──╌╌──▸ ◉
                └──╌╌──▸ ◉"#;

pub const TAGLINE: &str = "  tracing causal connections";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print the startup banner (no animation)
pub fn print_startup_banner() {
    println!();

    // Logo in violet
    for line in LOGO_TEXT.lines() {
        println!("{ANSI_VIOLET}{line}{ANSI_RESET}");
    }
    println!();

    // Causal graph in lavender
    for line in CAUSAL_GRAPH.lines() {
        println!("{ANSI_LAVENDER}{line}{ANSI_RESET}");
    }
    println!();

    // Tagline and version
    println!("{ANSI_VIOLET_BRIGHT}{TAGLINE}{ANSI_RESET}");
    println!("{ANSI_DIM}  v{VERSION}{ANSI_RESET}");
    println!();
}

/// Animated startup — nodes appear, edges trace, text fades in
/// Total duration: ~700ms
pub async fn animate_startup() {
    // Hide cursor
    print!("\x1b[?25l");

    // Clear screen
    print!("\x1b[2J\x1b[H");

    // Frame 1: First node (50ms)
    println!();
    println!();
    println!("{ANSI_VIOLET}      ◉{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Frame 2: Edge traces (80ms)
    print!("\x1b[H\x1b[2J");
    println!();
    println!();
    println!("{ANSI_VIOLET}      ◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Frame 3: Second node (50ms)
    print!("\x1b[H\x1b[2J");
    println!();
    println!();
    println!("{ANSI_VIOLET}      ◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Frame 4: More tracing (80ms)
    print!("\x1b[H\x1b[2J");
    println!();
    println!();
    println!("{ANSI_VIOLET}      ◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Frame 5: Branch appears (80ms)
    print!("\x1b[H\x1b[2J");
    println!();
    println!();
    println!("{ANSI_VIOLET}      ◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_RESET}");
    println!("{ANSI_LAVENDER}                └──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Frame 6: Logo text appears line by line (30ms per line)
    print!("\x1b[H\x1b[2J");
    println!();
    for line in LOGO_TEXT.lines() {
        println!("{ANSI_VIOLET}{line}{ANSI_RESET}");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    println!();
    println!("{ANSI_VIOLET}      ◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_LAVENDER}──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_RESET}");
    println!("{ANSI_LAVENDER}                └──╌╌──▸{ANSI_RESET} {ANSI_VIOLET}◉{ANSI_RESET}");
    println!();

    // Final: tagline and version
    tokio::time::sleep(Duration::from_millis(50)).await;
    println!("{ANSI_VIOLET_BRIGHT}{TAGLINE}{ANSI_RESET}");
    println!("{ANSI_DIM}  v{VERSION}{ANSI_RESET}");
    println!();

    // Show cursor
    print!("\x1b[?25h");

    // Brief pause before TUI takes over
    tokio::time::sleep(Duration::from_millis(100)).await;
}

/// Spinner frames for different agent states (violet-themed)
pub struct Spinner {
    frames: Vec<&'static str>,
    index: usize,
}

impl Spinner {
    pub fn thinking() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            index: 0,
        }
    }

    pub fn tracing() -> Self {
        Self {
            frames: vec![
                "◉╌", "◉╌╌", "◉╌╌╌", "◉╌╌╌▸",
                "◉╌╌╌▸◉", "◉╌╌╌▸◉╌", "◉╌╌╌▸◉╌╌▸",
            ],
            index: 0,
        }
    }

    pub fn causify() -> Self {
        Self {
            frames: vec!["◇", "◈", "◆", "◈", "◇", "○", "●", "○"],
            index: 0,
        }
    }

    pub fn next(&mut self) -> &str {
        let frame = self.frames[self.index];
        self.index = (self.index + 1) % self.frames.len();
        frame
    }
}
