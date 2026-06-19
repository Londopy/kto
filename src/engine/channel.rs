//! Channel planning and hop scheduling.
//!
//! Pure scheduling logic - no syscalls. The live radio path would call
//! `iwconfig`/`iw` to actually retune; that wiring belongs in `radio.rs`.

/// 2.4 GHz hop order from the spec (overlap-aware: 1/6/11 first).
pub const HOP_24: &[u8] = &[1, 6, 11, 2, 7, 3, 8, 4, 9, 5, 10];

/// 5 GHz non-DFS-first channel order (abbreviated; extend as needed).
pub const HOP_5: &[u8] = &[36, 40, 44, 48, 149, 153, 157, 161, 165, 52, 56, 60, 64];

/// Round-robin channel hopper with a configurable dwell.
#[derive(Debug, Clone)]
pub struct HopPlan {
    channels: Vec<u8>,
    idx: usize,
    pub dwell_ms: u64,
    /// When locked, hopping is suspended on the locked channel.
    locked: Option<u8>,
}

impl HopPlan {
    /// Build a plan over the 2.4 GHz set (and 5 GHz if `include_5ghz`).
    pub fn new(include_5ghz: bool, dwell_ms: u64) -> Self {
        let mut channels = HOP_24.to_vec();
        if include_5ghz {
            channels.extend_from_slice(HOP_5);
        }
        HopPlan { channels, idx: 0, dwell_ms, locked: None }
    }

    /// A single-channel plan (used when `--channel` is given).
    pub fn locked_to(channel: u8) -> Self {
        HopPlan { channels: vec![channel], idx: 0, dwell_ms: 0, locked: Some(channel) }
    }

    /// Lock onto a channel (e.g. after the target BSSID is confirmed).
    pub fn lock(&mut self, channel: u8) {
        self.locked = Some(channel);
    }

    /// Resume hopping.
    pub fn unlock(&mut self) {
        self.locked = None;
    }

    pub fn is_locked(&self) -> bool {
        self.locked.is_some()
    }

    /// The next channel to tune to. While locked, always returns the locked
    /// channel; otherwise advances round-robin.
    pub fn next_channel(&mut self) -> u8 {
        if let Some(ch) = self.locked {
            return ch;
        }
        let ch = self.channels[self.idx % self.channels.len()];
        self.idx = (self.idx + 1) % self.channels.len();
        ch
    }

    pub fn channels(&self) -> &[u8] {
        &self.channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_all_channels() {
        let mut plan = HopPlan::new(false, 100);
        let n = plan.channels().len();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..n {
            seen.insert(plan.next_channel());
        }
        assert_eq!(seen.len(), n);
        // wraps around
        assert_eq!(plan.next_channel(), HOP_24[0]);
    }

    #[test]
    fn lock_pins_channel() {
        let mut plan = HopPlan::new(false, 100);
        plan.lock(6);
        assert!(plan.is_locked());
        assert_eq!(plan.next_channel(), 6);
        assert_eq!(plan.next_channel(), 6);
    }
}
