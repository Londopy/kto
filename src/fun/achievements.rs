//! Achievement catalogue and unlock evaluation.
//!
//! The persistent *store* of unlocked ids lives in
//! [`crate::app::achievements::AchievementStore`]; this module owns the
//! catalogue and the conditions.

use chrono::{Local, Timelike};

use crate::app::state::AppState;

/// A single achievement definition.
#[derive(Debug, Clone, Copy)]
pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub xp_bonus: u64,
}

pub const ACHIEVEMENTS: &[Achievement] = &[
    Achievement { id: "first_blood", name: "First Blood", desc: "First ever kick", xp_bonus: 100 },
    Achievement { id: "century", name: "Century", desc: "100 total kicks (lifetime)", xp_bonus: 200 },
    Achievement { id: "rogue_hunter", name: "Rogue Hunter", desc: "Detect a rogue AP", xp_bonus: 150 },
    Achievement { id: "handshake_master", name: "Handshake Master", desc: "Capture 10 handshakes", xp_bonus: 500 },
    Achievement { id: "marathon", name: "Marathon", desc: "Run for more than 2 hours in one session", xp_bonus: 300 },
    Achievement { id: "night_owl", name: "Night Owl", desc: "Run between 02:00 and 05:00", xp_bonus: 50 },
    Achievement { id: "multi_target", name: "Multi-Target", desc: "Run with 3+ simultaneous targets", xp_bonus: 200 },
    Achievement { id: "konami", name: "Konami", desc: "Enter the Konami code", xp_bonus: 999 },
    Achievement { id: "swordfish", name: "Swordfish", desc: "Use the --swordfish flag", xp_bonus: 9999 },
    Achievement { id: "packet_god", name: "Packet God", desc: "Reach a 50-kill streak", xp_bonus: 2000 },
    Achievement { id: "the_disassociator", name: "The Disassociator", desc: "Reach level 15", xp_bonus: 1000 },
];

/// Look up an achievement definition by id.
pub fn get(id: &str) -> Option<&'static Achievement> {
    ACHIEVEMENTS.iter().find(|a| a.id == id)
}

/// True if the achievement's *condition* currently holds. (Flag-driven ones -
/// `swordfish`, `konami` - are unlocked imperatively and always report false
/// here so they aren't auto-granted.)
fn condition_met(id: &str, state: &AppState) -> bool {
    match id {
        "first_blood" => state.stats.total_kicks >= 1,
        "century" => state.achievements.lifetime_kicks >= 100,
        "rogue_hunter" => state.stats.rogue_aps >= 1,
        "handshake_master" => state.stats.handshakes >= 10,
        "marathon" => state.stats.elapsed_secs() >= 2 * 3600,
        "night_owl" => {
            let h = Local::now().hour();
            (2..5).contains(&h)
        }
        "multi_target" => state.targets.len() >= 3,
        "packet_god" => state.streak.best >= 50,
        "the_disassociator" => state.xp.level() >= 15,
        _ => false, // konami, swordfish: imperative
    }
}

/// Evaluate all auto achievements, returning those that are newly satisfied and
/// not yet recorded as unlocked.
pub fn newly_unlocked(state: &AppState) -> Vec<&'static Achievement> {
    ACHIEVEMENTS
        .iter()
        .filter(|a| !state.achievements.is_unlocked(a.id) && condition_met(a.id, state))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::config::Config;

    #[test]
    fn catalogue_lookup() {
        assert_eq!(get("first_blood").unwrap().xp_bonus, 100);
        assert!(get("nonexistent").is_none());
    }

    #[test]
    fn first_blood_unlocks_on_first_kick() {
        let mut st = AppState::new(Config::default());
        // Isolate from any persisted lifetime store on the test machine.
        st.achievements = crate::app::achievements::AchievementStore::default();
        assert!(newly_unlocked(&st).iter().all(|a| a.id != "first_blood"));
        st.stats.record_kick();
        assert!(newly_unlocked(&st).iter().any(|a| a.id == "first_blood"));
    }

    #[test]
    fn imperative_ones_not_auto_granted() {
        let st = AppState::new(Config::default());
        let ids: Vec<_> = newly_unlocked(&st).iter().map(|a| a.id).collect();
        assert!(!ids.contains(&"swordfish"));
        assert!(!ids.contains(&"konami"));
    }
}
