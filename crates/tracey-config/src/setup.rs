use crate::config::{config_dir, ProviderEntry, TraceyConfig, Transport};
use crate::credentials::CredentialPool;
use std::io::{self, Write};
use tracey_core::TraceyResult;

// Color palette using ANSI 256-color (works everywhere including tmux)
const V: &str = "\x1b[38;5;135m";    // violet
const VB: &str = "\x1b[38;5;141m";   // violet bright
const VD: &str = "\x1b[38;5;97m";    // violet dim
const LAV: &str = "\x1b[38;5;183m";  // lavender
const W: &str = "\x1b[1;37m";        // white bold
const D: &str = "\x1b[90m";          // dim
const G: &str = "\x1b[38;5;35m";     // green
const R: &str = "\x1b[38;5;196m";    // red
const Y: &str = "\x1b[38;5;220m";    // yellow
const RST: &str = "\x1b[0m";

pub struct SetupWizard;

impl SetupWizard {
    pub fn run() -> TraceyResult<()> {
        Self::print_header();
        Self::detect_existing_config();

        // Quick import: if Claude Code or Codex credentials exist, offer one-click setup
        if let Some(quick) = Self::try_quick_import()? {
            return Ok(());
        }

        // Step 1: Provider
        crate::interactive::print_section_header(1, 3, "Provider");
        let provider = Self::select_provider()?;
        crate::interactive::animate_step("Provider selected", 1);

        // Step 2: API Key
        crate::interactive::print_section_header(2, 3, "Authentication");
        let api_key = Self::get_api_key(&provider)?;
        if !api_key.is_empty() {
            Self::test_connection(&provider, &api_key);
        }
        crate::interactive::animate_step("Authenticated", 2);

        // Step 3: Model
        crate::interactive::print_section_header(3, 3, "Model");
        let model = Self::select_model(&provider)?;
        crate::interactive::animate_step("Model configured", 3);

        // Save everything
        Self::save_config(&provider, &api_key, &model)?;

        Ok(())
    }

