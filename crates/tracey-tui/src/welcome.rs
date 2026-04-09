use crate::theme::{self, *};
use ratatui::prelude::*;
use ratatui::widgets::*;

/// Information for the welcome panel
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

/// Render the Hermes-style welcome panel that fills the chat area.
/// Two-column layout: left (logo art + info) | right (tools + graph + commands)
pub fn render_welcome(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CHROME[7]))
        .border_type(BorderType::Rounded)
        .title(Span::styled(
            format!(" ◆ tracey v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 10 || inner.width < 40 {
        // Too small — just show minimal info
        let text = Paragraph::new(vec![
            Line::from(Span::styled("◆ tracey", Style::default().fg(CHROME[3]).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(
                format!("{} · {}", info.model, info.provider),
                Style::default().fg(DIM),
            )),
        ]);
        f.render_widget(text, inner);
        return;
    }

    // Split into left (60%) and right (40%)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner);

    render_left_column(f, cols[0], info);
    render_right_column(f, cols[1], info);
}

fn render_left_column(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    let mut lines: Vec<Line> = Vec::new();

    // Causal graph ASCII art (circuit trace style)
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("       ◉", Style::default().fg(CHROME[2])),
        Span::styled("──╌╌──▸", Style::default().fg(CHROME[7])),
        Span::styled(" ◉", Style::default().fg(CHROME[2])),
        Span::styled("──╌╌──▸", Style::default().fg(CHROME[7])),
        Span::styled(" ◉", Style::default().fg(CHROME[2])),
    ]));
    lines.push(Line::from(vec![
        Span::styled("                └──╌╌──▸", Style::default().fg(CHROME[7])),
        Span::styled(" ◉", Style::default().fg(CHROME[2])),
    ]));
    lines.push(Line::from(vec![
        Span::styled("       ◉", Style::default().fg(CHROME[3])),
        Span::styled("──╌╌──▸", Style::default().fg(CHROME[8])),
        Span::styled(" ◉", Style::default().fg(CHROME[3])),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Model and provider info
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(&info.model, Style::default().fg(CHROME[1]).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(&info.provider, Style::default().fg(CHROME[4])),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(truncate_str(&info.cwd, area.width as usize - 4), Style::default().fg(DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Session: ", Style::default().fg(DIM)),
        Span::styled(format!("#{}", info.session_number), Style::default().fg(CHROME[4])),
    ]));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn render_right_column(f: &mut Frame, area: Rect, info: &WelcomeInfo) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));

    // Tools section
    lines.push(Line::from(Span::styled(
        "  Available Tools",
        Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD),
    )));

    let tools_line = info.tools.iter()
        .map(|t| Span::styled(format!(" {t} "), Style::default().fg(CHROME[4])))
        .collect::<Vec<_>>();
    lines.push(Line::from(vec![Span::raw("  ")]));
    // Show tools in a wrapped line
    let tools_str = info.tools.join("  ");
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(tools_str, Style::default().fg(DIM)),
    ]));
    lines.push(Line::from(""));

    // Graph section
    lines.push(Line::from(Span::styled(
        "  Causal Graph",
        Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD),
    )));
    if info.graph_nodes > 0 {
        lines.push(Line::from(vec![
            Span::styled("  ◈ ", Style::default().fg(LAVENDER)),
            Span::styled(
                format!("{} nodes, {} edges", info.graph_nodes, info.graph_edges),
                Style::default().fg(CHROME[3]),
            ),
        ]));
        if !info.languages.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  ◈ ", Style::default().fg(LAVENDER)),
                Span::styled(
                    info.languages.join(", "),
                    Style::default().fg(DIM),
                ),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ◈ ", Style::default().fg(DIM)),
            Span::styled("empty (builds on first query)", Style::default().fg(DIM)),
        ]));
    }
    lines.push(Line::from(""));

    // Quick commands
    lines.push(Line::from(Span::styled(
        "  Quick Start",
        Style::default().fg(CHROME[2]).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("  /help    ", Style::default().fg(CHROME[4])),
        Span::styled("all commands", Style::default().fg(DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  /graph   ", Style::default().fg(CHROME[4])),
        Span::styled("causal graph", Style::default().fg(DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  /cost    ", Style::default().fg(CHROME[4])),
        Span::styled("token usage", Style::default().fg(DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  /why     ", Style::default().fg(CHROME[4])),
        Span::styled("root cause", Style::default().fg(DIM)),
    ]));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else if max > 6 { format!("...{}", &s[s.len() - max + 3..]) }
    else { s[..max].to_string() }
}
