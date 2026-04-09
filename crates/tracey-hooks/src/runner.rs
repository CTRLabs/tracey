use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;
use tracey_core::TraceyResult;

/// Lifecycle events that can trigger hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreToolCall,
    PostToolCall,
    PreLlmCall,
    PostLlmCall,
    SessionStart,
    SessionEnd,
    OnError,
}

/// Hook configuration — maps events to commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub event: HookEvent,
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub matcher: Option<String>, // regex filter on tool name
}

fn default_timeout() -> u64 {
    10_000
}

/// Result of running a hook
#[derive(Debug, Clone)]
pub struct HookResult {
    pub action: HookAction,
    pub output: String,
}

/// What the hook decided
#[derive(Debug, Clone, PartialEq)]
pub enum HookAction {
    Continue,
    Abort(String),
    Modify(Value),
}

/// Executes hooks for lifecycle events
pub struct HookRunner {
    hooks: Vec<HookConfig>,
}

impl HookRunner {
    pub fn new(hooks: Vec<HookConfig>) -> Self {
        Self { hooks }
    }

    pub fn empty() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Run all hooks matching the given event
    /// Protocol: JSON on stdin, JSON on stdout
    /// Exit codes: 0 = continue, 1 = abort, 2 = modify
    pub async fn run_hooks(
        &self,
        event: &HookEvent,
        context: &Value,
    ) -> TraceyResult<HookResult> {
        let matching: Vec<&HookConfig> = self
            .hooks
            .iter()
            .filter(|h| h.event == *event)
            .collect();

        if matching.is_empty() {
            return Ok(HookResult {
                action: HookAction::Continue,
                output: String::new(),
            });
        }

        for hook in matching {
            let result = self.execute_hook(hook, context).await?;
            match &result.action {
                HookAction::Abort(_) => return Ok(result),
                HookAction::Modify(_) => return Ok(result),
                HookAction::Continue => continue,
            }
        }

        Ok(HookResult {
            action: HookAction::Continue,
            output: String::new(),
        })
    }

    async fn execute_hook(
        &self,
        hook: &HookConfig,
        context: &Value,
    ) -> TraceyResult<HookResult> {
        let context_json = serde_json::to_string(context)?;
        let timeout = Duration::from_millis(hook.timeout_ms);

        let result = tokio::time::timeout(timeout, async {
            let output = Command::new("sh")
                .arg("-c")
                .arg(&hook.command)
                .env("TRACEY_HOOK_CONTEXT", &context_json)
                .output()
                .await
                .map_err(|e| tracey_core::TraceyError::Tool(format!("hook failed: {e}")))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            let action = match exit_code {
                0 => HookAction::Continue,
                1 => {
                    let reason = if stdout.is_empty() {
                        "hook aborted".to_string()
                    } else {
                        stdout.trim().to_string()
                    };
                    HookAction::Abort(reason)
                }
                2 => {
                    let modified: Value = serde_json::from_str(&stdout)
                        .unwrap_or(Value::Null);
                    HookAction::Modify(modified)
                }
                _ => HookAction::Continue, // fail-safe: unknown exit code = continue
            };

            Ok::<HookResult, tracey_core::TraceyError>(HookResult {
                action,
                output: stdout,
            })
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => {
                // Timeout = auto-continue (fail-safe)
                tracing::warn!("Hook timed out after {}ms: {}", hook.timeout_ms, hook.command);
                Ok(HookResult {
                    action: HookAction::Continue,
                    output: String::new(),
                })
            }
        }
    }

    pub fn has_hooks_for(&self, event: &HookEvent) -> bool {
        self.hooks.iter().any(|h| h.event == *event)
    }
}
