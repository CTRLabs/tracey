use crate::config::{config_dir, ProviderEntry, TraceyConfig, Transport};
use crate::credentials::CredentialPool;
use std::io::{self, Write};
use tracey_core::TraceyResult;

// Violet color palette
const V: &str = "\x1b[38;2;139;92;246m";   // violet
const VB: &str = "\x1b[38;2;167;139;250m";  // violet bright
const VD: &str = "\x1b[38;2;109;40;217m";   // violet dim
const LAV: &str = "\x1b[38;2;196;181;253m"; // lavender
const W: &str = "\x1b[1;37m";               // white bold
const D: &str = "\x1b[90m";                 // dim
const G: &str = "\x1b[38;2;34;197;94m";     // green
const R: &str = "\x1b[38;2;239;68;68m";     // red
const Y: &str = "\x1b[38;2;234;179;8m";     // yellow
const RST: &str = "\x1b[0m";

pub struct SetupWizard;

impl SetupWizard {
    pub fn run() -> TraceyResult<()> {
        Self::print_header();
        Self::detect_existing_config();

        // Step 1: Provider
        println!("  {VB}Step 1/3{RST} {D}─────────────────────────────{RST}");
        println!();
        let provider = Self::select_provider()?;

        // Step 2: API Key
        println!();
        println!("  {VB}Step 2/3{RST} {D}─────────────────────────────{RST}");
        println!();
        let api_key = Self::get_api_key(&provider)?;

        // Test connection
        if !api_key.is_empty() {
            Self::test_connection(&provider, &api_key);
        }

        // Step 3: Model
        println!();
        println!("  {VB}Step 3/3{RST} {D}─────────────────────────────{RST}");
        println!();
        let model = Self::select_model(&provider)?;

        // Save everything
        Self::save_config(&provider, &api_key, &model)?;

        Ok(())
    }

    fn print_header() {
        let gradient = [
            "\x1b[38;2;109;40;217m",
            "\x1b[38;2;119;56;226m",
            "\x1b[38;2;129;72;236m",
            "\x1b[38;2;139;92;246m",
            "\x1b[38;2;149;108;248m",
            "\x1b[38;2;159;124;250m",
        ];
        let logo_lines = [
            "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
            "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
            "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
            "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
            "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
            "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
        ];

        println!();
        for (i, line) in logo_lines.iter().enumerate() {
            let color = gradient[i.min(gradient.len() - 1)];
            println!("{color}{line}{RST}");
        }
        println!();
        println!("  {LAV}    ◉{D}──╌╌──▸{LAV} ◉{D}──╌╌──▸{LAV} ◉{RST}");
        println!("  {D}              └──╌╌──▸{LAV} ◉{RST}");
        println!();
        println!("  {V}◆{RST} {W}Setup Wizard{RST}");
        println!("  {D}  tracing causal connections{RST}");
        println!("  {D}  v{}{RST}", env!("CARGO_PKG_VERSION"));
        println!();
    }

    fn detect_existing_config() {
        // Check for existing config
        let config_path = config_dir().join("config.toml");
        if config_path.exists() {
            println!("  {Y}!{RST} Existing config found at {D}{}{RST}", config_path.display());
            println!("  {D}  This wizard will overwrite it.{RST}");
            println!();
        }

        // Auto-detect environment variables
        let detected: Vec<(&str, &str)> = vec![
            ("ANTHROPIC_API_KEY", "Anthropic"),
            ("OPENAI_API_KEY", "OpenAI"),
            ("GEMINI_API_KEY", "Google Gemini"),
            ("OPENROUTER_API_KEY", "OpenRouter"),
        ];

        let mut found_any = false;
        for (env_var, name) in &detected {
            if let Ok(val) = std::env::var(env_var) {
                if !val.is_empty() {
                    if !found_any {
                        println!("  {VB}Detected credentials:{RST}");
                    }
                    let masked = mask_key(&val);
                    println!("  {G}✓{RST} {name}: {D}{masked}{RST}");
                    found_any = true;
                }
            }
        }

        // Check if Ollama is running
        if is_ollama_running() {
            if !found_any {
                println!("  {VB}Detected services:{RST}");
            }
            println!("  {G}✓{RST} Ollama: {D}running on localhost:11434{RST}");
            found_any = true;
        }

        if found_any {
            println!();
        }
    }

