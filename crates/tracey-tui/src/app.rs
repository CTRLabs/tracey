use crate::theme;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::Duration;
use tracey_core::events::{AgentEvent, Submission, UiHandle};

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    Error,
    GraphUpdate,
}

pub struct App {
    pub messages: Vec<DisplayMessage>,
    pub input: String,
    pub scroll_offset: u16,
    pub status: String,
    pub spinner_state: usize,
    pub is_processing: bool,
    pub should_quit: bool,
    pub graph_stats: String,
    pub model_name: String,
    pub ui_handle: UiHandle,
}

impl App {
    pub fn new(ui_handle: UiHandle) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            scroll_offset: 0,
            status: "ready".into(),
            spinner_state: 0,
            is_processing: false,
            should_quit: false,
            graph_stats: String::new(),
            model_name: "tracey".into(),
            ui_handle,
        }
    }

    pub async fn run(&mut self, terminal: &mut ratatui::Terminal<impl Backend>) -> std::io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            if self.is_processing {
                                let _ = self.ui_handle.submit(Submission::Interrupt).await;
                                self.is_processing = false;
                                self.status = "interrupted".into();
                            } else {
                                self.should_quit = true;
                            }
                        }
                        (KeyCode::Enter, _) if !self.is_processing => {
                            if !self.input.trim().is_empty() {
                                let msg = self.input.clone();
                                self.input.clear();
                                self.scroll_offset = 0;
                                self.messages.push(DisplayMessage {
                                    role: MessageRole::User,
                                    content: msg.clone(),
                                    tool_name: None,
                                });
                                let _ = self.ui_handle.submit(Submission::UserMessage {
                                    content: msg,
                                    attachments: vec![],
                                }).await;
                                self.is_processing = true;
                                self.status = "thinking...".into();
                            }
                        }
                        (KeyCode::Char(c), _) if !self.is_processing => {
                            self.input.push(c);
                        }
                        (KeyCode::Backspace, _) if !self.is_processing => {
                            self.input.pop();
                        }
                        (KeyCode::Up, _) => {
                            self.scroll_offset = self.scroll_offset.saturating_add(3);
                        }
                        (KeyCode::Down, _) => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(3);
                        }
                        (KeyCode::Esc, _) if self.is_processing => {
                            let _ = self.ui_handle.submit(Submission::Interrupt).await;
                            self.is_processing = false;
                        }
                        _ => {}
                    }
                }
            }

            while let Ok(event) = self.ui_handle.event_rx.try_recv() {
                self.handle_agent_event(event);
            }

            if self.should_quit {
                let _ = self.ui_handle.submit(Submission::Shutdown).await;
                return Ok(());
            }

            if self.is_processing {
                self.spinner_state = (self.spinner_state + 1) % 10;
            }
        }
    }

    fn handle_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::AssistantChunk { text, .. } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Assistant {
                        last.content.push_str(&text);
                        return;
                    }
                }
                self.messages.push(DisplayMessage {
                    role: MessageRole::Assistant,
                    content: text,
                    tool_name: None,
                });
            }
            AgentEvent::ToolCallStart { tool_call, .. } => {
                self.status = format!("◆ {}...", tool_call.name);
                self.messages.push(DisplayMessage {
                    role: MessageRole::Tool,
                    content: format!("calling {}...", tool_call.name),
                    tool_name: Some(tool_call.name),
                });
            }
            AgentEvent::ToolCallEnd { result, is_error, .. } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Tool {
                        let name = last.tool_name.clone().unwrap_or_default();
                        let preview = truncate_lines(&result, 3);
                        last.content = if is_error {
                            format!("{name} failed: {preview}")
                        } else {
                            format!("{name} — {preview}")
                        };
                    }
                }
            }
            AgentEvent::GraphUpdate { description, .. } => {
                self.graph_stats = description;
            }
            AgentEvent::TurnComplete { usage, .. } => {
                self.is_processing = false;
                self.status = if let Some(u) = usage {
                    format!("{} in · {} out", u.input_tokens, u.output_tokens)
                } else {
                    "done".into()
                };
            }
            AgentEvent::Error { message, .. } => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: message,
                    tool_name: None,
                });
                self.is_processing = false;
                self.status = "error".into();
            }
            AgentEvent::Status { message } => {
                self.status = message;
            }
            _ => {}
        }
    }

    fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // messages
                Constraint::Length(3), // input
                Constraint::Length(1), // status
            ])
            .split(f.area());

        self.render_messages(f, chunks[0]);
        self.render_input(f, chunks[1]);
        self.render_status(f, chunks[2]);
    }

    fn render_messages(&self, f: &mut Frame, area: Rect) {
        let inner_width = area.width.saturating_sub(4) as usize;
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            match &msg.role {
                MessageRole::User => {
                    // Hermes-inspired box: ╭─◉ you ─╮ ... ╰────╯
                    lines.push(Line::from(""));
                    let header = format!("╭─◉ you {}", "─".repeat(inner_width.saturating_sub(10)));
                    lines.push(Line::from(Span::styled(header, theme::user_style())));
                    for line in msg.content.lines() {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(theme::VIOLET_DIM)),
                            Span::styled(line.to_string(), Style::default().fg(theme::FG)),
                        ]));
                    }
                    let footer = format!("╰{}", "─".repeat(inner_width.saturating_sub(2)));
                    lines.push(Line::from(Span::styled(footer, Style::default().fg(theme::VIOLET_DIM))));
                }

                MessageRole::Assistant => {
                    // ╭─◆ tracey ─╮ ... ╰────╯
                    lines.push(Line::from(""));
                    let header = format!("╭─◆ tracey {}", "─".repeat(inner_width.saturating_sub(13)));
                    lines.push(Line::from(Span::styled(header, Style::default().fg(theme::VIOLET))));
                    for line in msg.content.lines() {
                        let content_style = if line.starts_with("```") || line.starts_with("    ") || line.starts_with('\t') {
                            Style::default().fg(theme::LAVENDER)
                        } else if line.starts_with("- ") || line.starts_with("* ") {
                            Style::default().fg(theme::FG)
                        } else if line.starts_with('#') {
                            Style::default().fg(theme::VIOLET_BRIGHT).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme::FG)
                        };
                        lines.push(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(theme::VIOLET_DIM)),
                            Span::styled(line.to_string(), content_style),
                        ]));
                    }
                    let footer = format!("╰{}", "─".repeat(inner_width.saturating_sub(2)));
                    lines.push(Line::from(Span::styled(footer, Style::default().fg(theme::VIOLET_DIM))));
                }

                MessageRole::Tool => {
                    // Compact tool display: ┊ ✓ tool_name — result preview
                    let (icon, icon_color) = if msg.content.contains("failed") {
                        ("✗", theme::ERROR)
                    } else if msg.content.contains("calling") {
                        ("⧗", theme::DIM)
                    } else {
                        ("✓", theme::SUCCESS)
                    };
                    lines.push(Line::from(vec![
                        Span::styled("┊ ", Style::default().fg(theme::VIOLET_DIM)),
                        Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                        Span::styled(
                            truncate_str(&msg.content, inner_width.saturating_sub(6)),
                            Style::default().fg(theme::DIM),
                        ),
                    ]));
                }

                MessageRole::Error => {
                    lines.push(Line::from(""));
                    let header = format!("╭─✗ error {}", "─".repeat(inner_width.saturating_sub(12)));
                    lines.push(Line::from(Span::styled(header, theme::error_style())));
                    for line in msg.content.lines() {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(theme::ERROR)),
                            Span::styled(line.to_string(), Style::default().fg(theme::ERROR)),
                        ]));
                    }
                    let footer = format!("╰{}", "─".repeat(inner_width.saturating_sub(2)));
                    lines.push(Line::from(Span::styled(footer, Style::default().fg(theme::ERROR))));
                }

                MessageRole::GraphUpdate => {
                    lines.push(Line::from(vec![
                        Span::styled("┊ ", Style::default().fg(theme::VIOLET_DIM)),
                        Span::styled("◈ ", Style::default().fg(theme::LAVENDER)),
                        Span::styled(&msg.content, Style::default().fg(theme::LAVENDER)),
                    ]));
                }
            }
        }

        // Auto-scroll
        let visible_height = area.height.saturating_sub(2) as usize;
        let total_lines = lines.len();
        let auto_scroll = if total_lines > visible_height && self.scroll_offset == 0 {
            (total_lines - visible_height) as u16
        } else {
            self.scroll_offset
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_style())
            .border_type(BorderType::Rounded)
            .title(Span::styled(
                " ◆ tracey ",
                Style::default().fg(theme::VIOLET).add_modifier(Modifier::BOLD),
            ));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((auto_scroll, 0));

        f.render_widget(paragraph, area);
    }

    fn render_input(&self, f: &mut Frame, area: Rect) {
        let (display_text, text_style) = if self.is_processing {
            let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let s = spinners[self.spinner_state % spinners.len()];
            (format!("{s} {}", self.status), Style::default().fg(theme::VIOLET_BRIGHT))
        } else if self.input.is_empty() {
            ("ask tracey anything...".into(), Style::default().fg(theme::DIM))
        } else {
            (self.input.clone(), Style::default().fg(theme::FG))
        };

        let border_style = if self.is_processing {
            Style::default().fg(theme::DIM)
        } else {
            theme::active_border_style()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(BorderType::Rounded)
            .title(Span::styled(
                " ▸ ",
                Style::default().fg(theme::VIOLET_BRIGHT),
            ));

        let input = Paragraph::new(Span::styled(format!(" {display_text}"), text_style))
            .block(block);

        f.render_widget(input, area);

        if !self.is_processing && !self.input.is_empty() {
            f.set_cursor_position((area.x + self.input.len() as u16 + 2, area.y + 1));
        }
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(" ◆ tracey ", theme::status_bar_style()),
            Span::styled(" ", Style::default()),
        ];

        if !self.graph_stats.is_empty() {
            spans.push(Span::styled(
                format!("◈ {} ", self.graph_stats),
                Style::default().fg(theme::LAVENDER),
            ));
            spans.push(Span::styled("│ ", Style::default().fg(theme::DIM)));
        }

        spans.push(Span::styled(&self.status, Style::default().fg(theme::DIM)));

        let help = " ^C quit · ↑↓ scroll ";
        let used: u16 = spans.iter().map(|s| s.width() as u16).sum();
        let remaining = area.width.saturating_sub(used + help.len() as u16);
        if remaining > 0 {
            spans.push(Span::raw(" ".repeat(remaining as usize)));
        }
        spans.push(Span::styled(help, Style::default().fg(theme::DIM)));

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

fn truncate_lines(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().take(max_lines).collect();
    let result = lines.join(" ");
    if s.lines().count() > max_lines {
        format!("{result}...")
    } else {
        result
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}
