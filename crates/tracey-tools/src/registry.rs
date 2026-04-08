use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracey_core::traits::{ToolContext, ToolHandler, ToolOutput, ToolSchema};
use tracey_core::TraceyResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    FileOps,
    Search,
    Execution,
    Agent,
    Causal,
    Web,
    Custom,
}

pub struct ToolEntry {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub handler: Arc<dyn ToolHandler>,
    pub is_deferred: bool,
}

pub struct ToolRegistry {
    tools: HashMap<String, ToolEntry>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, entry: ToolEntry) {
        self.tools.insert(entry.name.clone(), entry);
    }

    pub fn get(&self, name: &str) -> Option<&ToolEntry> {
        self.tools.get(name)
    }

    pub async fn execute(
        &self,
        name: &str,
        arguments: Value,
        ctx: &ToolContext,
    ) -> TraceyResult<ToolOutput> {
        let entry = self
            .tools
            .get(name)
            .ok_or_else(|| tracey_core::TraceyError::Tool(format!("unknown tool: {name}")))?;

        entry.handler.execute(arguments, ctx).await
    }

    /// Get schemas for all non-deferred tools (for LLM consumption)
    pub fn active_schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .values()
            .filter(|e| !e.is_deferred)
            .map(|e| e.handler.schema())
            .collect()
    }

    /// Get name + description for deferred tools
    pub fn deferred_summaries(&self) -> Vec<(String, String)> {
        self.tools
            .values()
            .filter(|e| e.is_deferred)
            .map(|e| (e.name.clone(), e.description.clone()))
            .collect()
    }

    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}
