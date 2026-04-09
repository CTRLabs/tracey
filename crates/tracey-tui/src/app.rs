use crate::commands::{handle_command, CommandResult};
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
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    Error,
    GraphUpdate,
    System,
}

pub struct App {
    pub messages: Vec<DisplayMessage>,
    pub input: String,
    pub cursor_pos: usize,
    pub scroll_offset: u16,
    pub status: String,
    pub spinner_state: usize,
    pub is_processing: bool,
    pub should_quit: bool,
    // Graph & context tracking
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub graph_last_update: String,
    // Token tracking (Hermes-style capacity bar)
    pub tokens_used: u64,
    pub tokens_max: u64,
    pub model_name: String,
    pub provider_name: String,
    // Session info
    pub session_number: u64,
    pub turn_count: u32,
    pub ui_handle: UiHandle,
}

impl App {
    pub fn new(ui_handle: UiHandle) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            status: "ready".into(),
            spinner_state: 0,
            is_processing: false,
            should_quit: false,
            graph_nodes: 0,
            graph_edges: 0,
            graph_last_update: String::new(),
            tokens_used: 0,
            tokens_max: 200_000,
            model_name: String::new(),
            provider_name: String::new(),
            session_number: 1,
            turn_count: 0,
            ui_handle,
        }
    }

    /// Inject the welcome dashboard as the first message in the TUI
    pub fn inject_welcome_dashboard(&mut self) {
        let dashboard = format!(
            "╭─◆ tracey ─────────────────────────────────╮\n\
             │                                            │\n\
             │  Model:     {}{}│\n\
             │  Provider:  {}{}│\n\
             │  Graph:     {} nodes, {} edges{}│\n\
             │  Session:   #{}{}│\n\
             │                                            │\n\
             │  Tools:  Read  Write  Edit  Bash  Glob  Grep│\n\
             │                                            │\n\
             │  /help commands · /graph show · /cost       │\n\
             │  Ctrl+C quit · ↑↓ scroll · Esc interrupt   │\n\
             │                                            │\n\
             ╰────────────────────────────────────────────╯",
            self.model_name,
            " ".repeat(30usize.saturating_sub(self.model_name.len())),
            self.provider_name,
            " ".repeat(30usize.saturating_sub(self.provider_name.len())),
            self.graph_nodes,
            self.graph_edges,
            " ".repeat(22usize.saturating_sub(format!("{} nodes, {} edges", self.graph_nodes, self.graph_edges).len())),
            self.session_number,
            " ".repeat(30usize.saturating_sub(format!("#{}", self.session_number).len())),
        );

        self.messages.push(DisplayMessage {
            role: MessageRole::System,
            content: dashboard,
            tool_name: None,
            timestamp: now_time(),
        });
    }

    pub fn set_model_info(&mut self, model: &str, provider: &str) {
        self.model_name = model.to_string();
        self.provider_name = provider.to_string();
    }

    pub fn set_graph_stats(&mut self, nodes: usize, edges: usize) {
        self.graph_nodes = nodes;
        self.graph_edges = edges;
    }

    pub async fn run(&mut self, terminal: &mut ratatui::Terminal<impl Backend>) -> std::io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match (key.code, key.modifiers) {
                        // Quit / Interrupt
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            if self.is_processing {
                                let _ = self.ui_handle.submit(Submission::Interrupt).await;
                                self.is_processing = false;
                                self.status = "interrupted".into();
                            } else {
                                self.should_quit = true;
                            }
                        }
                        // Submit message or handle command
                        (KeyCode::Enter, _) if !self.is_processing => {
                            if !self.input.trim().is_empty() {
                                let input = self.input.clone();
                                self.input.clear();
                                self.cursor_pos = 0;
                                self.scroll_offset = 0;

                                // Check for slash commands first
                                match handle_command(self, &input) {
                                    CommandResult::Handled => {
                                        // Command processed locally
                                    }
                                    CommandResult::Quit => {
                                        self.should_quit = true;
                                    }
                                    CommandResult::NotACommand => {
                                        // Send to LLM
                                        self.messages.push(DisplayMessage {
                                            role: MessageRole::User,
                                            content: input.clone(),
                                            tool_name: None,
                                            timestamp: now_time(),
                                        });
                                        let _ = self.ui_handle.submit(Submission::UserMessage {
                                            content: input,
                                            attachments: vec![],
                                        }).await;
                                        self.is_processing = true;
                                        self.turn_count += 1;
                                        self.status = "thinking...".into();
                                    }
                                }
                            }
                        }
                        // Text input
                        (KeyCode::Char(c), _) if !self.is_processing => {
                            self.input.insert(self.cursor_pos, c);
                            self.cursor_pos += 1;
                        }
                        (KeyCode::Backspace, _) if !self.is_processing && self.cursor_pos > 0 => {
                            self.cursor_pos -= 1;
                            self.input.remove(self.cursor_pos);
                        }
                        (KeyCode::Delete, _) if !self.is_processing && self.cursor_pos < self.input.len() => {
                            self.input.remove(self.cursor_pos);
                        }
                        // Cursor movement
                        (KeyCode::Left, _) if !self.is_processing => {
                            self.cursor_pos = self.cursor_pos.saturating_sub(1);
                        }
                        (KeyCode::Right, _) if !self.is_processing => {
                            self.cursor_pos = (self.cursor_pos + 1).min(self.input.len());
                        }
                        (KeyCode::Home, _) if !self.is_processing => {
                            self.cursor_pos = 0;
                        }
                        (KeyCode::End, _) if !self.is_processing => {
                            self.cursor_pos = self.input.len();
                        }
                        // Scroll
                        (KeyCode::Up, KeyModifiers::NONE) if self.is_processing || self.input.is_empty() => {
                            self.scroll_offset = self.scroll_offset.saturating_add(3);
                        }
                        (KeyCode::Down, KeyModifiers::NONE) if self.is_processing || self.input.is_empty() => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(3);
                        }
                        (KeyCode::PageUp, _) => {
                            self.scroll_offset = self.scroll_offset.saturating_add(20);
                        }
                        (KeyCode::PageDown, _) => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(20);
                        }
                        // Escape to interrupt
                        (KeyCode::Esc, _) if self.is_processing => {
                            let _ = self.ui_handle.submit(Submission::Interrupt).await;
                            self.is_processing = false;
                            self.status = "interrupted".into();
                        }
                        // Clear input
                        (KeyCode::Char('u'), KeyModifiers::CONTROL) if !self.is_processing => {
                            self.input.clear();
                            self.cursor_pos = 0;
                        }
                        _ => {}
                    }
                }
            }

            // Drain agent events
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
                        // Auto-scroll to bottom on new content
                        self.scroll_offset = 0;
                        return;
                    }
                }
                self.messages.push(DisplayMessage {
                    role: MessageRole::Assistant,
                    content: text,
                    tool_name: None,
                    timestamp: now_time(),
                });
                self.scroll_offset = 0;
            }
            AgentEvent::ToolCallStart { tool_call, .. } => {
                self.status = format!("◆ {}...", tool_call.name);
                self.messages.push(DisplayMessage {
                    role: MessageRole::Tool,
                    content: format!("⧗ {}...", tool_call.name),
                    tool_name: Some(tool_call.name),
                    timestamp: now_time(),
                });
            }
            AgentEvent::ToolCallEnd { result, is_error, .. } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Tool {
                        let name = last.tool_name.clone().unwrap_or_default();
                        let preview = truncate_lines(&result, 2);
                        last.content = if is_error {
                            format!("✗ {name} — {preview}")
                        } else {
                            format!("✓ {name} — {preview}")
                        };
                    }
                }
            }
            AgentEvent::GraphUpdate { description, .. } => {
                self.graph_last_update = description.clone();
                // Parse graph stats if available
                if description.starts_with("new:") || description.starts_with("edge:") {
                    self.graph_nodes += if description.starts_with("new:") { 1 } else { 0 };
                    self.graph_edges += if description.starts_with("edge:") { 1 } else { 0 };
                }
                self.messages.push(DisplayMessage {
                    role: MessageRole::GraphUpdate,
                    content: description,
                    tool_name: None,
                    timestamp: now_time(),
                });
            }
            AgentEvent::TurnComplete { usage, .. } => {
                self.is_processing = false;
                if let Some(u) = usage {
                    self.tokens_used += u.input_tokens + u.output_tokens;
                    self.status = format!(
                        "turn {} · {}↑ {}↓",
                        self.turn_count, u.input_tokens, u.output_tokens
                    );
                } else {
                    self.status = format!("turn {} · done", self.turn_count);
                }
            }
            AgentEvent::Error { message, .. } => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: message,
                    tool_name: None,
                    timestamp: now_time(),
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
                Constraint::Length(1), // header bar
                Constraint::Min(5),    // messages
                Constraint::Length(3), // input
                Constraint::Length(1), // status bar
            ])
            .split(f.area());

        self.render_header(f, chunks[0]);
        self.render_messages(f, chunks[1]);
        self.render_input(f, chunks[2]);
        self.render_status(f, chunks[3]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(" ◆ tracey ", Style::default().fg(Color::Black).bg(theme::VIOLET).add_modifier(Modifier::BOLD)),
            Span::styled(" ", Style::default()),
        ];

        // Model info
        if !self.model_name.is_empty() {
            spans.push(Span::styled(
                format!("{} ", self.model_name),
                Style::default().fg(theme::DIM),
            ));
            spans.push(Span::styled("│ ", Style::default().fg(theme::VIOLET_DIM)));
        }

        // Graph stats
        spans.push(Span::styled(
            format!("◈ {}n {}e ", self.graph_nodes, self.graph_edges),
            Style::default().fg(theme::LAVENDER),
        ));
        spans.push(Span::styled("│ ", Style::default().fg(theme::VIOLET_DIM)));

        // Context capacity bar (Hermes-inspired)
        let capacity_pct = if self.tokens_max > 0 {
            ((self.tokens_used as f64 / self.tokens_max as f64) * 100.0) as u8
        } else {
            0
        };
        let bar_width = 10u16;
        let filled = ((capacity_pct as u16) * bar_width / 100).min(bar_width);
        let bar_color = match capacity_pct {
            0..=50 => theme::SUCCESS,
            51..=80 => theme::WARNING,
            81..=95 => Color::Rgb(249, 115, 22), // orange
            _ => theme::ERROR,
        };

        spans.push(Span::styled("ctx ", Style::default().fg(theme::DIM)));
        spans.push(Span::styled(
            "█".repeat(filled as usize),
            Style::default().fg(bar_color),
        ));
        spans.push(Span::styled(
            "░".repeat((bar_width - filled) as usize),
            Style::default().fg(theme::VIOLET_MUTED),
        ));
        spans.push(Span::styled(
            format!(" {capacity_pct}%"),
            Style::default().fg(bar_color),
        ));

        // Session info on the right
        let session_info = format!(" session {} · turn {} ", self.session_number, self.turn_count);
        let used: u16 = spans.iter().map(|s| s.width() as u16).sum();
        let remaining = area.width.saturating_sub(used + session_info.len() as u16);
        if remaining > 0 {
            spans.push(Span::raw(" ".repeat(remaining as usize)));
        }
        spans.push(Span::styled(session_info, Style::default().fg(theme::DIM)));

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn render_messages(&self, f: &mut Frame, area: Rect) {
        let inner_width = area.width.saturating_sub(4) as usize;
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            match &msg.role {
                MessageRole::User => {
                    lines.push(Line::from(""));
                    // Header with timestamp
                    let header_line = format!(
                        "╭─◉ you {}{}",
                        "─".repeat(inner_width.saturating_sub(10 + msg.timestamp.len() + 3)),
                        format!(" {} ", msg.timestamp)
                    );
                    lines.push(Line::from(Span::styled(header_line, theme::user_style())));
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
                    lines.push(Line::from(""));
                    let header_line = format!(
                        "╭─◆ tracey {}{}",
                        "─".repeat(inner_width.saturating_sub(13 + msg.timestamp.len() + 3)),
                        format!(" {} ", msg.timestamp)
                    );
                    lines.push(Line::from(Span::styled(header_line, Style::default().fg(theme::VIOLET))));

                    let mut in_code_block = false;
                    for line in msg.content.lines() {
                        if line.starts_with("```") {
                            in_code_block = !in_code_block;
                            lines.push(Line::from(vec![
                                Span::styled("│ ", Style::default().fg(theme::VIOLET_DIM)),
                                Span::styled(line.to_string(), Style::default().fg(theme::DIM)),
                            ]));
                            continue;
                        }

                        let content_style = if in_code_block {
                            Style::default().fg(theme::LAVENDER)
                        } else if line.starts_with('#') {
                            Style::default().fg(theme::VIOLET_BRIGHT).add_modifier(Modifier::BOLD)
                        } else if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("> ") {
                            Style::default().fg(theme::FG)
                        } else if line.starts_with("**") && line.ends_with("**") {
                            Style::default().fg(theme::FG).add_modifier(Modifier::BOLD)
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
                    let (icon, icon_color) = if msg.content.starts_with('✗') {
                        ("✗", theme::ERROR)
                    } else if msg.content.starts_with('⧗') {
                        ("⧗", theme::VIOLET_BRIGHT)
                    } else {
                        ("✓", theme::SUCCESS)
                    };
                    let text = msg.content.trim_start_matches(['✗', '✓', '⧗', ' ']);
                    lines.push(Line::from(vec![
                        Span::styled("┊ ", Style::default().fg(theme::VIOLET_DIM)),
                        Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                        Span::styled(
                            truncate_str(text, inner_width.saturating_sub(6)),
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
                        Span::styled(
                            truncate_str(&msg.content, inner_width.saturating_sub(6)),
                            Style::default().fg(theme::LAVENDER),
                        ),
                    ]));
                }

                MessageRole::System => {
                    lines.push(Line::from(vec![
                        Span::styled("┊ ", Style::default().fg(theme::VIOLET_DIM)),
                        Span::styled("⚙ ", Style::default().fg(theme::DIM)),
                        Span::styled(&msg.content, Style::default().fg(theme::DIM)),
                    ]));
                }
            }
        }

        // Auto-scroll to bottom
        let visible_height = area.height.saturating_sub(2) as usize;
        let total_lines = lines.len();
        let scroll = if self.scroll_offset > 0 {
            // Manual scroll active
            let max_scroll = total_lines.saturating_sub(visible_height) as u16;
            self.scroll_offset.min(max_scroll)
        } else if total_lines > visible_height {
            // Auto-scroll to bottom
            (total_lines - visible_height) as u16
        } else {
            0
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_style())
            .border_type(BorderType::Rounded);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

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

        // Cursor
        if !self.is_processing {
            f.set_cursor_position((
                area.x + self.cursor_pos as u16 + 2,
                area.y + 1,
            ));
        }
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![];

        // Status message
        spans.push(Span::styled(
            format!(" {} ", self.status),
            Style::default().fg(theme::DIM),
        ));

        // Last graph update
        if !self.graph_last_update.is_empty() {
            spans.push(Span::styled("│ ", Style::default().fg(theme::VIOLET_DIM)));
            spans.push(Span::styled(
                format!("◈ {} ", truncate_str(&self.graph_last_update, 30)),
                Style::default().fg(theme::LAVENDER),
            ));
        }

        // Help on the right
        let help = " ^C quit · ↑↓ scroll · esc interrupt ";
        let used: u16 = spans.iter().map(|s| s.width() as u16).sum();
        let remaining = area.width.saturating_sub(used + help.len() as u16);
        if remaining > 0 {
            spans.push(Span::raw(" ".repeat(remaining as usize)));
        }
        spans.push(Span::styled(help, Style::default().fg(theme::DIM)));

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

pub fn now_time() -> String {
    chrono::Local::now().format("%H:%M").to_string()
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