    fn print_header() {
        // Liquid chrome: silver at top → violet at bottom
        let colors = [
            "\x1b[38;5;252m",  // bright silver
            "\x1b[38;5;251m",  // silver
            "\x1b[38;5;189m",  // silver-lavender
            "\x1b[38;5;183m",  // lavender
            "\x1b[38;5;141m",  // light violet
            "\x1b[38;5;135m",  // violet
        ];

        let logo_lines = [
            "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
            "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
            "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
            "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
            "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
            "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
        ];

        // Logo with silver→violet gradient
        println!();
        for (i, line) in logo_lines.iter().enumerate() {
            println!("{}{line}{RST}", colors[i]);
        }
        println!();

        // Braille causal graph emblem (compact version, same technique as Hermes caduceus)
        let emblem = [
            ("⠀⠀⠀⣀⣤⣶⣿⣿⣶⣤⣀⠀⠀⠀", "\x1b[38;5;252m"),
            ("⠀⣠⣾⡿⠋⣿⡟⢿⣿⠙⢿⣷⣄⠀", "\x1b[38;5;189m"),
            ("⣼⣿⠃⢀⣾⠟⠀⠀⠻⣷⡀⠘⣿⣧", "\x1b[38;5;183m"),
            ("⠸⣿⣧⡈⢿⣷⣶⣶⣾⡿⢁⣼⣿⠇", "\x1b[38;5;141m"),
            ("⠀⠀⠉⠛⠿⣶⣤⣤⣶⠿⠛⠉⠀⠀", "\x1b[38;5;135m"),
        ];
        for (art, color) in &emblem {
            println!("  {color}{art}{RST}");
        }
        println!();

        // Title
        println!("  {VB}◆{W} Setup Wizard{RST}");
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

        // Auto-detect environment variables (expanded — from Hermes Agent patterns)
        let detected: Vec<(&str, &str)> = vec![
            ("ANTHROPIC_API_KEY", "Anthropic"),
            ("OPENAI_API_KEY", "OpenAI"),
            ("GEMINI_API_KEY", "Google Gemini"),
            ("GOOGLE_API_KEY", "Google Gemini"),
            ("OPENROUTER_API_KEY", "OpenRouter"),
            ("DEEPSEEK_API_KEY", "DeepSeek"),
            ("TOGETHER_API_KEY", "Together AI"),
            ("GROQ_API_KEY", "Groq"),
            ("XAI_API_KEY", "xAI / Grok"),
            ("FIREWORKS_API_KEY", "Fireworks AI"),
            ("NOUS_API_KEY", "Nous Research"),
            ("COPILOT_GITHUB_TOKEN", "GitHub Copilot"),
            ("MOONSHOT_API_KEY", "Kimi / Moonshot"),
            ("DASHSCOPE_API_KEY", "Alibaba DashScope"),
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

        // Check for Claude Code credentials
        let claude_creds = dirs::home_dir()
            .map(|h| h.join(".claude").join(".credentials.json"))
            .filter(|p| p.exists());
        if claude_creds.is_some() {
            if !found_any {
                println!("  {VB}Detected credentials:{RST}");
            }
            println!("  {G}✓{RST} Claude Code: {D}credentials found (~/.claude/){RST}");
            found_any = true;
        }

        // Check for Codex credentials
        let codex_creds = dirs::home_dir()
            .map(|h| h.join(".codex").join("auth.json"))
            .filter(|p| p.exists());
        if codex_creds.is_some() {
            if !found_any {
                println!("  {VB}Detected credentials:{RST}");
            }
            println!("  {G}✓{RST} Codex CLI: {D}credentials found (~/.codex/){RST}");
            found_any = true;
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
        use crate::interactive::{select_menu, MenuItem};

        let has_anthropic = std::env::var("ANTHROPIC_API_KEY").map_or(false, |v| !v.is_empty());
        let has_openai = std::env::var("OPENAI_API_KEY").map_or(false, |v| !v.is_empty());
        let has_ollama = is_ollama_running();
        let has_copilot = std::env::var("COPILOT_GITHUB_TOKEN").is_ok();

        let items = vec![
            MenuItem::new("Anthropic (Claude)")
                .with_indicator(if has_anthropic { "● key found" } else { "" }),
            MenuItem::new("OpenAI (GPT / Codex)")
                .with_indicator(if has_openai { "● key found" } else { "" }),
            MenuItem::new("Google (Gemini)"),
            MenuItem::new("Ollama (local)")
                .with_indicator(if has_ollama { "● running" } else { "○ not detected" }),
            MenuItem::new("OpenRouter (200+ models)"),
            MenuItem::new("DeepSeek"),
            MenuItem::new("Together AI"),
            MenuItem::new("Groq (fast inference)"),
            MenuItem::new("xAI / Grok"),
            MenuItem::new("Fireworks AI"),
            MenuItem::new("Kimi / Moonshot"),
            MenuItem::new("Alibaba DashScope"),
            MenuItem::new("GitHub Copilot")
                .with_indicator(if has_copilot { "● token found" } else { "" }),
            MenuItem::new("Custom (OpenAI-compatible)"),
        ];

        // Add blank lines for the menu to render into
        for _ in 0..items.len() + 5 {
            println!();
        }

        let choice = select_menu("Select your LLM provider:", &items)
            .ok_or_else(|| tracey_core::TraceyError::Config("selection cancelled".into()))?;

        let choice_str = (choice + 1).to_string();

        match choice_str.as_str() {
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
            "8" => Ok(ProviderEntry {
                name: "groq".into(),
                base_url: "https://api.groq.com/openai/v1".into(),
                api_key_env: "GROQ_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "9" => Ok(ProviderEntry {
                name: "xai".into(),
                base_url: "https://api.x.ai/v1".into(),
                api_key_env: "XAI_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "10" => Ok(ProviderEntry {
                name: "fireworks".into(),
                base_url: "https://api.fireworks.ai/inference/v1".into(),
                api_key_env: "FIREWORKS_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "11" => Ok(ProviderEntry {
                name: "moonshot".into(),
                base_url: "https://api.moonshot.ai/v1".into(),
                api_key_env: "MOONSHOT_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "12" => Ok(ProviderEntry {
                name: "dashscope".into(),
                base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1".into(),
                api_key_env: "DASHSCOPE_API_KEY".into(),
                transport: Transport::OpenAiChat,
            }),
            "13" => Ok(ProviderEntry {
                name: "copilot".into(),
                base_url: "https://api.githubcopilot.com".into(),
                api_key_env: "COPILOT_GITHUB_TOKEN".into(),
                transport: Transport::OpenAiChat,
            }),
            "14" => {
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

        // Offer credential import from Claude Code / Codex
        if provider.name == "anthropic" {
            if let Some(token) = crate::oauth::import_claude_code_credentials() {
                let masked = mask_key(&token);
                println!("  {G}✓{RST} Found Claude Code credentials: {D}{masked}{RST}");
                let use_it = Self::prompt(&format!("  {V}▸{RST} Use Claude Code credentials? [Y/n]: "))?;
                if use_it.is_empty() || use_it.to_lowercase().starts_with('y') {
                    return Ok(token);
                }
            }
        }

        if provider.name == "openai" {
            if let Some((token, _refresh)) = crate::oauth::import_codex_credentials() {
                let masked = mask_key(&token);
                println!("  {G}✓{RST} Found Codex CLI credentials: {D}{masked}{RST}");
                let use_it = Self::prompt(&format!("  {V}▸{RST} Use Codex credentials? [Y/n]: "))?;
                if use_it.is_empty() || use_it.to_lowercase().starts_with('y') {
                    return Ok(token);
                }
            }
        }

        // For providers that support OAuth, offer that option
        let supports_oauth = matches!(provider.name.as_str(), "nous");
        if supports_oauth {
            println!();
            println!("  {VB}Authentication method:{RST}");
            println!("    {V}1{RST}) API key (paste directly)");
            println!("    {V}2{RST}) OAuth (browser login)");
            println!();
            let method = Self::prompt(&format!("  {V}▸{RST} Choice [1]: "))?;

            if method == "2" {
                println!("  {D}OAuth login will open in your browser...{RST}");
                println!("  {D}(OAuth requires async runtime — run 'tracey login' instead){RST}");
                println!("  {D}Falling back to API key entry.{RST}");
                // Note: actual OAuth flow needs async runtime which setup wizard doesn't have
                // The proper flow would be: tracey login --provider nous
            }
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
        use crate::interactive::{select_menu, MenuItem};

        // Complete model lists (from Hermes model_metadata.py + provider docs)
        let models: Vec<(&str, &str)> = match provider.name.as_str() {
            "anthropic" => vec![
                ("claude-sonnet-4-20250514", "Claude Sonnet 4 — recommended, best balance"),
                ("claude-opus-4-6-20250610", "Claude Opus 4.6 — most capable, 1M context"),
                ("claude-sonnet-4-6-20250610", "Claude Sonnet 4.6 — latest sonnet"),
                ("claude-haiku-4-5-20251001", "Claude Haiku 4.5 — fastest, cheapest"),
            ],
            "openai" => vec![
                ("gpt-4o", "GPT-4o — recommended"),
                ("gpt-4o-mini", "GPT-4o Mini — fast, cheap"),
                ("o3-mini", "o3 Mini — reasoning"),
                ("o4-mini", "o4 Mini — latest reasoning"),
                ("gpt-5.4", "GPT-5.4 — latest, if available"),
                ("gpt-5.4-mini", "GPT-5.4 Mini — fast latest"),
                ("gpt-5.3-codex", "GPT-5.3 Codex — code-optimized"),
            ],
            "gemini" => vec![
                ("gemini-2.5-pro", "Gemini 2.5 Pro — recommended"),
                ("gemini-2.5-flash", "Gemini 2.5 Flash — fast"),
                ("gemini-2.0-flash", "Gemini 2.0 Flash — stable"),
                ("gemini-3.1-pro", "Gemini 3.1 Pro — latest"),
            ],
            "ollama" => vec![
                ("qwen2.5-coder:32b", "Qwen 2.5 Coder 32B — best local code"),
                ("llama3.3:70b", "Llama 3.3 70B — strong general"),
                ("deepseek-coder-v2:16b", "DeepSeek Coder V2 — good balance"),
                ("codellama:34b", "Code Llama 34B — Meta code model"),
                ("mistral:7b", "Mistral 7B — lightweight"),
                ("phi-3:14b", "Phi-3 14B — Microsoft compact"),
                ("gemma2:9b", "Gemma 2 9B — Google compact"),
            ],
            "openrouter" => vec![
                ("anthropic/claude-sonnet-4-20250514", "Claude Sonnet 4"),
                ("anthropic/claude-opus-4-6-20250610", "Claude Opus 4.6"),
                ("openai/gpt-4o", "GPT-4o"),
                ("google/gemini-2.5-pro", "Gemini 2.5 Pro"),
                ("deepseek/deepseek-coder", "DeepSeek Coder"),
                ("meta-llama/llama-3.3-70b-instruct", "Llama 3.3 70B"),
                ("qwen/qwen-2.5-coder-32b-instruct", "Qwen 2.5 Coder 32B"),
                ("mistralai/mixtral-8x22b-instruct", "Mixtral 8x22B"),
            ],
            "deepseek" => vec![
                ("deepseek-coder", "DeepSeek Coder V3 — code-optimized"),
                ("deepseek-chat", "DeepSeek Chat V3 — general purpose"),
                ("deepseek-reasoner", "DeepSeek R1 — reasoning"),
            ],
            "together" => vec![
                ("Qwen/Qwen2.5-Coder-32B-Instruct", "Qwen 2.5 Coder 32B"),
                ("meta-llama/Llama-3.3-70B-Instruct-Turbo", "Llama 3.3 70B"),
                ("deepseek-ai/DeepSeek-V3", "DeepSeek V3"),
                ("mistralai/Mixtral-8x22B-Instruct-v0.1", "Mixtral 8x22B"),
            ],
            "groq" => vec![
                ("llama-3.3-70b-versatile", "Llama 3.3 70B — fast"),
                ("llama-3.1-8b-instant", "Llama 3.1 8B — instant"),
                ("mixtral-8x7b-32768", "Mixtral 8x7B — 32K context"),
                ("gemma2-9b-it", "Gemma 2 9B — compact"),
            ],
            "xai" => vec![
                ("grok-2", "Grok 2 — latest"),
                ("grok-2-mini", "Grok 2 Mini — fast"),
            ],
            "fireworks" => vec![
                ("accounts/fireworks/models/qwen2p5-coder-32b-instruct", "Qwen 2.5 Coder 32B"),
                ("accounts/fireworks/models/llama-v3p3-70b-instruct", "Llama 3.3 70B"),
                ("accounts/fireworks/models/deepseek-v3", "DeepSeek V3"),
            ],
            "moonshot" => vec![
                ("moonshot-v1-128k", "Moonshot V1 — 128K context"),
                ("moonshot-v1-32k", "Moonshot V1 — 32K context"),
            ],
            _ => vec![
                ("auto", "Auto-detect"),
            ],
        };

        let mut items: Vec<MenuItem> = models
            .iter()
            .map(|(_, desc)| MenuItem::new(*desc))
            .collect();
        items.push(MenuItem::new("Custom model name..."));

        // Add blank lines for the menu
        for _ in 0..items.len() + 5 {
            println!();
        }

        let choice = select_menu("Select model:", &items)
            .ok_or_else(|| tracey_core::TraceyError::Config("selection cancelled".into()))?;

        if choice >= models.len() {
            // Custom model
            Self::prompt(&format!("  {V}▸{RST} Model name: "))
        } else {
            Ok(models[choice].0.to_string())
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

    /// Try to import credentials from Claude Code or Codex CLI.
    /// If found and accepted, creates full config and returns Ok(Some(())).
    fn try_quick_import() -> TraceyResult<Option<()>> {
        // Check Claude Code
        if let Some(token) = crate::oauth::import_claude_code_credentials() {
            let masked = mask_key(&token);
            println!("  {VB}Quick Setup:{RST} Claude Code credentials found");
            println!("  {D}Token: {masked}{RST}");
            println!();
            let use_it = Self::prompt(&format!("  {V}▸{RST} Import Claude Code credentials? [Y/n]: "))?;
            if use_it.is_empty() || use_it.to_lowercase().starts_with('y') {
                let provider = ProviderEntry {
                    name: "anthropic".into(),
                    base_url: "https://api.anthropic.com".into(),
                    api_key_env: "ANTHROPIC_API_KEY".into(),
                    transport: Transport::AnthropicMessages,
                };
                let model = "claude-sonnet-4-20250514".to_string();
                Self::save_config(&provider, &token, &model)?;

                crate::interactive::animate_step("Imported Claude Code credentials", 1);
                crate::interactive::animate_step("Provider: Anthropic", 2);
                crate::interactive::animate_step("Model: Claude Sonnet 4", 3);
                crate::interactive::animate_step("✓ Ready to trace", 4);
                println!();
                return Ok(Some(()));
            }
        }

        // Check Codex CLI
        if let Some((token, _refresh)) = crate::oauth::import_codex_credentials() {
            let masked = mask_key(&token);
            println!("  {VB}Quick Setup:{RST} Codex CLI credentials found");
            println!("  {D}Token: {masked}{RST}");
            println!();
            let use_it = Self::prompt(&format!("  {V}▸{RST} Import Codex credentials? [Y/n]: "))?;
            if use_it.is_empty() || use_it.to_lowercase().starts_with('y') {
                let provider = ProviderEntry {
                    name: "openai".into(),
                    base_url: "https://api.openai.com/v1".into(),
                    api_key_env: "OPENAI_API_KEY".into(),
                    transport: Transport::OpenAiChat,
                };
                let model = "gpt-4o".to_string();
                Self::save_config(&provider, &token, &model)?;

                crate::interactive::animate_step("Imported Codex credentials", 1);
                crate::interactive::animate_step("Provider: OpenAI", 2);
                crate::interactive::animate_step("Model: GPT-4o", 3);
                crate::interactive::animate_step("✓ Ready to trace", 4);
                println!();
                return Ok(Some(()));
            }
        }

        Ok(None)
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

/// Render text as liquid chrome per-character gradient (violet metallic)
fn chrome_gradient(text: &str) -> String {
    let stops: [(u8, u8, u8); 12] = [
        (245, 242, 255), (230, 220, 255), (210, 190, 255), (180, 160, 245),
        (155, 130, 240), (139, 92, 246),  (110, 70, 220),  (85, 50, 190),
        (65, 35, 150),   (45, 25, 100),   (30, 15, 60),    (15, 8, 35),
    ];
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len().max(1);
    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let t = (i as f64 / len as f64).clamp(0.0, 1.0);
        let t_curved = t * t * 0.7 + t * 0.3;
        let idx_f = t_curved * 11.0;
        let lo = (idx_f as usize).min(10);
        let hi = (lo + 1).min(11);
        let frac = idx_f - lo as f64;
        let r = (stops[lo].0 as f64 * (1.0 - frac) + stops[hi].0 as f64 * frac) as u8;
        let g = (stops[lo].1 as f64 * (1.0 - frac) + stops[hi].1 as f64 * frac) as u8;
        let b = (stops[lo].2 as f64 * (1.0 - frac) + stops[hi].2 as f64 * frac) as u8;
        result.push_str(&format!("\x1b[38;2;{r};{g};{b}m{ch}"));
    }
    result.push_str("\x1b[0m");
    result
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
