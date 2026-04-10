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

/// Render the logo with per-LINE 256-color gradient + braille emblem
fn render_logo(f: &mut Frame, area: Rect) {
    // Silver → violet gradient (liquid chrome: metallic at top, colored at bottom)
    let line_colors = [
        CHROME[0],  // bright silver
        CHROME[1],  // silver
        CHROME[2],  // silver-lavender
        CHROME[3],  // lavender
        CHROME[4],  // light violet
        CHROME[5],  // core violet
    ];

    let logo_text = [
        " TRACEY",
        " ══════",
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Simple clean text logo (block chars garble in some terminals)
    lines.push(Line::from(Span::styled(
        " T R A C E Y",
        Style::default().fg(CHROME[0]).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ═══════════════",
        Style::default().fg(CHROME[3]),
    )));
    lines.push(Line::from(""));

    // Braille causal graph emblem
    let emblem_colors = [
        CHROME[0], CHROME[0], CHROME[1], CHROME[2], CHROME[2],
        CHROME[3], CHROME[3], CHROME[4], CHROME[4], CHROME[5],
        CHROME[5], CHROME[6], CHROME[7], CHROME[7], CHROME[8],
    ];
    for (i, art_line) in crate::art::CAUSAL_GRAPH_EMBLEM.iter().enumerate() {
        let color = emblem_colors.get(i).copied().unwrap_or(CHROME[5]);
        lines.push(Line::from(Span::styled(
            format!("  {art_line}"),
            Style::default().fg(color),
        )));
    }

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
