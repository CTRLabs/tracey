//! Git integration — branch info, status, commit, diff.

use std::path::Path;
use std::process::Command;

/// Get current git branch name
pub fn git_branch(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get list of dirty/modified files
pub fn git_status(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).to_string();
        if s.trim().is_empty() { Some("clean".into()) } else { Some(s) }
    } else {
        None
    }
}

/// Get recent commit messages
pub fn git_recent_commits(cwd: &Path, n: usize) -> Option<String> {
    let output = Command::new("git")
        .args(["log", "--oneline", &format!("-{n}")])
        .current_dir(cwd)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Get unified diff of uncommitted changes
pub fn git_diff(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).to_string();
        if s.trim().is_empty() { Some("no changes".into()) } else { Some(s) }
    } else {
        None
    }
}

/// Get full unified diff
pub fn git_diff_full(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["diff"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Commit all changes with a message
pub fn git_commit(cwd: &Path, message: &str) -> Result<String, String> {
    // Stage all changes
    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git add failed: {e}"))?;
    if !add.status.success() {
        return Err(format!("git add: {}", String::from_utf8_lossy(&add.stderr)));
    }

    // Commit
    let commit = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git commit failed: {e}"))?;
    if commit.status.success() {
        Ok(String::from_utf8_lossy(&commit.stdout).trim().to_string())
    } else {
        Err(format!("git commit: {}", String::from_utf8_lossy(&commit.stderr)))
    }
}

/// Check if current directory is a git repo
pub fn is_git_repo(cwd: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(cwd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Build git context string for inclusion in system prompt
pub fn git_context(cwd: &Path) -> String {
    let mut ctx = String::new();
    if !is_git_repo(cwd) {
        return ctx;
    }
    if let Some(branch) = git_branch(cwd) {
        ctx.push_str(&format!("Git branch: {branch}\n"));
    }
    if let Some(status) = git_status(cwd) {
        ctx.push_str(&format!("Git status: {status}\n"));
    }
    if let Some(commits) = git_recent_commits(cwd, 5) {
        ctx.push_str(&format!("Recent commits:\n{commits}\n"));
    }
    ctx
}
