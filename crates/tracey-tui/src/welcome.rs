use crate::theme::{self, *};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub struct WelcomeInfo {
    pub model: String,
    pub provider: String,
    pub cwd: String,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub languages: Vec<String>,
    pub tools: Vec<String>,
    pub session_number: u64,
}

/// Render a premium welcome panel inspired by Hermes Agent.
/// Features: massive ASCII logo, two-column info panel, chrome colors.
pub fn render_welcome(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    // Split into: logo area (top) | info panel (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // ASCII art logo
            Constraint::Length(1),  // separator
            Constraint::Min(10),   // two-column info panel
            Constraint::Length(2), // welcome message
        ])
        .split(area);

    render_logo(f, chunks[0]);
    render_separator(f, chunks[1], info);
    render_info_panel(f, chunks[2], info);
    render_welcome_message(f, chunks[3]);
}

/// Render the ASCII art logo with per-LINE gradient colors
/// (Per-character breaks with block elements — per-line is clean like Hermes)
fn render_logo(f: &mut Frame, area: Rect) {
    // Per-line gradient: bright at top → deep at bottom (liquid chrome)
    let line_colors = [
        CHROME[1],  // bright lavender
        CHROME[2],  // chrome light
        CHROME[3],  // light violet
        CHROME[4],  // mid bright
        CHROME[5],  // core violet
        CHROME[6],  // deep mid
    ];

    let logo_text = [
        "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
        "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
        "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
        "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
        "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
        "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (i, text) in logo_text.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(line_colors[i]).add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));

    // Causal graph trace art
    lines.push(Line::from(vec![
        Span::styled("        ◉", Style::default().fg(CHROME[2])),
        Span::styled("──╌╌──▸", Style::default().fg(CHROME[6])),
        Span::styled(" ◉", Style::default().fg(CHROME[2])),
        Span::styled("──╌╌──▸", Style::default().fg(CHROME[6])),
        Span::styled(" ◉", Style::default().fg(CHROME[2])),
        Span::raw("     "),
        Span::styled("tracing causal connections", Style::default().fg(CHROME[3]).add_modifier(Modifier::ITALIC)),
    ]));

    f.render_widget(Paragraph::new(lines), area);
}

/// Separator with version info
fn render_separator(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    let version = env!("CARGO_PKG_VERSION");
    let sep_text = format!(
        "─── ◆ tracey v{version} ─── {} · {} ───",
        info.provider, info.model
    );

    // Pad to fill width
    let remaining = (area.width as usize).saturating_sub(sep_text.len());
    let full_sep = format!("{sep_text}{}", "─".repeat(remaining));

    let line = Line::from(Span::styled(full_sep, Style::default().fg(CHROME[6])));
    f.render_widget(Paragraph::new(vec![line]), area);
}

/// Two-column info panel
fn render_info_panel(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_left_info(f, cols[0], info);
    render_right_info(f, cols[1], info);
}

fn render_left_info(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Available Tools",
        Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(Line::from(""));

    // Tools in a nice layout
    for tool in &info.tools {
        lines.push(Line::from(vec![
            Span::styled("    ● ", Style::default().fg(CHROME[4])),
            Span::styled(tool.clone(), Style::default().fg(CHROME[2])),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Causal Graph",
        Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(Line::from(""));

    if info.graph_nodes > 0 {
        lines.push(Line::from(vec![
            Span::styled("    ◈ ", Style::default().fg(CHROME[2])),
            Span::styled(
                format!("{} nodes", info.graph_nodes),
                Style::default().fg(CHROME[3]),
            ),
            Span::styled(" · ", Style::default().fg(CHROME[7])),
            Span::styled(
                format!("{} edges", info.graph_edges),
                Style::default().fg(CHROME[3]),
            ),
        ]));
        if !info.languages.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("    ◈ ", Style::default().fg(CHROME[4])),
                Span::styled(
                    info.languages.join(", "),
                    Style::default().fg(DIM),
                ),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("    ◈ ", Style::default().fg(DIM)),
            Span::styled("builds on first query", Style::default().fg(DIM)),
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_right_info(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Quick Start",
        Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(Line::from(""));

    let commands = [
        ("/help", "all commands"),
        ("/graph show", "view causal graph"),
        ("/graph impact <file>", "impact analysis"),
        ("/why <error>", "root cause trace"),
        ("/whatif edit <file>", "predict changes"),
        ("/cost", "token usage & cost"),
        ("/model <name>", "switch model"),
        ("/clear", "new conversation"),
    ];

    for (cmd, desc) in &commands {
        lines.push(Line::from(vec![
            Span::styled(format!("    {cmd:<22}"), Style::default().fg(CHROME[3])),
            Span::styled(desc.to_string(), Style::default().fg(DIM)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Session Info",
        Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    Session  ", Style::default().fg(DIM)),
        Span::styled(format!("#{}", info.session_number), Style::default().fg(CHROME[3])),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    CWD      ", Style::default().fg(DIM)),
        Span::styled(
            truncate_str(&info.cwd, area.width as usize - 16),
            Style::default().fg(DIM),
        ),
    ]));

    f.render_widget(Paragraph::new(lines), area);
}

fn render_welcome_message(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Welcome! ", Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD)),
            Span::styled(
                "Type your message or /help for commands.",
                Style::default().fg(DIM),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else if max > 6 { format!("...{}", &s[s.len() - max + 3..]) }
    else { s[..max].to_string() }
}
