use std::path::{Path, PathBuf};
use tracey_core::TraceyResult;

#[derive(Debug, Clone)]
pub struct InstructionFile {
    pub path: PathBuf,
    pub content: String,
    pub scope: InstructionScope,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionScope {
    Global,
    Project,
    Directory,
}

/// Load TRACEY.md instruction files by walking up from cwd to repo root
pub fn load_instructions(cwd: &Path) -> TraceyResult<Vec<InstructionFile>> {
    let mut files = Vec::new();

    // Global instruction file
    let global_path = super::config::config_dir().join("TRACEY.md");
    if global_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&global_path) {
            files.push(InstructionFile {
                path: global_path,
                content,
                scope: InstructionScope::Global,
            });
        }
    }

    // Walk up from cwd to find .git root, collecting TRACEY.md files
    let mut current = cwd.to_path_buf();
    let mut found_root = false;

    loop {
        // Check for TRACEY.md in current directory
        let tracey_md = current.join("TRACEY.md");
        if tracey_md.exists() {
            if let Ok(content) = std::fs::read_to_string(&tracey_md) {
                let scope = if current == cwd {
                    InstructionScope::Directory
                } else {
                    InstructionScope::Project
                };
                files.push(InstructionFile {
                    path: tracey_md,
                    content,
                    scope,
                });
            }
        }

        // Also check .tracey/TRACEY.md
        let dotdir = current.join(".tracey").join("TRACEY.md");
        if dotdir.exists() {
            if let Ok(content) = std::fs::read_to_string(&dotdir) {
                files.push(InstructionFile {
                    path: dotdir,
                    content,
                    scope: InstructionScope::Project,
                });
            }
        }

        // Stop at .git root
        if current.join(".git").exists() {
            found_root = true;
            break;
        }

        // Move up
        if !current.pop() {
            break;
        }
    }

    if !found_root {
        tracing::debug!("no .git root found walking up from {}", cwd.display());
    }

    // Reverse so global comes first, then project, then directory
    files.sort_by(|a, b| {
        let order = |s: &InstructionScope| match s {
            InstructionScope::Global => 0,
            InstructionScope::Project => 1,
            InstructionScope::Directory => 2,
        };
        order(&a.scope).cmp(&order(&b.scope))
    });

    Ok(files)
}

/// Find the git root by walking up from the given path
pub fn find_git_root(from: &Path) -> Option<PathBuf> {
    let mut current = from.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}
