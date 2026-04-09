use futures::StreamExt;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracey_core::events::{AgentEvent, AgentHandle, Submission};
use tracey_core::id::{SessionId, TurnId};
use tracey_core::traits::{StreamChunk, ToolContext, ToolOutput};
use tracey_core::types::{ContentBlock, Message, MessageRole, ToolCall, UsageStats};
use tracey_core::{TraceyError, TraceyResult};
use tracey_graph::{
    entity_mention_seeds, personalized_pagerank, to_markdown_kv, GraphStore, SubgraphQuery,
};
use tracey_hooks::{HookAction, HookEvent, HookRunner};
use tracey_llm::ModelRouter;
use tracey_session::store::{SessionEntry, SessionStore};
use tracey_tools::ToolRegistry;

use crate::compaction::{compact_messages, needs_compaction};
use crate::observer::{GraphMutation, GraphObserver};

/// Maximum retry attempts for LLM API calls
const MAX_RETRIES: usize = 3;
/// Initial retry delay (doubles each attempt)
const INITIAL_RETRY_MS: u64 = 1000;

pub struct Agent {
    router: ModelRouter,
    tools: ToolRegistry,
    session_store: SessionStore,
    graph: Arc<RwLock<GraphStore>>,
    hook_runner: HookRunner,
    session_id: SessionId,
    session_counter: u64,
    messages: Vec<Message>,
    system_prompt: String,
    max_iterations: usize,
    max_context_tokens: u64,
    /// Last user message preserved for retry on error
    last_user_message: Option<String>,
    /// Flag set by interrupt handling
    interrupted: Arc<std::sync::atomic::AtomicBool>,
}

