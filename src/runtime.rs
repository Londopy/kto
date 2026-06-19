//! Engine->state runtime: feeds [`EngineEvent`]s into [`AppState`] and drives the
//! fun layer, notifications, and the session log. Shared by the CLI and TUI
//! front-ends so behavior is identical regardless of UI.

use crate::app::state::{ActivityKind, AppState, Client, ClientStatus, RunStatus, Shared, Target};
use crate::engine::EngineEvent;
use crate::fun::{achievements, xp};
use crate::net::{fingerprint, oui};
use crate::notify::{NotificationEvent, Notifier};
use crate::session::log::SessionLogger;
use crate::util::MacAddr;

/// Owns the side-effecting collaborators the controller needs.
pub struct Runtime {
    pub notifier: Notifier,
    pub logger: Option<SessionLogger>,
    /// Whether we've already awarded the "first client" XP this session.
    first_client_awarded: bool,
}

impl Runtime {
    pub fn new(notifier: Notifier, logger: Option<SessionLogger>) -> Self {
        Runtime { notifier, logger, first_client_awarded: false }
    }

    /// Apply one engine event, taking the write lock for the duration.
    pub fn apply(&mut self, shared: &Shared, ev: EngineEvent) {
        let mut st = shared.write();
        match ev {
            EngineEvent::TargetFound { ssid, bssid, channel, encryption, pmf } => {
                if st.targets.is_empty() {
                    st.targets.push(Target::new(&ssid));
                    st.active = 0;
                }
                if let Some(t) = st.target_mut() {
                    t.ssid = ssid.clone();
                    t.bssid = Some(bssid);
                    t.channel = Some(channel);
                    t.encryption = encryption.to_string();
                    t.pmf = pmf;
                }
                st.log(
                    ActivityKind::TargetFound,
                    format!("Target {ssid}  BSSID {bssid}  ch {channel}  {encryption}"),
                );
                if pmf {
                    st.log(
                        ActivityKind::Warn,
                        "PMF (802.11w) required - deauths may be ignored by compliant clients",
                    );
                }
            }

            EngineEvent::SweepStarted { n } => {
                st.stats.sweeps = n;
                st.log(ActivityKind::Info, format!("Sweep #{n} - scanning…"));
            }

            EngineEvent::SweepCompleted { n, clients_found } => {
                st.log(ActivityKind::Info, format!("Sweep #{n} - {clients_found} clients found"));
            }

            EngineEvent::ClientSeen { mac, rssi, probe_ssids, signals } => {
                let is_new = !st.target().map(|t| t.clients.contains_key(&mac)).unwrap_or(false);
                let vendor = oui::lookup(&mac).map(|s| s.to_string());
                let os_guess = fingerprint::guess_os(&signals);
                let label;
                if let Some(t) = st.target_mut() {
                    let entry = t.clients.entry(mac).or_insert_with(|| Client::new(mac, rssi));
                    entry.record_rssi(rssi);
                    if entry.vendor.is_none() {
                        entry.vendor = vendor.clone();
                    }
                    if entry.os_guess.is_none() {
                        entry.os_guess = os_guess;
                    }
                    for s in &probe_ssids {
                        entry.record_probe(s);
                    }
                    entry.status = ClientStatus::Active;
                    label = entry.display_name();
                } else {
                    label = mac.short();
                }

                if is_new {
                    st.log(
                        ActivityKind::NewClient,
                        format!("New client : {mac}  ({label})  {rssi} dBm"),
                    );
                    if let Some(log) = self.logger.as_mut() {
                        let _ = log.new_client(mac, &label, rssi);
                    }
                    let ssid = st.target().map(|t| t.ssid.clone()).unwrap_or_default();
                    self.notifier.handle(&NotificationEvent::NewClient {
                        mac,
                        vendor: label.clone(),
                        ssid,
                    });
                    if !self.first_client_awarded {
                        self.first_client_awarded = true;
                        self.award_xp(&mut st, xp::award::FIRST_CLIENT);
                    }
                }
            }

            EngineEvent::ClientKicked { mac, burst } => {
                let label = self.register_kick(&mut st, mac, burst);
                self.notifier.handle(&NotificationEvent::ClientKicked { mac, burst });
                if let Some(log) = self.logger.as_mut() {
                    let _ = log.kick(mac, &label, burst);
                }
            }

            EngineEvent::ClientGone { mac } => {
                if let Some(t) = st.target_mut() {
                    if let Some(c) = t.clients.get_mut(&mac) {
                        c.status = ClientStatus::Gone;
                    }
                }
                st.log(ActivityKind::Info, format!("Client gone: {mac}"));
            }

            EngineEvent::RogueAp { bssid, ssid, reasons } => {
                st.stats.rogue_aps += 1;
                let why = reasons
                    .iter()
                    .map(|r| r.describe())
                    .collect::<Vec<_>>()
                    .join("; ");
                st.log(
                    ActivityKind::Warn,
                    format!("ROGUE AP DETECTED: {bssid} broadcasting {ssid} - {why}"),
                );
                self.notifier.handle(&NotificationEvent::RogueAp { ssid });
                self.award_xp(&mut st, xp::award::ROGUE_AP);
                self.check_achievements(&mut st);
            }

            EngineEvent::HandshakeCaptured { mac } => {
                st.stats.handshakes += 1;
                let ssid = st.target().map(|t| t.ssid.clone()).unwrap_or_default();
                st.log(ActivityKind::Good, format!("Handshake captured from {mac}"));
                self.notifier.handle(&NotificationEvent::Handshake { mac, ssid });
                self.award_xp(&mut st, xp::award::HANDSHAKE);
                self.check_achievements(&mut st);
            }

            EngineEvent::PmkidCaptured { mac } => {
                st.stats.pmkids += 1;
                let ssid = st.target().map(|t| t.ssid.clone()).unwrap_or_default();
                st.log(ActivityKind::Good, format!("PMKID captured from {mac}"));
                self.notifier.handle(&NotificationEvent::Pmkid { ssid });
                self.award_xp(&mut st, xp::award::PMKID);
            }

            EngineEvent::StatusChanged(status) => {
                st.status = status;
                st.paused = status == RunStatus::Paused;
            }

            EngineEvent::Notice(msg) => {
                st.log(ActivityKind::Info, msg);
            }
            EngineEvent::Error(msg) => {
                st.log(ActivityKind::Bad, msg);
            }
            EngineEvent::Stopped => {
                st.status = RunStatus::Idle;
                st.log(ActivityKind::Info, "Engine stopped");
            }
        }
    }

