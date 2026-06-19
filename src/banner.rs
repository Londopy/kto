//! Startup banner.

/// The ASCII logo. `fsociety` glyph is appended when `reason == 1`.
pub fn banner(reason: u16) -> String {
    let art = r#"
    ╦╔═ ╔╦╗ ╔═╗     Kick Them Out
    ╠╩╗  ║  ║ ║     v3 · Rust
    ╩ ╩  ╩  ╚═╝     authorized testing only
"#;
    let mut out = art.to_string();
    if reason == 1 {
        out.push_str("    ·· fsociety ··\n");
    }
    out
}

/// Print the banner to stdout, respecting `--no-color`.
pub fn print_banner(no_color: bool, reason: u16) {
    let text = banner(reason);
    if no_color {
        println!("{text}");
    } else {
        // crimson, matching the default theme primary
        println!("\x1b[38;2;233;69;96m{text}\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsociety_only_on_reason_1() {
        assert!(banner(1).contains("fsociety"));
        assert!(!banner(7).contains("fsociety"));
    }
}
