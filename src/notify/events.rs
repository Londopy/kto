//! Notification event catalogue and rendering.

use crate::util::MacAddr;

/// A user-facing notification event.
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    NewClient { mac: MacAddr, vendor: String, ssid: String },
    ClientKicked { mac: MacAddr, burst: u64 },
    Handshake { mac: MacAddr, ssid: String },
    Pmkid { ssid: String },
    RogueAp { ssid: String },
    UpdateAvailable { version: String },
    KillStreak { name: String, count: u32 },
    LevelUp { level: u32 },
}

/// A rendered notification: title, body, and an emoji icon hint.
pub struct Rendered {
    pub title: String,
    pub body: String,
    pub icon: &'static str,
}

impl NotificationEvent {
    /// A stable key used for debouncing (per-client for kicks).
    pub fn debounce_key(&self) -> String {
        match self {
            NotificationEvent::ClientKicked { mac, .. } => format!("kick:{mac}"),
            NotificationEvent::NewClient { mac, .. } => format!("new:{mac}"),
            other => format!("{other:?}"),
        }
    }

    pub fn render(&self) -> Rendered {
        match self {
            NotificationEvent::NewClient { mac, vendor, ssid } => Rendered {
                title: "KTO - New Client".into(),
                body: format!("{mac} ({vendor}) joined {ssid}"),
                icon: "📡",
            },
            NotificationEvent::ClientKicked { mac, burst } => Rendered {
                title: "KTO - Client Kicked".into(),
                body: format!("{mac} - burst #{burst}"),
                icon: "🔫",
            },
            NotificationEvent::Handshake { mac, ssid } => Rendered {
                title: "KTO - Handshake!".into(),
                body: format!("4-way HS from {mac} on {ssid}"),
                icon: "🔑",
            },
            NotificationEvent::Pmkid { ssid } => Rendered {
                title: "KTO - PMKID!".into(),
                body: format!("PMKID from {ssid} AP"),
                icon: "🗝️",
            },
            NotificationEvent::RogueAp { ssid } => Rendered {
                title: "KTO - ⚠ Rogue AP".into(),
                body: format!("Unknown AP broadcasting {ssid} nearby"),
                icon: "⚠️",
            },
            NotificationEvent::UpdateAvailable { version } => Rendered {
                title: "KTO - Update Available".into(),
                body: format!("v{version} is available → click to open"),
                icon: "⬆️",
            },
            NotificationEvent::KillStreak { name, count } => Rendered {
                title: "KTO - Kill Streak!".into(),
                body: format!("{name} - {count} kicks in a row"),
                icon: "🔥",
            },
            NotificationEvent::LevelUp { level } => Rendered {
                title: "KTO - Level Up!".into(),
                body: format!("You reached level {level}!"),
                icon: "⭐",
            },
        }
    }
}
