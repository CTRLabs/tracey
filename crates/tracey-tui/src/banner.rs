use crate::theme::*;

/// Build the welcome banner — Hermes-inspired 2-column layout.
/// Left: logo + model + cwd. Right: tools + graph stats + commands.
pub fn print_welcome_banner(info: &BannerInfo) {
    let term_width = terminal_width();
    let wide = term_width >= 90;

    println!();

    // === LOGO (only on wide terminals) ===
    if wide {
        let gradient = [
            "\x1b[38;2;109;40;217m",
            "\x1b[38;2;119;56;226m",
            "\x1b[38;2;129;72;236m",
            "\x1b[38;2;139;92;246m",
            "\x1b[38;2;149;108;248m",
            "\x1b[38;2;159;124;250m",
            "\x1b[38;2;167;139;250m",
        ];

        let logo_lines = [
            "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
            "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
            "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
            "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
            "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
            "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
        ];

        for (i, line) in logo_lines.iter().enumerate() {
            let color = gradient[i.min(gradient.len() - 1)];
            println!("{color}{line}{ANSI_RESET}");
        }
        println!();
        println!("  {ANSI_LAVENDER}    ◉{ANSI_DIM}──╌╌──▸{ANSI_LAVENDER} ◉{ANSI_DIM}──╌╌──▸{ANSI_LAVENDER} ◉{ANSI_RESET}");
        println!("  {ANSI_DIM}              └──╌╌──▸{ANSI_LAVENDER} ◉{ANSI_RESET}");
        println!();
    }

    // === BANNER BOX ===
    let box_width = term_width.min(80);
    let border = "─".repeat(box_width - 2);

    // Top border
    println!("  {ANSI_VIOLET_DIM}╭{border}╮{ANSI_RESET}");

    // Title line
    let version = env!("CARGO_PKG_VERSION");
    let title = format!("◆ tracey v{version}");
    let padding = box_width.saturating_sub(title.len() + 4);
    println!("  {ANSI_VIOLET_DIM}│{ANSI_RESET} {ANSI_VIOLET}{ANSI_BOLD}{title}{ANSI_RESET}{}{ANSI_VIOLET_DIM}│{ANSI_RESET}",
        " ".repeat(padding));

    // Separator
    let sep = "─".repeat(box_width - 2);
    println!("  {ANSI_VIOLET_DIM}├{sep}┤{ANSI_RESET}");

    // Model info
    print_row(box_width, &format!("  Model     {ANSI_WHITE}{}{ANSI_RESET}", info.model));
    print_row(box_width, &format!("  Provider  {ANSI_WHITE}{}{ANSI_RESET}", info.provider));
    print_row(box_width, &format!("  CWD       {ANSI_DIM}{}{ANSI_RESET}", truncate_path(&info.cwd, box_width - 20)));

    // Separator
    println!("  {ANSI_VIOLET_DIM}├{sep}┤{ANSI_RESET}");

    // Graph stats
    if info.graph_nodes > 0 {
        print_row(box_width, &format!(
            "  {ANSI_LAVENDER}◈{ANSI_RESET} Graph     {ANSI_WHITE}{} nodes, {} edges{ANSI_RESET}",
            info.graph_nodes, info.graph_edges
        ));
        if !info.languages.is_empty() {
            print_row(box_width, &format!(
                "  {ANSI_LAVENDER}◈{ANSI_RESET} Languages {ANSI_DIM}{}{ANSI_RESET}",
                info.languages
            ));
        }
    } else {
        print_row(box_width, &format!("  {ANSI_DIM}◈ Graph     empty (will build on first query){ANSI_RESET}"));
    }

    // Separator
    println!("  {ANSI_VIOLET_DIM}├{sep}┤{ANSI_RESET}");

    // Tools
    print_row(box_width, &format!(
        "  Tools     {ANSI_WHITE}{}{ANSI_RESET}",
        info.tools.join(", ")
    ));

    // Skills
    if info.skill_count > 0 {
        print_row(box_width, &format!(
            "  Skills    {ANSI_WHITE}{} loaded{ANSI_RESET}",
            info.skill_count
        ));
    }

    // Separator
    println!("  {ANSI_VIOLET_DIM}├{sep}┤{ANSI_RESET}");

    // Quick reference
    print_row(box_width, &format!("  {ANSI_DIM}/help commands · /graph show · /cost · /why <error>{ANSI_RESET}"));
    print_row(box_width, &format!("  {ANSI_DIM}Ctrl+C quit · ↑↓ scroll · Esc interrupt{ANSI_RESET}"));

    // Bottom border
    println!("  {ANSI_VIOLET_DIM}╰{border}╯{ANSI_RESET}");
    println!();
}

fn print_row(box_width: usize, content: &str) {
    // Strip ANSI for width calculation
    let visible_len = strip_ansi_len(content);
    let padding = box_width.saturating_sub(visible_len + 4);
    println!("  {ANSI_VIOLET_DIM}│{ANSI_RESET}{content}{}{ANSI_VIOLET_DIM}│{ANSI_RESET}",
        " ".repeat(padding));
}

/// Count visible characters (ignoring ANSI escape sequences)
fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max + 3..])
    }
}

fn terminal_width() -> usize {
    crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80)
}

/// Information needed to render the welcome banner
pub struct BannerInfo {
    pub model: String,
    pub provider: String,
    pub cwd: String,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub languages: String,
    pub tools: Vec<String>,
    pub skill_count: usize,
}
