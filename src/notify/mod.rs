//! OS notifications.
//!
//! The platform dispatch uses `notify-rust` when the `notifications` feature is
//! enabled (Linux/macOS via libnotify/UNS; the spec also wires `winrt-
//! notification` on Windows). Without the feature it degrades to a tracing
//! debug line so the rest of the app is unaffected.

pub mod debounce;
pub mod events;

use std::time::Duration;

use crate::app::config::NotificationsCfg;
use debounce::Debouncer;
pub use events::NotificationEvent;

/// "Client kicked" fires at most once per client per this window.
const KICK_DEBOUNCE: Duration = Duration::from_secs(10);

/// Routes notification events to the OS, respecting config flags and debouncing.
pub struct Notifier {
    cfg: NotificationsCfg,
    debouncer: Debouncer,
}

impl Notifier {
    pub fn new(cfg: NotificationsCfg) -> Self {
        Notifier { cfg, debouncer: Debouncer::new(KICK_DEBOUNCE) }
    }

    /// True if this event type is enabled in config.
    fn enabled_for(&self, event: &NotificationEvent) -> bool {
        if !self.cfg.enabled {
            return false;
        }
        match event {
            NotificationEvent::NewClient { .. } => self.cfg.new_client,
            NotificationEvent::ClientKicked { .. } => self.cfg.client_kicked,
            NotificationEvent::Handshake { .. } => self.cfg.handshake,
            NotificationEvent::Pmkid { .. } => self.cfg.handshake,
            NotificationEvent::RogueAp { .. } => self.cfg.rogue_ap,
            NotificationEvent::UpdateAvailable { .. } => self.cfg.update_available,
            NotificationEvent::KillStreak { .. } => self.cfg.kill_streak,
            NotificationEvent::LevelUp { .. } => self.cfg.kill_streak,
        }
    }

    /// Handle an event: filter by config, debounce, then dispatch.
    pub fn handle(&mut self, event: &NotificationEvent) {
        if !self.enabled_for(event) {
            return;
        }
        if !self.debouncer.allow(&event.debounce_key()) {
            return;
        }
        let r = event.render();
        dispatch(&r.title, &r.body, r.icon);
    }
}

#[cfg(feature = "notifications")]
fn dispatch(title: &str, body: &str, _icon: &str) {
    use notify_rust::Notification;
    if let Err(e) = Notification::new().summary(title).body(body).appname("KTO").show() {
        tracing::debug!("notification dispatch failed: {e}");
    }
}

#[cfg(not(feature = "notifications"))]
fn dispatch(title: &str, body: &str, icon: &str) {
    tracing::debug!("[notify:{icon}] {title} - {body}");
}
