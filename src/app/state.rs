//! The shared application state.
//!
//! `AppState` is the single source of truth the UI renders from and the engine
//! controller writes into. It is wrapped in `Arc<RwLock<AppState>>` (see
//! [`Shared`]) so the engine, notifier, Discord, and UI threads can share it.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;

use crate::fun::streaks::StreakState;
use crate::fun::xp::XpState;
use crate::update::info::UpdateInfo;
use crate::util::MacAddr;

/// Max activity-log entries kept in memory.
const ACTIVITY_CAP: usize = 1000;
/// Max RSSI samples kept per client (~2 min at default scan rate).
const RSSI_HISTORY_CAP: usize = 120;
/// Max distinct probe SSIDs remembered per client.
const PROBE_HISTORY_CAP: usize = 10;
/// Window for the kicks/min rolling average and sparkline.
const KICK_WINDOW: usize = 256;

/// Convenience alias for the shared, lock-guarded state.
pub type Shared = Arc<RwLock<AppState>>;

/// What the engine is currently doing - drives the status indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Idle,
    Scanning,
    Deauthing,
    Paused,
    ScanOnly,
}

impl RunStatus {
    pub fn label(&self) -> &'static str {
        match self {
            RunStatus::Idle => "IDLE",
            RunStatus::Scanning => "SCANNING",
            RunStatus::Deauthing => "DEAUTHING",
            RunStatus::Paused => "PAUSED",
            RunStatus::ScanOnly => "SCAN ONLY",
        }
    }
}

/// Per-client liveness state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientStatus {
    /// Currently associated and being acted on.
    Active,
    /// Spared via the whitelist.
    Whitelisted,
    /// Not seen recently.
    Gone,
}

impl ClientStatus {
    /// The marker glyph used in the TUI client list.
    pub fn glyph(&self) -> char {
        match self {
            ClientStatus::Active => '●',
            ClientStatus::Whitelisted => '○',
            ClientStatus::Gone => '◌',
        }
    }
}

/// A discovered station.
#[derive(Debug, Clone)]
pub struct Client {
    pub mac: MacAddr,
    pub vendor: Option<String>,
    pub os_guess: Option<String>,
    pub nickname: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub rssi: i8,
    pub rssi_history: VecDeque<(DateTime<Utc>, i8)>,
    pub probe_ssids: VecDeque<String>,
    pub kick_count: u64,
    pub status: ClientStatus,
}

impl Client {
    pub fn new(mac: MacAddr, rssi: i8) -> Self {
        let now = Utc::now();
        let mut hist = VecDeque::with_capacity(RSSI_HISTORY_CAP);
        hist.push_back((now, rssi));
        Client {
            mac,
            vendor: None,
            os_guess: None,
            nickname: None,
            first_seen: now,
            last_seen: now,
            rssi,
            rssi_history: hist,
            probe_ssids: VecDeque::with_capacity(PROBE_HISTORY_CAP),
            kick_count: 0,
            status: ClientStatus::Active,
        }
    }

    /// Best human label: nickname, else vendor, else short MAC.
    pub fn display_name(&self) -> String {
        if let Some(n) = &self.nickname {
            n.clone()
        } else if let Some(v) = &self.vendor {
            v.clone()
        } else {
            self.mac.short()
        }
    }

    /// Push an RSSI sample, trimming the ring buffer.
    pub fn record_rssi(&mut self, rssi: i8) {
        let now = Utc::now();
        self.rssi = rssi;
        self.last_seen = now;
        self.rssi_history.push_back((now, rssi));
        while self.rssi_history.len() > RSSI_HISTORY_CAP {
            self.rssi_history.pop_front();
        }
    }

    /// Remember a probe-request SSID (dedup, bounded).
    pub fn record_probe(&mut self, ssid: &str) {
        if ssid.is_empty() || self.probe_ssids.iter().any(|s| s == ssid) {
            return;
        }
        self.probe_ssids.push_back(ssid.to_string());
        while self.probe_ssids.len() > PROBE_HISTORY_CAP {
            self.probe_ssids.pop_front();
        }
    }

