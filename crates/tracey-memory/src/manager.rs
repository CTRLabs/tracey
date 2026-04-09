use std::sync::{Arc, RwLock};
use tracey_core::id::NodeId;
use tracey_core::TraceyResult;
use tracey_graph::{
    CausalEdge, CausalNode, EdgeKind, EdgeSource, GraphLayer, GraphStore, NodeKind,
};
use tracey_search::{reciprocal_rank_fusion, VectorIndex};

/// A fact or piece of knowledge to remember
#[derive(Debug, Clone)]
pub struct MemoryFact {
    pub content: String,
    pub kind: MemoryKind,
    pub related_entities: Vec<String>, // labels of related nodes
    pub importance: f64,               // 0.0 to 1.0
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryKind {
    Fact,
    Preference,
    Pattern,
    BugReport,
    Solution,
    Decision,
}

impl MemoryKind {
    fn to_node_kind(&self) -> NodeKind {
        match self {
            MemoryKind::Fact => NodeKind::Fact,
            MemoryKind::Preference => NodeKind::Preference,
            MemoryKind::Pattern => NodeKind::Pattern,
            MemoryKind::BugReport => NodeKind::Bug,
            MemoryKind::Solution => NodeKind::Solution,
            MemoryKind::Decision => NodeKind::KnowledgeDecision,
        }
    }
}

/// MAGMA-inspired memory manager with 4-signal retrieval
pub struct MemoryManager {
    graph: Arc<RwLock<GraphStore>>,
    vector_index: VectorIndex,
}

impl MemoryManager {
    pub fn new(graph: Arc<RwLock<GraphStore>>) -> Self {
        Self {
            graph,
            vector_index: VectorIndex::new(),
        }
    }

    /// Store a memory fact in the knowledge graph
    pub fn store(&mut self, fact: MemoryFact, session: u64) -> TraceyResult<NodeId> {
        let mut store = self.graph.write().unwrap();

        let mut node = CausalNode::new(
            GraphLayer::Knowledge,
            fact.kind.to_node_kind(),
            &fact.content,
        );
        node.confidence = fact.importance;
        node.last_touched_session = session;

        let node_id = store.add_node(node);

        // Link to related entities
        for entity_label in &fact.related_entities {
            if let Some(entity_id) = store.find_id_by_label(entity_label) {
                let edge = CausalEdge::with_confidence(
                    EdgeKind::Explains,
                    EdgeSource::AgentObserved,
                    0.6,
                );
                store.add_edge(node_id, entity_id, edge);
            }
        }

        Ok(node_id)
    }

    /// Retrieve relevant memories using MAGMA 4-signal fusion
    /// Signals: semantic (vector), temporal (recency), causal (graph), entity (label match)
    pub fn recall(&self, query: &str, top_k: usize) -> Vec<(String, f64)> {
        let store = self.graph.read().unwrap();

        // Signal 1: Semantic (vector similarity)
        let semantic_results = if !self.vector_index.is_empty() {
            // For now, use empty query embedding (will be populated when LLM embed is called)
            vec![]
        } else {
            vec![]
        };

        // Signal 2: Temporal (recency-weighted)
        let knowledge_nodes = store.nodes_by_layer(GraphLayer::Knowledge);
        let max_session = knowledge_nodes
            .iter()
            .map(|n| n.last_touched_session)
            .max()
            .unwrap_or(1);
        let temporal_results: Vec<(String, f32)> = knowledge_nodes
            .iter()
            .map(|n| {
                let recency = if max_session > 0 {
                    n.last_touched_session as f32 / max_session as f32
                } else {
                    0.5
                };
                (n.label.clone(), recency * n.confidence as f32)
            })
            .collect();

        // Signal 3: Causal (spreading activation from query-matched nodes)
        let query_lower = query.to_lowercase();
        let causal_results: Vec<(String, f32)> = knowledge_nodes
            .iter()
            .filter(|n| n.label.to_lowercase().contains(&query_lower))
            .flat_map(|n| {
                let neighbors = store.edges_from(&n.id);
                let mut results = vec![(n.label.clone(), n.confidence as f32)];
                for (edge, target) in neighbors {
                    results.push((target.label.clone(), edge.confidence as f32 * 0.5));
                }
                results
            })
            .collect();

        // Signal 4: Entity match (keyword in label)
        let entity_results: Vec<(String, f32)> = knowledge_nodes
            .iter()
            .filter_map(|n| {
                let label_lower = n.label.to_lowercase();
                let words: Vec<&str> = query_lower.split_whitespace().collect();
                let match_count = words.iter().filter(|w| label_lower.contains(**w)).count();
                if match_count > 0 {
                    Some((n.label.clone(), match_count as f32 / words.len().max(1) as f32))
                } else {
                    None
                }
            })
            .collect();

        // Fuse all signals via Reciprocal Rank Fusion
        let lists = vec![semantic_results, temporal_results, causal_results, entity_results];
        let non_empty: Vec<Vec<(String, f32)>> = lists.into_iter().filter(|l| !l.is_empty()).collect();

        if non_empty.is_empty() {
            return vec![];
        }

        reciprocal_rank_fusion(&non_empty, 60.0, top_k)
    }

