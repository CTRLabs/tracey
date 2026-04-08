use serde_json::Value;
use tracey_core::id::NodeId;
use tracey_core::traits::ToolOutput;
use tracey_graph::{CausalEdge, CausalNode, EdgeKind, EdgeSource, GraphLayer, GraphStore, NodeKind};

/// Mutations produced by the graph observer
#[derive(Debug, Clone)]
pub enum GraphMutation {
    NodeTouched(NodeId),
    NodeCreated(NodeId, String), // id, label
    EdgeCreated(NodeId, NodeId, EdgeKind),
}

/// Observes tool executions and updates the causal graph
pub struct GraphObserver;

impl GraphObserver {
    /// Process a tool call result and update the graph accordingly
    pub fn observe(
        store: &mut GraphStore,
        tool_name: &str,
        args: &Value,
        result: &ToolOutput,
        session: u64,
    ) -> Vec<GraphMutation> {
        let mut mutations = Vec::new();

        match tool_name {
            "Read" => {
                if let Some(path) = args["file_path"].as_str() {
                    let file_id = ensure_file_node(store, path, session);
                    store.touch_node(&file_id, session);
                    mutations.push(GraphMutation::NodeTouched(file_id));
                }
            }

            "Write" => {
                if let Some(path) = args["file_path"].as_str() {
                    let file_id = ensure_file_node(store, path, session);
                    store.touch_node(&file_id, session);
                    mutations.push(GraphMutation::NodeTouched(file_id));

                    // Record the write action
                    let action_id = create_action_node(store, "write", path, session);
                    mutations.push(GraphMutation::NodeCreated(action_id, format!("write:{path}")));

                    let edge = CausalEdge::with_confidence(EdgeKind::Triggered, EdgeSource::AgentObserved, 0.6);
                    store.add_edge(action_id, file_id, edge);
                    mutations.push(GraphMutation::EdgeCreated(action_id, file_id, EdgeKind::Triggered));
                }
            }

            "Edit" => {
                if let Some(path) = args["file_path"].as_str() {
                    let file_id = ensure_file_node(store, path, session);
                    store.touch_node(&file_id, session);

                    // Record the edit action
                    let action_id = create_action_node(store, "edit", path, session);
                    mutations.push(GraphMutation::NodeCreated(action_id, format!("edit:{path}")));

                    let edge = CausalEdge::with_confidence(EdgeKind::Triggered, EdgeSource::AgentObserved, 0.6);
                    store.add_edge(action_id, file_id, edge);
                    mutations.push(GraphMutation::EdgeCreated(action_id, file_id, EdgeKind::Triggered));

                    // If the edit caused an error
                    if result.is_error {
                        let error_id = create_error_node(store, &result.content, session);
                        mutations.push(GraphMutation::NodeCreated(error_id, "error".into()));

                        let caused = CausalEdge::with_confidence(EdgeKind::Caused, EdgeSource::AgentObserved, 0.5);
                        store.add_edge(action_id, error_id, caused);
                        mutations.push(GraphMutation::EdgeCreated(action_id, error_id, EdgeKind::Caused));
                    }
                }
            }

            "Bash" => {
                let cmd = args["command"].as_str().unwrap_or("");
                let is_test_cmd = cmd.contains("test")
                    || cmd.contains("cargo test")
                    || cmd.contains("pytest")
                    || cmd.contains("npm test")
                    || cmd.contains("go test")
                    || cmd.contains("jest")
                    || cmd.contains("rspec");

                if is_test_cmd {
                    if result.is_error {
                        let error_id = create_error_node(store, "test failure", session);
                        mutations.push(GraphMutation::NodeCreated(error_id, "test_failure".into()));
                    } else {
                        let obs_id = create_observation_node(store, "tests passed", session);
                        mutations.push(GraphMutation::NodeCreated(obs_id, "tests_passed".into()));
                    }
                } else if result.is_error {
                    let error_id = create_error_node(store, &truncate(&result.content, 100), session);
                    mutations.push(GraphMutation::NodeCreated(error_id, "cmd_error".into()));
                }
            }

            "Glob" | "Grep" => {
                // Touch any files that appear in results
                if !result.is_error {
                    for line in result.content.lines().take(20) {
                        let path = line.split(':').next().unwrap_or(line).trim();
                        if path.contains('.') && !path.contains(' ') {
                            let file_id = ensure_file_node(store, path, session);
                            store.touch_node(&file_id, session);
                            mutations.push(GraphMutation::NodeTouched(file_id));
                        }
                    }
                }
            }

            _ => {}
        }

        mutations
    }
}

fn ensure_file_node(store: &mut GraphStore, path: &str, session: u64) -> NodeId {
    if let Some(id) = store.find_id_by_label(path) {
        id
    } else {
        let mut node = CausalNode::new(GraphLayer::Code, NodeKind::File, path);
        node.last_touched_session = session;
        store.add_node(node)
    }
}

fn create_action_node(store: &mut GraphStore, action: &str, target: &str, session: u64) -> NodeId {
    let label = format!("{action}:{target}:{}", chrono::Utc::now().timestamp());
    let mut node = CausalNode::new(GraphLayer::Execution, NodeKind::Action, label);
    node.last_touched_session = session;
    node.confidence = 0.7;
    store.add_node(node)
}

fn create_error_node(store: &mut GraphStore, description: &str, session: u64) -> NodeId {
    let label = format!("error:{}", truncate(description, 80));
    let mut node = CausalNode::new(GraphLayer::Execution, NodeKind::Error, label);
    node.last_touched_session = session;
    node.confidence = 0.8;
    store.add_node(node)
}

fn create_observation_node(store: &mut GraphStore, description: &str, session: u64) -> NodeId {
    let label = format!("obs:{description}");
    let mut node = CausalNode::new(GraphLayer::Execution, NodeKind::Observation, label);
    node.last_touched_session = session;
    node.confidence = 0.9;
    store.add_node(node)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())])
    }
}
