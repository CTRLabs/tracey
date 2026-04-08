pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod read;
pub mod write;

use crate::registry::{ToolCategory, ToolEntry, ToolRegistry};
use std::sync::Arc;

pub fn register_core_tools(registry: &mut ToolRegistry) {
    registry.register(ToolEntry {
        name: "Read".into(),
        description: "Read file contents with line numbers".into(),
        category: ToolCategory::FileOps,
        handler: Arc::new(read::ReadTool),
        is_deferred: false,
    });

    registry.register(ToolEntry {
        name: "Write".into(),
        description: "Create or overwrite a file".into(),
        category: ToolCategory::FileOps,
        handler: Arc::new(write::WriteTool),
        is_deferred: false,
    });

    registry.register(ToolEntry {
        name: "Edit".into(),
        description: "Replace a string in a file".into(),
        category: ToolCategory::FileOps,
        handler: Arc::new(edit::EditTool),
        is_deferred: false,
    });

    registry.register(ToolEntry {
        name: "Bash".into(),
        description: "Execute a shell command".into(),
        category: ToolCategory::Execution,
        handler: Arc::new(bash::BashTool::new()),
        is_deferred: false,
    });

    registry.register(ToolEntry {
        name: "Glob".into(),
        description: "Find files by pattern".into(),
        category: ToolCategory::Search,
        handler: Arc::new(glob::GlobTool),
        is_deferred: false,
    });

    registry.register(ToolEntry {
        name: "Grep".into(),
        description: "Search file contents with regex".into(),
        category: ToolCategory::Search,
        handler: Arc::new(grep::GrepTool),
        is_deferred: false,
    });
}
