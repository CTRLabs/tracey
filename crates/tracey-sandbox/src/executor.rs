use crate::policy::SandboxPolicy;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tracey_core::{TraceyError, TraceyResult};

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct SandboxedExecutor {
    policy: SandboxPolicy,
}

impl SandboxedExecutor {
    pub fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }

    pub async fn execute(
        &self,
        cmd: &str,
        cwd: &Path,
        timeout: Option<Duration>,
    ) -> TraceyResult<CommandOutput> {
        // Check blocked commands
        if self.policy.is_command_blocked(cmd) {
            return Err(TraceyError::PermissionDenied(format!(
                "command blocked by sandbox policy: {cmd}"
            )));
        }

        let timeout = timeout.unwrap_or(self.policy.max_execution_time);

        let result = tokio::time::timeout(timeout, async {
            let output = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(cwd)
                .output()
                .await
                .map_err(|e| TraceyError::Tool(format!("execute command: {e}")))?;

            Ok::<CommandOutput, TraceyError>(CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            })
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(TraceyError::Timeout(timeout)),
        }
    }
}
