# Tracey

**Tracing causal connections.**

```
  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗
  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝
     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝
     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝
     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║
     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝
      ◉──╌╌──▸ ◉──╌╌──▸ ◉
            └──╌╌──▸ ◉
```

A causal-graph-based coding agent. Tracey maintains a live causal graph of your codebase — tracing how files depend on each other, how your edits ripple through the project, and what patterns emerge across sessions. Every decision is inspectable.

## Why Tracey?

Other coding agents (Claude Code, Codex, Hermes) delegate all reasoning to the LLM. Tracey augments the LLM with an explicit causal graph:

- **Before every action**: queries the graph to predict impact ("if I edit this function, what might break?")
- **After every action**: updates the graph with new evidence
- **Across sessions**: the graph persists and gets smarter over time
- **Always inspectable**: when something goes wrong, trace the exact causal chain

## Features

- **Causal Graph Engine** — 4-layer graph (Code / Execution / Knowledge / Project) with confidence-weighted edges, Personalized PageRank for context selection, exponential decay, and contradiction detection
- **Model Agnostic** — Works with Claude, GPT, Gemini, Ollama, DeepSeek, OpenRouter, or any OpenAI-compatible API
- **Code Understanding** — Parses your codebase (Rust, Python, TypeScript, Go, Java, C/C++, Ruby, C#) to build a dependency graph from day one
- **MAGMA Memory** — 4-signal memory retrieval (semantic + temporal + causal + entity) via Reciprocal Rank Fusion
- **Polished TUI** — Violet-themed terminal UI with gradient ASCII logo, box-drawing message frames, context capacity bar, and animated causal graph
- **Telegram Bot** — Text streaming with Unicode graph rendering
- **Hook System** — JSON stdin/stdout protocol with exit code semantics (0=continue, 1=abort, 2=modify)
- **Skill System** — SKILL.md files with YAML frontmatter for extensibility
- **SQLite Persistence** — Graph survives between sessions with WAL mode
- **Edge Provenance** — Every edge tracks its source (StaticAnalysis, GitCoChange, AgentObserved, UserDefined) with confidence capping

## Install

```bash
cargo install --git https://github.com/CTRLabs/tracey tracey-cli
```

## Quick Start

```bash
# Configure your LLM provider
tracey --setup

# Start interactive session
tracey

# One-shot mode
tracey "fix the null check in auth.rs"

# Print mode (pipe-friendly)
echo "explain this codebase" | tracey --print
```

## How It Works

### The Agent Loop

Every turn follows the OODA-C lifecycle:

1. **Observe** — Gather context from the codebase
2. **Orient** — Query the causal graph (Personalized PageRank → focused 25-node subgraph)
3. **Decide** — LLM reasons with causal context injected as Markdown-KV
4. **Act** — Execute tools (Read, Write, Edit, Bash, Glob, Grep)
5. **Causify** — GraphObserver updates the graph based on tool results
6. **Verify** — Check graph consistency (DAG invariant, contradictions)

### The Causal Graph

```
Code Layer (from AST):     src/auth.rs --[calls]--> src/db.rs
Execution Layer (traces):  edit:auth.rs --[caused]--> test_failure
Knowledge Layer (memory):  "JWT validation panics on expired tokens"
Project Layer (goals):     fix-auth-bug --[blocks]--> deploy-v2
```

Edges have **provenance** (StaticAnalysis is trusted at 1.0; AgentObserved is capped at 0.7) and **exponential decay** (unused edges fade over sessions).

### Architecture

16 Rust crates in a Cargo workspace:

| Crate | Purpose |
|-------|---------|
| `tracey-core` | Types, events (SQ/EQ protocol), traits |
| `tracey-config` | TOML config, credential pool, setup wizard |
| `tracey-llm` | Anthropic + OpenAI providers, smart routing |
| `tracey-tools` | Tool registry + 6 core tools |
| `tracey-graph` | 4-layer causal graph, PPR, serialization, verification |
| `tracey-memory` | MAGMA 4-signal memory with RRF fusion |
| `tracey-agent` | OODA-C agent loop, graph observer, compaction |
| `tracey-ast` | Code graph builder (Rust/Python/TS/Go/Java/C/Ruby/C#) |
| `tracey-search` | Vector index + hybrid search |
| `tracey-session` | JSONL session persistence |
| `tracey-sandbox` | Permission model (deny > ask > allow) |
| `tracey-hooks` | Lifecycle hooks (PreToolCall, PostToolCall, etc.) |
| `tracey-skills` | SKILL.md loading with YAML frontmatter |
| `tracey-tui` | Violet-themed ratatui terminal UI |
| `tracey-telegram` | Telegram bot with text streaming |
| `tracey-cli` | CLI entry point |

## Configuration

Tracey uses TOML configuration with hierarchy:

```
~/.config/tracey/config.toml     (global)
<project>/.tracey/config.toml    (project)
Environment variables            (override)
```

### TRACEY.md

Like Claude Code's CLAUDE.md — project-specific instructions loaded automatically:

```markdown
# TRACEY.md

Always use async Rust patterns.
Run `cargo test` after every edit.
The auth module is critical — be careful with changes.
```

## Research Foundation

Tracey is built on peer-reviewed research:

- **DeepMind (ICLR 2024)**: "Robust agents learn causal world models" — mathematical proof that generalizing agents must learn causal structure
- **MAGMA (2026)**: Multi-graph memory with 4-signal retrieval — +45% reasoning accuracy over flat memory
- **Personalized PageRank**: Context-relevant subgraph extraction (from HippoRAG, NeurIPS 2024)
- **LocAgent (ACL 2025)**: Graph-guided localization — 92.7% file-level accuracy
- **Causal Abstraction**: Hierarchical causal models for code (Beckers & Halpern, 2019)

## License

MIT
