use crate::app::{App, DisplayMessage, MessageRole};

/// Result of processing a slash command
pub enum CommandResult {
    /// Command was handled locally — don't send to LLM
    Handled,
    /// Not a command — send to LLM as normal message
    NotACommand,
    /// Command wants to quit
    Quit,
}

/// Process slash commands locally in the TUI
pub fn handle_command(app: &mut App, input: &str) -> CommandResult {
    let input = input.trim();

    if !input.starts_with('/') {
        return CommandResult::NotACommand;
    }

    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match cmd.as_str() {
        "/help" | "/h" | "/?" => {
            cmd_help(app);
            CommandResult::Handled
        }
        "/clear" | "/reset" | "/new" => {
            cmd_clear(app);
            CommandResult::Handled
        }
        "/compact" => {
            cmd_compact(app, args);
            CommandResult::Handled
        }
        "/cost" | "/usage" | "/tokens" => {
            cmd_cost(app);
            CommandResult::Handled
        }
        "/model" => {
            cmd_model(app, args);
            CommandResult::Handled
        }
        "/graph" => {
            cmd_graph(app, args);
            CommandResult::Handled
        }
        "/why" => {
            // /why sends to the LLM with causal context prepended
            CommandResult::NotACommand
        }
        "/whatif" => {
            cmd_whatif(app, args);
            CommandResult::Handled
        }
        "/status" => {
            cmd_status(app);
            CommandResult::Handled
        }
        "/quit" | "/exit" | "/q" => {
            CommandResult::Quit
        }
        _ => {
            app.messages.push(DisplayMessage {
                role: MessageRole::Error,
                content: format!("Unknown command: {cmd}. Type /help for available commands."),
                tool_name: None,
                timestamp: crate::app::now_time(),
            });
            CommandResult::Handled
        }
    }
}

fn cmd_help(app: &mut App) {
    let help = r#"## Commands

### Session
  /clear, /reset     — Clear conversation and start fresh
  /compact [focus]    — Compress conversation context
  /quit, /exit        — Exit Tracey

### Info
  /help               — Show this help
  /cost, /usage       — Show token usage and estimated cost
  /status             — Show session info
  /model [name]       — Show or switch model

### Causal Graph
  /graph show         — Display the causal graph
  /graph impact <file> — Show what's affected by changing a file
  /graph stats        — Show graph statistics
  /graph search <q>   — Search graph nodes
  /graph export <fmt>  — Export graph (mermaid/dot/json)
  /why <error>        — Trace root cause through causal graph
  /whatif edit <file>  — Predict impact of editing a file

### Shortcuts
  Ctrl+C              — Interrupt / Quit
  Esc                 — Interrupt current generation
  Ctrl+U              — Clear input
  ↑↓ / PgUp/PgDn     — Scroll conversation"#;

    app.messages.push(DisplayMessage {
        role: MessageRole::System,
        content: help.to_string(),
        tool_name: None,
        timestamp: crate::app::now_time(),
    });
}

fn cmd_clear(app: &mut App) {
    app.messages.clear();
    app.scroll_offset = 0;
    app.tokens_used = 0;
    app.turn_count = 0;
    app.messages.push(DisplayMessage {
        role: MessageRole::System,
        content: "Conversation cleared. Starting fresh.".to_string(),
        tool_name: None,
        timestamp: crate::app::now_time(),
    });
}

fn cmd_compact(app: &mut App, focus: &str) {
    let focus_msg = if focus.is_empty() {
        "Compacting conversation...".to_string()
    } else {
        format!("Compacting with focus: {focus}")
    };
    app.messages.push(DisplayMessage {
        role: MessageRole::System,
        content: focus_msg,
        tool_name: None,
        timestamp: crate::app::now_time(),
    });
    // Note: actual compaction happens in the agent loop via needs_compaction()
    // This is a UI acknowledgment. A future version will send a Submission
    // to trigger manual compaction.
}

fn cmd_cost(app: &mut App) {
    let cost_estimate = estimate_cost(&app.model_name, app.tokens_used);
    let content = format!(
        "## Token Usage\n\
         \n\
         Tokens used: {}\n\
         Estimated cost: ${:.4}\n\
         Model: {}\n\
         Turns: {}\n\
         Context capacity: {:.0}%",
        app.tokens_used,
        cost_estimate,
        if app.model_name.is_empty() { "unknown" } else { &app.model_name },
        app.turn_count,
        (app.tokens_used as f64 / app.tokens_max as f64) * 100.0,
    );
    app.messages.push(DisplayMessage {
        role: MessageRole::System,
        content,
        tool_name: None,
        timestamp: crate::app::now_time(),
    });
}

fn cmd_model(app: &mut App, name: &str) {
    if name.is_empty() {
        app.messages.push(DisplayMessage {
            role: MessageRole::System,
            content: format!("Current model: {}", if app.model_name.is_empty() { "not set" } else { &app.model_name }),
            tool_name: None,
            timestamp: crate::app::now_time(),
        });
    } else {
        app.model_name = name.to_string();
        app.messages.push(DisplayMessage {
            role: MessageRole::System,
            content: format!("Model switched to: {name}"),
            tool_name: None,
            timestamp: crate::app::now_time(),
        });
        // Note: actual model switch needs a Submission variant to reach the agent
    }
}

