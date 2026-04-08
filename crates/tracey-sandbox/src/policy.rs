use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub allowed_read_paths: Vec<PathBuf>,
    pub allowed_write_paths: Vec<PathBuf>,
    pub blocked_commands: Vec<String>,
    pub network_allowed: bool,
    pub max_execution_time: std::time::Duration,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            allowed_read_paths: vec![PathBuf::from("/")],
            allowed_write_paths: vec![PathBuf::from(".")],
            blocked_commands: vec![
                "rm -rf /".into(),
                "mkfs".into(),
                "dd if=/dev".into(),
            ],
            network_allowed: true,
            max_execution_time: std::time::Duration::from_secs(120),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    Deny,
    Ask,
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub tool_name: Option<String>,
    pub action_pattern: Option<String>,
    pub level: PermissionLevel,
}

pub struct PermissionResolver {
    rules: Vec<PermissionRule>,
}

impl PermissionResolver {
    pub fn new(rules: Vec<PermissionRule>) -> Self {
        Self { rules }
    }

    /// Resolve permission for a tool call. Deny > Ask > Allow.
    pub fn resolve(&self, tool_name: &str, action: &str) -> PermissionLevel {
        let mut result = PermissionLevel::Allow;

        for rule in &self.rules {
            let tool_match = rule.tool_name.as_ref().map_or(true, |t| t == tool_name);
            let action_match = rule.action_pattern.as_ref().map_or(true, |p| action.contains(p));

            if tool_match && action_match {
                match rule.level {
                    PermissionLevel::Deny => return PermissionLevel::Deny,
                    PermissionLevel::Ask => result = PermissionLevel::Ask,
                    PermissionLevel::Allow => {}
                }
            }
        }

        result
    }
}

impl SandboxPolicy {
    pub fn can_write(&self, path: &Path) -> bool {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.allowed_write_paths.iter().any(|allowed| {
            let allowed_canonical = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            canonical.starts_with(&allowed_canonical)
        })
    }

    pub fn is_command_blocked(&self, cmd: &str) -> bool {
        self.blocked_commands.iter().any(|blocked| cmd.contains(blocked))
    }
}
