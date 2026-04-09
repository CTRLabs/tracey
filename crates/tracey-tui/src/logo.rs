use crate::theme::{ANSI_CHROME, ANSI_DIM, ANSI_RESET};
use std::time::Duration;

const LOGO_LINES: [&str; 6] = [
    "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
    "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
    "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
    "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
    "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
    "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
];

// Per-LINE gradient (not per-character — block chars break with per-char ANSI)
// Bright lavender → core violet → deep violet (liquid chrome top-to-bottom)
const LINE_COLORS: [&str; 6] = [
    "\x1b[38;2;230;220;255m",  // bright lavender
    "\x1b[38;2;200;180;255m",  // light violet
    "\x1b[38;2;170;140;250m",  // mid violet
    "\x1b[38;2;139;92;246m",   // core violet
    "\x1b[38;2;110;70;220m",   // deep violet
    "\x1b[38;2;85;50;190m",    // darker violet
];

/// Print the logo with per-LINE liquid chrome gradient
pub fn print_chrome_logo() {
    println!();
    for (i, line) in LOGO_LINES.iter().enumerate() {
        println!("{}{line}{ANSI_RESET}", LINE_COLORS[i]);
    }
    println!();
    let c2 = ANSI_CHROME[2]; // chrome light
    let c6 = ANSI_CHROME[6]; // deep
    println!("  {c2}    ◉{c6}──╌╌──▸{c2} ◉{c6}──╌╌──▸{c2} ◉{ANSI_RESET}");
    println!("  {c6}              └──╌╌──▸{c2} ◉{ANSI_RESET}");
    println!();
}

/// Print startup banner (no animation)
pub fn print_startup_banner() {
    print_chrome_logo();
    let c3 = ANSI_CHROME[3];
    println!("  {c3}tracing causal connections{ANSI_RESET}");
    println!("  {ANSI_DIM}v{}{ANSI_RESET}", env!("CARGO_PKG_VERSION"));
    println!();
}

/// Animated startup — logo traces in line by line
pub async fn animate_startup() {
    print!("\x1b[?25l"); // hide cursor
    print!("\x1b[2J\x1b[H"); // clear

    let c2 = ANSI_CHROME[2];
    let c6 = ANSI_CHROME[6];

    // Phase 1: Graph nodes trace in (300ms)
    println!();
    print!("  {c2}    ◉{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!("{c6}──╌╌──▸{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!(" {c2}◉{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!("{c6}──╌╌──▸{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    println!(" {c2}◉{ANSI_RESET}");
    println!("  {c6}              └──╌╌──▸{c2} ◉{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Phase 2: Logo lines appear with gradient (240ms)
    println!();
    for (i, line) in LOGO_LINES.iter().enumerate() {
        println!("{}{line}{ANSI_RESET}", LINE_COLORS[i]);
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    // Phase 3: Tagline
    println!();
    let c3 = ANSI_CHROME[3];
    println!("  {c3}tracing causal connections{ANSI_RESET}");
    println!("  {ANSI_DIM}v{}{ANSI_RESET}", env!("CARGO_PKG_VERSION"));
    println!();

    print!("\x1b[?25h"); // show cursor
    tokio::time::sleep(Duration::from_millis(100)).await;
}

fn flush() {
    use std::io::Write;
    std::io::stdout().flush().ok();
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
