fn main() {
    println!("\n=== TRACEY COLOR DIAGNOSTIC ===\n");
    
    // Test 1: Basic 16 colors
    println!("--- Basic 16 colors ---");
    for i in 0..8 {
        print!("\x1b[3{i}m color{i} \x1b[0m");
    }
    println!();
    for i in 0..8 {
        print!("\x1b[9{i}m bright{i} \x1b[0m");
    }
    println!("\n");
    
    // Test 2: 256-color violet range
    println!("--- 256-color (violet range) ---");
    let violet_colors = [53, 54, 55, 56, 57, 93, 97, 98, 99, 128, 129, 135, 141, 147, 183, 189, 225, 252, 253];
    for c in &violet_colors {
        print!("\x1b[38;5;{c}m [{c:3}] \x1b[0m");
    }
    println!("\n");
    
    // Test 3: True color (24-bit)
    println!("--- True color (24-bit RGB) ---");
    let rgb_tests = [
        (139, 92, 246, "violet"),
        (230, 220, 255, "lavender"),
        (252, 252, 252, "silver"),
        (255, 0, 0, "red"),
        (0, 255, 0, "green"),
    ];
    for (r, g, b, name) in &rgb_tests {
        print!("\x1b[38;2;{r};{g};{b}m {name} \x1b[0m");
    }
    println!("\n");
    
    // Test 4: Background colors
    println!("--- Background test ---");
    print!("\x1b[48;5;55m\x1b[38;5;252m violet bg + silver text \x1b[0m ");
    print!("\x1b[48;5;17m\x1b[38;5;135m dark bg + violet text \x1b[0m");
    println!("\n");
    
    // Test 5: Bold + color
    println!("--- Bold + color ---");
    print!("\x1b[1m\x1b[38;5;135m bold violet \x1b[0m ");
    print!("\x1b[1m\x1b[38;5;183m bold lavender \x1b[0m ");
    print!("\x1b[1m\x1b[38;5;252m bold silver \x1b[0m");
    println!("\n");
    
    // Test 6: What Tracey SHOULD look like
    println!("--- What Tracey should look like ---\n");
    let gradient = [252, 251, 189, 183, 141, 135];
    let logo = [
        "  ████████╗██████╗  █████╗  ██████╗███████╗██╗   ██╗",
        "  ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝",
        "     ██║   ██████╔╝███████║██║     █████╗   ╚████╔╝ ",
        "     ██║   ██╔══██╗██╔══██║██║     ██╔══╝    ╚██╔╝  ",
        "     ██║   ██║  ██║██║  ██║╚██████╗███████╗   ██║   ",
        "     ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝   ╚═╝   ",
    ];
    for (i, line) in logo.iter().enumerate() {
        println!("\x1b[38;5;{}m{}\x1b[0m", gradient[i], line);
    }
    println!();
    println!("  \x1b[38;5;183m    ◉\x1b[38;5;97m──╌╌──▸\x1b[38;5;183m ◉\x1b[38;5;97m──╌╌──▸\x1b[38;5;183m ◉\x1b[0m");
    println!("  \x1b[38;5;97m              └──╌╌──▸\x1b[38;5;183m ◉\x1b[0m");
    
    // Environment info
    println!("\n--- Environment ---");
    println!("TERM={}", std::env::var("TERM").unwrap_or("(not set)".into()));
    println!("COLORTERM={}", std::env::var("COLORTERM").unwrap_or("(not set)".into()));
    println!("TERM_PROGRAM={}", std::env::var("TERM_PROGRAM").unwrap_or("(not set)".into()));
    println!("TMUX={}", if std::env::var("TMUX").is_ok() { "yes" } else { "no" });
    println!();
}
