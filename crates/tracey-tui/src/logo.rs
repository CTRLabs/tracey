use crate::theme::{chrome_gradient_ansi, ANSI_CHROME, ANSI_DIM, ANSI_RESET};
use std::time::Duration;

const LOGO_LINES: [&str; 6] = [
    "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
    "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
    "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
    "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
    "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
    "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
];

/// Print the logo with per-character liquid chrome gradient
pub fn print_chrome_logo() {
    println!();
    for line in &LOGO_LINES {
        println!("{}", chrome_gradient_ansi(line));
    }
    println!();
    // Causal graph trace art
    let c2 = ANSI_CHROME[2]; // chrome light
    let c5 = ANSI_CHROME[5]; // core violet
    let c7 = ANSI_CHROME[7]; // deep
    println!("  {c2}    ◉{c7}──╌╌──▸{c2} ◉{c7}──╌╌──▸{c2} ◉{ANSI_RESET}");
    println!("  {c7}              └──╌╌──▸{c2} ◉{ANSI_RESET}");
    println!();
}

/// Print startup banner (no animation)
pub fn print_startup_banner() {
    print_chrome_logo();
    println!("  {}", chrome_gradient_ansi("tracing causal connections"));
    println!("  {ANSI_DIM}v{}{ANSI_RESET}", env!("CARGO_PKG_VERSION"));
    println!();
}

/// Animated startup — liquid chrome logo traces in
pub async fn animate_startup() {
    print!("\x1b[?25l"); // hide cursor
    print!("\x1b[2J\x1b[H"); // clear

    let c2 = ANSI_CHROME[2];
    let c7 = ANSI_CHROME[7];

    // Phase 1: Graph nodes trace in (300ms)
    println!();
    print!("  {c2}    ◉{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!("{c7}──╌╌──▸{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!(" {c2}◉{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    print!("{c7}──╌╌──▸{ANSI_RESET}");
    flush();
    tokio::time::sleep(Duration::from_millis(60)).await;

    println!(" {c2}◉{ANSI_RESET}");
    println!("  {c7}              └──╌╌──▸{c2} ◉{ANSI_RESET}");
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Phase 2: Chrome logo appears (240ms)
    println!();
    for line in &LOGO_LINES {
        println!("{}", chrome_gradient_ansi(line));
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    // Phase 3: Tagline (50ms)
    println!();
    println!("  {}", chrome_gradient_ansi("tracing causal connections"));
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
