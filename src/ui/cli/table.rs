//! In-place client table for `--live-table` (CLI only).

use console::style;

use crate::app::state::AppState;
use crate::util::signal;

/// Render the current client table as a multi-line string.
pub fn render(st: &AppState, no_color: bool) -> String {
    let mut out = String::new();
    let header = match st.target() {
        Some(t) => format!(
            "Target {}  ch {}  {}   kicks {}  active {}",
            t.ssid,
            t.channel.map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
            t.encryption,
            st.stats.total_kicks,
            st.active_client_count(),
        ),
        None => "Scanning…".into(),
    };
    out.push_str(&header);
    out.push('\n');
    out.push_str(&format!(
        "{:<3} {:<17} {:<16} {:>5}  {:<8} {:>5}\n",
        "", "MAC", "Vendor/Nick", "dBm", "Signal", "Kicks"
    ));

    if let Some(t) = st.target() {
        let mut clients: Vec<_> = t.clients.values().collect();
        clients.sort_by_key(|c| c.mac.octets());
        for c in clients {
            let bar = signal::bar(c.rssi);
            let row = format!(
                "{:<3} {:<17} {:<16} {:>5}  {:<8} {:>5}",
                c.status.glyph(),
                c.mac.to_string(),
                truncate(&c.display_name(), 16),
                c.rssi,
                bar,
                c.kick_count,
            );
            if no_color {
                out.push_str(&row);
            } else {
                let colored = match signal::SignalTier::from_rssi(c.rssi) {
                    signal::SignalTier::Strong => style(row).green(),
                    signal::SignalTier::Moderate => style(row).yellow(),
                    signal::SignalTier::Weak => style(row).red(),
                };
                out.push_str(&colored.to_string());
            }
            out.push('\n');
        }
    }
    out
}

/// Clear the screen and redraw the table.
pub fn draw(st: &AppState, no_color: bool) {
    // ANSI: move cursor home + clear screen.
    print!("\x1b[2J\x1b[H");
    print!("{}", render(st, no_color));
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}
