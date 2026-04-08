use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tracey_core::events::{AgentEvent, Submission, UiHandle};

pub struct App {
    pub messages: Vec<DisplayMessage>,
    pub input: String,
    pub scroll_offset: u16,
    pub status: String,
    pub spinner_state: usize,
    pub is_processing: bool,
    pub should_quit: bool,
    pub ui_handle: UiHandle,
}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: String,
    pub content: String,
    pub is_error: bool,
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
            ui_handle,
        }
    }

    pub async fn run(&mut self, terminal: &mut ratatui::Terminal<impl Backend>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            // Poll for events with a small timeout
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
                                self.messages.push(DisplayMessage {
                                    role: "you".into(),
                                    content: msg.clone(),
                                    is_error: false,
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
                            self.scroll_offset = self.scroll_offset.saturating_add(1);
                        }
                        (KeyCode::Down, _) => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            }

            // Check for agent events
            while let Ok(event) = self.ui_handle.event_rx.try_recv() {
                self.handle_agent_event(event);
            }

            if self.should_quit {
                let _ = self.ui_handle.submit(Submission::Shutdown).await;
                return Ok(());
            }

            // Update spinner
            if self.is_processing {
                self.spinner_state = (self.spinner_state + 1) % 10;
            }
        }
    }

    fn handle_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::AssistantChunk { text, .. } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "tracey" {
                        last.content.push_str(&text);
                        return;
                    }
                }
                self.messages.push(DisplayMessage {
                    role: "tracey".into(),
                    content: text,
                    is_error: false,
                });
            }
            AgentEvent::ToolCallStart { tool_call, .. } => {
                self.status = format!("calling {}...", tool_call.name);
            }
            AgentEvent::ToolCallEnd { is_error, .. } => {
                if is_error {
                    self.status = "tool error".into();
                }
            }
            AgentEvent::GraphUpdate { description, .. } => {
                self.status = format!("◆ {description}");
            }
            AgentEvent::TurnComplete { .. } => {
                self.is_processing = false;
                self.status = "ready".into();
            }
            AgentEvent::Error { message, .. } => {
                self.messages.push(DisplayMessage {
                    role: "error".into(),
                    content: message,
                    is_error: true,
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
                Constraint::Min(3),    // messages
                Constraint::Length(3), // input
                Constraint::Length(1), // status bar
            ])
            .split(f.area());

        self.render_messages(f, chunks[0]);
        self.render_input(f, chunks[1]);
        self.render_status(f, chunks[2]);
    }

    fn render_messages(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            let (prefix, style) = match msg.role.as_str() {
                "you" => ("▸ you", Style::default().fg(Color::Blue)),
                "tracey" => ("◆ tracey", Style::default().fg(Color::Cyan)),
                "error" => ("✗ error", Style::default().fg(Color::Red)),
                _ => ("•", Style::default()),
            };

            lines.push(Line::from(Span::styled(prefix, style.bold())));
            for content_line in msg.content.lines() {
                lines.push(Line::from(format!("  {content_line}")));
            }
            lines.push(Line::from(""));
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" tracey "))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));

        f.render_widget(paragraph, area);
    }

    fn render_input(&self, f: &mut Frame, area: Rect) {
        let input_display = if self.is_processing {
            let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let s = spinners[self.spinner_state % spinners.len()];
            format!("{s} processing...")
        } else {
            self.input.clone()
        };

        let input = Paragraph::new(input_display)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" input ")
                    .border_style(if self.is_processing {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::Cyan)
                    }),
            );

        f.render_widget(input, area);

        // Show cursor
        if !self.is_processing {
            f.set_cursor_position((area.x + self.input.len() as u16 + 1, area.y + 1));
        }
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" tracey ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(&self.status, Style::default().fg(Color::DarkGray)),
        ]));

        f.render_widget(status, area);
    }
}
