use std::time::Duration;

/// The Tracey ASCII logo frames — animated causal graph tracing
pub const LOGO_FRAMES: &[&str] = &[
    // Frame 1: Empty
    r#"

"#,
    // Frame 2: First node appears
    r#"
                    ╭────────╮
                    │  start │
                    ╰────────╯
"#,
    // Frame 3: Edge traces down
    r#"
                    ╭────────╮
                    │  start │
                    ╰───┬────╯
                        │
                        ▼
"#,
    // Frame 4: Second node
    r#"
                    ╭────────╮
                    │  start │
                    ╰───┬────╯
                        │ causes
                    ╭───▼────╮
                    │ parse  │
                    ╰────────╯
"#,
    // Frame 5: Branch
    r#"
                    ╭────────╮
                    │  start │
                    ╰───┬────╯
                        │ causes
                    ╭───▼────╮
                    │ parse  │
                    ╰───┬────╯
                   ┌────┴────┐
                   ▼         ▼
"#,
    // Frame 6: Two leaves
    r#"
                    ╭────────╮
                    │  start │
                    ╰───┬────╯
                        │ causes
                    ╭───▼────╮
                    │ parse  │
                    ╰───┬────╯
                   ┌────┴────┐
              ╭────▼───╮ ╭───▼────╮
              │ reason │ │  act   │
              ╰────────╯ ╰───┬────╯
                             │ traces
                         ╭───▼────╮
                         │ verify │
                         ╰────────╯
"#,
    // Frame 7: Full logo with name
    r#"
                    ╭────────╮
                    │  start │
                    ╰───┬────╯
                        │ causes
                    ╭───▼────╮
                    │ parse  │
                    ╰───┬────╯
                   ┌────┴────┐
              ╭────▼───╮ ╭───▼────╮
              │ reason │ │  act   │
              ╰────────╯ ╰───┬────╯
                             │ traces
                         ╭───▼────╮
                         │ verify │
                         ╰────────╯
"#,
];

pub const LOGO_TEXT: &str = r#"
  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗
  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝
     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝
     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝
     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║
     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝"#;

pub const TAGLINE: &str = "  tracing causal connections";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Startup banner printed to terminal
pub fn print_startup_banner() {
    let cyan = "\x1b[36m";
    let white = "\x1b[1;37m";
    let dim = "\x1b[90m";
    let reset = "\x1b[0m";
    let yellow = "\x1b[33m";

    println!();

    // Print the causal graph
    let graph = LOGO_FRAMES.last().unwrap();
    for line in graph.lines() {
        // Color nodes cyan, edges dim
        let colored = line
            .replace("╭", &format!("{cyan}╭"))
            .replace("╰", &format!("{cyan}╰"))
            .replace("│", &format!("{cyan}│"))
            .replace("╮", &format!("╮{reset}"))
            .replace("causes", &format!("{dim}causes{reset}"))
            .replace("traces", &format!("{dim}traces{reset}"))
            .replace("start", &format!("{white}start{reset}"))
            .replace("parse", &format!("{white}parse{reset}"))
            .replace("reason", &format!("{white}reason{reset}"))
            .replace("act", &format!("{white}act{reset}"))
            .replace("verify", &format!("{white}verify{reset}"));
        println!("{colored}");
    }

    // Print the big text logo
    for line in LOGO_TEXT.lines() {
        println!("{cyan}{line}{reset}");
    }

    println!();
    println!("{yellow}{TAGLINE}{reset}");
    println!("{dim}  v{VERSION}{reset}");
    println!();
}

/// Animated startup — prints frames with delays
pub async fn animate_startup() {
    let cyan = "\x1b[36m";
    let reset = "\x1b[0m";
    let dim = "\x1b[90m";
    let yellow = "\x1b[33m";

    // Clear and hide cursor
    print!("\x1b[?25l");

    for (i, frame) in LOGO_FRAMES.iter().enumerate() {
        // Move cursor to top
        print!("\x1b[H\x1b[2J");

        for line in frame.lines() {
            println!("{cyan}{line}{reset}");
        }

        let delay = if i < 3 { 120 } else { 200 };
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    // Print the big text logo with typewriter effect
    for line in LOGO_TEXT.lines() {
        for ch in line.chars() {
            print!("{cyan}{ch}{reset}");
        }
        println!();
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    println!();
    println!("{yellow}{TAGLINE}{reset}");
    println!("{dim}  v{VERSION}{reset}");
    println!();

    // Show cursor
    print!("\x1b[?25h");
}

/// Spinner frames for different agent states
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
                "╭─", "╭──", "╭───", "╭────", "╭─────",
                "╭──────", "╭───────▶",
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
