use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracey_core::TraceyResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceyConfig {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub routing: ModelRouting,
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub session: SessionConfig,
}

impl Default for TraceyConfig {
    fn default() -> Self {
        Self {
            provider: ProviderConfig::default(),
            routing: ModelRouting::default(),
            sandbox: SandboxConfig::default(),
            ui: UiConfig::default(),
            memory: MemoryConfig::default(),
            session: SessionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default = "default_provider")]
    pub default_provider: String,
    #[serde(default)]
    pub providers: Vec<ProviderEntry>,
}

fn default_provider() -> String {
    "anthropic".to_string()
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            default_provider: default_provider(),
            providers: vec![
                ProviderEntry {
                    name: "anthropic".into(),
                    base_url: "https://api.anthropic.com".into(),
                    api_key_env: "ANTHROPIC_API_KEY".into(),
                    transport: Transport::AnthropicMessages,
                },
                ProviderEntry {
                    name: "openai".into(),
                    base_url: "https://api.openai.com/v1".into(),
                    api_key_env: "OPENAI_API_KEY".into(),
                    transport: Transport::OpenAiChat,
                },
                ProviderEntry {
                    name: "ollama".into(),
                    base_url: "http://localhost:11434/v1".into(),
                    api_key_env: "".into(),
                    transport: Transport::OpenAiChat,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub name: String,
    pub base_url: String,
    pub api_key_env: String,
    #[serde(default)]
    pub transport: Transport,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    #[default]
    OpenAiChat,
    AnthropicMessages,
    GeminiGenerate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRouting {
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_cheap_model")]
    pub cheap_model: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

fn default_model() -> String { "claude-sonnet-4-20250514".to_string() }
fn default_cheap_model() -> String { "claude-haiku-4-5-20251001".to_string() }
fn default_embedding_model() -> String { "text-embedding-3-small".to_string() }

impl Default for ModelRouting {
    fn default() -> Self {
        Self {
            default_model: default_model(),
            cheap_model: default_cheap_model(),
            embedding_model: default_embedding_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub level: SandboxLevel,
    #[serde(default)]
    pub allowed_write_paths: Vec<String>,
    #[serde(default)]
    pub network_allowed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxLevel {
    Off,
    #[default]
    Permissive,
    Strict,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            level: SandboxLevel::Permissive,
            allowed_write_paths: vec![".".into()],
            network_allowed: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub show_logo: bool,
    #[serde(default = "default_true")]
    pub animations: bool,
    #[serde(default = "default_true")]
    pub syntax_highlighting: bool,
}

fn default_true() -> bool { true }

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_logo: true,
            animations: true,
            syntax_highlighting: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_nudge_interval")]
    pub nudge_interval_turns: u32,
}

fn default_nudge_interval() -> u32 { 10 }

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            nudge_interval_turns: default_nudge_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_compact_threshold")]
    pub compact_threshold_pct: u8,
    #[serde(default = "default_max_tokens")]
    pub max_context_tokens: u64,
}

fn default_compact_threshold() -> u8 { 85 }
fn default_max_tokens() -> u64 { 200_000 }

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            compact_threshold_pct: default_compact_threshold(),
            max_context_tokens: default_max_tokens(),
        }
    }
}

impl TraceyConfig {
    /// Load config with hierarchy: global -> project -> env overrides
    pub fn load(project_root: Option<&Path>) -> TraceyResult<Self> {
        let global_path = config_dir().join("config.toml");
        let mut config = if global_path.exists() {
            let content = std::fs::read_to_string(&global_path)
                .map_err(|e| tracey_core::TraceyError::Config(format!("read global config: {e}")))?;
            toml::from_str(&content)
                .map_err(|e| tracey_core::TraceyError::Config(format!("parse global config: {e}")))?
        } else {
            TraceyConfig::default()
        };

        if let Some(root) = project_root {
            let project_path = root.join(".tracey").join("config.toml");
            if project_path.exists() {
                let content = std::fs::read_to_string(&project_path)
                    .map_err(|e| tracey_core::TraceyError::Config(format!("read project config: {e}")))?;
                let project_config: TraceyConfig = toml::from_str(&content)
                    .map_err(|e| tracey_core::TraceyError::Config(format!("parse project config: {e}")))?;
                config.merge(project_config);
            }
        }

        config.apply_env_overrides();
        Ok(config)
    }

    fn merge(&mut self, other: TraceyConfig) {
        if other.provider.default_provider != default_provider() {
            self.provider.default_provider = other.provider.default_provider;
        }
        if other.routing.default_model != default_model() {
            self.routing.default_model = other.routing.default_model;
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(model) = std::env::var("TRACEY_MODEL") {
            self.routing.default_model = model;
        }
        if let Ok(provider) = std::env::var("TRACEY_PROVIDER") {
            self.provider.default_provider = provider;
        }
    }

    pub fn save(&self, path: &Path) -> TraceyResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| tracey_core::TraceyError::Config(format!("serialize config: {e}")))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("tracey")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("tracey")
}
