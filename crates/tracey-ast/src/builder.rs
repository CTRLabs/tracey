use crate::parser::{detect_language, parse_file};
use ignore::WalkBuilder;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracey_graph::{CausalEdge, CausalNode, EdgeKind, EdgeSource, GraphLayer, GraphStore, NodeKind};

/// Statistics from building the code graph
#[derive(Debug, Clone)]
pub struct BuildStats {
    pub files_parsed: usize,
    pub nodes_created: usize,
    pub edges_created: usize,
    pub duration: Duration,
    pub languages_detected: Vec<String>,
}

/// Build a code graph from a project directory using regex-based parsing.
/// All edges are StaticAnalysis (confidence 1.0).
pub fn build_code_graph(root: &Path, store: &mut GraphStore) -> BuildStats {
    let start = Instant::now();
    let mut files_parsed = 0;
    let mut nodes_before = store.node_count();
    let mut edges_before = store.edge_count();
    let mut languages: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Walk directory respecting .gitignore
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| {
            let path = entry.path();
            // Skip common non-code directories
            let skip = ["node_modules", "target", ".git", "__pycache__", "dist", "build",
                        ".next", "vendor", "venv", ".venv", "env", ".env"];
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                !skip.contains(&name)
            } else {
                true
            }
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(lang) = detect_language(path) else {
            continue;
        };

        // Skip very large files (>500KB)
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > 500_000 {
                continue;
            }
        }

        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };

        // Use relative path from root
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy().to_string();

        languages.insert(format!("{lang:?}"));

        let result = parse_file(rel_path, &content, lang);
        files_parsed += 1;

        // Add entities as nodes
        for entity in &result.entities {
            let label = if entity.kind == NodeKind::File {
                rel_str.clone()
            } else {
                entity.name.clone()
            };

            // Skip if node already exists
            if store.find_by_label(&label).is_some() {
                continue;
            }

            let mut node = CausalNode::new(GraphLayer::Code, entity.kind.clone(), &label);
            node.metadata = serde_json::json!({
                "file": rel_str,
                "line_start": entity.line_start,
                "line_end": entity.line_end,
            });
            store.add_node(node);
        }

        // Add relations as edges
        for relation in &result.relations {
            let from_id = store.find_id_by_label(&relation.from_label);
            let to_id = store.find_id_by_label(&relation.to_label);

            if let (Some(from), Some(to)) = (from_id, to_id) {
                let edge = CausalEdge::new(relation.kind.clone(), EdgeSource::StaticAnalysis);
                store.add_edge(from, to, edge);
            }
        }
    }

    // Add test edges: files in test directories test files in src directories
    add_test_edges(root, store);

    let duration = start.elapsed();
    let nodes_created = store.node_count() - nodes_before;
    let edges_created = store.edge_count() - edges_before;

    BuildStats {
        files_parsed,
        nodes_created,
        edges_created,
        duration,
        languages_detected: languages.into_iter().collect(),
    }
}

/// Heuristic: files matching *test* or in tests/ directory get Tests edges
/// to corresponding source files
fn add_test_edges(_root: &Path, store: &mut GraphStore) {
    let file_nodes: Vec<(tracey_core::id::NodeId, String)> = store
        .nodes_by_layer(GraphLayer::Code)
        .iter()
        .filter(|n| n.kind == NodeKind::File)
        .map(|n| (n.id, n.label.clone()))
        .collect();

    let test_files: Vec<(tracey_core::id::NodeId, String)> = file_nodes
        .iter()
        .filter(|(_, label)| {
            label.contains("test") || label.contains("spec") || label.contains("_test.")
        })
        .cloned()
        .collect();

    let src_files: Vec<(tracey_core::id::NodeId, String)> = file_nodes
        .iter()
        .filter(|(_, label)| !label.contains("test") && !label.contains("spec"))
        .cloned()
        .collect();

    for (test_id, test_label) in &test_files {
        // Try to match test file to source file
        let test_stem = test_label
            .replace("test_", "")
            .replace("_test", "")
            .replace("tests/", "src/")
            .replace(".test.", ".")
            .replace(".spec.", ".");

        for (src_id, src_label) in &src_files {
            if src_label.contains(&test_stem) || test_stem.contains(src_label.as_str()) {
                let edge = CausalEdge::new(EdgeKind::Tests, EdgeSource::StaticAnalysis);
                store.add_edge(*test_id, *src_id, edge);
                break;
            }
        }
    }
}

/// Build code graph asynchronously (for use in the agent startup)
pub async fn build_code_graph_async(
    root: &Path,
    graph: &Arc<RwLock<GraphStore>>,
) -> BuildStats {
    let root = root.to_path_buf();
    let graph = graph.clone();

    tokio::task::spawn_blocking(move || {
        let mut store = graph.write().unwrap();
        build_code_graph(&root, &mut store)
    })
    .await
    .unwrap_or(BuildStats {
        files_parsed: 0,
        nodes_created: 0,
        edges_created: 0,
        duration: Duration::ZERO,
        languages_detected: vec![],
    })
}
