use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tracey_core::traits::{ToolContext, ToolHandler, ToolOutput, ToolSchema};
use tracey_core::{TraceyError, TraceyResult};

pub struct ReadTool;

#[async_trait]
impl ToolHandler for ReadTool {
    async fn execute(&self, args: Value, ctx: &ToolContext) -> TraceyResult<ToolOutput> {
        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| TraceyError::Tool("file_path required".into()))?;

        let path = if file_path.starts_with('/') {
            PathBuf::from(file_path)
        } else {
            ctx.cwd.join(file_path)
        };

        if !path.exists() {
            return Ok(ToolOutput::error(format!("File not found: {}", path.display())));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| TraceyError::Tool(format!("read {}: {e}", path.display())))?;

        let offset = args["offset"].as_u64().unwrap_or(0) as usize;
        let limit = args["limit"].as_u64().unwrap_or(2000) as usize;

        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let end = (offset + limit).min(total);
        let selected = &lines[offset.min(total)..end];

        let numbered: String = selected
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}\t{}", offset + i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolOutput::success(numbered))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "Read".into(),
            description: "Read a file's contents with line numbers.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start from (0-indexed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max lines to read (default 2000)"
                    }
                },
                "required": ["file_path"]
            }),
        }
    }
}
