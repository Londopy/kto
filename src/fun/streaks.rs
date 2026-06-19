//! Kill-streak tracking.
//!
//! A streak increments on each kick and resets if more than
//! [`STREAK_TIMEOUT_SECS`] elapse between kicks.

use chrono::{DateTime, Duration, Utc};

/// Quiet period (seconds) that breaks a streak.
pub const STREAK_TIMEOUT_SECS: i64 = 30;

/// A milestone streak tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tier {
    pub threshold: u32,
    pub name: &'static str,
    pub color: &'static str,
    pub discord_suffix: &'static str,
}

/// All milestone tiers, ascending.
pub const TIERS: &[Tier] = &[
    Tier { threshold: 2, name: "Double Kill", color: "yellow", discord_suffix: "2x" },
    Tier { threshold: 3, name: "Triple Kill", color: "orange", discord_suffix: "3x 🔥" },
    Tier { threshold: 5, name: "Killing Spree", color: "red", discord_suffix: "spree 🔥" },
    Tier { threshold: 7, name: "Rampage", color: "bright_red", discord_suffix: "RAMPAGE 💀" },
    Tier { threshold: 10, name: "Dominating", color: "purple", discord_suffix: "DOMINATING 👑" },
    Tier { threshold: 15, name: "Unstoppable", color: "pink", discord_suffix: "UNSTOPPABLE ⚡" },
    Tier { threshold: 20, name: "Godlike", color: "gold", discord_suffix: "GODLIKE 🌟" },
    Tier { threshold: 25, name: "Holy Shit", color: "rainbow", discord_suffix: "HOLY SHIT 🤯" },
    Tier { threshold: 50, name: "Packet God", color: "explosion", discord_suffix: "PACKET GOD 💫" },
];

/// The active tier for a streak value (highest threshold <= streak), if any.
pub fn tier_for(streak: u32) -> Option<Tier> {
    TIERS.iter().rev().find(|t| streak >= t.threshold).copied()
}

/// Exact tier when a streak first hits a milestone, used to fire events.
fn exact_tier(streak: u32) -> Option<Tier> {
    TIERS.iter().find(|t| t.threshold == streak).copied()
}

#[derive(Debug, Clone, Default)]
pub struct StreakState {
    pub current: u32,
    pub best: u32,
    last_kick: Option<DateTime<Utc>>,
}

impl StreakState {
    /// Register a kick. Returns `Some(tier)` if this kick reaches a new
    /// milestone exactly (e.g. hitting 5 -> "Killing Spree").
    pub fn record_kick(&mut self) -> Option<Tier> {
        self.record_kick_at(Utc::now())
    }

    /// Testable variant taking an explicit timestamp.
    pub fn record_kick_at(&mut self, now: DateTime<Utc>) -> Option<Tier> {
        let broken = match self.last_kick {
            Some(prev) => now - prev > Duration::seconds(STREAK_TIMEOUT_SECS),
            None => false,
        };
        self.current = if broken { 1 } else { self.current + 1 };
        self.last_kick = Some(now);
        self.best = self.best.max(self.current);
        exact_tier(self.current)
    }

    /// Break the streak if the timeout has elapsed (call from a periodic tick).
    pub fn expire_if_idle(&mut self, now: DateTime<Utc>) {
        if let Some(prev) = self.last_kick {
            if now - prev > Duration::seconds(STREAK_TIMEOUT_SECS) {
                self.current = 0;
                self.last_kick = None;
            }
        }
    }

    /// Active tier name for the current streak, if any.
    pub fn current_tier(&self) -> Option<Tier> {
        tier_for(self.current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increments_and_reports_milestones() {
        let mut s = StreakState::default();
        let t0 = Utc::now();
        assert!(s.record_kick_at(t0).is_none()); // 1
        assert_eq!(s.record_kick_at(t0).unwrap().name, "Double Kill"); // 2
        assert!(s.record_kick_at(t0).unwrap().name == "Triple Kill"); // 3
        assert!(s.record_kick_at(t0).is_none()); // 4
        assert_eq!(s.record_kick_at(t0).unwrap().name, "Killing Spree"); // 5
        assert_eq!(s.current, 5);
        assert_eq!(s.best, 5);
    }

    #[test]
    fn resets_after_timeout() {
        let mut s = StreakState::default();
        let t0 = Utc::now();
        s.record_kick_at(t0);
        s.record_kick_at(t0); // streak 2
        let later = t0 + Duration::seconds(STREAK_TIMEOUT_SECS + 1);
        s.record_kick_at(later); // broken → back to 1
        assert_eq!(s.current, 1);
        assert_eq!(s.best, 2);
    }

    #[test]
    fn tier_lookup() {
        assert!(tier_for(1).is_none());
        assert_eq!(tier_for(4).unwrap().name, "Triple Kill");
        assert_eq!(tier_for(100).unwrap().name, "Packet God");
    }
}
