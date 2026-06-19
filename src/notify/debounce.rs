//! Per-key debouncing so aggressive mode doesn't spam notifications.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks the last time each key fired and rate-limits accordingly.
pub struct Debouncer {
    last: HashMap<String, Instant>,
    window: Duration,
}

impl Debouncer {
    pub fn new(window: Duration) -> Self {
        Debouncer { last: HashMap::new(), window }
    }

    /// Returns `true` if an event with this key may fire now (and records it).
    pub fn allow(&mut self, key: &str) -> bool {
        self.allow_at(key, Instant::now())
    }

    fn allow_at(&mut self, key: &str, now: Instant) -> bool {
        match self.last.get(key) {
            Some(&prev) if now.duration_since(prev) < self.window => false,
            _ => {
                self.last.insert(key.to_string(), now);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_within_window() {
        let mut d = Debouncer::new(Duration::from_secs(10));
        let t0 = Instant::now();
        assert!(d.allow_at("kick:x", t0));
        assert!(!d.allow_at("kick:x", t0 + Duration::from_secs(2)));
        assert!(d.allow_at("kick:x", t0 + Duration::from_secs(11)));
        // distinct key is independent
        assert!(d.allow_at("kick:y", t0 + Duration::from_secs(2)));
    }
}