impl Agent {
    pub fn new(
        router: ModelRouter,
        tools: ToolRegistry,
        graph: Arc<RwLock<GraphStore>>,
        hook_runner: HookRunner,
        system_prompt: String,
        session_counter: u64,
    ) -> Self {
        Self {
            router,
            tools,
            session_store: SessionStore::new(),
            graph,
            hook_runner,
            session_id: SessionId::new(),
            session_counter,
            messages: Vec::new(),
            system_prompt,
            max_iterations: 50,
            max_context_tokens: 200_000,
            last_user_message: None,
            interrupted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Run the agent loop, communicating via SQ/EQ channels
    pub async fn run(&mut self, mut handle: AgentHandle) {
        // Apply confidence decay at session start
        {
            let mut graph = self.graph.write().unwrap();
            graph.apply_decay(self.session_counter, 0.05, 0.15);
            tracing::info!(
                "Session {} started. Graph: {} nodes, {} edges",
                self.session_counter,
                graph.node_count(),
                graph.edge_count()
            );
        }

        // Fire SessionStart hook
        let _ = self.hook_runner.run_hooks(
            &HookEvent::SessionStart,
            &serde_json::json!({"session_id": self.session_id.to_string()}),
        ).await;

        // Add system message and persist
        self.messages.push(Message::system(&self.system_prompt));
        self.persist_message(self.messages.last().unwrap());

        while let Some(submission) = handle.recv().await {
            match submission {
                Submission::UserMessage { content, .. } => {
                    self.last_user_message = Some(content.clone());
                    self.interrupted.store(false, std::sync::atomic::Ordering::SeqCst);

                    let msg = Message::user(&content);
                    self.persist_message(&msg);
                    self.messages.push(msg);

                    // Check compaction before starting turn
                    if needs_compaction(&self.messages, self.max_context_tokens, 85) {
                        let removed = compact_messages(&mut self.messages, 3, 5);
                        if removed > 0 {
                            let _ = handle.emit(AgentEvent::Status {
                                message: format!("compacted {removed} messages"),
                            }).await;
                        }
                    }

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
                                message: format!("{e} (press Enter to retry)"),
                                recoverable: true,
                            }).await;
                        }
                    }
                }
                Submission::Interrupt => {
                    self.interrupted.store(true, std::sync::atomic::Ordering::SeqCst);
                    let _ = handle.emit(AgentEvent::Status {
                        message: "interrupted".into(),
                    }).await;
                }
                Submission::Shutdown => {
                    // Fire SessionEnd hook
                    let _ = self.hook_runner.run_hooks(
                        &HookEvent::SessionEnd,
                        &serde_json::json!({"session_id": self.session_id.to_string()}),
                    ).await;

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

        for iteration in 0..self.max_iterations {
            // Check for interruption
            if self.interrupted.load(std::sync::atomic::Ordering::SeqCst) {
                tracing::info!("Turn interrupted at iteration {iteration}");
                return Ok(total_usage);
            }

            // === PHASE 1: GRAPH PRIMING ===
            if iteration == 0 || self.should_requery(iteration) {
                let graph_context = self.build_graph_context();
                if !graph_context.is_empty() {
                    self.inject_graph_context(&graph_context);
                    let _ = handle.emit(AgentEvent::Status {
                        message: format!(
                            "◈ graph: {} nodes queried",
                            self.graph.read().unwrap().node_count()
                        ),
                    }).await;
                }
            }

            // === PHASE 2: LLM CALL WITH RETRY ===
            let provider = self.router.default_provider()
                .ok_or_else(|| TraceyError::Llm("no provider available".into()))?;

            let model = self.router.default_model().to_string();

            // Fire PreLlmCall hook
            let _ = self.hook_runner.run_hooks(
                &HookEvent::PreLlmCall,
                &serde_json::json!({"model": model, "iteration": iteration}),
            ).await;

            let mut stream = self.call_llm_with_retry(
                provider.as_ref(),
                &tool_schemas,
                &model,
            ).await?;

            let mut assistant_text = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();
            let mut current_tool_name = String::new();
            let mut current_tool_args = String::new();
            let mut current_tool_id = tracey_core::ToolCallId::new();

            // Stream with interruption checking
            while let Some(chunk) = stream.next().await {
                if self.interrupted.load(std::sync::atomic::Ordering::SeqCst) {
                    // Keep partial response
                    if !assistant_text.is_empty() {
                        self.messages.push(Message::assistant(&assistant_text));
                        self.persist_message(self.messages.last().unwrap());
                    }
                    return Ok(total_usage);
                }

                match chunk? {
                    StreamChunk::Text(text) => {
                        assistant_text.push_str(&text);
                        let _ = handle.emit(AgentEvent::AssistantChunk {
                            turn_id,
                            text,
                        }).await;
                    }
                    StreamChunk::ToolCallStart { id, name } => {
                        if !current_tool_name.is_empty() {
                            let args: serde_json::Value =
                                serde_json::from_str(&current_tool_args)
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

            // Build and persist assistant message
            let mut content_blocks = Vec::new();
            if !assistant_text.is_empty() {
                content_blocks.push(ContentBlock::Text(assistant_text.clone()));
            }
            for tc in &tool_calls {
                content_blocks.push(ContentBlock::ToolUse(tc.clone()));
            }

            let assistant_msg = Message {
                role: MessageRole::Assistant,
                content: content_blocks,
                name: None,
                tool_call_id: None,
                timestamp: chrono::Utc::now(),
            };
            self.persist_message(&assistant_msg);
            self.messages.push(assistant_msg);

            // Fire PostLlmCall hook
            let _ = self.hook_runner.run_hooks(
                &HookEvent::PostLlmCall,
                &serde_json::json!({"iteration": iteration, "tool_calls": tool_calls.len()}),
            ).await;

            if tool_calls.is_empty() {
                break;
            }

            // === PHASE 3: TOOL EXECUTION + GRAPH UPDATE ===
            let ctx = ToolContext {
                cwd: std::env::current_dir().unwrap_or_default(),
                session_id: self.session_id,
                timeout: Duration::from_secs(120),
            };

            for tc in &tool_calls {
                // Fire PreToolCall hook
                let hook_result = self.hook_runner.run_hooks(
                    &HookEvent::PreToolCall,
                    &serde_json::json!({
                        "tool": tc.name,
                        "arguments": tc.arguments,
                    }),
                ).await;

                // Check if hook blocked the call
                if let Ok(hr) = &hook_result {
                    if matches!(hr.action, HookAction::Abort(_)) {
                        let msg = match &hr.action {
                            HookAction::Abort(reason) => reason.clone(),
                            _ => "blocked by hook".into(),
                        };
                        let _ = handle.emit(AgentEvent::ToolCallEnd {
                            turn_id,
                            tool_call_id: tc.id,
                            result: format!("blocked: {msg}"),
                            is_error: true,
                        }).await;
                        self.messages.push(Message::tool_result(tc.id, &format!("blocked: {msg}"), true));
                        continue;
                    }
                }

                // Execute tool
                let result = self.tools.execute(&tc.name, tc.arguments.clone(), &ctx).await;

                let tool_output = match &result {
                    Ok(output) => output.clone(),
                    Err(e) => ToolOutput::error(e.to_string()),
                };

                // Fire PostToolCall hook
                let _ = self.hook_runner.run_hooks(
                    &HookEvent::PostToolCall,
                    &serde_json::json!({
                        "tool": tc.name,
                        "is_error": tool_output.is_error,
                        "output_preview": &tool_output.content[..tool_output.content.len().min(200)],
                    }),
                ).await;

                // Update causal graph
                let mutations = {
                    let mut graph = self.graph.write().unwrap();
                    GraphObserver::observe(
                        &mut graph,
                        &tc.name,
                        &tc.arguments,
                        &tool_output,
                        self.session_counter,
                    )
                };

                for mutation in &mutations {
                    match mutation {
                        GraphMutation::NodeCreated(_, label) => {
                            let _ = handle.emit(AgentEvent::GraphUpdate {
                                node_id: None,
                                description: format!("new: {label}"),
                            }).await;
                        }
                        GraphMutation::EdgeCreated(_, _, kind) => {
                            let _ = handle.emit(AgentEvent::GraphUpdate {
                                node_id: None,
                                description: format!("edge: {kind:?}"),
                            }).await;
                        }
                        _ => {}
                    }
                }

                let _ = handle.emit(AgentEvent::ToolCallEnd {
                    turn_id,
                    tool_call_id: tc.id,
                    result: tool_output.content.clone(),
                    is_error: tool_output.is_error,
                }).await;

                // Persist tool result
                let tool_msg = Message::tool_result(tc.id, &tool_output.content, tool_output.is_error);
                self.persist_message(&tool_msg);
                self.messages.push(tool_msg);
            }
        }

        Ok(total_usage)
    }

    /// Call LLM with exponential backoff retry on transient errors
    async fn call_llm_with_retry(
        &self,
        provider: &dyn tracey_core::traits::LlmProvider,
        tools: &[tracey_core::traits::ToolSchema],
        model: &str,
    ) -> TraceyResult<Box<dyn futures::Stream<Item = TraceyResult<StreamChunk>> + Send + Unpin>> {
        let mut delay = Duration::from_millis(INITIAL_RETRY_MS);

        for attempt in 0..MAX_RETRIES {
            match provider.chat(&self.messages, tools, model).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    let err_msg = e.to_string();
                    let is_retryable = err_msg.contains("429")
                        || err_msg.contains("500")
                        || err_msg.contains("503")
                        || err_msg.contains("timeout")
                        || err_msg.contains("connection");

                    if !is_retryable || attempt == MAX_RETRIES - 1 {
                        return Err(e);
                    }

                    tracing::warn!(
                        "LLM call failed (attempt {}/{}): {err_msg}. Retrying in {:?}...",
                        attempt + 1,
                        MAX_RETRIES,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }

        Err(TraceyError::Llm("max retries exceeded".into()))
    }

    /// Persist a message to the session store
    fn persist_message(&self, msg: &Message) {
        if let Err(e) = self.session_store.append(
            &self.session_id,
            &SessionEntry::Message(msg.clone()),
        ) {
            tracing::warn!("Failed to persist message: {e}");
        }
    }

    /// Build graph context by extracting entities and running PPR
    fn build_graph_context(&self) -> String {
        let graph = self.graph.read().unwrap();

        if graph.node_count() < 3 {
            return String::new();
        }

        let user_text = self
            .messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .map(|m| m.text_content())
            .unwrap_or_default();

        if user_text.is_empty() {
            return String::new();
        }

        let seeds = entity_mention_seeds(&graph, &user_text);

        if seeds.is_empty() {
            return format!(
                "<graph-context>\nGraph: {} code, {} exec, {} knowledge nodes\n</graph-context>",
                graph.nodes_by_layer(tracey_graph::GraphLayer::Code).len(),
                graph.nodes_by_layer(tracey_graph::GraphLayer::Execution).len(),
                graph.nodes_by_layer(tracey_graph::GraphLayer::Knowledge).len(),
            );
        }

        let query = SubgraphQuery {
            seeds,
            max_nodes: 25,
            alpha: 0.15,
            min_score: 0.01,
        };

        let subgraph = personalized_pagerank(&graph, &query);
        to_markdown_kv(&subgraph)
    }

    fn inject_graph_context(&mut self, context: &str) {
        self.messages.retain(|m| {
            if m.role == MessageRole::System {
                !m.text_content().starts_with("<graph-context>")
            } else {
                true
            }
        });

        let insert_pos = self
            .messages
            .iter()
            .position(|m| m.role != MessageRole::System)
            .unwrap_or(1);

        self.messages.insert(insert_pos, Message::system(context));
    }

    fn should_requery(&self, iteration: usize) -> bool {
        iteration > 0 && iteration % 3 == 0
    }
}
