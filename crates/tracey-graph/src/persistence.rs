use crate::store::*;
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;
use tracey_core::id::{EdgeId, NodeId};
use tracey_core::TraceyResult;

pub struct GraphDb {
    conn: Connection,
}

impl GraphDb {
    pub fn open(path: &Path) -> TraceyResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)
            .map_err(|e| tracey_core::TraceyError::Graph(format!("open db: {e}")))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| tracey_core::TraceyError::Graph(format!("pragma: {e}")))?;

        let db = Self { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> TraceyResult<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                layer TEXT NOT NULL,
                kind TEXT NOT NULL,
                label TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                confidence REAL NOT NULL DEFAULT 1.0,
                last_touched_session INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_nodes_label ON nodes(label);
            CREATE INDEX IF NOT EXISTS idx_nodes_layer ON nodes(layer);
            CREATE INDEX IF NOT EXISTS idx_nodes_kind ON nodes(kind);

            CREATE TABLE IF NOT EXISTS edges (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                source_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                confidence REAL NOT NULL DEFAULT 1.0,
                evidence_count INTEGER NOT NULL DEFAULT 1,
                last_touched_session INTEGER NOT NULL DEFAULT 0,
                metadata TEXT DEFAULT '{}'
            );
            CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);

            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '1');
            ",
            )
            .map_err(|e| tracey_core::TraceyError::Graph(format!("create tables: {e}")))?;
        Ok(())
    }

    pub fn save(&self, store: &GraphStore) -> TraceyResult<()> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| tracey_core::TraceyError::Graph(format!("begin tx: {e}")))?;

        // Clear existing data
        tx.execute_batch("DELETE FROM edges; DELETE FROM nodes;")
            .map_err(|e| tracey_core::TraceyError::Graph(format!("clear: {e}")))?;

        // Save nodes
        {
            let mut node_stmt = tx
                .prepare(
                    "INSERT INTO nodes (id, layer, kind, label, metadata, confidence, last_touched_session, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .map_err(|e| tracey_core::TraceyError::Graph(format!("prepare node: {e}")))?;

            for node_id in store.all_node_ids() {
                if let Some(node) = store.get_node(&node_id) {
                    node_stmt
                        .execute(params![
                            node.id.to_string(),
                            serde_json::to_string(&node.layer).unwrap_or_default(),
                            serde_json::to_string(&node.kind).unwrap_or_default(),
                            node.label,
                            node.metadata.to_string(),
                            node.confidence,
                            node.last_touched_session,
                            node.created_at.to_rfc3339(),
                            node.updated_at.to_rfc3339(),
                        ])
                        .map_err(|e| tracey_core::TraceyError::Graph(format!("insert node: {e}")))?;
                }
            }
        }

        // Save edges
        {
            let mut edge_stmt = tx
                .prepare(
                    "INSERT INTO edges (id, source_id, target_id, kind, source_type, weight, confidence, evidence_count, last_touched_session, metadata)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(|e| tracey_core::TraceyError::Graph(format!("prepare edge: {e}")))?;

            for node_id in store.all_node_ids() {
                for (edge, target) in store.edges_from(&node_id) {
                    edge_stmt
                        .execute(params![
                            edge.id.to_string(),
                            node_id.to_string(),
                            target.id.to_string(),
                            serde_json::to_string(&edge.kind).unwrap_or_default(),
                            serde_json::to_string(&edge.source).unwrap_or_default(),
                            edge.weight,
                            edge.confidence,
                            edge.evidence_count,
                            edge.last_touched_session,
                            edge.metadata.to_string(),
                        ])
                        .map_err(|e| tracey_core::TraceyError::Graph(format!("insert edge: {e}")))?;
                }
            }
        }

        tx.commit()
            .map_err(|e| tracey_core::TraceyError::Graph(format!("commit: {e}")))?;

        Ok(())
    }

    pub fn load(&self) -> TraceyResult<GraphStore> {
        let mut store = GraphStore::new();

        // Load nodes
        let mut stmt = self
            .conn
            .prepare("SELECT id, layer, kind, label, metadata, confidence, last_touched_session, created_at, updated_at FROM nodes")
            .map_err(|e| tracey_core::TraceyError::Graph(format!("prepare load nodes: {e}")))?;

        let node_rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let layer_str: String = row.get(1)?;
                let kind_str: String = row.get(2)?;
                let label: String = row.get(3)?;
                let metadata_str: String = row.get(4)?;
                let confidence: f64 = row.get(5)?;
                let last_touched: i64 = row.get(6)?;
                let created_str: String = row.get(7)?;
                let updated_str: String = row.get(8)?;

                Ok((id_str, layer_str, kind_str, label, metadata_str, confidence, last_touched, created_str, updated_str))
            })
            .map_err(|e| tracey_core::TraceyError::Graph(format!("query nodes: {e}")))?;

        let mut id_map: std::collections::HashMap<String, NodeId> = std::collections::HashMap::new();

        for row in node_rows {
            let (id_str, layer_str, kind_str, label, metadata_str, confidence, last_touched, created_str, updated_str) =
                row.map_err(|e| tracey_core::TraceyError::Graph(format!("read node: {e}")))?;

            let layer: GraphLayer = serde_json::from_str(&layer_str).unwrap_or(GraphLayer::Code);
            let kind: NodeKind = serde_json::from_str(&kind_str).unwrap_or(NodeKind::File);
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or(serde_json::Value::Null);

            let mut node = CausalNode::new(layer, kind, &label);
            node.confidence = confidence;
            node.last_touched_session = last_touched as u64;
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&created_str) {
                node.created_at = dt.with_timezone(&chrono::Utc);
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&updated_str) {
                node.updated_at = dt.with_timezone(&chrono::Utc);
            }
            node.metadata = metadata;

            let actual_id = store.add_node(node);
            id_map.insert(id_str, actual_id);
        }

        // Load edges
        let mut stmt = self
            .conn
            .prepare("SELECT id, source_id, target_id, kind, source_type, weight, confidence, evidence_count, last_touched_session, metadata FROM edges")
            .map_err(|e| tracey_core::TraceyError::Graph(format!("prepare load edges: {e}")))?;

        let edge_rows = stmt
            .query_map([], |row| {
                let _id_str: String = row.get(0)?;
                let source_str: String = row.get(1)?;
                let target_str: String = row.get(2)?;
                let kind_str: String = row.get(3)?;
                let source_type_str: String = row.get(4)?;
                let weight: f64 = row.get(5)?;
                let confidence: f64 = row.get(6)?;
                let evidence_count: i32 = row.get(7)?;
                let last_touched: i64 = row.get(8)?;
                let metadata_str: String = row.get(9)?;

                Ok((source_str, target_str, kind_str, source_type_str, weight, confidence, evidence_count, last_touched, metadata_str))
            })
            .map_err(|e| tracey_core::TraceyError::Graph(format!("query edges: {e}")))?;

        for row in edge_rows {
            let (source_str, target_str, kind_str, source_type_str, weight, confidence, evidence_count, last_touched, metadata_str) =
                row.map_err(|e| tracey_core::TraceyError::Graph(format!("read edge: {e}")))?;

            let Some(&from_id) = id_map.get(&source_str) else { continue };
            let Some(&to_id) = id_map.get(&target_str) else { continue };

            let kind: EdgeKind = serde_json::from_str(&kind_str).unwrap_or(EdgeKind::DependsOn);
            let source: EdgeSource = serde_json::from_str(&source_type_str).unwrap_or(EdgeSource::AgentObserved);
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or(serde_json::Value::Null);

            let mut edge = CausalEdge::with_confidence(kind, source, confidence);
            edge.weight = weight;
            edge.evidence_count = evidence_count as u32;
            edge.last_touched_session = last_touched as u64;
            edge.metadata = metadata;

            store.add_edge(from_id, to_id, edge);
        }

        Ok(store)
    }

    pub fn load_session_counter(&self) -> TraceyResult<u64> {
        let result: SqlResult<String> = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'session_counter'", [], |row| {
                row.get(0)
            });

        match result {
            Ok(val) => Ok(val.parse().unwrap_or(0)),
            Err(_) => Ok(0),
        }
    }

    pub fn save_session_counter(&self, counter: u64) -> TraceyResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('session_counter', ?1)",
                params![counter.to_string()],
            )
            .map_err(|e| tracey_core::TraceyError::Graph(format!("save counter: {e}")))?;
        Ok(())
    }
}

