use crate::theme::*;
use std::time::Duration;

/// Gradient logo — violet to lavender, inspired by Hermes's hex gradient approach
/// Each line gets a slightly different shade for a gradient effect
pub fn print_startup_banner() {
    let gradient = [
        "\x1b[38;2;109;40;217m",  // deep violet
        "\x1b[38;2;119;56;226m",
        "\x1b[38;2;129;72;236m",
        "\x1b[38;2;139;92;246m",  // primary violet
        "\x1b[38;2;149;108;248m",
        "\x1b[38;2;159;124;250m",
        "\x1b[38;2;167;139;250m", // bright violet
    ];

    let logo_lines = [
        "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
        "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
        "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
        "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
        "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
        "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
    ];

    println!();

    // Gradient logo
    for (i, line) in logo_lines.iter().enumerate() {
        let color = gradient[i.min(gradient.len() - 1)];
        println!("{color}{line}{ANSI_RESET}");
    }

    // Causal graph beneath logo
    println!();
    println!("  {ANSI_LAVENDER}    ◉{ANSI_DIM}──╌╌──▸{ANSI_LAVENDER} ◉{ANSI_DIM}──╌╌──▸{ANSI_LAVENDER} ◉{ANSI_RESET}");
    println!("  {ANSI_DIM}              └──╌╌──▸{ANSI_LAVENDER} ◉{ANSI_RESET}");
    println!();

    // Tagline
    println!("  {ANSI_VIOLET_BRIGHT}tracing causal connections{ANSI_RESET}");
    println!("  {ANSI_DIM}v{}{ANSI_RESET}", env!("CARGO_PKG_VERSION"));
    println!();
}

/// Animated startup — <700ms total
pub async fn animate_startup() {
    print!("\x1b[?25l"); // hide cursor

    let gradient = [
        "\x1b[38;2;109;40;217m",
        "\x1b[38;2;119;56;226m",
        "\x1b[38;2;129;72;236m",
        "\x1b[38;2;139;92;246m",
        "\x1b[38;2;149;108;248m",
        "\x1b[38;2;159;124;250m",
        "\x1b[38;2;167;139;250m",
    ];

    let logo_lines = [
        "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
        "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
        "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
        "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
        "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
        "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
    ];

    print!("\x1b[2J\x1b[H");
    println!();

    // Phase 1: Graph traces (250ms)
    print!("  {ANSI_LAVENDER}    ◉{ANSI_RESET}");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!("{ANSI_DIM}──╌╌──▸{ANSI_RESET}");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!(" {ANSI_LAVENDER}◉{ANSI_RESET}");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!("{ANSI_DIM}──╌╌──▸{ANSI_RESET}");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    tokio::time::sleep(Duration::from_millis(60)).await;

    println!(" {ANSI_LAVENDER}◉{ANSI_RESET}");
    println!("  {ANSI_DIM}              └──╌╌──▸{ANSI_LAVENDER} ◉{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Phase 2: Logo lines appear (240ms)
    println!();
    for (i, line) in logo_lines.iter().enumerate() {
        let color = gradient[i.min(gradient.len() - 1)];
        println!("{color}{line}{ANSI_RESET}");
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    // Phase 3: Tagline (50ms)
    println!();
    println!("  {ANSI_VIOLET_BRIGHT}tracing causal connections{ANSI_RESET}");
    println!("  {ANSI_DIM}v{}{ANSI_RESET}", env!("CARGO_PKG_VERSION"));
    println!();

    print!("\x1b[?25h"); // show cursor
    tokio::time::sleep(Duration::from_millis(100)).await;
}

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