    /// Check if we should nudge the agent to extract memories
    pub fn should_nudge(turn_count: u32, interval: u32) -> bool {
        turn_count > 0 && turn_count % interval == 0
    }

    /// Generate MEMORY.md content from the knowledge graph
    pub fn generate_markdown(&self) -> String {
        let store = self.graph.read().unwrap();
        let knowledge = store.nodes_by_layer(GraphLayer::Knowledge);

        if knowledge.is_empty() {
            return "# Memory\n\n*No memories stored yet.*\n".to_string();
        }

        let mut md = String::from("# Memory\n\n");

        // Group by kind
        let mut facts: Vec<&CausalNode> = Vec::new();
        let mut preferences: Vec<&CausalNode> = Vec::new();
        let mut patterns: Vec<&CausalNode> = Vec::new();
        let mut bugs: Vec<&CausalNode> = Vec::new();

        for node in &knowledge {
            match node.kind {
                NodeKind::Fact => facts.push(node),
                NodeKind::Preference => preferences.push(node),
                NodeKind::Pattern => patterns.push(node),
                NodeKind::Bug => bugs.push(node),
                _ => facts.push(node),
            }
        }

        if !facts.is_empty() {
            md.push_str("## Facts\n");
            for f in &facts {
                md.push_str(&format!("- {} (conf: {:.0}%)\n", f.label, f.confidence * 100.0));
            }
            md.push('\n');
        }

        if !preferences.is_empty() {
            md.push_str("## Preferences\n");
            for p in &preferences {
                md.push_str(&format!("- {}\n", p.label));
            }
            md.push('\n');
        }

        if !patterns.is_empty() {
            md.push_str("## Patterns\n");
            for p in &patterns {
                md.push_str(&format!("- {}\n", p.label));
            }
            md.push('\n');
        }

        if !bugs.is_empty() {
            md.push_str("## Known Bugs\n");
            for b in &bugs {
                md.push_str(&format!("- {}\n", b.label));
            }
            md.push('\n');
        }

        md
    }

    /// Write MEMORY.md to disk
    pub fn sync_to_markdown(&self, path: &std::path::Path) -> TraceyResult<()> {
        let md = self.generate_markdown();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, md)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_recall() {
        let graph = Arc::new(RwLock::new(GraphStore::new()));
        let mut mgr = MemoryManager::new(graph);

        mgr.store(MemoryFact {
            content: "user prefers async rust".into(),
            kind: MemoryKind::Preference,
            related_entities: vec![],
            importance: 0.8,
        }, 1).unwrap();

        mgr.store(MemoryFact {
            content: "the auth module has a known race condition".into(),
            kind: MemoryKind::BugReport,
            related_entities: vec![],
            importance: 0.9,
        }, 1).unwrap();

        let results = mgr.recall("auth race", 5);
        assert!(!results.is_empty(), "Should find auth-related memory");
    }

    #[test]
    fn test_generate_markdown() {
        let graph = Arc::new(RwLock::new(GraphStore::new()));
        let mut mgr = MemoryManager::new(graph);

        mgr.store(MemoryFact {
            content: "project uses tokio runtime".into(),
            kind: MemoryKind::Fact,
            related_entities: vec![],
            importance: 0.7,
        }, 1).unwrap();

        let md = mgr.generate_markdown();
        assert!(md.contains("# Memory"));
        assert!(md.contains("tokio runtime"));
        assert!(md.contains("## Facts"));
    }

    #[test]
    fn test_nudge_interval() {
        assert!(!MemoryManager::should_nudge(0, 10));
        assert!(MemoryManager::should_nudge(10, 10));
        assert!(!MemoryManager::should_nudge(11, 10));
        assert!(MemoryManager::should_nudge(20, 10));
    }
}