/// Get the graph database path for a project
pub fn graph_db_path(project_root: &Path) -> std::path::PathBuf {
    project_root.join(".tracey").join("graph.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sqlite_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        // Create and save
        let db = GraphDb::open(&db_path).unwrap();
        let mut store = GraphStore::new();

        let n1 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::File, "main.rs"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Code, NodeKind::Function, "main"));
        store.add_edge(n1, n2, CausalEdge::new(EdgeKind::DependsOn, EdgeSource::StaticAnalysis));

        db.save(&store).unwrap();
        db.save_session_counter(5).unwrap();

        // Load back
        let loaded = db.load().unwrap();
        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.edge_count(), 1);
        assert_eq!(db.load_session_counter().unwrap(), 5);

        // Verify labels survived
        assert!(loaded.find_by_label("main.rs").is_some());
        assert!(loaded.find_by_label("main").is_some());
    }

    #[test]
    fn test_edge_properties_survive() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test2.db");

        let db = GraphDb::open(&db_path).unwrap();
        let mut store = GraphStore::new();

        let n1 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Action, "edit"));
        let n2 = store.add_node(CausalNode::new(GraphLayer::Execution, NodeKind::Error, "fail"));
        let mut edge = CausalEdge::with_confidence(EdgeKind::Caused, EdgeSource::AgentObserved, 0.5);
        edge.evidence_count = 3;
        edge.last_touched_session = 10;
        store.add_edge(n1, n2, edge);

        db.save(&store).unwrap();
        let loaded = db.load().unwrap();

        // Find the edge and check properties
        let edit_id = loaded.find_id_by_label("edit").unwrap();
        let edges = loaded.edges_from(&edit_id);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0.evidence_count, 3);
        assert!(edges[0].0.confidence <= 0.7); // AgentObserved cap
    }
}
