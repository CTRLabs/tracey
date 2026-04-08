use async_trait::async_trait;
use serde_json::{json, Value};
use tracey_core::traits::{ToolContext, ToolHandler, ToolOutput, ToolSchema};
use tracey_core::TraceyResult;

pub struct GlobTool;

#[async_trait]
impl ToolHandler for GlobTool {
    async fn execute(&self, args: Value, ctx: &ToolContext) -> TraceyResult<ToolOutput> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| tracey_core::TraceyError::Tool("pattern required".into()))?;

        let base = args["path"]
            .as_str()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| ctx.cwd.clone());

        let full_pattern = base.join(pattern).to_string_lossy().to_string();

        let mut matches: Vec<String> = glob::glob(&full_pattern)
            .map_err(|e| tracey_core::TraceyError::Tool(format!("invalid glob: {e}")))?
            .filter_map(|entry| entry.ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        matches.sort();

        if matches.is_empty() {
            Ok(ToolOutput::success("No files matched."))
        } else {
            Ok(ToolOutput::success(matches.join("\n")))
        }
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "Glob".into(),
            description: "Find files matching a glob pattern.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g. '**/*.rs')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }
}