    /// Mean RSSI over the recorded history.
    pub fn rssi_avg(&self) -> i8 {
        if self.rssi_history.is_empty() {
            return self.rssi;
        }
        let sum: i32 = self.rssi_history.iter().map(|(_, r)| *r as i32).sum();
        (sum / self.rssi_history.len() as i32) as i8
    }

    /// The last `n` RSSI samples as plain values, for sparklines.
    pub fn rssi_samples(&self, n: usize) -> Vec<i8> {
        let len = self.rssi_history.len();
        let start = len.saturating_sub(n);
        self.rssi_history.iter().skip(start).map(|(_, r)| *r).collect()
    }
}

/// One target network and its clients.
#[derive(Debug, Clone)]
pub struct Target {
    pub ssid: String,
    pub bssid: Option<MacAddr>,
    pub channel: Option<u8>,
    pub encryption: String,
    pub pmf: bool,
    pub clients: HashMap<MacAddr, Client>,
}

impl Target {
    pub fn new(ssid: impl Into<String>) -> Self {
        Target {
            ssid: ssid.into(),
            bssid: None,
            channel: None,
            encryption: "?".into(),
            pmf: false,
            clients: HashMap::new(),
        }
    }
}

/// A single activity-log line kind, used for color coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Info,
    NewClient,
    Kick,
    Warn,
    TargetFound,
    Good,
    Bad,
}

/// A rendered activity-log entry.
#[derive(Debug, Clone)]
pub struct Activity {
    /// Monotonic sequence number (never reused, survives ring-buffer eviction).
    pub seq: u64,
    pub at: DateTime<Utc>,
    pub kind: ActivityKind,
    pub message: String,
}

/// Running counters and rolling windows.
#[derive(Debug, Clone)]
pub struct Stats {
    pub session_start: DateTime<Utc>,
    pub total_kicks: u64,
    pub sweeps: u64,
    pub handshakes: u64,
    pub pmkids: u64,
    pub rogue_aps: u64,
    pub cheat_codes_used: u64,
    /// Timestamps of recent kicks, for kicks/min + sparkline.
    kick_times: VecDeque<DateTime<Utc>>,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            session_start: Utc::now(),
            total_kicks: 0,
            sweeps: 0,
            handshakes: 0,
            pmkids: 0,
            rogue_aps: 0,
            cheat_codes_used: 0,
            kick_times: VecDeque::with_capacity(KICK_WINDOW),
        }
    }
}

impl Stats {
    pub fn record_kick(&mut self) {
        self.total_kicks += 1;
        self.kick_times.push_back(Utc::now());
        while self.kick_times.len() > KICK_WINDOW {
            self.kick_times.pop_front();
        }
    }

    /// Session length in seconds.
    pub fn elapsed_secs(&self) -> u64 {
        (Utc::now() - self.session_start).num_seconds().max(0) as u64
    }

    /// Kicks in the last 60 seconds, scaled to per-minute (here it *is* the
    /// 60s count, which equals the per-minute rate).
    pub fn kicks_per_min(&self) -> f64 {
        let cutoff = Utc::now() - chrono::Duration::seconds(60);
        self.kick_times.iter().filter(|&&t| t >= cutoff).count() as f64
    }

    /// Bucketed kick counts over the last `buckets` seconds, for a sparkline.
    pub fn kick_sparkline(&self, buckets: usize) -> Vec<u64> {
        let now = Utc::now();
        let mut out = vec![0u64; buckets];
        for &t in &self.kick_times {
            let age = (now - t).num_seconds();
            if age >= 0 && (age as usize) < buckets {
                let idx = buckets - 1 - age as usize;
                out[idx] += 1;
            }
        }
        out
    }
}

/// The top-level shared state.
pub struct AppState {
    pub config: crate::app::config::Config,
    pub theme: String,
    pub targets: Vec<Target>,
    pub active: usize,
    pub status: RunStatus,
    pub paused: bool,
    pub boss_mode: bool,
    pub activity: VecDeque<Activity>,
    pub stats: Stats,
    pub xp: XpState,
    pub streak: StreakState,
    pub achievements: crate::app::achievements::AchievementStore,
    pub update_available: Option<UpdateInfo>,
    /// Whether a 2x XP cheat window is active and until when.
    pub xp_multiplier_until: Option<DateTime<Utc>>,
    pub should_quit: bool,
    /// Monotonic counter backing `Activity::seq`.
    log_seq: u64,
}

