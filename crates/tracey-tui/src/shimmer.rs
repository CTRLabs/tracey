//! Shimmer animation — adapted from Codex's shimmer.rs.
//! Creates a time-based sweeping highlight that moves across text,
//! producing a liquid chrome metallic effect.
//!
//! Source: codex-rs/tui/src/shimmer.rs (OpenAI Codex, Apache 2.0)

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();

fn elapsed_since_start() -> Duration {
    let start = PROCESS_START.get_or_init(Instant::now);
    start.elapsed()
}

/// Create shimmering spans — a highlight band sweeps across the text.
/// Uses violet/silver palette for our liquid chrome aesthetic.
pub fn shimmer_spans(text: &str) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let padding = 10usize;
    let period = chars.len() + padding * 2;
    let sweep_seconds = 2.0f32;
    let pos_f =
        (elapsed_since_start().as_secs_f32() % sweep_seconds) / sweep_seconds * (period as f32);
    let band_half_width = 5.0;

    // Our liquid chrome colors: silver highlight sweeping over violet base
    let base_color: (u8, u8, u8) = (139, 92, 246);     // violet
    let highlight_color: (u8, u8, u8) = (230, 230, 240); // silver

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(chars.len());
    for (i, ch) in chars.iter().enumerate() {
        let i_pos = i as isize + padding as isize;
        let pos = pos_f as isize;
        let dist = (i_pos - pos).abs() as f32;

        let t = if dist <= band_half_width {
            let x = std::f32::consts::PI * (dist / band_half_width);
            0.5 * (1.0 + x.cos())
        } else {
            0.0
        };

        let highlight = t.clamp(0.0, 1.0);
        let (r, g, b) = blend(highlight_color, base_color, highlight * 0.9);
        let style = Style::default()
            .fg(Color::Rgb(r, g, b))
            .add_modifier(Modifier::BOLD);
        spans.push(Span::styled(ch.to_string(), style));
    }
    spans
}

/// Shimmer for causal graph nodes — pulses between silver and violet
pub fn shimmer_node(text: &str) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let pulse_seconds = 1.5f32;
    let t = (elapsed_since_start().as_secs_f32() % pulse_seconds) / pulse_seconds;
    let intensity = (t * std::f32::consts::PI * 2.0).sin() * 0.5 + 0.5;

    let violet: (u8, u8, u8) = (139, 92, 246);
    let silver: (u8, u8, u8) = (210, 210, 225);

    let (r, g, b) = blend(silver, violet, intensity);
    let style = Style::default().fg(Color::Rgb(r, g, b)).add_modifier(Modifier::BOLD);

    chars.iter().map(|ch| Span::styled(ch.to_string(), style)).collect()
}

/// Blend two RGB colors with alpha (0.0 = bg, 1.0 = fg)
pub fn blend(fg: (u8, u8, u8), bg: (u8, u8, u8), alpha: f32) -> (u8, u8, u8) {
    let r = (fg.0 as f32 * alpha + bg.0 as f32 * (1.0 - alpha)) as u8;
    let g = (fg.1 as f32 * alpha + bg.1 as f32 * (1.0 - alpha)) as u8;
    let b = (fg.2 as f32 * alpha + bg.2 as f32 * (1.0 - alpha)) as u8;
    (r, g, b)
}

/// Spinner dot with shimmer (like Codex's • spinner)
pub fn spinner_dot() -> Span<'static> {
    let spans = shimmer_spans("◉");
    spans.into_iter().next().unwrap_or_else(|| "◉".into())
}
