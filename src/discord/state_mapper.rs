//! Maps `AppState` to Discord Rich Presence fields. No side effects, easy to test.

use crate::app::state::{AppState, RunStatus};

const REDACTED: &str = "[redacted]";

/// A resolved presence payload, ready to push over the RPC pipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    pub details: String,
    pub state: String,
    pub large_image: &'static str,
    pub small_image: Option<&'static str>,
    /// (active_clients, total_seen)
    pub party: Option<(u32, u32)>,
}

/// Build presence from the current app state. When `obfuscate` is set, the SSID
/// is replaced with `[redacted]` everywhere.
pub fn presence(state: &AppState, obfuscate: bool) -> Presence {
    let ssid = match state.target() {
        Some(t) if !obfuscate => t.ssid.clone(),
        Some(_) => REDACTED.to_string(),
        None => String::new(),
    };
    let channel = state.target().and_then(|t| t.channel);
    let enc = state.target().map(|t| t.encryption.clone()).unwrap_or_default();
    let active = state.active_client_count() as u32;
    let total = state.total_client_count() as u32;
    let kicks = state.stats.total_kicks;
    let streak_suffix = state
        .streak
        .current_tier()
        .map(|t| format!(" {}", t.discord_suffix))
        .unwrap_or_default();

    match state.status {
        RunStatus::Idle => Presence {
            details: "Standing by…".into(),
            state: "KTO v3".into(),
            large_image: "kto_logo",
            small_image: None,
            party: None,
        },
        RunStatus::Scanning => Presence {
            details: format!("Scanning {ssid}"),
            state: format!("ch {} · {enc}", channel.map(|c| c.to_string()).unwrap_or_default()),
            large_image: "kto_scan",
            small_image: Some("wifi_icon"),
            party: None,
        },
        RunStatus::Deauthing => {
            let aggressive = state.config.deauth.aggressive;
            Presence {
                details: if aggressive {
                    format!("Hammering {ssid}")
                } else {
                    format!("Kicking devices off {ssid}")
                },
                state: format!("{active} clients · {kicks} kicks{streak_suffix}"),
                large_image: if aggressive { "kto_aggressive" } else { "kto_deauth" },
                small_image: Some(if aggressive { "fire_icon" } else { "skull_icon" }),
                party: Some((active.max(1), total.max(1))),
            }
        }
        RunStatus::ScanOnly => Presence {
            details: format!("Passive recon on {ssid}"),
            state: format!("{total} clients discovered"),
            large_image: "kto_passive",
            small_image: Some("eye_icon"),
            party: Some((active.max(1), total.max(1))),
        },
        RunStatus::Paused => Presence {
            details: "Paused".into(),
            state: format!("{ssid} · {kicks} kicks so far"),
            large_image: "kto_logo",
            small_image: Some("pause_icon"),
            party: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::config::Config;
    use crate::app::state::{AppState, Target};

    fn state_with_target() -> AppState {
        let mut st = AppState::new(Config::default());
        let mut t = Target::new("CorpNet");
        t.channel = Some(6);
        t.encryption = "WPA2".into();
        st.targets.push(t);
        st
    }

    #[test]
    fn scanning_presence() {
        let mut st = state_with_target();
        st.status = RunStatus::Scanning;
        let p = presence(&st, false);
        assert!(p.details.contains("CorpNet"));
        assert_eq!(p.large_image, "kto_scan");
    }

    #[test]
    fn obfuscation_redacts_ssid() {
        let mut st = state_with_target();
        st.status = RunStatus::Deauthing;
        let p = presence(&st, true);
        assert!(p.details.contains("[redacted]"));
        assert!(!p.details.contains("CorpNet"));
    }
}
