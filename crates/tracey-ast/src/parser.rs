use std::path::Path;
use tracey_graph::{EdgeKind, NodeKind};

/// Supported languages for AST parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    C,
    Cpp,
    Ruby,
    CSharp,
}

/// Detect language from file extension
pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some(Language::Rust),
        "py" | "pyw" => Some(Language::Python),
        "ts" | "tsx" => Some(Language::TypeScript),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "c" | "h" => Some(Language::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => Some(Language::Cpp),
        "rb" => Some(Language::Ruby),
        "cs" => Some(Language::CSharp),
        _ => None,
    }
}

/// An entity extracted from source code
#[derive(Debug, Clone)]
pub struct CodeEntity {
    pub kind: NodeKind,
    pub name: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
}

/// A relationship between code entities
#[derive(Debug, Clone)]
pub struct CodeRelation {
    pub from_label: String,
    pub to_label: String,
    pub kind: EdgeKind,
}

/// Result of parsing a source file
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub entities: Vec<CodeEntity>,
    pub relations: Vec<CodeRelation>,
}

/// Extract entities and relations from a source file using regex-based parsing.
/// This is a lightweight approach that works across all languages without
/// tree-sitter grammar dependencies. It catches ~80% of structural relationships.
pub fn parse_file(path: &Path, content: &str, lang: Language) -> ParseResult {
    let file_path = path.to_string_lossy().to_string();
    let mut entities = Vec::new();
    let mut relations = Vec::new();

    // Always create a File entity
    entities.push(CodeEntity {
        kind: NodeKind::File,
        name: file_path.clone(),
        file_path: file_path.clone(),
        line_start: 0,
        line_end: content.lines().count(),
    });

    match lang {
        Language::Rust => extract_rust(content, &file_path, &mut entities, &mut relations),
        Language::Python => extract_python(content, &file_path, &mut entities, &mut relations),
        Language::TypeScript | Language::JavaScript => {
            extract_typescript(content, &file_path, &mut entities, &mut relations)
        }
        Language::Go => extract_go(content, &file_path, &mut entities, &mut relations),
        _ => extract_generic(content, &file_path, &mut entities, &mut relations),
    }

    ParseResult { entities, relations }
}

fn extract_rust(
    content: &str,
    file_path: &str,
    entities: &mut Vec<CodeEntity>,
    relations: &mut Vec<CodeRelation>,
) {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Functions: pub fn name, fn name, pub async fn name
        if let Some(name) = extract_after_keyword(trimmed, "fn ") {
            let is_test = content.lines().nth(i.saturating_sub(1))
                .map_or(false, |prev| prev.contains("#[test]") || prev.contains("#[tokio::test]"));
            let kind = if is_test { NodeKind::Test } else { NodeKind::Function };
            entities.push(CodeEntity {
                kind,
                name: name.clone(),
                file_path: file_path.to_string(),
                line_start: i + 1,
                line_end: i + 1,
            });
            relations.push(CodeRelation {
                from_label: file_path.to_string(),
                to_label: name,
                kind: EdgeKind::DependsOn,
            });
        }

        // Structs and enums
        if let Some(name) = extract_after_keyword(trimmed, "struct ") {
            entities.push(CodeEntity {
                kind: NodeKind::Class,
                name: name.clone(),
                file_path: file_path.to_string(),
                line_start: i + 1,
                line_end: i + 1,
            });
        }

        // Imports: use crate::..., use super::...
        if trimmed.starts_with("use ") {
            let import_path = trimmed.trim_start_matches("use ").trim_end_matches(';').to_string();
            relations.push(CodeRelation {
                from_label: file_path.to_string(),
                to_label: import_path,
                kind: EdgeKind::Imports,
            });
        }

        // Modules: mod name;
        if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
            let mod_name = trimmed.trim_start_matches("mod ").trim_end_matches(';').trim();
            entities.push(CodeEntity {
                kind: NodeKind::Module,
                name: mod_name.to_string(),
                file_path: file_path.to_string(),
                line_start: i + 1,
                line_end: i + 1,
            });
        }
    }
}