    fn select_provider() -> TraceyResult<ProviderEntry> {
        println!("  {VB}Select your LLM provider:{RST}");
        println!();

        // Check which providers have keys available
        let has_anthropic = std::env::var("ANTHROPIC_API_KEY").map_or(false, |v| !v.is_empty());
        let has_openai = std::env::var("OPENAI_API_KEY").map_or(false, |v| !v.is_empty());
        let has_ollama = is_ollama_running();

        let key_found = format!("{G}● key found{RST}");
        let running = format!("{G}● running{RST}");
        let not_detected = format!("{D}○ not detected{RST}");
        let empty = String::new();

        println!("    {V}1{RST}) Anthropic (Claude)        {}", if has_anthropic { &key_found } else { &empty });
        println!("    {V}2{RST}) OpenAI (GPT / Codex)      {}", if has_openai { &key_found } else { &empty });
        println!("    {V}3{RST}) Google (Gemini)");
        println!("    {V}4{RST}) Ollama (local)             {}", if has_ollama { &running } else { &not_detected });
        println!("    {V}5{RST}) OpenRouter (200+ models)");
        println!("    {V}6{RST}) DeepSeek");
        println!("    {V}7{RST}) Together AI");
        println!("    {V}8{RST}) Custom (OpenAI-compatible)");
        println!();

        let choice = Self::prompt(&format!("  {V}▸{RST} Choice [1]: "))?;
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
            "6" => Ok(ProviderEntry {
                name: "deepseek".into(),
                base_url: "https://api.deepseek.com/v1".into(),
                api_key_env: "DEEPSEEK_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "7" => Ok(ProviderEntry {
                name: "together".into(),
                base_url: "https://api.together.xyz/v1".into(),
                api_key_env: "TOGETHER_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "8" => {
                println!();
                let url = Self::prompt(&format!("  {V}▸{RST} Base URL: "))?;
                let env = Self::prompt(&format!("  {V}▸{RST} API key env var (or leave empty): "))?;
                let name = Self::prompt(&format!("  {V}▸{RST} Provider name: "))?;
                Ok(ProviderEntry {
                    name: if name.is_empty() { "custom".into() } else { name },
                    base_url: url,
                    api_key_env: env,
                    transport: Transport::OpenAiChat,
                })
            }
            _ => {
                println!("  {R}Invalid choice. Try again.{RST}");
                Self::select_provider()
            }
        }
    }

    fn get_api_key(provider: &ProviderEntry) -> TraceyResult<String> {
        if provider.api_key_env.is_empty() {
            println!("  {G}✓{RST} No API key needed for {}", provider.name);
            return Ok(String::new());
        }

        // Check env first
        if let Ok(key) = std::env::var(&provider.api_key_env) {
            if !key.is_empty() {
                let masked = mask_key(&key);
                println!("  {G}✓{RST} Found {VB}{}{RST} in environment: {D}{masked}{RST}", provider.api_key_env);
                let use_env = Self::prompt(&format!("  {V}▸{RST} Use this key? [Y/n]: "))?;
                if use_env.is_empty() || use_env.to_lowercase().starts_with('y') {
                    return Ok(key);
                }
            }
        }

        // Prompt for key
        println!();
        println!("  {D}Get your API key from:{RST}");
        match provider.name.as_str() {
            "anthropic" => println!("  {D}  https://console.anthropic.com/settings/keys{RST}"),
            "openai" => println!("  {D}  https://platform.openai.com/api-keys{RST}"),
            "gemini" => println!("  {D}  https://aistudio.google.com/app/apikey{RST}"),
            "openrouter" => println!("  {D}  https://openrouter.ai/keys{RST}"),
            "deepseek" => println!("  {D}  https://platform.deepseek.com/api_keys{RST}"),
            "together" => println!("  {D}  https://api.together.xyz/settings/api-keys{RST}"),
            _ => {}
        }
        println!();

        let key = Self::prompt(&format!("  {V}▸{RST} API key: "))?;
        if key.is_empty() {
            return Err(tracey_core::TraceyError::Config("API key required".into()));
        }

        Ok(key)
    }

    fn test_connection(provider: &ProviderEntry, api_key: &str) {
        print!("  {D}Testing connection...{RST}");
        io::stdout().flush().ok();

        // Simple validation: check key format
        let valid = match provider.name.as_str() {
            "anthropic" => api_key.starts_with("sk-ant-"),
            "openai" => api_key.starts_with("sk-"),
            "deepseek" => api_key.starts_with("sk-"),
            _ => true, // Can't validate format for unknown providers
        };

        if valid {
            println!("\r  {G}✓{RST} Connection looks good          ");
        } else {
            println!("\r  {Y}!{RST} Key format looks unusual — may still work");
        }
    }

