use crate::theme::{ANSI_CHROME, ANSI_DIM, ANSI_RESET};
use std::time::Duration;

const S: &str = "\x1b[38;5;252m";  // silver
const L: &str = "\x1b[38;5;183m";  // lavender
const D: &str = "\x1b[38;5;97m";   // deep
const V: &str = "\x1b[38;5;135m";  // violet
const RST: &str = "\x1b[0m";

/// Print the causal graph logo (no block chars, no braille)
pub fn print_chrome_logo() {
    println!();
    let lines = [
        format!("               {S}╭───────╮{RST}"),
        format!("        {S}╭──────│{RST} {L}parse{RST} {S}│──────╮{RST}"),
        format!("        {D}│{RST}      {S}╰───────╯{RST}      {D}│{RST}"),
        format!("   {S}╭────▼───╮{RST}       {S}╭────▼───╮{RST}"),
        format!("   {S}│{RST} {L}reason{RST} {S}│{RST}       {S}│{RST}  {L}act{RST}   {S}│{RST}"),
        format!("   {S}╰────┬───╯{RST}       {S}╰────┬───╯{RST}"),
        format!("        {D}╰──────╮verify╭──────╯{RST}"),
        format!("               {D}╰──┬───╯{RST}"),
    ];
    for line in &lines {
        println!("{line}");
    }
    println!();
    println!("     {S}\x1b[1mT  R  A  C  E  Y{RST}");
    println!("  {L}tracing causal connections{RST}");
    println!();
}

/// Print startup banner
pub fn print_startup_banner() {
    print_chrome_logo();
    println!("  {ANSI_DIM}v{}{RST}", env!("CARGO_PKG_VERSION"));
    println!();
}

/// Animated startup — graph nodes trace in one by one
pub async fn animate_startup() {
    print!("\x1b[?25l"); // hide cursor
    print!("\x1b[2J\x1b[H"); // clear

    let frames = [
        format!("               {S}╭───────╮{RST}"),
        format!("        {S}╭──────│{RST} {L}parse{RST} {S}│──────╮{RST}"),
        format!("        {D}│{RST}      {S}╰───────╯{RST}      {D}│{RST}"),
        format!("   {S}╭────▼───╮{RST}       {S}╭────▼───╮{RST}"),
        format!("   {S}│{RST} {L}reason{RST} {S}│{RST}       {S}│{RST}  {L}act{RST}   {S}│{RST}"),
        format!("   {S}╰────┬───╯{RST}       {S}╰────┬───╯{RST}"),
        format!("        {D}╰──────╮verify╭──────╯{RST}"),
        format!("               {D}╰──┬───╯{RST}"),
    ];

    println!();
    for frame in &frames {
        println!("{frame}");
        tokio::time::sleep(Duration::from_millis(60)).await;
    }

    println!();
    // Typewriter effect for the name
    for ch in "T  R  A  C  E  Y".chars() {
        print!("{S}\x1b[1m{ch}{RST}");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    println!();
    println!("  {L}tracing causal connections{RST}");
    println!("  {ANSI_DIM}v{}{RST}", env!("CARGO_PKG_VERSION"));
    println!();

    print!("\x1b[?25h"); // show cursor
    tokio::time::sleep(Duration::from_millis(200)).await;
}

pub struct Spinner {
    frames: Vec<&'static str>,
    index: usize,
}

impl Spinner {
    pub fn thinking() -> Self {
        Self { frames: crate::art::GRAPH_TRACE_FRAMES.to_vec(), index: 0 }
    }
    pub fn tool_call() -> Self {
        Self { frames: crate::art::PULSE_FRAMES.to_vec(), index: 0 }
    }
    pub fn verifying() -> Self {
        Self { frames: crate::art::VERIFY_FRAMES.to_vec(), index: 0 }
    }
    pub fn next(&mut self) -> &str {
        let frame = self.frames[self.index];
        self.index = (self.index + 1) % self.frames.len();
        frame
    }
}
