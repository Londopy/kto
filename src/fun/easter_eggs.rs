//! Hidden flags and easter eggs (,,,).

use std::time::{Duration, Instant};

/// XP awarded for the `--swordfish` flag.
pub const SWORDFISH_XP: u64 = 9999;

/// Text printed by the hidden `--swordfish` flag, then the program exits.
pub fn swordfish_text() -> String {
    [
        "\"The password is 'swordfish'. I never guess a man's password.\"",
        "  — Costello (Hackers, 1995)",
        "",
        "  ┌─────────────────────────────────────────────────────────┐",
        "  │ HACK THE PLANET                                         │",
        "  │ HACK THE PLANET                                         │",
        "  │ HACK THE PLANET                                         │",
        "  └─────────────────────────────────────────────────────────┘",
        "",
        "Don't hack the planet. Hack test environments. With permission.",
    ]
    .join("\n")
}

/// Text printed by the hidden `--404` flag after a fake scan.
pub fn not_found_text() -> String {
    [
        "[-] Network not found.",
        "[-] AP not found.",
        "[-] Clients not found.",
        "[-] Motivation not found.",
        "[*] HTTP 404: WiFi not found. Try turning it off and on again.",
    ]
    .join("\n")
}

/// The Mr. Robot line shown once a session has run for ~60 minutes.
pub fn mr_robot_line() -> &'static str {
    "[~] \"Give me a hug.\" — Elliot Alderson"
}

/// Banner shown after a successful Konami entry.
pub fn konami_banner() -> String {
    [
        "★  CHEAT CODE ACTIVATED  ★",
        "   Unlimited Wi-Fi Ammo",
        "   (you already had unlimited Wi-Fi ammo)",
        "   XP x2 for 5 minutes",
    ]
    .join("\n")
}

/// How long the Konami 2x XP window lasts.
pub const KONAMI_BONUS: Duration = Duration::from_secs(5 * 60);

/// Abstract key tokens for the Konami detector, separate from any UI backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KonamiKey {
    Up,
    Down,
    Left,
    Right,
    B,
    A,
    Other,
}

const KONAMI_SEQUENCE: [KonamiKey; 10] = [
    KonamiKey::Up,
    KonamiKey::Up,
    KonamiKey::Down,
    KonamiKey::Down,
    KonamiKey::Left,
    KonamiKey::Right,
    KonamiKey::Left,
    KonamiKey::Right,
    KonamiKey::B,
    KonamiKey::A,
];

/// Tracks progress through the Konami sequence with a 3-second overall window.
#[derive(Debug, Default)]
pub struct KonamiDetector {
    progress: usize,
    started: Option<Instant>,
}

impl KonamiDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a key. Returns `true` exactly when the full sequence completes
    /// within the time window.
    pub fn feed(&mut self, key: KonamiKey) -> bool {
        let now = Instant::now();
        // Reset if the window expired.
        if let Some(started) = self.started {
            if now.duration_since(started) > Duration::from_secs(3) {
                self.progress = 0;
                self.started = None;
            }
        }
        if self.progress == 0 {
            self.started = Some(now);
        }
        if key == KONAMI_SEQUENCE[self.progress] {
            self.progress += 1;
            if self.progress == KONAMI_SEQUENCE.len() {
                self.progress = 0;
                self.started = None;
                return true;
            }
        } else {
            // Restart, but allow this key to begin a fresh sequence.
            self.progress = if key == KONAMI_SEQUENCE[0] { 1 } else { 0 };
            self.started = if self.progress == 1 { Some(now) } else { None };
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_sequence_triggers() {
        let mut d = KonamiDetector::new();
        let seq = KONAMI_SEQUENCE;
        for (i, k) in seq.iter().enumerate() {
            let done = d.feed(*k);
            if i < seq.len() - 1 {
                assert!(!done);
            } else {
                assert!(done);
            }
        }
    }

    #[test]
    fn wrong_key_resets() {
        let mut d = KonamiDetector::new();
        d.feed(KonamiKey::Up);
        d.feed(KonamiKey::Up);
        assert!(!d.feed(KonamiKey::A)); // breaks it
        // Now complete a fresh run.
        for k in KONAMI_SEQUENCE {
            d.feed(k);
        }
    }

    #[test]
    fn texts_present() {
        assert!(swordfish_text().contains("HACK THE PLANET"));
        assert!(not_found_text().contains("404"));
    }
}
