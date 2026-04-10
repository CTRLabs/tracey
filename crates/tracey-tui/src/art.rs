//! Causal graph ASCII art — uses ONLY box-drawing + symbols that render cleanly.
//! NO block chars (████), NO braille (⣿). Confirmed working: ╭╮╰╯│─◉◆◇▸○●

/// The Tracey logo — a causal DAG showing the reasoning flow.
/// This IS the product: parse → reason/act → verify → resolve.
pub const GRAPH_LOGO: &str = "\
                    ╭───────╮
             ╭──────│ parse │──────╮
             │      ╰───────╯      │
        ╭────▼───╮            ╭────▼───╮
        │ reason │            │  act   │
        ╰────┬───╯            ╰────┬───╯
             │      ╭───────╮      │
             ╰──────│verify │──────╯
                    ╰───┬───╯
                        │
                   ╭────▼────╮
                   │ resolve │
                   ╰─────────╯";

/// Compact version of the graph logo (for narrow terminals)
pub const GRAPH_LOGO_COMPACT: &str = "\
  ◉ parse
  ├──▸ ◉ reason
  ├──▸ ◉ act
  └──▸ ◉ verify
       └──▸ ◉ resolve";

/// The text identity
pub const TRACEY_TEXT: &str = "T  R  A  C  E  Y";
pub const TAGLINE: &str = "tracing causal connections";

/// Animated graph trace frames (our signature animation)
pub const GRAPH_TRACE_FRAMES: [&str; 8] = [
    "◉",
    "◉──",
    "◉────▸",
    "◉────▸ ◉",
    "◉────▸ ◉──",
    "◉────▸ ◉────▸",
    "◉────▸ ◉────▸ ◉",
    "◉────▸ ◉────▸ ◉──▸",
];

/// Pulsing node frames (for tool calls)
pub const PULSE_FRAMES: [&str; 6] = ["◇", "◈", "◆", "●", "◆", "◈"];

/// Verification animation
pub const VERIFY_FRAMES: [&str; 6] = ["○", "◎", "◉", "●", "◉", "◎"];

/// Edge animation (for editing)
pub const EDGE_FRAMES: [&str; 4] = ["───▸", "╌╌╌▸", "───▸", "━━━▸"];

/// Wizard step graph — builds up as steps complete.
/// Each entry: (node box, label after arrow)
/// Renders as a vertical causal chain.
pub struct WizardGraph {
    steps: Vec<WizardStep>,
}

pub struct WizardStep {
    pub name: &'static str,
    pub result: Option<String>,
    pub complete: bool,
}

impl WizardGraph {
    pub fn new() -> Self {
        Self {
            steps: vec![
                WizardStep { name: "provider", result: None, complete: false },
                WizardStep { name: "auth", result: None, complete: false },
                WizardStep { name: "model", result: None, complete: false },
                WizardStep { name: "ready", result: None, complete: false },
            ],
        }
    }

    pub fn complete_step(&mut self, index: usize, result: String) {
        if let Some(step) = self.steps.get_mut(index) {
            step.result = Some(result);
            step.complete = true;
        }
    }

    /// Render the current state of the wizard graph
    pub fn render(&self) -> String {
        let silver = "\x1b[38;5;252m";
        let violet = "\x1b[38;5;135m";
        let lavender = "\x1b[38;5;183m";
        let green = "\x1b[38;5;35m";
        let dim = "\x1b[90m";
        let bold = "\x1b[1m";
        let rst = "\x1b[0m";

        let mut out = String::new();

        for (i, step) in self.steps.iter().enumerate() {
            if i > 0 && self.steps[i - 1].complete {
                out.push_str(&format!("  {violet}     │{rst}\n"));
            } else if i > 0 {
                break; // Don't show future steps
            }

            if step.complete {
                let result = step.result.as_deref().unwrap_or("");
                out.push_str(&format!(
                    "  {silver}╭────────╮{rst}\n  {silver}│{rst} {lavender}{bold}{:<6}{rst} {silver}│{rst}───▸ {green}{result} ✓{rst}\n  {silver}╰────┬───╯{rst}\n",
                    step.name
                ));
            } else if i == 0 || self.steps.get(i.wrapping_sub(1)).map_or(false, |s| s.complete) {
                // Current step (active)
                out.push_str(&format!(
                    "  {violet}╭────────╮{rst}\n  {violet}│{rst} {lavender}{bold}{:<6}{rst} {violet}│{rst}───▸ {dim}...{rst}\n  {violet}╰────────╯{rst}\n",
                    step.name
                ));
            }
        }

        out
    }

    /// Animate a single step node appearing
    pub fn animate_step_appear(name: &str, result: &str) {
        use std::io::Write;
        let silver = "\x1b[38;5;252m";
        let violet = "\x1b[38;5;135m";
        let lavender = "\x1b[38;5;183m";
        let green = "\x1b[38;5;35m";
        let bold = "\x1b[1m";
        let rst = "\x1b[0m";

        let mut out = std::io::stdout();

        // Edge from previous
        write!(out, "  {violet}     │{rst}\n").ok();
        out.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(40));

        // Top border
        write!(out, "  {silver}╭────────╮{rst}\n").ok();
        out.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(30));

        // Node content + result
        write!(out, "  {silver}│{rst} {lavender}{bold}{name:<6}{rst} {silver}│{rst}").ok();
        out.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(40));

        write!(out, "───▸ ").ok();
        out.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(30));

        write!(out, "{green}{result} ✓{rst}\n").ok();
        out.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Bottom border
        write!(out, "  {silver}╰────┬───╯{rst}\n").ok();
        out.flush().ok();
    }
}