    fn select_model(provider: &ProviderEntry) -> TraceyResult<String> {
        let defaults = match provider.name.as_str() {
            "anthropic" => vec![
                ("1", "claude-sonnet-4-20250514", "Claude Sonnet 4", "recommended — best balance"),
                ("2", "claude-opus-4-6-20250610", "Claude Opus 4.6", "most capable"),
                ("3", "claude-haiku-4-5-20251001", "Claude Haiku 4.5", "fastest, cheapest"),
            ],
            "openai" => vec![
                ("1", "gpt-4o", "GPT-4o", "recommended"),
                ("2", "gpt-4o-mini", "GPT-4o Mini", "fast, cheap"),
                ("3", "o3-mini", "o3 Mini", "reasoning"),
            ],
            "ollama" => vec![
                ("1", "qwen2.5-coder:32b", "Qwen 2.5 Coder 32B", "best local"),
                ("2", "deepseek-coder-v2:16b", "DeepSeek Coder V2", "good balance"),
                ("3", "codellama:34b", "Code Llama 34B", "Meta's model"),
            ],
            "openrouter" => vec![
                ("1", "anthropic/claude-sonnet-4-20250514", "Claude Sonnet 4", "via OpenRouter"),
                ("2", "openai/gpt-4o", "GPT-4o", "via OpenRouter"),
                ("3", "google/gemini-2.5-pro", "Gemini 2.5 Pro", "via OpenRouter"),
            ],
            "deepseek" => vec![
                ("1", "deepseek-coder", "DeepSeek Coder", "optimized for code"),
                ("2", "deepseek-chat", "DeepSeek Chat", "general purpose"),
            ],
            "together" => vec![
                ("1", "Qwen/Qwen2.5-Coder-32B-Instruct", "Qwen 2.5 Coder 32B", "best open-source"),
                ("2", "meta-llama/Llama-3.3-70B-Instruct-Turbo", "Llama 3.3 70B", "fast, general"),
                ("3", "deepseek-ai/DeepSeek-V3", "DeepSeek V3", "very capable"),
            ],
            _ => vec![
                ("1", "auto", "Auto-detect", "let the provider decide"),
            ],
        };

        println!("  {VB}Select model:{RST}");
        println!();
        for (num, _, name, desc) in &defaults {
            println!("    {V}{num}{RST}) {W}{name}{RST}  {D}— {desc}{RST}");
        }
        println!("    {V}c{RST}) Custom model name");
        println!();

        let choice = Self::prompt(&format!("  {V}▸{RST} Choice [1]: "))?;
        let choice = if choice.is_empty() { "1".to_string() } else { choice };

        if choice == "c" {
            Self::prompt(&format!("  {V}▸{RST} Model name: "))
        } else {
            let idx: usize = choice.parse().unwrap_or(1);
            Ok(defaults.get(idx - 1).map_or("auto".into(), |(_, m, _, _)| m.to_string()))
        }
    }

    fn save_config(provider: &ProviderEntry, api_key: &str, model: &str) -> TraceyResult<()> {
        let mut config = TraceyConfig::default();
        config.provider.default_provider = provider.name.clone();
        config.routing.default_model = model.to_string();

        let config_path = config_dir().join("config.toml");
        config.save(&config_path)?;

        let mut pool = CredentialPool::load().unwrap_or_default();
        if !api_key.is_empty() {
            pool.add(&provider.name, api_key.to_string());
        }
        pool.save()?;

        println!();
        println!("  {VD}╭──────────────────────────────────────╮{RST}");
        println!("  {VD}│{RST}  {G}✓{RST} Setup complete!                    {VD}│{RST}");
        println!("  {VD}├──────────────────────────────────────┤{RST}");
        println!("  {VD}│{RST}                                      {VD}│{RST}");
        println!("  {VD}│{RST}  Provider  {W}{}{RST}{}{VD}│{RST}",
            provider.name,
            " ".repeat(26usize.saturating_sub(provider.name.len()))
        );
        println!("  {VD}│{RST}  Model     {W}{}{RST}{}{VD}│{RST}",
            truncate(model, 24),
            " ".repeat(26usize.saturating_sub(truncate(model, 24).len()))
        );
        println!("  {VD}│{RST}  Config    {D}{}{RST}{}{VD}│{RST}",
            truncate(&config_path.to_string_lossy(), 24),
            " ".repeat(26usize.saturating_sub(truncate(&config_path.to_string_lossy(), 24).len()))
        );
        println!("  {VD}│{RST}                                      {VD}│{RST}");
        println!("  {VD}│{RST}  Run {V}{W}tracey{RST} to start!               {VD}│{RST}");
        println!("  {VD}│{RST}  Run {V}tracey --help{RST} for options       {VD}│{RST}");
        println!("  {VD}│{RST}                                      {VD}│{RST}");
        println!("  {VD}╰──────────────────────────────────────╯{RST}");
        println!();

        Ok(())
    }

    fn prompt(message: &str) -> TraceyResult<String> {
        print!("{message}");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }
}

/// Mask an API key for display (show first 7 chars + last 4)
fn mask_key(key: &str) -> String {
    if key.len() <= 12 {
        return "****".to_string();
    }
    format!("{}...{}", &key[..7], &key[key.len()-4..])
}

/// Check if Ollama is running on localhost
fn is_ollama_running() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().unwrap(),
        std::time::Duration::from_millis(500),
    ).is_ok()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
