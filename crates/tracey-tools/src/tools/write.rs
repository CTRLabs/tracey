use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tracey_core::traits::{ToolContext, ToolHandler, ToolOutput, ToolSchema};
use tracey_core::{TraceyError, TraceyResult};

pub struct WriteTool;

#[async_trait]
impl ToolHandler for WriteTool {
    async fn execute(&self, args: Value, ctx: &ToolContext) -> TraceyResult<ToolOutput> {
        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| TraceyError::Tool("file_path required".into()))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| TraceyError::Tool("content required".into()))?;

        let path = if file_path.starts_with('/') {
            PathBuf::from(file_path)
        } else {
            ctx.cwd.join(file_path)
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&path, content)?;

        Ok(ToolOutput::success(format!(
            "File written: {} ({} bytes)",
            path.display(),
            content.len()
        )))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "Write".into(),
            description: "Write content to a file, creating it if needed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write"
                    }
                },
                "required": ["file_path", "content"]
            }),
        }
    }
}
