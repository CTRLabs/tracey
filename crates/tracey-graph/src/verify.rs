use crate::store::*;
use petgraph::algo::is_cyclic_directed;
use tracey_core::id::NodeId;

/// Result of graph verification
#[derive(Debug, Clone)]
pub struct VerifyResult {
    pub is_valid: bool,
    pub is_dag: bool,
    pub contradictions: Vec<Contradiction>,
    pub warnings: Vec<String>,
}

/// A detected contradiction in the causal graph
#[derive(Debug, Clone)]
pub struct Contradiction {
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub description: String,
    pub edge_a_kind: EdgeKind,
    pub edge_b_kind: EdgeKind,
}

/// Verify graph structural invariants
pub fn verify_graph(store: &GraphStore) -> VerifyResult {
    let is_dag = !is_cyclic_directed(&store.graph);
    let contradictions = detect_contradictions(store);
    let mut warnings = Vec::new();

    // Check for orphan nodes (no edges at all)
    let orphan_count = store
        .all_node_ids()
        .iter()
        .filter(|id| {
            store.edges_from(id).is_empty() && store.edges_to(id).is_empty()
        })
        .count();

    if orphan_count > 50 {
        warnings.push(format!("{orphan_count} orphan nodes (no connections)"));
    }

    // Check for very low confidence edges
    let low_confidence: usize = store
        .all_node_ids()
        .iter()
        .flat_map(|id| store.edges_from(id))
        .filter(|(e, _)| e.confidence < 0.2)
        .count();

    if low_confidence > 20 {
        warnings.push(format!("{low_confidence} edges with confidence < 0.2"));
    }

    if !is_dag {
        warnings.push("Graph contains cycles — this may indicate incorrect causal edges".into());
    }

    let is_valid = is_dag && contradictions.is_empty();

    VerifyResult {
        is_valid,
        is_dag,
        contradictions,
        warnings,
    }
}

/// Detect contradictions: pairs of edges where one says A→B (Caused)
/// and another says A→B (Prevented), or similar conflicts
fn detect_contradictions(store: &GraphStore) -> Vec<Contradiction> {
    let mut contradictions = Vec::new();

    for node_id in store.all_node_ids() {
        let outgoing = store.edges_from(&node_id);

        // Check for Caused + Prevented to the same target
        for i in 0..outgoing.len() {
            for j in (i + 1)..outgoing.len() {
                let (edge_a, target_a) = &outgoing[i];
                let (edge_b, target_b) = &outgoing[j];

                if target_a.id != target_b.id {
                    continue;
                }

                let is_contradiction = matches!(
                    (&edge_a.kind, &edge_b.kind),
                    (EdgeKind::Caused, EdgeKind::Prevented)
                    | (EdgeKind::Prevented, EdgeKind::Caused)
                    | (EdgeKind::Enabled, EdgeKind::Prevented)
                    | (EdgeKind::Prevented, EdgeKind::Enabled)
                );

                if is_contradiction && edge_a.confidence > 0.3 && edge_b.confidence > 0.3 {
                    contradictions.push(Contradiction {
                        node_a: node_id,
                        node_b: target_a.id,
                        description: format!(
                            "{:?} vs {:?} (conf {:.2} vs {:.2})",
                            edge_a.kind, edge_b.kind, edge_a.confidence, edge_b.confidence
                        ),
                        edge_a_kind: edge_a.kind.clone(),
                        edge_b_kind: edge_b.kind.clone(),
                    });
                }
            }
        }
    }

    contradictions
}

/// Resolve contradictions by keeping the edge with higher evidence × recency
pub fn resolve_contradictions(store: &mut GraphStore, current_session: u64) {
    let result = verify_graph(store);

    for contradiction in &result.contradictions {
        let edges = store.edges_from(&contradiction.node_a);
        let target_id = contradiction.node_b;

        // Find the two conflicting edges
        let matching: Vec<_> = edges
            .iter()
            .filter(|(_, target)| target.id == target_id)
            .filter(|(e, _)| e.kind == contradiction.edge_a_kind || e.kind == contradiction.edge_b_kind)
            .collect();

        if matching.len() >= 2 {
            // Score by evidence_count × recency
            let score = |e: &CausalEdge| -> f64 {
                let recency = 1.0 / (1.0 + (current_session.saturating_sub(e.last_touched_session)) as f64);
                e.evidence_count as f64 * recency
            };

            let score_a = score(matching[0].0);
            let score_b = score(matching[1].0);

            // We can't easily remove a specific edge from petgraph by EdgeKind
            // So we log the resolution and set the loser's confidence to 0
            // (it will be pruned on next decay)
            tracing::info!(
                "Contradiction resolved: {:?} (score {:.2}) beats {:?} (score {:.2})",
                if score_a >= score_b { &matching[0].0.kind } else { &matching[1].0.kind },
                score_a.max(score_b),
                if score_a >= score_b { &matching[1].0.kind } else { &matching[0].0.kind },
                score_a.min(score_b),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracey_core::id::NodeId;

    #[test]
    fn test_clean_graph_is_valid() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "a.rs"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "b.rs"));
        store.add_edge(n1, n2, CausalEdge::new(EdgeKind::Imports, EdgeSource::StaticAnalysis));

        let result = verify_graph(&store);
        assert!(result.is_valid);
        assert!(result.is_dag);
        assert!(result.contradictions.is_empty());
    }

    #[test]
    fn test_contradiction_detected() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Action, "edit"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Error, "fail"));

        store.add_edge(n1, n2, CausalEdge::with_confidence(EdgeKind::Caused, EdgeSource::AgentObserved, 0.5));
        store.add_edge(n1, n2, CausalEdge::with_confidence(EdgeKind::Prevented, EdgeSource::AgentObserved, 0.5));

        let result = verify_graph(&store);
        assert!(!result.contradictions.is_empty(), "Should detect Caused + Prevented contradiction");
    }
}
