//! Interactive arrow-key menu for terminal selection.
//! Inspired by Hermes Agent's curses_ui.py checklist pattern.
//! Uses crossterm raw mode for keyboard input.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    terminal::{self, ClearType},
    ExecutableCommand,
};
use std::io::{self, Write};

// Liquid chrome silver + violet palette (256-color)
const SILVER: &str = "\x1b[38;5;252m";
const SILVER_DIM: &str = "\x1b[38;5;248m";
const LAVENDER: &str = "\x1b[38;5;183m";
const VIOLET: &str = "\x1b[38;5;135m";
const VIOLET_DIM: &str = "\x1b[38;5;97m";
const GREEN: &str = "\x1b[38;5;35m";
const DIM: &str = "\x1b[90m";
const BOLD: &str = "\x1b[1m";
const RST: &str = "\x1b[0m";

pub struct MenuItem {
    pub label: String,
    pub description: String,
    pub indicator: Option<String>,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: String::new(),
            indicator: None,
        }
    }

    pub fn with_desc(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_indicator(mut self, ind: impl Into<String>) -> Self {
        self.indicator = Some(ind.into());
        self
    }
}

/// Run an interactive single-select menu with arrow keys.
/// Returns the selected index, or None if cancelled.
pub fn select_menu(title: &str, items: &[MenuItem]) -> Option<usize> {
    let mut cursor_pos: usize = 0;
    let total = items.len();

    // Enter raw mode
    terminal::enable_raw_mode().ok()?;
    let mut stdout = io::stdout();

    // Initial render
    render_menu(&mut stdout, title, items, cursor_pos);

    loop {
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = if cursor_pos == 0 { total - 1 } else { cursor_pos - 1 };
                    render_menu(&mut stdout, title, items, cursor_pos);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    cursor_pos = (cursor_pos + 1) % total;
                    render_menu(&mut stdout, title, items, cursor_pos);
                }
                KeyCode::Enter => {
                    terminal::disable_raw_mode().ok();
                    // Print the selection
                    println!();
                    println!("  {GREEN}✓{RST} Selected: {LAVENDER}{}{RST}", items[cursor_pos].label);
                    return Some(cursor_pos);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    terminal::disable_raw_mode().ok();
                    println!();
                    return None;
                }
                _ => {}
            }
        }
    }
}

fn render_menu(stdout: &mut io::Stdout, title: &str, items: &[MenuItem], cursor: usize) {
    // Move cursor up to overwrite previous render
    let lines_to_clear = items.len() + 4; // title + blank + items + footer + blank
    for _ in 0..lines_to_clear {
        stdout.execute(cursor::MoveUp(1)).ok();
        stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();
    }

    // Title
    write!(stdout, "\r  {LAVENDER}{title}{RST}\n\n").ok();

    // Items
    for (i, item) in items.iter().enumerate() {
        let is_selected = i == cursor;

        let arrow = if is_selected {
            format!("{VIOLET}{BOLD}  → {RST}")
        } else {
            "    ".to_string()
        };

        let label_style = if is_selected {
            format!("{SILVER}{BOLD}")
        } else {
            DIM.to_string()
        };

        let indicator = item.indicator.as_ref()
            .map(|ind| format!("  {GREEN}{ind}{RST}"))
            .unwrap_or_default();

        write!(
            stdout,
            "\r{arrow}{label_style}{}{RST}{indicator}\n",
            item.label
        ).ok();
    }

    // Footer
    write!(stdout, "\r\n  {DIM}↑↓ navigate · Enter select · Esc cancel{RST}\n").ok();
    stdout.flush().ok();
}

/// Animate a causal graph step transition between wizard steps
pub fn animate_step(step_name: &str, step_num: usize) {
    let mut stdout = io::stdout();

    if step_num > 1 {
        // Draw connecting edge from previous node
        write!(stdout, "  {VIOLET_DIM}│{RST}\n").ok();
        stdout.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(40));
    }

    // Animate: node appears
    write!(stdout, "  {SILVER}◉{RST}").ok();
    stdout.flush().ok();
    std::thread::sleep(std::time::Duration::from_millis(60));

    // Edge traces right
    write!(stdout, "{VIOLET}──▸{RST} ").ok();
    stdout.flush().ok();
    std::thread::sleep(std::time::Duration::from_millis(40));

    // Step name appears
    write!(stdout, "{LAVENDER}{step_name}{RST}\n").ok();
    stdout.flush().ok();
    std::thread::sleep(std::time::Duration::from_millis(30));
}

/// Print a section header in the wizard
pub fn print_section_header(step: usize, total: usize, title: &str) {
    println!();
    println!("  {SILVER_DIM}Step {step}/{total}{RST} {VIOLET_DIM}{}{RST}",
        "─".repeat(40));
    println!();
}
