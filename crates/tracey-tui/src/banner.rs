use crate::theme::{self, *};

/// Build the welcome banner — liquid chrome aesthetic.
pub fn print_welcome_banner(info: &BannerInfo) {
    let term_width = terminal_width();
    let wide = term_width >= 90;

    println!();

    // Logo (liquid chrome per-character gradient)
    if wide {
        crate::logo::print_chrome_logo();
    }

    // Banner box with chrome borders
    let box_width = term_width.min(80);
    let border = "─".repeat(box_width - 2);
    let c = ANSI_CHROME;

    // Top border in chrome gradient
    println!("  {}", chrome_gradient_ansi(&format!("╭{border}╮")));

    // Title
    let version = env!("CARGO_PKG_VERSION");
    let title = format!("◆ tracey v{version}");
    let pad = box_width.saturating_sub(title.len() + 4);
    println!("  {}│{} {} {}{} {}│{}",
        c[7], ANSI_RESET, chrome_gradient_ansi(&title), ANSI_RESET,
        " ".repeat(pad), c[7], ANSI_RESET);

    // Separator
    println!("  {}├{}┤{}", c[8], "─".repeat(box_width - 2), ANSI_RESET);

    // Model + provider
    print_row(box_width, &format!("  Model     {}{}{}", ANSI_CHROME[1], info.model, ANSI_RESET));
    print_row(box_width, &format!("  Provider  {}{}{}", ANSI_CHROME[3], info.provider, ANSI_RESET));
    print_row(box_width, &format!("  CWD       {}{}{}", ANSI_DIM, truncate_path(&info.cwd, box_width - 20), ANSI_RESET));

    // Separator
    println!("  {}├{}┤{}", c[8], "─".repeat(box_width - 2), ANSI_RESET);

    // Graph stats
    if info.graph_nodes > 0 {
        print_row(box_width, &format!(
            "  {}◈{} Graph     {}{} nodes, {} edges{}",
            c[2], ANSI_RESET, ANSI_CHROME[1], info.graph_nodes, info.graph_edges, ANSI_RESET
        ));
        if !info.languages.is_empty() {
            print_row(box_width, &format!(
                "  {}◈{} Languages {}{}{}",
                c[2], ANSI_RESET, ANSI_DIM, info.languages, ANSI_RESET
            ));
        }
    } else {
        print_row(box_width, &format!("  {}◈ Graph     empty (builds on first query){}", ANSI_DIM, ANSI_RESET));
    }

    // Separator
    println!("  {}├{}┤{}", c[8], "─".repeat(box_width - 2), ANSI_RESET);

    // Tools
    print_row(box_width, &format!(
        "  Tools     {}{}{}",
        ANSI_CHROME[3], info.tools.join("  "), ANSI_RESET
    ));

    if info.skill_count > 0 {
        print_row(box_width, &format!(
            "  Skills    {}{} loaded{}", ANSI_CHROME[3], info.skill_count, ANSI_RESET
        ));
    }

    // Separator
    println!("  {}├{}┤{}", c[8], "─".repeat(box_width - 2), ANSI_RESET);

    // Commands
    print_row(box_width, &format!("  {}/help commands  /graph show  /cost  /why <error>{}", ANSI_DIM, ANSI_RESET));
    print_row(box_width, &format!("  {}Ctrl+C quit  ↑↓ scroll  Esc interrupt{}", ANSI_DIM, ANSI_RESET));

    // Bottom border in chrome gradient
    println!("  {}", chrome_gradient_ansi(&format!("╰{border}╯")));
    println!();
}

fn print_row(box_width: usize, content: &str) {
    let visible_len = strip_ansi_len(content);
    let pad = box_width.saturating_sub(visible_len + 4);
    let c8 = ANSI_CHROME[8];
    println!("  {c8}│{ANSI_RESET}{content}{}{c8}│{ANSI_RESET}", " ".repeat(pad));
}

fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' { in_escape = false; }
        } else {
            len += 1;
        }
    }
    len
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max { path.to_string() }
    else { format!("...{}", &path[path.len() - max + 3..]) }
}

fn terminal_width() -> usize {
    crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80)
}

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
