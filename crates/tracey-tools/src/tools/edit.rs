use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tracey_core::traits::{ToolContext, ToolHandler, ToolOutput, ToolSchema};
use tracey_core::{TraceyError, TraceyResult};

pub struct EditTool;

#[async_trait]
impl ToolHandler for EditTool {
    async fn execute(&self, args: Value, ctx: &ToolContext) -> TraceyResult<ToolOutput> {
        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| TraceyError::Tool("file_path required".into()))?;
        let old_string = args["old_string"]
            .as_str()
            .ok_or_else(|| TraceyError::Tool("old_string required".into()))?;
        let new_string = args["new_string"]
            .as_str()
            .ok_or_else(|| TraceyError::Tool("new_string required".into()))?;
        let replace_all = args["replace_all"].as_bool().unwrap_or(false);

        let path = if file_path.starts_with('/') {
            PathBuf::from(file_path)
        } else {
            ctx.cwd.join(file_path)
        };

        let content = std::fs::read_to_string(&path)
            .map_err(|e| TraceyError::Tool(format!("read {}: {e}", path.display())))?;

        if !content.contains(old_string) {
            return Ok(ToolOutput::error(format!(
                "old_string not found in {}",
                path.display()
            )));
        }

        if !replace_all {
            let count = content.matches(old_string).count();
            if count > 1 {
                return Ok(ToolOutput::error(format!(
                    "old_string matches {count} times in {}. Provide more context or use replace_all.",
                    path.display()
                )));
            }
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        std::fs::write(&path, &new_content)?;

        Ok(ToolOutput::success(format!(
            "Edited {}",
            path.display()
        )))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "Edit".into(),
            description: "Replace a string in a file.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact string to find and replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement string"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default false)"
                    }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        }
    }
}
