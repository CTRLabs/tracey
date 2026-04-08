use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Provenance of an edge — determines trust level and confidence cap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeSource {
    /// From tree-sitter AST parsing. Confidence always 1.0, no decay.
    StaticAnalysis,
    /// From git log co-change mining. Capped at 0.8.
    GitCoChange,
    /// From agent tool call observations. Capped at 0.7.
    AgentObserved,
    /// From PCMCI temporal discovery. Capped at 0.9.
    PcmciLearned,
    /// Explicitly stated by user. Confidence 0.9, no decay.
    UserDefined,
}

impl EdgeSource {
    /// Maximum confidence allowed for this source type
    pub fn confidence_cap(&self) -> f64 {
        match self {
            EdgeSource::StaticAnalysis => 1.0,
            EdgeSource::GitCoChange => 0.8,
            EdgeSource::AgentObserved => 0.7,
            EdgeSource::PcmciLearned => 0.9,
            EdgeSource::UserDefined => 0.9,
        }
    }

    /// Whether this source is exempt from confidence decay
    pub fn exempt_from_decay(&self) -> bool {
        matches!(self, EdgeSource::StaticAnalysis | EdgeSource::UserDefined)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalNode {
    pub id: NodeId,
    pub layer: GraphLayer,
    pub kind: NodeKind,
    pub label: String,
    pub metadata: Value,
    pub confidence: f64,
    pub last_touched_session: u64,
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
            last_touched_session: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEdge {
    pub id: EdgeId,
    pub kind: EdgeKind,
    pub source: EdgeSource,
    pub weight: f64,
    pub confidence: f64,
    pub evidence_count: u32,
    pub last_touched_session: u64,
    pub metadata: Value,
}

impl CausalEdge {
    pub fn new(kind: EdgeKind, source: EdgeSource) -> Self {
        let confidence = source.confidence_cap();
        Self {
            id: EdgeId::new(),
            kind,
            source,
            weight: 1.0,
            confidence,
            evidence_count: 1,
            last_touched_session: 0,
            metadata: Value::Null,
        }
    }

    /// Create with explicit confidence (clamped to source cap)
    pub fn with_confidence(kind: EdgeKind, source: EdgeSource, confidence: f64) -> Self {
        let clamped = confidence.min(source.confidence_cap());
        Self {
            id: EdgeId::new(),
            kind,
            source,
            weight: 1.0,
            confidence: clamped,
            evidence_count: 1,
            last_touched_session: 0,
            metadata: Value::Null,
        }
    }
}

/// The 4-layer causal graph store with indexes and caching
pub struct GraphStore {
    pub(crate) graph: DiGraph<CausalNode, CausalEdge>,
    // Primary indexes
    node_index: HashMap<NodeId, NodeIndex>,
    label_index: HashMap<String, NodeId>,
    // Secondary indexes for fast lookups
    layer_index: HashMap<GraphLayer, Vec<NodeIndex>>,
    kind_index: HashMap<NodeKind, Vec<NodeIndex>>,
    // Generation counter for cache invalidation
    generation: u64,
    // Query cache: (query_key, root_id) -> (generation, result_ids)
    cache: HashMap<(String, NodeId), (u64, Vec<NodeId>)>,
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
            label_index: HashMap::new(),
            layer_index: HashMap::new(),
            kind_index: HashMap::new(),
            generation: 0,
            cache: HashMap::new(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn bump_generation(&mut self) {
        self.generation += 1;
    }

    pub fn add_node(&mut self, node: CausalNode) -> NodeId {
        let id = node.id;
        let label = node.label.clone();
        let layer = node.layer;
        let kind = node.kind.clone();
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        self.label_index.insert(label, id);
        self.layer_index.entry(layer).or_default().push(idx);
        self.kind_index.entry(kind).or_default().push(idx);
        self.bump_generation();
        id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, edge: CausalEdge) -> Option<EdgeId> {
        let from_idx = self.node_index.get(&from)?;
        let to_idx = self.node_index.get(&to)?;
        let edge_id = edge.id;
        self.graph.add_edge(*from_idx, *to_idx, edge);
        self.bump_generation();
        Some(edge_id)
    }

    /// Reinforce an existing edge (increase evidence count and confidence)
    pub fn reinforce_edge(&mut self, from: NodeId, to: NodeId, kind: &EdgeKind) -> bool {
        let Some(&from_idx) = self.node_index.get(&from) else { return false };
        let Some(&to_idx) = self.node_index.get(&to) else { return false };

        let edge_idx = self.graph.find_edge(from_idx, to_idx);
        if let Some(idx) = edge_idx {
            if let Some(edge) = self.graph.edge_weight_mut(idx) {
                if edge.kind == *kind {
                    edge.evidence_count += 1;
                    let cap = edge.source.confidence_cap();
                    edge.confidence += (cap - edge.confidence) * 0.1;
                    edge.confidence = edge.confidence.min(cap);
                    self.bump_generation();
                    return true;
                }
            }
        }
        false
    }

    pub fn get_node(&self, id: &NodeId) -> Option<&CausalNode> {
        let idx = self.node_index.get(id)?;
        self.graph.node_weight(*idx)
    }

    pub fn get_node_mut(&mut self, id: &NodeId) -> Option<&mut CausalNode> {
        let idx = self.node_index.get(id)?;
        self.graph.node_weight_mut(*idx)
    }

    pub fn find_by_label(&self, label: &str) -> Option<&CausalNode> {
        let id = self.label_index.get(label)?;
        self.get_node(id)
    }

    pub fn find_id_by_label(&self, label: &str) -> Option<NodeId> {
        self.label_index.get(label).copied()
    }

    /// Touch a node (update timestamps and session counter)
    pub fn touch_node(&mut self, id: &NodeId, session: u64) {
        if let Some(node) = self.get_node_mut(id) {
            node.updated_at = Utc::now();
            node.last_touched_session = session;
        }
    }

    /// Get outgoing edges with their target nodes
    pub fn edges_from(&self, id: &NodeId) -> Vec<(&CausalEdge, &CausalNode)> {
        let Some(&idx) = self.node_index.get(id) else { return vec![] };
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .filter_map(|e| {
                let target = self.graph.node_weight(e.target())?;
                Some((e.weight(), target))
            })
            .collect()
    }

    /// Get incoming edges with their source nodes
    pub fn edges_to(&self, id: &NodeId) -> Vec<(&CausalEdge, &CausalNode)> {
        let Some(&idx) = self.node_index.get(id) else { return vec![] };
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .filter_map(|e| {
                let source = self.graph.node_weight(e.source())?;
                Some((e.weight(), source))
            })
            .collect()
    }

    /// Get nodes by layer (using index)
    pub fn nodes_by_layer(&self, layer: GraphLayer) -> Vec<&CausalNode> {
        self.layer_index
            .get(&layer)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|idx| self.graph.node_weight(*idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get nodes by kind (using index)
    pub fn nodes_by_kind(&self, kind: &NodeKind) -> Vec<&CausalNode> {
        self.kind_index
            .get(kind)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|idx| self.graph.node_weight(*idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// BFS descendants with cached results
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

    /// Impact zone with confidence-weighted scoring
    pub fn impact_zone(&self, id: &NodeId) -> Vec<(&CausalNode, f64)> {
        let Some(&start) = self.node_index.get(id) else {
            return vec![];
        };

        let mut visited = HashMap::new();
        let mut queue = vec![(start, 0usize, 1.0f64)];
        let mut result = Vec::new();

        while let Some((idx, depth, accumulated_conf)) = queue.pop() {
            if depth > 5 || visited.contains_key(&idx) {
                continue;
            }
            visited.insert(idx, accumulated_conf);

            if depth > 0 {
                if let Some(node) = self.graph.node_weight(idx) {
                    result.push((node, accumulated_conf));
                }
            }

            for edge in self.graph.edges_directed(idx, Direction::Outgoing) {
                let edge_conf = edge.weight().confidence;
                let next_conf = accumulated_conf * edge_conf;
                if next_conf > 0.1 {
                    queue.push((edge.target(), depth + 1, next_conf));
                }
            }
        }

        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    /// Apply exponential confidence decay to all non-exempt edges
    pub fn apply_decay(&mut self, current_session: u64, lambda: f64, prune_threshold: f64) {
        let mut edges_to_remove = Vec::new();

        for edge_idx in self.graph.edge_indices() {
            if let Some(edge) = self.graph.edge_weight_mut(edge_idx) {
                if edge.source.exempt_from_decay() {
                    continue;
                }
                let sessions_since = current_session.saturating_sub(edge.last_touched_session);
                if sessions_since > 0 {
                    edge.confidence *= (-lambda * sessions_since as f64).exp();
                    if edge.confidence < prune_threshold {
                        edges_to_remove.push(edge_idx);
                    }
                }
            }
        }

        // Remove pruned edges (in reverse order to maintain indices)
        edges_to_remove.sort_unstable();
        for idx in edges_to_remove.into_iter().rev() {
            self.graph.remove_edge(idx);
        }

        self.bump_generation();
    }

    /// Get all node IDs (for PPR iteration)
    pub fn all_node_ids(&self) -> Vec<NodeId> {
        self.node_index.keys().copied().collect()
    }

    /// Get all node indices and their IDs
    pub fn node_entries(&self) -> impl Iterator<Item = (&NodeId, &NodeIndex)> {
        self.node_index.iter()
    }

    /// Get neighbors (outgoing) of a node by index
    pub fn outgoing_neighbors(&self, idx: NodeIndex) -> Vec<(NodeIndex, f64)> {
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| (e.target(), e.weight().confidence))
            .collect()
    }

    /// Get the petgraph NodeIndex for a NodeId
    pub fn get_index(&self, id: &NodeId) -> Option<NodeIndex> {
        self.node_index.get(id).copied()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// All labels in the graph (for entity matching)
    pub fn all_labels(&self) -> impl Iterator<Item = (&str, &NodeId)> {
        self.label_index.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node_and_edge() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "main.rs"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "main"));
        let edge = CausalEdge::new(EdgeKind::DependsOn, EdgeSource::StaticAnalysis);
        assert!(store.add_edge(n1, n2, edge).is_some());
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1);
    }

    #[test]
    fn test_confidence_clamping() {
        let edge = CausalEdge::with_confidence(EdgeKind::Caused, EdgeSource::AgentObserved, 0.95);
        assert!(edge.confidence <= 0.7, "AgentObserved should be capped at 0.7");

        let edge = CausalEdge::with_confidence(EdgeKind::Calls, EdgeSource::StaticAnalysis, 0.5);
        assert_eq!(edge.confidence, 0.5, "StaticAnalysis allows any confidence up to 1.0");
    }

    #[test]
    fn test_decay() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Action, "edit"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Error, "fail"));
        let mut edge = CausalEdge::new(EdgeKind::Caused, EdgeSource::AgentObserved);
        edge.last_touched_session = 0;
        store.add_edge(n1, n2, edge);

        store.apply_decay(20, 0.05, 0.15);
        // After 20 sessions with lambda=0.05: 0.7 * e^(-0.05*20) = 0.7 * e^(-1) ≈ 0.257
        assert_eq!(store.edge_count(), 1, "Edge should survive at ~0.257");

        store.apply_decay(50, 0.05, 0.15);
        // After 50 total: 0.257 * e^(-0.05*30) ≈ 0.257 * 0.223 ≈ 0.057
        assert_eq!(store.edge_count(), 0, "Edge should be pruned below 0.15");
    }

    #[test]
    fn test_static_analysis_exempt_from_decay() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "a.rs"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "b.rs"));
        let edge = CausalEdge::new(EdgeKind::Imports, EdgeSource::StaticAnalysis);
        store.add_edge(n1, n2, edge);

        store.apply_decay(1000, 0.1, 0.15);
        assert_eq!(store.edge_count(), 1, "StaticAnalysis edges should never decay");
    }

