//! Boss Mode decoy spreadsheet.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

const ROWS: &[[&str; 5]] = &[
    ["Marketing", "$142,500", "$156,000", "$149,230", "-$6,770"],
    ["Engineering", "$890,000", "$920,000", "$903,115", "-$16,885"],
    ["Sales", "$340,000", "$365,000", "$372,440", "+$7,440"],
    ["HR", "$210,000", "$215,000", "$208,900", "-$6,100"],
    ["Operations", "$445,000", "$460,000", "$451,220", "-$8,780"],
];

/// Render the full-screen budget decoy. Innocuous-looking on a glance.
pub fn render(f: &mut Frame, area: Rect) {
    let header = Row::new(["Department", "Q1 Budget", "Q2 Budget", "Q3 Actual", "Variance"])
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

    let rows = ROWS.iter().map(|r| {
        let variance_color = if r[4].starts_with('+') { Color::Green } else { Color::Red };
        Row::new(vec![
            Span::raw(r[0]),
            Span::raw(r[1]),
            Span::raw(r[2]),
            Span::raw(r[3]),
            Span::styled(r[4], Style::default().fg(variance_color)),
        ])
    });

    let widths = [
        ratatui::layout::Constraint::Length(14),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(12),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Budget Tracker - Q3 FY25  (Sheet1) ")
            .title_style(Style::default().fg(Color::White)),
    );

    f.render_widget(table, area);

    // Subtle hint line at the very bottom.
    let hint = Paragraph::new(Line::from(Span::styled(
        "Ready   |   F12 to return",
        Style::default().fg(Color::DarkGray),
    )));
    let hint_area = Rect { x: area.x, y: area.y + area.height.saturating_sub(1), width: area.width, height: 1 };
    f.render_widget(hint, hint_area);
}
