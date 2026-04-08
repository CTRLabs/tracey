use async_trait::async_trait;
use serde_json::{json, Value};
use tracey_core::traits::{ToolContext, ToolHandler, ToolOutput, ToolSchema};
use tracey_core::TraceyResult;
use tracey_sandbox::{SandboxPolicy, SandboxedExecutor};

pub struct BashTool {
    executor: SandboxedExecutor,
}

impl BashTool {
    pub fn new() -> Self {
        Self {
            executor: SandboxedExecutor::new(SandboxPolicy::default()),
        }
    }
}

#[async_trait]
impl ToolHandler for BashTool {
    async fn execute(&self, args: Value, ctx: &ToolContext) -> TraceyResult<ToolOutput> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| tracey_core::TraceyError::Tool("command required".into()))?;

        let timeout = args["timeout"]
            .as_u64()
            .map(std::time::Duration::from_millis);

        let result = self.executor.execute(command, &ctx.cwd, timeout).await?;

        let mut output = String::new();
        if !result.stdout.is_empty() {
            output.push_str(&result.stdout);
        }
        if !result.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&result.stderr);
        }

        // Truncate very long output
        if output.len() > 50_000 {
            output.truncate(50_000);
            output.push_str("\n... (output truncated)");
        }

        if result.exit_code != 0 {
            Ok(ToolOutput::error(format!(
                "Exit code: {}\n{}",
                result.exit_code, output
            )))
        } else {
            Ok(ToolOutput::success(output))
        }
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "Bash".into(),
            description: "Execute a shell command and return its output.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to execute"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in milliseconds (default 120000)"
                    }
                },
                "required": ["command"]
            }),
        }
    }
}