    #[test]
    fn test_layer_and_kind_indexes() {
        let mut store = GraphStore::new();
        store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "a.rs"));
        store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "foo"));
        store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Action, "edit"));

        assert_eq!(store.nodes_by_layer(GraphLayer::Code).len(), 2);
        assert_eq!(store.nodes_by_layer(GraphLayer::Execution).len(), 1);
        assert_eq!(store.nodes_by_kind(&NodeKind::File).len(), 1);
    }

    #[test]
    fn test_reinforce_edge() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Action, "edit"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Error, "fail"));
        let edge = CausalEdge::with_confidence(EdgeKind::Caused, EdgeSource::AgentObserved, 0.4);
        store.add_edge(n1, n2, edge);

        assert!(store.reinforce_edge(n1, n2, &EdgeKind::Caused));
        // Check confidence increased
        let edges = store.edges_from(&n1);
        assert_eq!(edges[0].0.evidence_count, 2);
        assert!(edges[0].0.confidence > 0.4);
    }

    #[test]
    fn test_impact_zone_confidence_weighted() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "root"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "child1"));
        let n3 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "child2"));

        store.add_edge(n1, n2, CausalEdge::with_confidence(EdgeKind::Calls, EdgeSource::StaticAnalysis, 1.0));
        store.add_edge(n1, n3, CausalEdge::with_confidence(EdgeKind::Calls, EdgeSource::AgentObserved, 0.3));

        let impact = store.impact_zone(&n1);
        assert_eq!(impact.len(), 2);
        // child1 should have higher impact score (confidence 1.0) than child2 (0.3)
        assert!(impact[0].1 > impact[1].1);
    }
}
