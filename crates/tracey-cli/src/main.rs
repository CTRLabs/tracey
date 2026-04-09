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
    let system_prompt = build_system_prompt(&instructions, &tools);

    // Load or create causal graph with SQLite persistence
    let graph_db_path = tracey_graph::graph_db_path(&cwd);
    let graph_db = tracey_graph::GraphDb::open(&graph_db_path)?;
    let session_counter = graph_db.load_session_counter().unwrap_or(0) + 1;

    let loaded_graph = if graph_db_path.exists() {
        match graph_db.load() {
            Ok(g) if g.node_count() > 0 => {
                tracing::info!("Loaded graph: {} nodes, {} edges", g.node_count(), g.edge_count());
                g
            }
            _ => GraphStore::new(),
        }
    } else {
        GraphStore::new()
    };

    let graph = Arc::new(RwLock::new(loaded_graph));

    // Build code graph from AST if this is a code project
    let code_stats = tracey_ast::build_code_graph_async(&cwd, &graph).await;
    if code_stats.files_parsed > 0 {
        tracing::info!(
            "Code graph: {} files parsed, {} nodes, {} edges ({:?}) [{}]",
            code_stats.files_parsed,
            code_stats.nodes_created,
            code_stats.edges_created,
            code_stats.duration,
            code_stats.languages_detected.join(", ")
        );
    }

    tracing::info!(
        "Session {} — graph: {} nodes, {} edges",
        session_counter,
        graph.read().unwrap().node_count(),
        graph.read().unwrap().edge_count()
    );

    // Create SQ/EQ channels
    let (agent_handle, ui_handle) = EventChannel::new(256);

    // Create hook runner (empty for now — populated from config in future)
    let hook_runner = tracey_hooks::HookRunner::empty();

    // Spawn agent
    let mut agent = tracey_agent::Agent::new(
        router,
        tools,
        graph.clone(),
        hook_runner,
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
        // Interactive TUI mode — show welcome banner
        {
            let g = graph.read().unwrap();
            let banner_info = tracey_tui::banner::BannerInfo {
                model: config.routing.default_model.clone(),
                provider: config.provider.default_provider.clone(),
                cwd: cwd.to_string_lossy().to_string(),
                graph_nodes: g.node_count(),
                graph_edges: g.edge_count(),
                languages: if code_stats.files_parsed > 0 {
                    code_stats.languages_detected.join(", ")
                } else {
                    String::new()
                },
                tools: vec!["Read".into(), "Write".into(), "Edit".into(), "Bash".into(), "Glob".into(), "Grep".into()],
                skill_count: 0, // TODO: count loaded skills
            };

            if config.ui.show_logo && config.ui.animations {
                logo::animate_startup().await;
            }
            tracey_tui::banner::print_welcome_banner(&banner_info);

            // Pause briefly to let user read the banner
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
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
        app.set_model_info(&config.routing.default_model, &config.provider.default_provider);
        app.set_graph_stats(
            graph.read().unwrap().node_count(),
            graph.read().unwrap().edge_count(),
        );
        app.session_number = session_counter;
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

    // Save graph to SQLite at session end
    {
        let g = graph.read().unwrap();
        if g.node_count() > 0 {
            if let Err(e) = graph_db.save(&g) {
                tracing::warn!("Failed to save graph: {e}");
            } else {
                tracing::info!("Graph saved: {} nodes, {} edges", g.node_count(), g.edge_count());
            }
        }
        graph_db.save_session_counter(session_counter).ok();
    }

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
