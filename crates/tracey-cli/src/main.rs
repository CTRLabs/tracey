use clap::Parser;
use std::sync::{Arc, RwLock};
use tracey_core::events::EventChannel;
use tracey_graph::GraphStore;
use tracey_tui::logo;

#[derive(Parser)]
#[command(name = "tracey", about = "tracing causal connections", version)]
struct Cli {
    /// Start with a prompt (non-interactive)
    prompt: Option<String>,

    /// Run setup wizard
    #[arg(long)]
    setup: bool,

    /// Override model
    #[arg(long)]
    model: Option<String>,

    /// Override provider
    #[arg(long)]
    provider: Option<String>,

    /// Print mode (no TUI)
    #[arg(long)]
    print: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("tracey=info".parse().unwrap()),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    // Setup wizard
    if cli.setup {
        tracey_config::setup::SetupWizard::run()?;
        return Ok(());
    }

    // Load config
    let cwd = std::env::current_dir()?;
    let git_root = tracey_config::instruction_files::find_git_root(&cwd);
    let mut config = tracey_config::TraceyConfig::load(git_root.as_deref())?;

    // Apply CLI overrides
    if let Some(model) = cli.model {
        config.routing.default_model = model;
    }
    if let Some(provider) = cli.provider {
        config.provider.default_provider = provider;
    }

    // Load credentials
    let mut pool = tracey_config::CredentialPool::load().unwrap_or_default();
    pool.load_from_env(&config.provider.providers);

    // Build LLM router
    let router = tracey_llm::ModelRouter::from_config(&config, &pool);

    if router.available_providers().is_empty() {
        eprintln!("\x1b[31merror\x1b[0m: no LLM providers configured");
        eprintln!("Run \x1b[1mtracey --setup\x1b[0m to configure a provider");
        std::process::exit(1);
    }

    // Build tool registry
    let mut tools = tracey_tools::ToolRegistry::new();
    tracey_tools::register_core_tools(&mut tools);

    // Load instruction files
    let instructions = tracey_config::instruction_files::load_instructions(&cwd)?;
    let mut system_prompt = build_system_prompt(&instructions, &tools);

    // Create causal graph (Arc<RwLock<>> for concurrent access)
    let graph = Arc::new(RwLock::new(GraphStore::new()));
    let session_counter = 1_u64; // TODO: load from persistence

    tracing::info!(
        "Graph initialized: {} nodes, {} edges",
        graph.read().unwrap().node_count(),
        graph.read().unwrap().edge_count()
    );

    // Create SQ/EQ channels
    let (agent_handle, ui_handle) = EventChannel::new(256);

    // Spawn agent
    let mut agent = tracey_agent::Agent::new(
        router,
        tools,
        graph.clone(),
        system_prompt,
        session_counter,
    );
    let agent_task = tokio::spawn(async move {
        agent.run(agent_handle).await;
    });

    if let Some(prompt) = cli.prompt {
        // One-shot mode
        use tracey_core::events::Submission;
        let _ = ui_handle.submit(Submission::UserMessage {
            content: prompt,
            attachments: vec![],
        }).await;

        let mut ui_handle = ui_handle;
        while let Some(event) = ui_handle.next_event().await {
            match event {
                tracey_core::events::AgentEvent::AssistantChunk { text, .. } => {
                    print!("{text}");
                }
                tracey_core::events::AgentEvent::TurnComplete { .. } => {
                    println!();
                    let _ = ui_handle.submit(Submission::Shutdown).await;
                    break;
                }
                tracey_core::events::AgentEvent::Error { message, .. } => {
                    eprintln!("\x1b[31merror\x1b[0m: {message}");
                    let _ = ui_handle.submit(Submission::Shutdown).await;
                    break;
                }
                _ => {}
            }
        }
    } else if cli.print {
        // Print mode (pipe-friendly)
        eprintln!("tracey: waiting for input on stdin...");
        let mut input = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;

        use tracey_core::events::Submission;
        let _ = ui_handle.submit(Submission::UserMessage {
            content: input,
            attachments: vec![],
        }).await;

        let mut ui_handle = ui_handle;
        while let Some(event) = ui_handle.next_event().await {
            match event {
                tracey_core::events::AgentEvent::AssistantChunk { text, .. } => {
                    print!("{text}");
                }
                tracey_core::events::AgentEvent::TurnComplete { .. } => {
                    println!();
                    let _ = ui_handle.submit(Submission::Shutdown).await;
                    break;
                }
                _ => {}
            }
        }
    } else {
        // Interactive TUI mode
        if config.ui.show_logo && config.ui.animations {
            logo::animate_startup().await;
        } else if config.ui.show_logo {
            logo::print_startup_banner();
        }

        // Init terminal
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = ratatui::backend::CrosstermBackend::new(stdout);
        let mut terminal = ratatui::Terminal::new(backend)?;

        let mut app = tracey_tui::app::App::new(ui_handle);
        let result = app.run(&mut terminal).await;

        // Restore terminal
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result?;
    }

    agent_task.await?;

    Ok(())
}

fn build_system_prompt(
    instructions: &[tracey_config::instruction_files::InstructionFile],
    tools: &tracey_tools::ToolRegistry,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are Tracey, a causal-graph-aware coding agent.\n\n");
    prompt.push_str("You trace causal connections between code, actions, and outcomes.\n");
    prompt.push_str("Before taking action, consider the causal impact of your changes.\n");
    prompt.push_str("After acting, update your understanding of causal relationships.\n\n");

    prompt.push_str("# Tools Available\n\n");
    for name in tools.tool_names() {
        prompt.push_str(&format!("- {name}\n"));
    }
    prompt.push('\n');

    // Add instruction files
    for file in instructions {
        prompt.push_str(&format!("# Instructions ({})\n\n", file.path.display()));
        prompt.push_str(&file.content);
        prompt.push_str("\n\n");
    }

    prompt
}
