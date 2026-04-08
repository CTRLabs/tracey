use futures::StreamExt;
use std::sync::Arc;
use tracey_core::events::{AgentEvent, AgentHandle, Submission};
use tracey_core::id::{SessionId, TurnId};
use tracey_core::traits::{LlmProvider, StreamChunk, ToolContext, ToolSchema};
use tracey_core::types::{ContentBlock, Message, MessageRole, ToolCall, UsageStats};
use tracey_core::TraceyResult;
use tracey_graph::GraphStore;
use tracey_llm::ModelRouter;
use tracey_session::SessionStore;
use tracey_tools::ToolRegistry;

pub struct Agent {
    router: ModelRouter,
    tools: ToolRegistry,
    session_store: SessionStore,
    graph: GraphStore,
    session_id: SessionId,
    messages: Vec<Message>,
    system_prompt: String,
    max_iterations: usize,
}

impl Agent {
    pub fn new(
        router: ModelRouter,
        tools: ToolRegistry,
        system_prompt: String,
    ) -> Self {
        Self {
            router,
            tools,
            session_store: SessionStore::new(),
            graph: GraphStore::new(),
            session_id: SessionId::new(),
            messages: Vec::new(),
            system_prompt,
            max_iterations: 50,
        }
    }

    /// Run the agent loop, communicating via SQ/EQ channels
    pub async fn run(&mut self, mut handle: AgentHandle) {
        // Add system message
        self.messages.push(Message::system(&self.system_prompt));

        while let Some(submission) = handle.recv().await {
            match submission {
                Submission::UserMessage { content, .. } => {
                    self.messages.push(Message::user(&content));

                    let turn_id = TurnId::new();
                    let _ = handle.emit(AgentEvent::TurnStarted {
                        turn_id,
                        session_id: self.session_id,
                    }).await;

                    match self.execute_turn(&handle, turn_id).await {
                        Ok(usage) => {
                            let _ = handle.emit(AgentEvent::TurnComplete {
                                turn_id,
                                usage,
                            }).await;
                        }
                        Err(e) => {
                            let _ = handle.emit(AgentEvent::Error {
                                message: e.to_string(),
                                recoverable: true,
                            }).await;
                        }
                    }
                }
                Submission::Interrupt => {
                    let _ = handle.emit(AgentEvent::Status {
                        message: "interrupted".into(),
                    }).await;
                }
                Submission::Shutdown => {
                    let _ = handle.emit(AgentEvent::SessionEnd {
                        session_id: self.session_id,
                    }).await;
                    break;
                }
                _ => {}
            }
        }
    }

    async fn execute_turn(
        &mut self,
        handle: &AgentHandle,
        turn_id: TurnId,
    ) -> TraceyResult<Option<UsageStats>> {
        let tool_schemas = self.tools.active_schemas();
        let mut total_usage: Option<UsageStats> = None;

        for _iteration in 0..self.max_iterations {
            let (provider_name, model) = self.router.route(
                &self.messages.last().map(|m| m.text_content()).unwrap_or_default()
            );

            let provider = self.router.get_provider(provider_name)
                .or_else(|| self.router.default_provider())
                .ok_or_else(|| tracey_core::TraceyError::Llm("no provider available".into()))?;

            let model = self.router.default_model().to_string();

            // Call LLM
            let mut stream = provider.chat(&self.messages, &tool_schemas, &model).await?;

            let mut assistant_text = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();
            let mut current_tool_name = String::new();
            let mut current_tool_args = String::new();
            let mut current_tool_id = tracey_core::ToolCallId::new();

            while let Some(chunk) = stream.next().await {
                match chunk? {
                    StreamChunk::Text(text) => {
                        assistant_text.push_str(&text);
                        let _ = handle.emit(AgentEvent::AssistantChunk {
                            turn_id,
                            text,
                        }).await;
                    }
                    StreamChunk::ToolCallStart { id, name } => {
                        // Flush any previous tool call
                        if !current_tool_name.is_empty() {
                            let args: serde_json::Value = serde_json::from_str(&current_tool_args)
                                .unwrap_or(serde_json::Value::Object(Default::default()));
                            tool_calls.push(ToolCall {
                                id: current_tool_id,
                                name: current_tool_name.clone(),
                                arguments: args,
                            });
                        }
                        current_tool_id = id;
                        current_tool_name = name.clone();
                        current_tool_args.clear();

                        let _ = handle.emit(AgentEvent::ToolCallStart {
                            turn_id,
                            tool_call: ToolCall {
                                id,
                                name,
                                arguments: serde_json::Value::Null,
                            },
                        }).await;
                    }
                    StreamChunk::ToolCallDelta { arguments_delta, .. } => {
                        current_tool_args.push_str(&arguments_delta);
                    }
                    StreamChunk::Usage(usage) => {
                        total_usage = Some(usage);
                    }
                    StreamChunk::Done => break,
                    _ => {}
                }
            }

            // Flush last tool call
            if !current_tool_name.is_empty() {
                let args: serde_json::Value = serde_json::from_str(&current_tool_args)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                tool_calls.push(ToolCall {
                    id: current_tool_id,
                    name: current_tool_name,
                    arguments: args,
                });
            }

            // Build assistant message
            let mut content_blocks = Vec::new();
            if !assistant_text.is_empty() {
                content_blocks.push(ContentBlock::Text(assistant_text.clone()));
            }
            for tc in &tool_calls {
                content_blocks.push(ContentBlock::ToolUse(tc.clone()));
            }

            self.messages.push(Message {
                role: MessageRole::Assistant,
                content: content_blocks,
                name: None,
                tool_call_id: None,
                timestamp: chrono::Utc::now(),
            });

            // If no tool calls, we're done
            if tool_calls.is_empty() {
                break;
            }

            // Execute tool calls
            let ctx = ToolContext {
                cwd: std::env::current_dir().unwrap_or_default(),
                session_id: self.session_id,
                timeout: std::time::Duration::from_secs(120),
            };

            for tc in &tool_calls {
                let result = self.tools.execute(&tc.name, tc.arguments.clone(), &ctx).await;

                let (content, is_error) = match result {
                    Ok(output) => (output.content, output.is_error),
                    Err(e) => (e.to_string(), true),
                };

                let _ = handle.emit(AgentEvent::ToolCallEnd {
                    turn_id,
                    tool_call_id: tc.id,
                    result: content.clone(),
                    is_error,
                }).await;

                self.messages.push(Message::tool_result(tc.id, &content, is_error));
            }

            // Continue the loop — the model may make more tool calls
        }

        Ok(total_usage)
    }
}