fn cmd_graph(app: &mut App, args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let subcmd = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();
    let subargs = parts.get(1).map(|s| *s).unwrap_or("");

    match subcmd.as_str() {
        "" | "show" => {
            app.messages.push(DisplayMessage {
                role: MessageRole::GraphUpdate,
                content: format!(
                    "Graph: {} nodes, {} edges\n\
                     (Use /graph stats for detailed breakdown, /graph impact <file> for impact analysis)",
                    app.graph_nodes, app.graph_edges
                ),
                tool_name: None,
                timestamp: crate::app::now_time(),
            });
        }
        "stats" => {
            app.messages.push(DisplayMessage {
                role: MessageRole::GraphUpdate,
                content: format!(
                    "## Graph Statistics\n\
                     \n\
                     Total: {} nodes, {} edges\n\
                     Session: {} (turn {})\n\
                     Last update: {}",
                    app.graph_nodes,
                    app.graph_edges,
                    app.session_number,
                    app.turn_count,
                    if app.graph_last_update.is_empty() { "none" } else { &app.graph_last_update },
                ),
                tool_name: None,
                timestamp: crate::app::now_time(),
            });
        }
        "impact" => {
            if subargs.is_empty() {
                app.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: "Usage: /graph impact <file>".to_string(),
                    tool_name: None,
                    timestamp: crate::app::now_time(),
                });
            } else {
                // Impact analysis — this will be sent to the agent as a special query
                app.messages.push(DisplayMessage {
                    role: MessageRole::GraphUpdate,
                    content: format!("Analyzing impact of: {subargs}..."),
                    tool_name: None,
                    timestamp: crate::app::now_time(),
                });
                // TODO: wire to GraphStore::impact_zone via a Submission
            }
        }
        "search" => {
            if subargs.is_empty() {
                app.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: "Usage: /graph search <query>".to_string(),
                    tool_name: None,
                    timestamp: crate::app::now_time(),
                });
            } else {
                app.messages.push(DisplayMessage {
                    role: MessageRole::GraphUpdate,
                    content: format!("Searching graph for: {subargs}..."),
                    tool_name: None,
                    timestamp: crate::app::now_time(),
                });
            }
        }
        "export" => {
            let format = if subargs.is_empty() { "json" } else { subargs };
            app.messages.push(DisplayMessage {
                role: MessageRole::System,
                content: format!("Exporting graph as {format}..."),
                tool_name: None,
                timestamp: crate::app::now_time(),
            });
            // TODO: wire to serialize::to_mermaid/to_dot/to_json
        }
        _ => {
            app.messages.push(DisplayMessage {
                role: MessageRole::Error,
                content: format!("Unknown graph command: {subcmd}. Try: show, stats, impact, search, export"),
                tool_name: None,
                timestamp: crate::app::now_time(),
            });
        }
    }
}

fn cmd_whatif(app: &mut App, args: &str) {
    if args.is_empty() {
        app.messages.push(DisplayMessage {
            role: MessageRole::Error,
            content: "Usage: /whatif edit <file>".to_string(),
            tool_name: None,
            timestamp: crate::app::now_time(),
        });
    } else {
        app.messages.push(DisplayMessage {
            role: MessageRole::GraphUpdate,
            content: format!("What-if analysis: {args}..."),
            tool_name: None,
            timestamp: crate::app::now_time(),
        });
        // TODO: wire to impact_zone + risk assessment
    }
}

fn cmd_status(app: &mut App) {
    let content = format!(
        "## Status\n\
         \n\
         Model: {}\n\
         Provider: {}\n\
         Session: #{}\n\
         Turns: {}\n\
         Graph: {} nodes, {} edges\n\
         Tokens: {} / {} ({:.0}%)\n\
         Last graph update: {}",
        if app.model_name.is_empty() { "not set" } else { &app.model_name },
        if app.provider_name.is_empty() { "not set" } else { &app.provider_name },
        app.session_number,
        app.turn_count,
        app.graph_nodes,
        app.graph_edges,
        app.tokens_used,
        app.tokens_max,
        (app.tokens_used as f64 / app.tokens_max as f64) * 100.0,
        if app.graph_last_update.is_empty() { "none" } else { &app.graph_last_update },
    );
    app.messages.push(DisplayMessage {
        role: MessageRole::System,
        content,
        tool_name: None,
        timestamp: crate::app::now_time(),
    });
}

/// Rough cost estimation based on model name
fn estimate_cost(model: &str, tokens: u64) -> f64 {
    let per_million = if model.contains("opus") {
        15.0 + 75.0 // avg of input ($15) and output ($75) per million
    } else if model.contains("sonnet") {
        3.0 + 15.0
    } else if model.contains("haiku") {
        0.25 + 1.25
    } else if model.contains("gpt-4o") {
        2.5 + 10.0
    } else if model.contains("gpt-4o-mini") {
        0.15 + 0.6
    } else {
        5.0 // default estimate
    };

    (tokens as f64 / 1_000_000.0) * (per_million / 2.0) // rough avg of in/out
}

/// Make now_time public for use in commands
pub use crate::app::now_time;
