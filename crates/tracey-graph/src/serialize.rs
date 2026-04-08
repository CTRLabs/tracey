use crate::query::Subgraph;
use crate::store::{EdgeSource, GraphLayer, NodeKind};

/// Serialize a subgraph to Markdown-KV format for LLM context injection.
/// Research shows Markdown-KV outperforms JSON by ~16 points for LLM comprehension.
/// Output is capped at ~2000 tokens (~8000 chars).
pub fn to_markdown_kv(subgraph: &Subgraph) -> String {
    const MAX_CHARS: usize = 8000;

    if subgraph.nodes.is_empty() {
        return String::new();
    }

    let mut output = String::with_capacity(MAX_CHARS);
    output.push_str("<graph-context>\n");
    output.push_str(&format!(
        "## Causal Context ({} nodes, {} edges)\n\n",
        subgraph.nodes.len(),
        subgraph.edges.len()
    ));

    // Group nodes by layer
    let mut code_nodes = Vec::new();
    let mut exec_nodes = Vec::new();
    let mut knowledge_nodes = Vec::new();
    let mut project_nodes = Vec::new();

    for (node, score) in &subgraph.nodes {
        let entry = format!(
            "- [{}] {} (conf: {:.2}, relevance: {:.3})\n",
            kind_label(&node.kind),
            node.label,
            node.confidence,
            score
        );
        match node.layer {
            GraphLayer::Code => code_nodes.push(entry),
            GraphLayer::Execution => exec_nodes.push(entry),
            GraphLayer::Knowledge => knowledge_nodes.push(entry),
            GraphLayer::Project => project_nodes.push(entry),
        }
    }

    if !code_nodes.is_empty() {
        output.push_str("### Code Structure\n");
        for entry in &code_nodes {
            if output.len() + entry.len() > MAX_CHARS - 200 {
                output.push_str(&format!("... and {} more code nodes\n", code_nodes.len()));
                break;
            }
            output.push_str(entry);
        }
        output.push('\n');
    }

    if !exec_nodes.is_empty() {
        output.push_str("### Recent Actions\n");
        for entry in &exec_nodes {
            if output.len() + entry.len() > MAX_CHARS - 200 {
                break;
            }
            output.push_str(entry);
        }
        output.push('\n');
    }

    if !knowledge_nodes.is_empty() {
        output.push_str("### Known Facts\n");
        for entry in &knowledge_nodes {
            if output.len() + entry.len() > MAX_CHARS - 200 {
                break;
            }
            output.push_str(entry);
        }
        output.push('\n');
    }

    if !project_nodes.is_empty() {
        output.push_str("### Project Context\n");
        for entry in &project_nodes {
            if output.len() + entry.len() > MAX_CHARS - 200 {
                break;
            }
            output.push_str(entry);
        }
        output.push('\n');
    }

    // Edges section
    if !subgraph.edges.is_empty() {
        output.push_str("### Relationships\n");
        for (edge, from_id, to_id) in &subgraph.edges {
            if output.len() > MAX_CHARS - 100 {
                output.push_str("... (more edges truncated)\n");
                break;
            }
            // Find labels for source and target
            let from_label = subgraph
                .nodes
                .iter()
                .find(|(n, _)| n.id == *from_id)
                .map(|(n, _)| n.label.as_str())
                .unwrap_or("?");
            let to_label = subgraph
                .nodes
                .iter()
                .find(|(n, _)| n.id == *to_id)
                .map(|(n, _)| n.label.as_str())
                .unwrap_or("?");

            output.push_str(&format!(
                "- {} --[{}]--> {} (conf: {:.2}, src: {})\n",
                from_label,
                edge_kind_label(&edge.kind),
                to_label,
                edge.confidence,
                source_label(&edge.source),
            ));
        }
    }

    output.push_str("</graph-context>");
    output
}