    /// Record a kick: counters, streak, XP, achievements, log line.
    fn register_kick(&mut self, st: &mut AppState, mac: MacAddr, burst: u64) -> String {
        st.stats.record_kick();
        st.achievements.lifetime_kicks += 1;

        let label = if let Some(t) = st.target_mut() {
            let c = t.clients.entry(mac).or_insert_with(|| Client::new(mac, -70));
            c.kick_count += 1;
            c.status = ClientStatus::Active;
            c.display_name()
        } else {
            mac.short()
        };

        st.log(ActivityKind::Kick, format!("Kicked {mac} ({label}) (burst #{burst}) 🔫"));

        // Kill streak.
        if st.config.fun.kill_streaks {
            if let Some(tier) = st.streak.record_kick() {
                st.log(
                    ActivityKind::Good,
                    format!("Kill streak: {}  ({})", st.streak.current, tier.name),
                );
                self.notifier.handle(&NotificationEvent::KillStreak {
                    name: tier.name.to_string(),
                    count: st.streak.current,
                });
                self.award_xp(st, (tier.threshold as u64) * 10);
            }
        }

        // Base XP per burst.
        if st.config.fun.xp_system {
            self.award_xp(st, xp::award::KICK_BURST);
        }

        self.check_achievements(st);
        label
    }

    /// Award XP (scaled by any active multiplier), logging + notifying on level up.
    fn award_xp(&mut self, st: &mut AppState, amount: u64) {
        if !st.config.fun.xp_system {
            return;
        }
        let mult = st.xp_multiplier();
        if let Some(level) = st.xp.add_scaled(amount, mult) {
            let title = st.xp.title();
            st.log(ActivityKind::Good, format!("Level up! Level {level} - {title}"));
            self.notifier.handle(&NotificationEvent::LevelUp { level });
        }
    }

    /// Unlock any newly satisfied achievements and award their bonuses.
    fn check_achievements(&mut self, st: &mut AppState) {
        let newly: Vec<(&'static str, &'static str, u64)> = achievements::newly_unlocked(st)
            .into_iter()
            .map(|a| (a.id, a.name, a.xp_bonus))
            .collect();
        for (id, name, bonus) in newly {
            if st.achievements.unlock(id) {
                st.log(ActivityKind::Good, format!("🏆 Achievement unlocked: {name} (+{bonus} XP)"));
                self.award_xp(st, bonus);
            }
        }
        let _ = st.achievements.save_default();
    }
}
