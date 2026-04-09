use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracey_core::TraceyResult;

/// A loaded skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub trigger: SkillTrigger,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// The markdown body (instructions for the LLM)
    #[serde(skip)]
    pub instructions: String,
    /// Path to the SKILL.md file
    #[serde(skip)]
    pub path: PathBuf,
}

/// How a skill is triggered
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillTrigger {
    /// Triggered by /command
    SlashCommand(String),
    /// Triggered by keyword pattern match
    Pattern(String),
    /// Always active (injected into system prompt)
    #[default]
    Always,
}

/// Loads skills from SKILL.md files
pub struct SkillLoader {
    skill_dirs: Vec<PathBuf>,
}

impl SkillLoader {
    pub fn new() -> Self {
        let mut dirs = Vec::new();

        // Project-local skills
        if let Ok(cwd) = std::env::current_dir() {
            let project_skills = cwd.join(".tracey").join("skills");
            if project_skills.exists() {
                dirs.push(project_skills);
            }
        }

        // User-global skills
        let global_skills = tracey_config::config::config_dir().join("skills");
        if global_skills.exists() {
            dirs.push(global_skills);
        }

        Self { skill_dirs: dirs }
    }

    pub fn with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self { skill_dirs: dirs }
    }

    /// Load all available skills
    pub fn load_all(&self) -> Vec<Skill> {
        let mut skills = Vec::new();

        for dir in &self.skill_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    // Look for SKILL.md files in subdirectories
                    let skill_file = if path.is_dir() {
                        path.join("SKILL.md")
                    } else if path.file_name().map_or(false, |n| n == "SKILL.md") {
                        path.clone()
                    } else {
                        continue;
                    };

                    if skill_file.exists() {
                        match self.load_skill(&skill_file) {
                            Ok(skill) => skills.push(skill),
                            Err(e) => {
                                tracing::warn!("Failed to load skill {}: {e}", skill_file.display());
                            }
                        }
                    }
                }
            }
        }

        skills
    }

    /// Load a single SKILL.md file
    fn load_skill(&self, path: &Path) -> TraceyResult<Skill> {
        let content = std::fs::read_to_string(path)?;
        let (frontmatter, body) = parse_frontmatter(&content);

        let mut skill: Skill = if let Some(fm) = frontmatter {
            serde_yaml_parse(&fm)?
        } else {
            Skill {
                name: path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string(),
                description: String::new(),
                trigger: SkillTrigger::default(),
                tools: Vec::new(),
                tags: Vec::new(),
                instructions: String::new(),
                path: PathBuf::new(),
            }
        };

        skill.instructions = body;
        skill.path = path.to_path_buf();

        Ok(skill)
    }

    /// Get skill summaries for deferred loading (name + description only)
    pub fn summaries(&self) -> Vec<(String, String)> {
        self.load_all()
            .iter()
            .map(|s| (s.name.clone(), s.description.clone()))
            .collect()
    }

    /// Find skills matching a query (for slash commands or keyword matching)
    pub fn find_matching(&self, query: &str) -> Vec<Skill> {
        self.load_all()
            .into_iter()
            .filter(|s| {
                match &s.trigger {
                    SkillTrigger::SlashCommand(cmd) => query.starts_with(&format!("/{cmd}")),
                    SkillTrigger::Pattern(pat) => query.to_lowercase().contains(&pat.to_lowercase()),
                    SkillTrigger::Always => true,
                }
            })
            .collect()
    }
}

/// Parse YAML frontmatter from a Markdown file
/// Format: --- YAML --- body
fn parse_frontmatter(content: &str) -> (Option<String>, String) {
    if !content.starts_with("---") {
        return (None, content.to_string());
    }

    let rest = &content[3..];
    if let Some(end_idx) = rest.find("\n---") {
        let yaml = rest[..end_idx].trim().to_string();
        let body = rest[end_idx + 4..].trim().to_string();
        (Some(yaml), body)
    } else {
        (None, content.to_string())
    }
}

/// Simple YAML-like parser for skill frontmatter
/// (Avoids pulling in the full serde_yaml crate)
fn serde_yaml_parse(yaml: &str) -> TraceyResult<Skill> {
    let mut name = String::new();
    let mut description = String::new();
    let mut trigger = SkillTrigger::default();
    let mut tools = Vec::new();
    let mut tags = Vec::new();

    for line in yaml.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "name" => name = value.to_string(),
                "description" => description = value.to_string(),
                "trigger" => {
                    if value.starts_with('/') {
                        trigger = SkillTrigger::SlashCommand(value.trim_start_matches('/').to_string());
                    } else {
                        trigger = SkillTrigger::Pattern(value.to_string());
                    }
                }
                "tools" => {
                    // Simple array parsing: [Tool1, Tool2]
                    let cleaned = value.trim_matches(['[', ']']);
                    tools = cleaned.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
                "tags" => {
                    let cleaned = value.trim_matches(['[', ']']);
                    tags = cleaned.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
                _ => {}
            }
        }
    }

    Ok(Skill {
        name,
        description,
        trigger,
        tools,
        tags,
        instructions: String::new(),
        path: PathBuf::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
name: test-skill
description: A test skill
trigger: /test
tools: [Bash, Read]
---

# Test Skill

Run the tests with `cargo test`.
"#;

        let (fm, body) = parse_frontmatter(content);
        assert!(fm.is_some());
        assert!(body.contains("Run the tests"));

        let skill = serde_yaml_parse(&fm.unwrap()).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test skill");
        assert_eq!(skill.tools, vec!["Bash", "Read"]);
        assert!(matches!(skill.trigger, SkillTrigger::SlashCommand(ref s) if s == "test"));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "# Just Markdown\n\nNo frontmatter here.";
        let (fm, body) = parse_frontmatter(content);
        assert!(fm.is_none());
        assert!(body.contains("Just Markdown"));
    }
}
