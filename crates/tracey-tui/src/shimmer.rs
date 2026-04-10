//! Shimmer animation — forked from Codex (Apache 2.0), adapted for Tracey.
//! Time-based sweeping highlight. Falls back to DIM/BOLD when no TrueColor.

use std::sync::OnceLock;
use std::time::{Duration, Instant};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::palette;

static START: OnceLock<Instant> = OnceLock::new();
fn elapsed() -> Duration { START.get_or_init(Instant::now).elapsed() }

/// Shimmer spans — liquid chrome sweep across text
pub fn shimmer_spans(text: &str) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() { return Vec::new(); }

    let padding = 10usize;
    let period = chars.len() + padding * 2;
    let sweep_secs = 2.0f32;
    let pos_f = (elapsed().as_secs_f32() % sweep_secs) / sweep_secs * (period as f32);
    let band = 5.0f32;

    let use_rgb = palette::color_level() == palette::ColorLevel::TrueColor;

    chars.iter().enumerate().map(|(i, ch)| {
        let dist = ((i as isize + padding as isize) - pos_f as isize).abs() as f32;
        let t = if dist <= band { 0.5 * (1.0 + (std::f32::consts::PI * dist / band).cos()) } else { 0.0 };

        let style = if use_rgb {
            let (r, g, b) = blend((230, 230, 240), (139, 92, 246), t * 0.9);
            Style::default().fg(Color::Rgb(r, g, b)).add_modifier(Modifier::BOLD)
        } else {
            // Codex fallback: just DIM/normal/BOLD
            if t < 0.2 { Style::default().add_modifier(Modifier::DIM) }
            else if t < 0.6 { Style::default().fg(palette::violet()) }
            else { Style::default().fg(palette::silver()).add_modifier(Modifier::BOLD) }
        };
        Span::styled(ch.to_string(), style)
    }).collect()
}

fn blend(fg: (u8, u8, u8), bg: (u8, u8, u8), a: f32) -> (u8, u8, u8) {
    ((fg.0 as f32 * a + bg.0 as f32 * (1.0 - a)) as u8,
     (fg.1 as f32 * a + bg.1 as f32 * (1.0 - a)) as u8,
     (fg.2 as f32 * a + bg.2 as f32 * (1.0 - a)) as u8)
}
