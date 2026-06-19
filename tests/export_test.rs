//! Integration: JSON / CSV / HTML exports round-trip.

use kto::app::config::Config;
use kto::app::state::{AppState, Client, Target};
use kto::session::{export_csv, export_html, export_json, SessionReport};
use kto::util::MacAddr;

fn state_with_clients() -> AppState {
    let mut st = AppState::new(Config::default());
    let mut t = Target::new("CorpNet");
    t.bssid = Some("AA:BB:CC:DD:EE:FF".parse().unwrap());
    t.channel = Some(6);
    t.encryption = "WPA2".into();

    let mac: MacAddr = "11:22:33:44:55:66".parse().unwrap();
    let mut c = Client::new(mac, -62);
    c.vendor = Some("Apple".into());
    c.os_guess = Some("iOS".into());
    c.kick_count = 12;
    c.record_probe("CorpNet");
    c.record_probe("Starbucks");
    t.clients.insert(mac, c);

    st.targets.push(t);
    st.stats.total_kicks = 12;
    st
}

#[test]
fn json_export_round_trips() {
    let report = SessionReport::from_state(&state_with_clients());
    let json = export_json::to_string(&report).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["target"]["ssid"], "CorpNet");
    assert_eq!(v["summary"]["total_kicks"], 12);
    assert_eq!(v["clients"][0]["mac"], "11:22:33:44:55:66");
}

#[test]
fn csv_export_has_header_and_data() {
    let report = SessionReport::from_state(&state_with_clients());
    let csv = export_csv::to_string(&report).unwrap();
    assert!(csv.lines().next().unwrap().starts_with("mac,vendor,os_guess"));
    assert!(csv.contains("11:22:33:44:55:66"));
    assert!(csv.contains("CorpNet;Starbucks"));
}

#[test]
fn html_export_is_self_contained() {
    let report = SessionReport::from_state(&state_with_clients());
    let html = export_html::to_string(&report);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("CorpNet"));
    assert!(html.contains("11:22:33:44:55:66"));
    assert!(html.contains("<svg")); // inline charts, no external deps
    // No external resource fetches (the SVG xmlns URI doesn't count).
    assert!(!html.contains("src=\"http"));
    assert!(!html.contains("href=\"http"));
    assert!(!html.to_lowercase().contains("cdn"));
}