fn extract_python(
    content: &str,
    file_path: &str,
    entities: &mut Vec<CodeEntity>,
    relations: &mut Vec<CodeRelation>,
) {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Functions and methods
        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            let after_def = if trimmed.starts_with("async ") {
                &trimmed[10..]
            } else {
                &trimmed[4..]
            };
            if let Some(name) = after_def.split('(').next() {
                let is_test = name.starts_with("test_");
                let kind = if is_test { NodeKind::Test } else { NodeKind::Function };
                entities.push(CodeEntity {
                    kind,
                    name: name.to_string(),
                    file_path: file_path.to_string(),
                    line_start: i + 1,
                    line_end: i + 1,
                });
            }
        }

        // Classes
        if trimmed.starts_with("class ") {
            if let Some(name) = trimmed[6..].split(['(', ':']).next() {
                entities.push(CodeEntity {
                    kind: NodeKind::Class,
                    name: name.trim().to_string(),
                    file_path: file_path.to_string(),
                    line_start: i + 1,
                    line_end: i + 1,
                });
            }
        }

        // Imports
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            let module = if trimmed.starts_with("from ") {
                trimmed.split_whitespace().nth(1).unwrap_or("")
            } else {
                trimmed.split_whitespace().nth(1).unwrap_or("")
            };
            if !module.is_empty() {
                relations.push(CodeRelation {
                    from_label: file_path.to_string(),
                    to_label: module.to_string(),
                    kind: EdgeKind::Imports,
                });
            }
        }
    }
}

fn extract_typescript(
    content: &str,
    file_path: &str,
    entities: &mut Vec<CodeEntity>,
    relations: &mut Vec<CodeRelation>,
) {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Functions: function name, export function name, const name = () =>
        if let Some(name) = extract_after_keyword(trimmed, "function ") {
            entities.push(CodeEntity {
                kind: NodeKind::Function,
                name: name.clone(),
                file_path: file_path.to_string(),
                line_start: i + 1,
                line_end: i + 1,
            });
        }

        // Classes
        if let Some(name) = extract_after_keyword(trimmed, "class ") {
            entities.push(CodeEntity {
                kind: NodeKind::Class,
                name,
                file_path: file_path.to_string(),
                line_start: i + 1,
                line_end: i + 1,
            });
        }

        // Imports
        if trimmed.starts_with("import ") {
            if let Some(from_idx) = trimmed.find("from ") {
                let module = trimmed[from_idx + 5..].trim_matches(|c: char| c == '\'' || c == '"' || c == ';');
                relations.push(CodeRelation {
                    from_label: file_path.to_string(),
                    to_label: module.to_string(),
                    kind: EdgeKind::Imports,
                });
            }
        }
    }
}

fn extract_go(
    content: &str,
    file_path: &str,
    entities: &mut Vec<CodeEntity>,
    relations: &mut Vec<CodeRelation>,
) {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("func ") {
            let rest = &trimmed[5..];
            // Skip receiver (func (r *Receiver) Name)
            let name_part = if rest.starts_with('(') {
                rest.split(')').nth(1).unwrap_or(rest).trim()
            } else {
                rest
            };
            if let Some(name) = name_part.split('(').next() {
                let name = name.trim();
                if !name.is_empty() {
                    let is_test = name.starts_with("Test");
                    let kind = if is_test { NodeKind::Test } else { NodeKind::Function };
                    entities.push(CodeEntity {
                        kind,
                        name: name.to_string(),
                        file_path: file_path.to_string(),
                        line_start: i + 1,
                        line_end: i + 1,
                    });
                }
            }
        }

        if trimmed.starts_with("type ") && trimmed.contains("struct") {
            if let Some(name) = trimmed[5..].split_whitespace().next() {
                entities.push(CodeEntity {
                    kind: NodeKind::Class,
                    name: name.to_string(),
                    file_path: file_path.to_string(),
                    line_start: i + 1,
                    line_end: i + 1,
                });
            }
        }
    }
}

fn extract_generic(
    content: &str,
    file_path: &str,
    entities: &mut Vec<CodeEntity>,
    _relations: &mut Vec<CodeRelation>,
) {
    // Fallback: just count functions/classes by common patterns
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = extract_after_keyword(trimmed, "function ") {
            entities.push(CodeEntity {
                kind: NodeKind::Function,
                name,
                file_path: file_path.to_string(),
                line_start: i + 1,
                line_end: i + 1,
            });
        }
        if let Some(name) = extract_after_keyword(trimmed, "class ") {
            entities.push(CodeEntity {
                kind: NodeKind::Class,
                name,
                file_path: file_path.to_string(),
                line_start: i + 1,
                line_end: i + 1,
            });
        }
    }
}

/// Extract the identifier after a keyword (e.g., "fn " -> function name)
fn extract_after_keyword(line: &str, keyword: &str) -> Option<String> {
    // Handle visibility modifiers
    let effective = line
        .trim_start_matches("pub ")
        .trim_start_matches("pub(crate) ")
        .trim_start_matches("export ")
        .trim_start_matches("async ")
        .trim_start_matches("default ")
        .trim_start_matches("override ")
        .trim_start_matches("static ");

    if effective.starts_with(keyword) {
        let rest = &effective[keyword.len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            Some(name)
        } else {
            None
        }
    } else {
        None
    }
}
