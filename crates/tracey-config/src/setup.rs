use crate::config::{config_dir, ProviderEntry, TraceyConfig, Transport};
use crate::credentials::CredentialPool;
use std::io::{self, Write};
use tracey_core::TraceyResult;

pub struct SetupWizard;

impl SetupWizard {
    pub fn run() -> TraceyResult<()> {
        println!();
        println!("  \x1b[36m╔══════════════════════════════════════╗\x1b[0m");
        println!("  \x1b[36m║\x1b[0m    \x1b[1;37mTracey Setup Wizard\x1b[0m              \x1b[36m║\x1b[0m");
        println!("  \x1b[36m║\x1b[0m    \x1b[90mtracing causal connections\x1b[0m       \x1b[36m║\x1b[0m");
        println!("  \x1b[36m╚══════════════════════════════════════╝\x1b[0m");
        println!();

        let provider = Self::select_provider()?;
        let api_key = Self::get_api_key(&provider)?;
        let model = Self::select_model(&provider)?;

        let mut config = TraceyConfig::default();
        config.provider.default_provider = provider.name.clone();
        config.routing.default_model = model;

        let config_path = config_dir().join("config.toml");
        config.save(&config_path)?;

        let mut pool = CredentialPool::new();
        pool.add(&provider.name, api_key);
        pool.save()?;

        println!();
        println!("  \x1b[32m✓\x1b[0m Config saved to \x1b[90m{}\x1b[0m", config_path.display());
        println!("  \x1b[32m✓\x1b[0m Credentials saved");
        println!();
        println!("  Run \x1b[1mtracey\x1b[0m to start!");
        println!();

        Ok(())
    }

    fn select_provider() -> TraceyResult<ProviderEntry> {
        println!("  \x1b[1mSelect your LLM provider:\x1b[0m");
        println!();
        println!("    \x1b[36m1\x1b[0m) Anthropic (Claude)");
        println!("    \x1b[36m2\x1b[0m) OpenAI (GPT)");
        println!("    \x1b[36m3\x1b[0m) Google (Gemini)");
        println!("    \x1b[36m4\x1b[0m) Ollama (local)");
        println!("    \x1b[36m5\x1b[0m) OpenRouter");
        println!("    \x1b[36m6\x1b[0m) Custom (OpenAI-compatible)");
        println!();

        let choice = Self::prompt("  Choice [1]: ")?;
        let choice = if choice.is_empty() { "1".to_string() } else { choice };

        match choice.as_str() {
            "1" => Ok(ProviderEntry {
                name: "anthropic".into(),
                base_url: "https://api.anthropic.com".into(),
                api_key_env: "ANTHROPIC_API_KEY".into(),
                transport: Transport::AnthropicMessages,
            }),
            "2" => Ok(ProviderEntry {
                name: "openai".into(),
                base_url: "https://api.openai.com/v1".into(),
                api_key_env: "OPENAI_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "3" => Ok(ProviderEntry {
                name: "gemini".into(),
                base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
                api_key_env: "GEMINI_API_KEY".into(),
                transport: Transport::GeminiGenerate,
            }),
            "4" => Ok(ProviderEntry {
                name: "ollama".into(),
                base_url: "http://localhost:11434/v1".into(),
                api_key_env: "".into(),
                transport: Transport::OpenAiChat,
            }),
            "5" => Ok(ProviderEntry {
                name: "openrouter".into(),
                base_url: "https://openrouter.ai/api/v1".into(),
                api_key_env: "OPENROUTER_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "6" => {
                let url = Self::prompt("  Base URL: ")?;
                let env = Self::prompt("  API key env var: ")?;
                Ok(ProviderEntry {
                    name: "custom".into(),
                    base_url: url,
                    api_key_env: env,
                    transport: Transport::OpenAiChat,
                })
            }
            _ => Self::select_provider(),
        }
    }

    fn get_api_key(provider: &ProviderEntry) -> TraceyResult<String> {
        if provider.api_key_env.is_empty() {
            return Ok(String::new());
        }

        // Check env first
        if let Ok(key) = std::env::var(&provider.api_key_env) {
            if !key.is_empty() {
                println!("  \x1b[32m✓\x1b[0m Found {} in environment", provider.api_key_env);
                return Ok(key);
            }
        }

        println!();
        let key = Self::prompt(&format!("  {} API key: ", provider.name))?;
        if key.is_empty() {
            return Err(tracey_core::TraceyError::Config("API key required".into()));
        }
        Ok(key)
    }

    fn select_model(provider: &ProviderEntry) -> TraceyResult<String> {
        println!();
        let defaults = match provider.name.as_str() {
            "anthropic" => vec![
                ("1", "claude-sonnet-4-20250514", "Claude Sonnet 4 (recommended)"),
                ("2", "claude-opus-4-6-20250610", "Claude Opus 4.6"),
                ("3", "claude-haiku-4-5-20251001", "Claude Haiku 4.5 (fast)"),
            ],
            "openai" => vec![
                ("1", "gpt-4o", "GPT-4o (recommended)"),
                ("2", "gpt-4o-mini", "GPT-4o Mini (fast)"),
                ("3", "o3-mini", "o3 Mini (reasoning)"),
            ],
            "ollama" => vec![
                ("1", "qwen2.5-coder:32b", "Qwen 2.5 Coder 32B"),
                ("2", "deepseek-coder-v2:16b", "DeepSeek Coder V2"),
                ("3", "codellama:34b", "Code Llama 34B"),
            ],
            _ => vec![
                ("1", "auto", "Auto-detect"),
            ],
        };

        println!("  \x1b[1mSelect model:\x1b[0m");
        println!();
        for (num, _, desc) in &defaults {
            println!("    \x1b[36m{num}\x1b[0m) {desc}");
        }
        println!("    \x1b[36mc\x1b[0m) Custom model name");
        println!();

        let choice = Self::prompt("  Choice [1]: ")?;
        let choice = if choice.is_empty() { "1".to_string() } else { choice };

        if choice == "c" {
            Self::prompt("  Model name: ")
        } else {
            let idx: usize = choice.parse().unwrap_or(1);
            Ok(defaults.get(idx - 1).map_or("auto".into(), |(_, m, _)| m.to_string()))
        }
    }

    fn prompt(message: &str) -> TraceyResult<String> {
        print!("{message}");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }
}
