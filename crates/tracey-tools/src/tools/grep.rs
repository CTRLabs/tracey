use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use tracey_core::traits::{ToolContext, ToolHandler, ToolOutput, ToolSchema};
use tracey_core::TraceyResult;

pub struct GrepTool;

#[async_trait]
impl ToolHandler for GrepTool {
    async fn execute(&self, args: Value, ctx: &ToolContext) -> TraceyResult<ToolOutput> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| tracey_core::TraceyError::Tool("pattern required".into()))?;

        let path = args["path"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| ctx.cwd.clone());

        let output_mode = args["output_mode"].as_str().unwrap_or("files_with_matches");

        // Use grep/rg for actual search
        let mut cmd = if which("rg") {
            let mut c = Command::new("rg");
            c.arg("--no-heading");
            match output_mode {
                "files_with_matches" => { c.arg("-l"); }
                "count" => { c.arg("-c"); }
                _ => { c.arg("-n"); }
            }
            if let Some(glob) = args["glob"].as_str() {
                c.arg("--glob").arg(glob);
            }
            if args["-i"].as_bool().unwrap_or(false) {
                c.arg("-i");
            }
            c.arg(pattern);
            c.arg(path);
            c
        } else {
            let mut c = Command::new("grep");
            c.arg("-r").arg("-n");
            if output_mode == "files_with_matches" {
                c.arg("-l");
            }
            if args["-i"].as_bool().unwrap_or(false) {
                c.arg("-i");
            }
            c.arg(pattern);
            c.arg(path);
            c
        };

        let output = cmd.output().map_err(|e| {
            tracey_core::TraceyError::Tool(format!("grep failed: {e}"))
        })?;

        let result = String::from_utf8_lossy(&output.stdout).to_string();

        // Limit output
        let limited: String = result.lines().take(250).collect::<Vec<_>>().join("\n");

        Ok(ToolOutput::success(limited))
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "Grep".into(),
            description: "Search file contents with regex.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search"
                    },
                    "output_mode": {
                        "type": "string",
                        "enum": ["content", "files_with_matches", "count"],
                        "description": "Output mode"
                    },
                    "glob": {
                        "type": "string",
                        "description": "File glob filter"
                    },
                    "-i": {
                        "type": "boolean",
                        "description": "Case insensitive"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
