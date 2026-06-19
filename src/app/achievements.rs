//! Persistent achievement store.
//!
//! The set of *unlocked* achievement ids is persisted to
//! `~/.local/share/kto/achievements.json` so lifetime achievements survive
//! across sessions. The catalogue and unlock *conditions* live in
//! [`crate::fun::achievements`].

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AchievementStore {
    /// Stable ids of unlocked achievements, e.g. `"first_blood"`.
    pub unlocked: BTreeSet<String>,
    /// Lifetime kick counter used by the `century` achievement.
    #[serde(default)]
    pub lifetime_kicks: u64,
}

impl AchievementStore {
    pub fn default_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kto")
            .join("achievements.json")
    }

    pub fn load_default() -> Self {
        Self::load(&Self::default_path())
    }

    pub fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_default(&self) -> std::io::Result<()> {
        self.save(&Self::default_path())
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        std::fs::write(path, json)
    }

    /// Returns true if this call newly unlocked the achievement.
    pub fn unlock(&mut self, id: &str) -> bool {
        self.unlocked.insert(id.to_string())
    }

    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked.contains(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlock_is_idempotent() {
        let mut s = AchievementStore::default();
        assert!(s.unlock("first_blood"));
        assert!(!s.unlock("first_blood"));
        assert!(s.is_unlocked("first_blood"));
    }

    #[test]
    fn roundtrips_json() {
        let mut s = AchievementStore::default();
        s.unlock("century");
        s.lifetime_kicks = 100;
        let json = serde_json::to_string(&s).unwrap();
        let back: AchievementStore = serde_json::from_str(&json).unwrap();
        assert!(back.is_unlocked("century"));
        assert_eq!(back.lifetime_kicks, 100);
    }
}
