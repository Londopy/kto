//! Colored CLI output (output format).

use console::style;

use crate::app::state::{Activity, ActivityKind};
use crate::util::time;

/// `[x]` prefix tag for a kind.
fn tag(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::TargetFound | ActivityKind::Good => "[+]",
        ActivityKind::Kick | ActivityKind::Bad => "[!]",
        ActivityKind::Warn => "[!]",
        ActivityKind::NewClient | ActivityKind::Info => "[*]",
    }
}

/// Render one activity entry to a string, optionally colored.
pub fn format_activity(entry: &Activity, no_color: bool) -> String {
    let clock = time::clock(entry.at);
    let tag = tag(entry.kind);
    let line = format!("{clock}  {tag} {}", entry.message);
    if no_color {
        return line;
    }
    let styled = match entry.kind {
        ActivityKind::Kick | ActivityKind::Bad => style(line).red(),
        ActivityKind::Warn => style(line).yellow(),
        ActivityKind::NewClient => style(line).cyan(),
        ActivityKind::TargetFound | ActivityKind::Good => style(line).green(),
        ActivityKind::Info => style(line).dim(),
    };
    styled.to_string()
}

/// Print one activity entry.
pub fn print_activity(entry: &Activity, no_color: bool) {
    println!("{}", format_activity(entry, no_color));
}

/// Print a startup info line: `13:42:01 [*] Interface : wlan0mon`.
pub fn info_line(label: &str, value: &str, no_color: bool) {
    let line = format!("{}  [*] {label:<11}: {value}", time::clock_now());
    if no_color {
        println!("{line}");
    } else {
        println!("{}", style(line).dim());
    }
}
