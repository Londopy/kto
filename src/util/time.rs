//! Timestamp and duration formatting helpers.

use chrono::{DateTime, Local, Utc};

/// `HH:MM:SS` clock for the current local time (used in log lines).
pub fn clock_now() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

/// `HH:MM:SS` for a given UTC instant, rendered in local time.
pub fn clock(ts: DateTime<Utc>) -> String {
    ts.with_timezone(&Local).format("%H:%M:%S").to_string()
}

/// A compact elapsed-time string like `00:04:32` from a number of seconds.
pub fn hms(total_secs: u64) -> String {
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Human "x ago" using the `humantime` crate, e.g. `4m 13s ago`.
pub fn ago(since: DateTime<Utc>) -> String {
    let secs = (Utc::now() - since).num_seconds().max(0) as u64;
    if secs == 0 {
        return "just now".to_string();
    }
    let d = std::time::Duration::from_secs(secs);
    // humantime renders "4m 13s"; we trim to two coarsest units for brevity.
    let full = humantime::format_duration(d).to_string();
    let trimmed: Vec<&str> = full.split_whitespace().take(2).collect();
    format!("{} ago", trimmed.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hms_formats_padded() {
        assert_eq!(hms(0), "00:00:00");
        assert_eq!(hms(272), "00:04:32");
        assert_eq!(hms(3661), "01:01:01");
    }
}
