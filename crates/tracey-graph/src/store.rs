use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracey_core::id::{EdgeId, NodeId};

/// Which of the 4 causal graph layers a node/edge belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphLayer {
    Code,
    Execution,
    Knowledge,
    Project,
}

/// Kind of node, scoped by layer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    // Code layer
    File, Function, Class, Module, Type, Test, Import,
    // Execution layer
    Action, Observation, Decision, Error, ToolCall,
    // Knowledge layer
    Fact, Preference, Pattern, Bug, Solution, KnowledgeDecision,
    // Project layer
    Task, Goal, Constraint, Deadline,
}

/// Kind of causal edge
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    // Code
    Calls, Imports, Inherits, Implements, Tests, DependsOn,
    // Execution
    Caused, Enabled, Prevented, Triggered, Resolved,
    // Knowledge
    Explains, Contradicts, Supersedes, CausedBy, LedTo,
    // Project
    Blocks, Requires, ContributesTo, Risks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalNode {
    pub id: NodeId,
    pub layer: GraphLayer,
    pub kind: NodeKind,
    pub label: String,
    pub metadata: Value,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CausalNode {
    pub fn new(layer: GraphLayer, kind: NodeKind, label: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: NodeId::new(),
            layer,
            kind,
            label: label.into(),
            metadata: Value::Null,
            confidence: 1.0,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEdge {
    pub id: EdgeId,
    pub kind: EdgeKind,
    pub weight: f64,
    pub confidence: f64,
    pub evidence_count: u32,
    pub metadata: Value,
}

impl CausalEdge {
    pub fn new(kind: EdgeKind) -> Self {
        Self {
            id: EdgeId::new(),
            kind,
            weight: 1.0,
            confidence: 1.0,
            evidence_count: 1,
            metadata: Value::Null,
        }
    }
}

/// The 4-layer causal graph store
pub struct GraphStore {
    graph: DiGraph<CausalNode, CausalEdge>,
    node_index: HashMap<NodeId, NodeIndex>,
    label_index: HashMap<String, NodeId>,
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
            label_index: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: CausalNode) -> NodeId {
        let id = node.id;
        let label = node.label.clone();
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        self.label_index.insert(label, id);
        id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, edge: CausalEdge) -> Option<EdgeId> {
        let from_idx = self.node_index.get(&from)?;
        let to_idx = self.node_index.get(&to)?;
        self.graph.add_edge(*from_idx, *to_idx, edge.clone());
        Some(edge.id)
    }

    pub fn get_node(&self, id: &NodeId) -> Option<&CausalNode> {
        let idx = self.node_index.get(id)?;
        self.graph.node_weight(*idx)
    }

    pub fn find_by_label(&self, label: &str) -> Option<&CausalNode> {
        let id = self.label_index.get(label)?;
        self.get_node(id)
    }

    pub fn descendants(&self, id: &NodeId, max_depth: usize) -> Vec<&CausalNode> {
        let Some(&start) = self.node_index.get(id) else {
            return vec![];
        };

        let mut visited = HashMap::new();
        let mut queue = vec![(start, 0usize)];
        let mut result = Vec::new();

        while let Some((idx, depth)) = queue.pop() {
            if depth > max_depth || visited.contains_key(&idx) {
                continue;
            }
            visited.insert(idx, depth);

            if depth > 0 {
                if let Some(node) = self.graph.node_weight(idx) {
                    result.push(node);
                }
            }

            for neighbor in self.graph.neighbors_directed(idx, Direction::Outgoing) {
                queue.push((neighbor, depth + 1));
            }
        }

        result
    }

    pub fn ancestors(&self, id: &NodeId, max_depth: usize) -> Vec<&CausalNode> {
        let Some(&start) = self.node_index.get(id) else {
            return vec![];
        };

        let mut visited = HashMap::new();
        let mut queue = vec![(start, 0usize)];
        let mut result = Vec::new();

        while let Some((idx, depth)) = queue.pop() {
            if depth > max_depth || visited.contains_key(&idx) {
                continue;
            }
            visited.insert(idx, depth);

            if depth > 0 {
                if let Some(node) = self.graph.node_weight(idx) {
                    result.push(node);
                }
            }

            for neighbor in self.graph.neighbors_directed(idx, Direction::Incoming) {
                queue.push((neighbor, depth + 1));
            }
        }

        result
    }

    /// Impact zone: all nodes reachable from this node via causal edges
    pub fn impact_zone(&self, id: &NodeId) -> Vec<(&CausalNode, f64)> {
        self.descendants(id, 10)
            .into_iter()
            .map(|n| {
                let impact = n.confidence;
                (n, impact)
            })
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get nodes filtered by layer
    pub fn nodes_by_layer(&self, layer: GraphLayer) -> Vec<&CausalNode> {
        self.graph
            .node_weights()
            .filter(|n| n.layer == layer)
            .collect()
    }

    /// Render a subgraph as ASCII art
    pub fn render_ascii(&self, root: &NodeId, max_depth: usize) -> String {
        let Some(root_node) = self.get_node(root) else {
            return "  (empty graph)".to_string();
        };

        let mut lines = Vec::new();
        self.render_node_ascii(root_node, &mut lines, 0, max_depth, "");
        lines.join("\n")
    }

    fn render_node_ascii(
        &self,
        node: &CausalNode,
        lines: &mut Vec<String>,
        depth: usize,
        max_depth: usize,
        prefix: &str,
    ) {
        let indent = "  ".repeat(depth);
        let kind_symbol = match node.kind {
            NodeKind::Function => "fn",
            NodeKind::File => "📄",
            NodeKind::Class => "cls",
            NodeKind::Test => "test",
            NodeKind::Action => "act",
            NodeKind::Error => "err",
            NodeKind::Fact => "fact",
            NodeKind::Bug => "bug",
            NodeKind::Task => "task",
            _ => "•",
        };

        let conf = if node.confidence < 1.0 {
            format!(" ({:.0}%)", node.confidence * 100.0)
        } else {
            String::new()
        };

        lines.push(format!(
            "{indent}{prefix}╭─[{kind_symbol}] {}{conf}",
            node.label
        ));

        if depth < max_depth {
            let children = self.descendants(&node.id, 1);
            for (i, child) in children.iter().enumerate() {
                let is_last = i == children.len() - 1;
                let edge_label = "──▶ ";
                self.render_node_ascii(
                    child,
                    lines,
                    depth + 1,
                    max_depth,
                    edge_label,
                );
            }
        }
    }
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}
