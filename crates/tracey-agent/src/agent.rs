use futures::StreamExt;
use std::sync::{Arc, RwLock};
use tracey_core::events::{AgentEvent, AgentHandle, Submission};
use tracey_core::id::{SessionId, TurnId};
use tracey_core::traits::{StreamChunk, ToolContext, ToolOutput};
use tracey_core::types::{ContentBlock, Message, MessageRole, ToolCall, UsageStats};
use tracey_core::TraceyResult;
use tracey_graph::{
    entity_mention_seeds, personalized_pagerank, to_markdown_kv, GraphStore, SubgraphQuery,
};
use tracey_llm::ModelRouter;
use tracey_session::SessionStore;
use tracey_tools::ToolRegistry;

use crate::observer::{GraphMutation, GraphObserver};

pub struct Agent {
    router: ModelRouter,
    tools: ToolRegistry,
    session_store: SessionStore,
    graph: Arc<RwLock<GraphStore>>,
    session_id: SessionId,
    session_counter: u64,
    messages: Vec<Message>,
    system_prompt: String,
    max_iterations: usize,
}

impl Agent {
    pub fn new(
        router: ModelRouter,
        tools: ToolRegistry,
        graph: Arc<RwLock<GraphStore>>,
        system_prompt: String,
        session_counter: u64,
    ) -> Self {
        Self {
            router,
            tools,
            session_store: SessionStore::new(),
            graph,
            session_id: SessionId::new(),
            session_counter,
            messages: Vec::new(),
            system_prompt,
            max_iterations: 50,
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

        // Add system message
        self.messages.push(Message::system(&self.system_prompt));

        while let Some(submission) = handle.recv().await {
            match submission {
                Submission::UserMessage { content, .. } => {
                    self.messages.push(Message::user(&content));

                    let turn_id = TurnId::new();
                    let _ = handle
                        .emit(AgentEvent::TurnStarted {
                            turn_id,
                            session_id: self.session_id,
                        })
                        .await;

                    match self.execute_turn(&handle, turn_id).await {
                        Ok(usage) => {
                            let _ = handle
                                .emit(AgentEvent::TurnComplete { turn_id, usage })
                                .await;
                        }
                        Err(e) => {
                            let _ = handle
                                .emit(AgentEvent::Error {
                                    message: e.to_string(),
                                    recoverable: true,
                                })
                                .await;
                        }
                    }
                }
                Submission::Interrupt => {
                    let _ = handle
                        .emit(AgentEvent::Status {
                            message: "interrupted".into(),
                        })
                        .await;
                }
                Submission::Shutdown => {
                    let _ = handle
                        .emit(AgentEvent::SessionEnd {
                            session_id: self.session_id,
                        })
                        .await;
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
            // === PHASE 1: GRAPH PRIMING ===
            // Extract entities from the last user message and inject causal context
            if iteration == 0 || self.should_requery(iteration) {
                let graph_context = self.build_graph_context();
                if !graph_context.is_empty() {
                    // Insert graph context as a system message after the main system prompt
                    self.inject_graph_context(&graph_context);

                    let _ = handle
                        .emit(AgentEvent::Status {
                            message: format!(
                                "graph: {} nodes in context",
                                self.graph.read().unwrap().node_count()
                            ),
                        })
                        .await;
                }
            }

            // === PHASE 2: LLM CALL ===
            let provider = self
                .router
                .default_provider()
                .ok_or_else(|| tracey_core::TraceyError::Llm("no provider available".into()))?;

            let model = self.router.default_model().to_string();
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
                        let _ = handle
                            .emit(AgentEvent::AssistantChunk {
                                turn_id,
                                text,
                            })
                            .await;
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

                        let _ = handle
                            .emit(AgentEvent::ToolCallStart {
                                turn_id,
                                tool_call: ToolCall {
                                    id,
                                    name,
                                    arguments: serde_json::Value::Null,
                                },
                            })
                            .await;
                    }
                    StreamChunk::ToolCallDelta {
                        arguments_delta, ..
                    } => {
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

            // === PHASE 3: TOOL EXECUTION + GRAPH UPDATE ===
            let ctx = ToolContext {
                cwd: std::env::current_dir().unwrap_or_default(),
                session_id: self.session_id,
                timeout: std::time::Duration::from_secs(120),
            };

            for tc in &tool_calls {
                let result = self.tools.execute(&tc.name, tc.arguments.clone(), &ctx).await;

                let tool_output = match &result {
                    Ok(output) => output.clone(),
                    Err(e) => ToolOutput::error(e.to_string()),
                };

                // Update causal graph with tool observations
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

                // Emit graph update events
                for mutation in &mutations {
                    match mutation {
                        GraphMutation::NodeCreated(_, label) => {
                            let _ = handle
                                .emit(AgentEvent::GraphUpdate {
                                    node_id: None,
                                    description: format!("new: {label}"),
                                })
                                .await;
                        }
                        GraphMutation::EdgeCreated(_, _, kind) => {
                            let _ = handle
                                .emit(AgentEvent::GraphUpdate {
                                    node_id: None,
                                    description: format!("edge: {kind:?}"),
                                })
                                .await;
                        }
                        _ => {}
                    }
                }

                let _ = handle
                    .emit(AgentEvent::ToolCallEnd {
                        turn_id,
                        tool_call_id: tc.id,
                        result: tool_output.content.clone(),
                        is_error: tool_output.is_error,
                    })
                    .await;

                self.messages
                    .push(Message::tool_result(tc.id, &tool_output.content, tool_output.is_error));
            }
        }

        Ok(total_usage)
    }

    /// Build graph context by extracting entities from the last user message
    /// and running Personalized PageRank to find the most relevant subgraph
    fn build_graph_context(&self) -> String {
        let graph = self.graph.read().unwrap();

        // Don't bother if graph is too small
        if graph.node_count() < 3 {
            return String::new();
        }

        // Find the last user message for entity extraction
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

        // Extract seed nodes from entity mentions
        let seeds = entity_mention_seeds(&graph, &user_text);

        if seeds.is_empty() {
            // No entity matches — return a summary instead
            return format!(
                "<graph-context>\nGraph: {} code nodes, {} execution nodes, {} knowledge nodes\n</graph-context>",
                graph.nodes_by_layer(tracey_graph::GraphLayer::Code).len(),
                graph.nodes_by_layer(tracey_graph::GraphLayer::Execution).len(),
                graph.nodes_by_layer(tracey_graph::GraphLayer::Knowledge).len(),
            );
        }

        // Run PPR to extract focused subgraph
        let query = SubgraphQuery {
            seeds,
            max_nodes: 25,
            alpha: 0.15,
            min_score: 0.01,
        };

        let subgraph = personalized_pagerank(&graph, &query);
        to_markdown_kv(&subgraph)
    }

    /// Inject graph context into the message history
    fn inject_graph_context(&mut self, context: &str) {
        // Remove any previous graph context message
        self.messages.retain(|m| {
            if m.role == MessageRole::System {
                !m.text_content().starts_with("<graph-context>")
            } else {
                true
            }
        });

        // Insert after the first system message (the main system prompt)
        let insert_pos = self
            .messages
            .iter()
            .position(|m| m.role != MessageRole::System)
            .unwrap_or(1);

        self.messages
            .insert(insert_pos, Message::system(context));
    }

    /// Determine if we should re-query the graph (after iteration 3+ if graph changed)
    fn should_requery(&self, iteration: usize) -> bool {
        iteration > 0 && iteration % 3 == 0
    }
}
