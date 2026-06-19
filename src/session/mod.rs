//! Session reporting and logging.
//!
//! [`SessionReport`] is the serializable snapshot shared by the JSON, CSV, and
//! HTML exporters. It is built from the live [`AppState`].

pub mod export_csv;
pub mod export_html;
pub mod export_json;
pub mod log;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::app::state::AppState;
use crate::util::MacAddr;

#[derive(Debug, Clone, Serialize)]
pub struct TargetReport {
    pub ssid: String,
    pub bssid: Option<MacAddr>,
    pub channel: Option<u8>,
    pub encryption: String,
    pub pmf_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total_kicks: u64,
    pub unique_clients: usize,
    pub handshakes_captured: u64,
    pub pmkids_captured: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientRecord {
    pub mac: MacAddr,
    pub vendor: Option<String>,
    pub os_guess: Option<String>,
    pub nickname: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub rssi_avg: i8,
    pub kick_count: u64,
    pub probe_ssids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionReport {
    pub kto_version: String,
    pub session_start: DateTime<Utc>,
    pub session_end: DateTime<Utc>,
    pub target: TargetReport,
    pub summary: Summary,
    pub clients: Vec<ClientRecord>,
}

impl SessionReport {
    /// Build a report from the active target's state.
    pub fn from_state(state: &AppState) -> SessionReport {
        let target = state.target();
        let target_report = match target {
            Some(t) => TargetReport {
                ssid: t.ssid.clone(),
                bssid: t.bssid,
                channel: t.channel,
                encryption: t.encryption.clone(),
                pmf_enabled: t.pmf,
            },
            None => TargetReport {
                ssid: "(none)".into(),
                bssid: None,
                channel: None,
                encryption: "?".into(),
                pmf_enabled: false,
            },
        };

        let mut clients: Vec<ClientRecord> = target
            .map(|t| {
                t.clients
                    .values()
                    .map(|c| ClientRecord {
                        mac: c.mac,
                        vendor: c.vendor.clone(),
                        os_guess: c.os_guess.clone(),
                        nickname: c.nickname.clone(),
                        first_seen: c.first_seen,
                        last_seen: c.last_seen,
                        rssi_avg: c.rssi_avg(),
                        kick_count: c.kick_count,
                        probe_ssids: c.probe_ssids.iter().cloned().collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        clients.sort_by(|a, b| b.kick_count.cmp(&a.kick_count));

        SessionReport {
            kto_version: env!("CARGO_PKG_VERSION").to_string(),
            session_start: state.stats.session_start,
            session_end: Utc::now(),
            target: target_report,
            summary: Summary {
                total_kicks: state.stats.total_kicks,
                unique_clients: clients.len(),
                handshakes_captured: state.stats.handshakes,
                pmkids_captured: state.stats.pmkids,
            },
            clients,
        }
    }
}
