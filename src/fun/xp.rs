//! XP and leveling.
//!
//! Level curve: `threshold(L) = round(100 * L^1.5)` XP is the amount required to
//! *attain* level `L`. The current level is the highest `L` whose threshold is
//! satisfied (floored at 1).

/// Fixed XP awards for game actions.
pub mod award {
    pub const FIRST_CLIENT: u64 = 50;
    pub const KICK_BURST: u64 = 10;
    pub const HANDSHAKE: u64 = 500;
    pub const PMKID: u64 = 300;
    pub const ROGUE_AP: u64 = 100;
    pub const SESSION_HOUR: u64 = 200;
}

#[derive(Debug, Clone, Default)]
pub struct XpState {
    pub total: u64,
}

/// XP required to attain a given level. `threshold(0) == 0`.
pub fn threshold(level: u32) -> u64 {
    if level == 0 {
        return 0;
    }
    (100.0 * (level as f64).powf(1.5)).round() as u64
}

/// The current level for a given XP total (minimum 1).
pub fn level_for_xp(xp: u64) -> u32 {
    let mut level = 0u32;
    while threshold(level + 1) <= xp {
        level += 1;
    }
    level.max(1)
}

/// Honorific for a level. Uses the highest milestone <= level.
pub fn title_for_level(level: u32) -> &'static str {
    const TITLES: &[(u32, &str)] = &[
        (1, "War-Driver"),
        (2, "Packet Wrangler"),
        (3, "Frame Flipper"),
        (4, "Deauth Daemon"),
        (5, "802.11 Assassin"),
        (6, "Spectrum Ghost"),
        (7, "RF Reaper"),
        (8, "Channel Phantom"),
        (9, "BSSID Banisher"),
        (10, "WiFi Warlord"),
        (15, "The Disassociator"),
        (20, "Null-Packet Nightmare"),
        (30, "Ethereal Overlord"),
        (50, "Pwnage Incarnate"),
    ];
    let mut title = TITLES[0].1;
    for &(lvl, name) in TITLES {
        if level >= lvl {
            title = name;
        } else {
            break;
        }
    }
    title
}

impl XpState {
    pub fn level(&self) -> u32 {
        level_for_xp(self.total)
    }

    pub fn title(&self) -> &'static str {
        title_for_level(self.level())
    }

    /// Progress through the current level, in `0.0..=1.0`.
    pub fn progress(&self) -> f64 {
        let level = self.level();
        let mut lower = threshold(level);
        if self.total < lower {
            lower = 0; // forced-minimum level-1 case
        }
        let upper = threshold(level + 1);
        if upper <= lower {
            return 0.0;
        }
        ((self.total - lower) as f64 / (upper - lower) as f64).clamp(0.0, 1.0)
    }

    /// XP remaining until the next level.
    pub fn to_next_level(&self) -> u64 {
        threshold(self.level() + 1).saturating_sub(self.total)
    }

    /// Add XP; returns `Some(new_level)` if at least one level was gained.
    pub fn add(&mut self, amount: u64) -> Option<u32> {
        let before = self.level();
        self.total = self.total.saturating_add(amount);
        let after = self.level();
        (after > before).then_some(after)
    }

    /// Add XP scaled by an active multiplier (e.g. the Konami 2x window).
    pub fn add_scaled(&mut self, amount: u64, multiplier: f64) -> Option<u32> {
        let scaled = (amount as f64 * multiplier).round() as u64;
        self.add(scaled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_match_spec_anchors() {
        assert_eq!(threshold(1), 100);
        assert_eq!(threshold(10), 3162);
    }

    #[test]
    fn level_floor_is_one() {
        assert_eq!(level_for_xp(0), 1);
        assert_eq!(level_for_xp(50), 1);
    }

    #[test]
    fn level_boundaries() {
        assert_eq!(level_for_xp(threshold(10)), 10);
        assert_eq!(level_for_xp(threshold(10) - 1), 9);
    }

    #[test]
    fn add_reports_levelups() {
        let mut xp = XpState::default();
        assert_eq!(xp.add(50), None);
        let lvl = xp.add(10_000);
        assert!(lvl.is_some());
    }

    #[test]
    fn progress_in_range() {
        let mut xp = XpState::default();
        xp.total = 200;
        let p = xp.progress();
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn titles_pick_highest_milestone() {
        assert_eq!(title_for_level(1), "War-Driver");
        assert_eq!(title_for_level(12), "WiFi Warlord");
        assert_eq!(title_for_level(15), "The Disassociator");
        assert_eq!(title_for_level(60), "Pwnage Incarnate");
    }
}
