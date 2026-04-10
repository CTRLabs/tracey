/// Pre-TUI welcome banner — uses basic ANSI colors (works everywhere)
pub fn print_welcome_banner(info: &BannerInfo) {
    let m = "\x1b[35m";     // magenta
    let bm = "\x1b[1;95m";  // bright magenta
    let c = "\x1b[36m";     // cyan
    let w = "\x1b[1;37m";   // bold white
    let d = "\x1b[90m";     // dim
    let g = "\x1b[32m";     // green
    let rst = "\x1b[0m";

    println!();
    println!("  {bm}◆ tracey{rst} v{} ─── {d}{} · {}{rst}",
        env!("CARGO_PKG_VERSION"), info.provider, info.model);
    println!();

    if info.graph_nodes > 0 {
        println!("  {c}◈{rst} Graph: {w}{} nodes, {} edges{rst}", info.graph_nodes, info.graph_edges);
    }
    if !info.languages.is_empty() {
        println!("  {d}  Languages: {}{rst}", info.languages);
    }
    println!("  {d}  Tools: {}{rst}", info.tools.join(", "));
    println!("  {d}  /help for commands{rst}");
    println!();
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
