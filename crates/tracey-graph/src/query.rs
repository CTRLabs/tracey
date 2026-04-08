use crate::store::{CausalEdge, CausalNode, GraphStore};
use std::collections::HashMap;
use tracey_core::id::NodeId;

/// Configuration for subgraph extraction
pub struct SubgraphQuery {
    pub seeds: Vec<NodeId>,
    pub max_nodes: usize,
    pub alpha: f64,
    pub min_score: f64,
}

impl Default for SubgraphQuery {
    fn default() -> Self {
        Self {
            seeds: vec![],
            max_nodes: 25,
            alpha: 0.15,
            min_score: 0.01,
        }
    }
}

/// A focused subgraph extracted from the full graph
pub struct Subgraph {
    pub nodes: Vec<(CausalNode, f64)>, // node + PPR score
    pub edges: Vec<(CausalEdge, NodeId, NodeId)>, // edge + source + target
}

/// Run Personalized PageRank to extract the most relevant subgraph.
/// Teleports to seed nodes with probability alpha, walks edges otherwise.
pub fn personalized_pagerank(store: &GraphStore, query: &SubgraphQuery) -> Subgraph {
    if query.seeds.is_empty() || store.node_count() == 0 {
        return Subgraph { nodes: vec![], edges: vec![] };
    }

    // If graph is tiny, just return everything
    if store.node_count() <= query.max_nodes {
        return extract_full_graph(store);
    }

    let all_ids: Vec<NodeId> = store.all_node_ids();
    let n = all_ids.len();

    // Map NodeId -> index for PPR vector
    let id_to_idx: HashMap<NodeId, usize> = all_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    // Initialize PPR scores
    let mut scores = vec![0.0f64; n];
    let seed_weight = 1.0 / query.seeds.len() as f64;

    // Teleport distribution: uniform across seeds
    let mut teleport = vec![0.0f64; n];
    for seed in &query.seeds {
        if let Some(&idx) = id_to_idx.get(seed) {
            teleport[idx] = seed_weight;
        }
    }

    // Initialize scores to teleport distribution
    scores.clone_from(&teleport);

    // Power iteration
    let max_iterations = 50;
    let epsilon = 1e-6;

    for _iter in 0..max_iterations {
        let mut new_scores = vec![0.0f64; n];

        // Teleport component
        for i in 0..n {
            new_scores[i] += query.alpha * teleport[i];
        }

        // Walk component
        for (i, node_id) in all_ids.iter().enumerate() {
            if scores[i] < epsilon {
                continue;
            }

            if let Some(node_idx) = store.get_index(node_id) {
                let neighbors = store.outgoing_neighbors(node_idx);
                if neighbors.is_empty() {
                    // Dangling node: distribute to teleport
                    for j in 0..n {
                        new_scores[j] += (1.0 - query.alpha) * scores[i] * teleport[j];
                    }
                } else {
                    // Weight edges by confidence
                    let total_weight: f64 = neighbors.iter().map(|(_, w)| w).sum();
                    if total_weight > 0.0 {
                        for (neighbor_petgraph_idx, weight) in &neighbors {
                            if let Some(neighbor_node) = store.graph_node_weight(*neighbor_petgraph_idx) {
                                if let Some(&j) = id_to_idx.get(&neighbor_node.id) {
                                    new_scores[j] += (1.0 - query.alpha) * scores[i] * (weight / total_weight);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check convergence
        let diff: f64 = scores.iter().zip(new_scores.iter()).map(|(a, b)| (a - b).abs()).sum();
        scores = new_scores;

        if diff < epsilon {
            break;
        }
    }

    // Select top-K nodes by PPR score
    let mut scored: Vec<(usize, f64)> = scores.iter().enumerate().map(|(i, &s)| (i, s)).collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(query.max_nodes);

    let selected_ids: HashMap<NodeId, f64> = scored
        .iter()
        .filter(|(_, score)| *score >= query.min_score)
        .filter_map(|(idx, score)| {
            let id = all_ids.get(*idx)?;
            Some((*id, *score))
        })
        .collect();

    // Build subgraph
    let mut nodes = Vec::new();
    for (&id, &score) in &selected_ids {
        if let Some(node) = store.get_node(&id) {
            nodes.push((node.clone(), score));
        }
    }
    nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Include edges between selected nodes
    let mut edges = Vec::new();
    for (&id, _) in &selected_ids {
        for (edge, target) in store.edges_from(&id) {
            if selected_ids.contains_key(&target.id) {
                edges.push((edge.clone(), id, target.id));
            }
        }
    }

    Subgraph { nodes, edges }
}

/// Extract seed nodes by matching entity mentions in text against graph labels
pub fn entity_mention_seeds(store: &GraphStore, text: &str) -> Vec<NodeId> {
    let text_lower = text.to_lowercase();
    let mut seeds = Vec::new();

    for (label, id) in store.all_labels() {
        // Match file paths, function names, class names
        let label_lower = label.to_lowercase();

        // Exact substring match
        if text_lower.contains(&label_lower) {
            seeds.push(*id);
            continue;
        }

        // Match just the filename (without path)
        if let Some(filename) = label.rsplit('/').next() {
            if text_lower.contains(&filename.to_lowercase()) {
                seeds.push(*id);
            }
        }
    }

    // Deduplicate
    seeds.sort_unstable();
    seeds.dedup();
    seeds
}

fn extract_full_graph(store: &GraphStore) -> Subgraph {
    let nodes: Vec<(CausalNode, f64)> = store
        .all_node_ids()
        .iter()
        .filter_map(|id| {
            let node = store.get_node(id)?;
            Some((node.clone(), 1.0))
        })
        .collect();

    let mut edges = Vec::new();
    for id in store.all_node_ids() {
        for (edge, target) in store.edges_from(&id) {
            edges.push((edge.clone(), id, target.id));
        }
    }

    Subgraph { nodes, edges }
}

// Helper to access node weight by petgraph NodeIndex
impl GraphStore {
    pub fn graph_node_weight(&self, idx: petgraph::graph::NodeIndex) -> Option<&CausalNode> {
        self.graph.node_weight(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::*;

    fn build_test_graph() -> GraphStore {
        let mut store = GraphStore::new();
        let auth = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "src/auth.rs"));
        let db = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "src/db.rs"));
        let main = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "src/main.rs"));
        let validate = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "validate_token"));
        let get_user = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "get_user"));

        store.add_edge(auth, validate, CausalEdge::new(EdgeKind::DependsOn, EdgeSource::StaticAnalysis));
        store.add_edge(validate, get_user, CausalEdge::new(EdgeKind::Calls, EdgeSource::StaticAnalysis));
        store.add_edge(main, auth, CausalEdge::new(EdgeKind::Imports, EdgeSource::StaticAnalysis));
        store.add_edge(auth, db, CausalEdge::new(EdgeKind::DependsOn, EdgeSource::GitCoChange));

        store
    }

    #[test]
    fn test_entity_mention_seeds() {
        let store = build_test_graph();
        let seeds = entity_mention_seeds(&store, "fix the validate_token function in auth.rs");
        assert!(seeds.len() >= 2, "Should find auth.rs and validate_token");
    }

    #[test]
    fn test_ppr_returns_bounded_nodes() {
        let store = build_test_graph();
        let seeds = entity_mention_seeds(&store, "auth.rs");
        let query = SubgraphQuery {
            seeds,
            max_nodes: 3,
            alpha: 0.15,
            min_score: 0.001,
        };
        let subgraph = personalized_pagerank(&store, &query);
        assert!(subgraph.nodes.len() <= 3, "Should respect max_nodes");
        assert!(!subgraph.nodes.is_empty(), "Should return at least seed node");
    }

    #[test]
    fn test_ppr_includes_seeds() {
        let store = build_test_graph();
        let auth_id = store.find_id_by_label("src/auth.rs").unwrap();
        let query = SubgraphQuery {
            seeds: vec![auth_id],
            max_nodes: 25,
            alpha: 0.15,
            min_score: 0.001,
        };
        let subgraph = personalized_pagerank(&store, &query);
        let node_ids: Vec<NodeId> = subgraph.nodes.iter().map(|(n, _)| n.id).collect();
        assert!(node_ids.contains(&auth_id), "Subgraph should include seed node");
    }
}
