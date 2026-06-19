//! TUI rendering. Draws the header, the three panes (clients / log /
//! stats), the bottom hotkey bar, and the detail/help modals.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Sparkline, Wrap,
};
use ratatui::Frame;

use super::app::{InputMode, TuiApp};
use crate::app::state::{ActivityKind, AppState, ClientStatus};
use crate::fun::xp;
use crate::ui::{Palette, Rgb};
use crate::util::{signal, time};

fn col(c: Rgb) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

fn kind_color(kind: ActivityKind, p: &Palette) -> Color {
    match kind {
        ActivityKind::Kick | ActivityKind::Bad => col(p.alert),
        ActivityKind::Warn => Color::Yellow,
        ActivityKind::NewClient => Color::Cyan,
        ActivityKind::TargetFound | ActivityKind::Good => Color::Green,
        ActivityKind::Info => Color::Gray,
    }
}

/// Top-level draw entry point.
pub fn draw(f: &mut Frame, st: &AppState, app: &mut TuiApp) {
    let palette = Palette::by_name(&st.theme);
    let area = f.area();

    if st.boss_mode {
        super::boss_mode::render(f, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    draw_header(f, chunks[0], st, &palette);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(42),
            Constraint::Percentage(30),
        ])
        .split(chunks[1]);

    draw_clients(f, body[0], st, app, &palette);
    draw_log(f, body[1], st, app, &palette);
    draw_stats(f, body[2], st, &palette);
    draw_footer(f, chunks[2], st, &palette);

    if app.show_detail {
        draw_detail(f, area, st, app, &palette);
    }
    if app.show_help {
        draw_help(f, area, &palette);
    }
    if let InputMode::Nickname(buf) = &app.input {
        draw_nickname_prompt(f, area, buf, &palette);
    }
}

fn draw_header(f: &mut Frame, area: Rect, st: &AppState, p: &Palette) {
    let (ssid, bssid, ch, enc) = match st.target() {
        Some(t) => (
            t.ssid.clone(),
            t.bssid.map(|b| b.to_string()).unwrap_or_else(|| "-".into()),
            t.channel.map(|c| c.to_string()).unwrap_or_else(|| "-".into()),
            t.encryption.clone(),
        ),
        None => ("(scanning)".into(), "-".into(), "-".into(), "-".into()),
    };
    let mut spans = vec![
        Span::styled("KTO v3", Style::default().fg(col(p.primary)).add_modifier(Modifier::BOLD)),
        Span::raw("  │  "),
        Span::styled(format!("{ssid} ({bssid})"), Style::default().fg(Color::White)),
        Span::raw(format!("  │  ch {ch}  │  {enc}  │  ")),
        Span::styled(time::clock_now(), Style::default().fg(Color::Gray)),
    ];
    if let Some(u) = &st.update_available {
        spans.push(Span::styled(
            format!("   ▲ v{} available", u.version),
            Style::default().fg(Color::Yellow),
        ));
    }
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(col(p.accent)));
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn draw_clients(f: &mut Frame, area: Rect, st: &AppState, app: &TuiApp, p: &Palette) {
    let mut macs: Vec<_> = st.target().map(|t| t.clients.keys().copied().collect()).unwrap_or_default();
    macs.sort_by_key(|m: &crate::util::MacAddr| m.octets());

    let items: Vec<ListItem> = macs
        .iter()
        .filter_map(|m| st.target().and_then(|t| t.clients.get(m)))
        .map(|c| {
            let tier_color = match signal::SignalTier::from_rssi(c.rssi) {
                signal::SignalTier::Strong => Color::Green,
                signal::SignalTier::Moderate => Color::Yellow,
                signal::SignalTier::Weak => col(p.alert),
            };
            let glyph_color = match c.status {
                ClientStatus::Active => col(p.primary),
                ClientStatus::Whitelisted => Color::DarkGray,
                ClientStatus::Gone => Color::DarkGray,
            };
            let l1 = Line::from(vec![
                Span::styled(format!("{} ", c.status.glyph()), Style::default().fg(glyph_color)),
                Span::styled(c.mac.short(), Style::default().fg(Color::White)),
            ]);
            let l2 = Line::from(vec![Span::raw("  "), Span::styled(c.display_name(), Style::default().fg(Color::Gray))]);
            let l3 = Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:>4} dBm ", c.rssi), Style::default().fg(tier_color)),
                Span::styled(signal::bar(c.rssi), Style::default().fg(tier_color)),
            ]);
            ListItem::new(vec![l1, l2, l3])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" CLIENTS ")
        .border_style(Style::default().fg(col(p.accent)));
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    if !macs.is_empty() {
        state.select(Some(app.selected.min(macs.len() - 1)));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_log(f: &mut Frame, area: Rect, st: &AppState, app: &mut TuiApp, p: &Palette) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ACTIVITY LOG ")
        .border_style(Style::default().fg(col(p.accent)));

    // Matrix-rain easter egg overrides the log while active.
    if let Some(rain) = app.rain.as_mut() {
        if rain.is_active() {
            let inner = block.inner(area);
            f.render_widget(block, area);
            rain.tick();
            rain.render(f, inner);
            return;
        } else {
            app.rain = None;
        }
    }

    let height = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = st
        .activity
        .iter()
        .rev()
        .take(height)
        .map(|a| {
            Line::from(vec![
                Span::styled(format!("{} ", time::clock(a.at)), Style::default().fg(Color::DarkGray)),
                Span::styled(a.message.clone(), Style::default().fg(kind_color(a.kind, p))),
            ])
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    f.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: true }), area);
}