/// Render subgraph as compact Unicode text for Telegram
pub fn to_unicode_tree(subgraph: &Subgraph) -> String {
    if subgraph.nodes.is_empty() {
        return "  (no graph context)".to_string();
    }

    let mut output = String::new();
    // Simple tree rendering: list nodes with edges
    for (i, (node, score)) in subgraph.nodes.iter().enumerate().take(15) {
        let prefix = if i == 0 { "◉" } else { "├──▸ ◉" };
        output.push_str(&format!(
            "{} {} [{}] ({:.0}%)\n",
            prefix,
            node.label,
            kind_label(&node.kind),
            node.confidence * 100.0
        ));

        // Show outgoing edges from this node
        for (edge, _from, to) in &subgraph.edges {
            if _from == &node.id {
                let to_label = subgraph
                    .nodes
                    .iter()
                    .find(|(n, _)| n.id == *to)
                    .map(|(n, _)| n.label.as_str())
                    .unwrap_or("?");
                output.push_str(&format!(
                    "│    └──▸ {} ({})\n",
                    to_label,
                    edge_kind_label(&edge.kind)
                ));
            }
        }
    }

    if subgraph.nodes.len() > 15 {
        output.push_str(&format!("... and {} more nodes\n", subgraph.nodes.len() - 15));
    }

    output
}

fn kind_label(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::File => "File",
        NodeKind::Function => "fn",
        NodeKind::Class => "Class",
        NodeKind::Module => "Module",
        NodeKind::Type => "Type",
        NodeKind::Test => "Test",
        NodeKind::Import => "Import",
        NodeKind::Action => "Action",
        NodeKind::Observation => "Obs",
        NodeKind::Decision => "Decision",
        NodeKind::Error => "Error",
        NodeKind::ToolCall => "ToolCall",
        NodeKind::Fact => "Fact",
        NodeKind::Preference => "Pref",
        NodeKind::Pattern => "Pattern",
        NodeKind::Bug => "Bug",
        NodeKind::Solution => "Solution",
        NodeKind::KnowledgeDecision => "Decision",
        NodeKind::Task => "Task",
        NodeKind::Goal => "Goal",
        NodeKind::Constraint => "Constraint",
        NodeKind::Deadline => "Deadline",
    }
}

fn edge_kind_label(kind: &crate::store::EdgeKind) -> &'static str {
    match kind {
        crate::store::EdgeKind::Calls => "calls",
        crate::store::EdgeKind::Imports => "imports",
        crate::store::EdgeKind::Inherits => "inherits",
        crate::store::EdgeKind::Implements => "implements",
        crate::store::EdgeKind::Tests => "tests",
        crate::store::EdgeKind::DependsOn => "depends_on",
        crate::store::EdgeKind::Caused => "caused",
        crate::store::EdgeKind::Enabled => "enabled",
        crate::store::EdgeKind::Prevented => "prevented",
        crate::store::EdgeKind::Triggered => "triggered",
        crate::store::EdgeKind::Resolved => "resolved",
        crate::store::EdgeKind::Explains => "explains",
        crate::store::EdgeKind::Contradicts => "contradicts",
        crate::store::EdgeKind::Supersedes => "supersedes",
        crate::store::EdgeKind::CausedBy => "caused_by",
        crate::store::EdgeKind::LedTo => "led_to",
        crate::store::EdgeKind::Blocks => "blocks",
        crate::store::EdgeKind::Requires => "requires",
        crate::store::EdgeKind::ContributesTo => "contributes_to",
        crate::store::EdgeKind::Risks => "risks",
    }
}

fn source_label(source: &EdgeSource) -> &'static str {
    match source {
        EdgeSource::StaticAnalysis => "static",
        EdgeSource::GitCoChange => "git",
        EdgeSource::AgentObserved => "agent",
        EdgeSource::PcmciLearned => "learned",
        EdgeSource::UserDefined => "user",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::*;

    #[test]
    fn test_markdown_kv_has_tags() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "main.rs"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "main"));
        store.add_edge(n1, n2, CausalEdge::new(EdgeKind::DependsOn, EdgeSource::StaticAnalysis));

        let subgraph = crate::query::personalized_pagerank(
            &store,
            &crate::query::SubgraphQuery {
                seeds: vec![n1],
                max_nodes: 25,
                ..Default::default()
            },
        );

        let md = to_markdown_kv(&subgraph);
        assert!(md.starts_with("<graph-context>"), "Should start with tag");
        assert!(md.ends_with("</graph-context>"), "Should end with tag");
        assert!(md.contains("main.rs"), "Should contain node label");
        assert!(md.len() <= 8200, "Should be under ~8000 chars");
    }

    #[test]
    fn test_unicode_tree() {
        let mut store = GraphStore::new();
        let n1 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "auth.rs"));
        let subgraph = crate::query::personalized_pagerank(
            &store,
            &crate::query::SubgraphQuery {
                seeds: vec![n1],
                max_nodes: 25,
                ..Default::default()
            },
        );
        let tree = to_unicode_tree(&subgraph);
        assert!(tree.contains("auth.rs"));
    }
}