impl AppState {
    pub fn new(config: crate::app::config::Config) -> Self {
        let theme = config.ui.theme.clone();
        AppState {
            config,
            theme,
            targets: Vec::new(),
            active: 0,
            status: RunStatus::Idle,
            paused: false,
            boss_mode: false,
            activity: VecDeque::with_capacity(ACTIVITY_CAP),
            stats: Stats::default(),
            xp: XpState::default(),
            streak: StreakState::default(),
            achievements: crate::app::achievements::AchievementStore::load_default(),
            update_available: None,
            xp_multiplier_until: None,
            should_quit: false,
            log_seq: 0,
        }
    }

    /// Wrap in the shared `Arc<RwLock<_>>`.
    pub fn shared(self) -> Shared {
        Arc::new(RwLock::new(self))
    }

    /// The currently focused target, if any.
    pub fn target(&self) -> Option<&Target> {
        self.targets.get(self.active)
    }

    pub fn target_mut(&mut self) -> Option<&mut Target> {
        self.targets.get_mut(self.active)
    }

    /// Append an activity-log line (bounded). Returns the entry's sequence id.
    pub fn log(&mut self, kind: ActivityKind, message: impl Into<String>) -> u64 {
        let seq = self.log_seq;
        self.log_seq += 1;
        self.activity.push_back(Activity { seq, at: Utc::now(), kind, message: message.into() });
        while self.activity.len() > ACTIVITY_CAP {
            self.activity.pop_front();
        }
        seq
    }

    /// The sequence id that will be assigned to the next logged entry.
    pub fn next_log_seq(&self) -> u64 {
        self.log_seq
    }

    /// Count of currently active (associated, non-whitelisted) clients across
    /// all targets.
    pub fn active_client_count(&self) -> usize {
        self.targets
            .iter()
            .flat_map(|t| t.clients.values())
            .filter(|c| c.status == ClientStatus::Active)
            .count()
    }

    /// Total unique clients seen across all targets.
    pub fn total_client_count(&self) -> usize {
        self.targets.iter().map(|t| t.clients.len()).sum()
    }

    /// Effective XP multiplier right now (2.0 during a cheat window, else 1.0).
    pub fn xp_multiplier(&self) -> f64 {
        match self.xp_multiplier_until {
            Some(until) if Utc::now() < until => 2.0,
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(b: u8) -> MacAddr {
        MacAddr([b, b, b, b, b, b])
    }

    #[test]
    fn client_display_name_precedence() {
        let mut c = Client::new(mac(0x11), -60);
        assert_eq!(c.display_name(), c.mac.short());
        c.vendor = Some("Apple".into());
        assert_eq!(c.display_name(), "Apple");
        c.nickname = Some("CEO laptop".into());
        assert_eq!(c.display_name(), "CEO laptop");
    }

    #[test]
    fn rssi_history_bounded_and_avg() {
        let mut c = Client::new(mac(1), -50);
        for _ in 0..500 {
            c.record_rssi(-70);
        }
        assert!(c.rssi_history.len() <= RSSI_HISTORY_CAP);
        assert_eq!(c.rssi_avg(), -70);
    }

    #[test]
    fn probe_dedup_and_bound() {
        let mut c = Client::new(mac(2), -50);
        c.record_probe("A");
        c.record_probe("A");
        c.record_probe("B");
        assert_eq!(c.probe_ssids.len(), 2);
    }

    #[test]
    fn stats_kicks_per_min() {
        let mut s = Stats::default();
        for _ in 0..5 {
            s.record_kick();
        }
        assert_eq!(s.total_kicks, 5);
        assert_eq!(s.kicks_per_min(), 5.0);
        assert_eq!(s.kick_sparkline(60).len(), 60);
    }
}
