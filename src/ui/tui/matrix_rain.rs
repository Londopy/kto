//! Matrix-rain easter egg animation, rendered into the activity pane
//! for a few seconds after the Konami code.

use std::time::{Duration, Instant};

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const GLYPHS: &[char] = &[
    'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ﾅ', 'ﾆ', '0', '1', '7', 'Z', 'λ', '∆',
];

/// Animated rain state. `column_heads[i]` is the row index of the leading glyph
/// in column `i`.
pub struct MatrixRain {
    started: Instant,
    duration: Duration,
    column_heads: Vec<u16>,
    seed: u64,
}

impl MatrixRain {
    pub fn new(width: u16, duration: Duration) -> Self {
        let mut heads = Vec::with_capacity(width as usize);
        let mut seed = 0x9e3779b97f4a7c15u64;
        for _ in 0..width.max(1) {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            heads.push((seed >> 33) as u16 % 24);
        }
        MatrixRain { started: Instant::now(), duration, column_heads: heads, seed }
    }

    pub fn is_active(&self) -> bool {
        self.started.elapsed() < self.duration
    }

    fn next_rand(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.seed >> 33
    }

    /// Advance one frame.
    pub fn tick(&mut self) {
        for h in &mut self.column_heads {
            *h = h.wrapping_add(1);
        }
    }

    /// Render the rain into `area`.
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let w = area.width as usize;
        let h = area.height as usize;
        if w == 0 || h == 0 {
            return;
        }
        let mut lines: Vec<Line> = Vec::with_capacity(h);
        for row in 0..h {
            let mut spans = Vec::with_capacity(w);
            for col in 0..w {
                let head = *self.column_heads.get(col).unwrap_or(&0) as usize % (h + 8);
                let glyph = GLYPHS[(self.next_rand() as usize) % GLYPHS.len()];
                let color = if row == head {
                    Color::White
                } else if head >= row && head - row < 6 {
                    Color::Green
                } else {
                    Color::Rgb(0, 60, 0)
                };
                let ch = if (row + col + head) % 3 == 0 { glyph } else { ' ' };
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            }
            lines.push(Line::from(spans));
        }
        f.render_widget(Paragraph::new(lines), area);
    }
}