fn draw_stats(f: &mut Frame, area: Rect, st: &AppState, p: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    let s = &st.stats;
    let streak_line = match st.streak.current_tier() {
        Some(t) => format!("Kill streak  {}  {}", st.streak.current, t.name),
        None => format!("Kill streak  {}", st.streak.current),
    };
    let lines = vec![
        kv("Session time", &time::hms(s.elapsed_secs())),
        kv("Total kicks", &s.total_kicks.to_string()),
        kv("Clients seen", &st.total_client_count().to_string()),
        kv("Active now", &st.active_client_count().to_string()),
        kv("Bursts/min", &format!("{:.1}", s.kicks_per_min())),
        Line::from(""),
        Line::from(Span::styled(streak_line, Style::default().fg(col(p.primary)).add_modifier(Modifier::BOLD))),
        Line::from(""),
        kv("Status", st.status.label()),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" STATS ")
        .border_style(Style::default().fg(col(p.accent)));
    f.render_widget(Paragraph::new(lines).block(block), rows[0]);

    // Kick-rate sparkline.
    let spark_data: Vec<u64> = s.kick_sparkline(30);
    let spark = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(" kicks/min "))
        .data(&spark_data)
        .style(Style::default().fg(col(p.primary)));
    f.render_widget(spark, rows[1]);

    // XP / level gauge.
    let level = st.xp.level();
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(format!(" Lv {level} - {} ", xp::title_for_level(level))))
        .gauge_style(Style::default().fg(col(p.primary)))
        .ratio(st.xp.progress().clamp(0.0, 1.0));
    f.render_widget(gauge, rows[2]);
}

fn draw_footer(f: &mut Frame, area: Rect, st: &AppState, p: &Palette) {
    let hints = "[q]Quit [p]Pause [s]Sweep [a]Aggro [w]Whitelist [n]Nick [e]Export [?]Help [F12]Boss";
    let status = format!("[{}]", st.status.label());
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(col(p.accent)));
    let line = Line::from(vec![
        Span::styled(status, Style::default().fg(col(p.primary)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(hints, Style::default().fg(Color::Gray)),
    ]);
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_detail(f: &mut Frame, area: Rect, st: &AppState, app: &TuiApp, p: &Palette) {
    let Some(mac) = app.selected_macs_sorted(st).get(app.selected).copied() else { return };
    let Some(c) = st.target().and_then(|t| t.clients.get(&mac)) else { return };

    let rect = centered_rect(60, 70, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" CLIENT DETAIL ")
        .border_style(Style::default().fg(col(p.primary)));

    let samples = c.rssi_samples(60);
    let spark = signal::sparkline(&samples);
    let lines = vec![
        kv("MAC", &c.mac.to_string()),
        kv("Vendor", c.vendor.as_deref().unwrap_or("-")),
        kv("Nickname", c.nickname.as_deref().unwrap_or("- [n to set]")),
        kv("OS guess", c.os_guess.as_deref().unwrap_or("-")),
        Line::from(""),
        kv("Signal", &format!("{} dBm  {}", c.rssi, signal::bar(c.rssi))),
        kv("First seen", &time::ago(c.first_seen)),
        kv("Last seen", &time::clock(c.last_seen)),
        kv("Times kicked", &c.kick_count.to_string()),
        Line::from(""),
        kv("Probe SSIDs", &c.probe_ssids.iter().cloned().collect::<Vec<_>>().join(", ")),
        Line::from(""),
        Line::from(Span::styled(format!("Signal: {spark}"), Style::default().fg(Color::Green))),
        Line::from(""),
        Line::from(Span::styled("[w]Whitelist  [n]Nickname  [Esc]Back", Style::default().fg(Color::Gray))),
    ];
    f.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: true }), rect);
}

fn draw_help(f: &mut Frame, area: Rect, p: &Palette) {
    let rect = centered_rect(55, 70, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" HELP ")
        .border_style(Style::default().fg(col(p.primary)));
    let rows = [
        ("q / Ctrl+C", "Quit (exports on exit)"),
        ("p", "Pause / resume"),
        ("s", "Force a scan sweep"),
        ("a / b", "Toggle aggressive / broadcast"),
        ("w / n", "Whitelist / nickname selected"),
        ("e", "Export session now"),
        ("c", "Clear activity log"),
        ("↑/↓ j/k", "Navigate clients"),
        ("Enter", "Client detail"),
        ("Tab", "Cycle focus"),
        ("F12", "Boss mode"),
        ("? ", "Toggle this help"),
        ("↑↑↓↓←→←→BA", "… try it"),
    ];
    let lines: Vec<Line> = rows
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k:<12}"), Style::default().fg(col(p.primary))),
                Span::styled(*v, Style::default().fg(Color::Gray)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(block), rect);
}

fn draw_nickname_prompt(f: &mut Frame, area: Rect, buf: &str, p: &Palette) {
    let rect = centered_rect(40, 16, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Set nickname (Enter=ok, Esc=cancel) ")
        .border_style(Style::default().fg(col(p.primary)));
    let para = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(buf, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("█", Style::default().fg(col(p.primary))),
    ]))
    .block(block);
    f.render_widget(para, rect);
}

fn kv(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<13}"), Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

/// A centered rectangle `pct_x` × `pct_y` percent of `area`.
fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(v[1])[1]
}

// Helper used by the detail modal to resolve the selected client.
impl TuiApp {
    fn selected_macs_sorted(&self, st: &AppState) -> Vec<crate::util::MacAddr> {
        let mut macs: Vec<_> = st.target().map(|t| t.clients.keys().copied().collect()).unwrap_or_default();
        macs.sort_by_key(|m: &crate::util::MacAddr| m.octets());
        macs
    }
}

#[allow(unused)]
fn _alignment_marker() -> Alignment {
    Alignment::Left
}
