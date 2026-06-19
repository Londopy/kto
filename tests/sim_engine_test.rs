//! Integration: the simulation engine emits a sensible event stream and the
//! runtime feeds it into state (, exercising the engine abstraction).

use std::time::Duration;

use kto::app::config::Config;
use kto::app::state::AppState;
use kto::engine::{Engine, EngineEvent, RunParams};
use kto::engine::sim::SimEngine;

#[test]
fn sim_engine_discovers_target_and_kicks() {
    // 1s sweeps so discovery happens fast and the test stays deterministic.
    let params = RunParams { targets: vec!["CorpNet".into()], sweep_interval: 1.0, ..Default::default() };
    let handle = Box::new(SimEngine::new(params)).spawn();

    let mut saw_target = false;
    let mut saw_kick = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);

    while std::time::Instant::now() < deadline && !(saw_target && saw_kick) {
        if let Ok(ev) = handle.events.recv_timeout(Duration::from_millis(500)) {
            match ev {
                EngineEvent::TargetFound { ssid, .. } => {
                    assert_eq!(ssid, "CorpNet");
                    saw_target = true;
                }
                EngineEvent::ClientKicked { .. } => saw_kick = true,
                _ => {}
            }
        }
    }

    assert!(saw_target, "engine never reported a target");
    assert!(saw_kick, "engine never reported a kick");

    let _ = handle.commands.send(kto::engine::EngineCommand::Stop);
    let _ = handle.join.join();
}

#[test]
fn runtime_folds_events_into_state() {
    use kto::engine::EngineEvent;
    use kto::net::Encryption;
    use kto::notify::Notifier;
    use kto::runtime::Runtime;
    use kto::util::MacAddr;

    let shared = AppState::new(Config::default()).shared();
    let mut rt = Runtime::new(Notifier::new(Config::default().notifications), None);

    let bssid: MacAddr = "AA:BB:CC:DD:EE:FF".parse().unwrap();
    let mac: MacAddr = "A4:83:E7:11:22:66".parse().unwrap();

    rt.apply(
        &shared,
        EngineEvent::TargetFound {
            ssid: "CorpNet".into(),
            bssid,
            channel: 6,
            encryption: Encryption::Wpa2,
            pmf: false,
        },
    );
    rt.apply(
        &shared,
        EngineEvent::ClientSeen {
            mac,
            rssi: -60,
            probe_ssids: vec!["CorpNet".into()],
            signals: Default::default(),
        },
    );
    rt.apply(&shared, EngineEvent::ClientKicked { mac, burst: 1 });

    let st = shared.read();
    assert_eq!(st.stats.total_kicks, 1);
    assert_eq!(st.total_client_count(), 1);
    let target = st.target().unwrap();
    let client = target.clients.get(&mac).unwrap();
    assert_eq!(client.vendor.as_deref(), Some("Apple"));
    assert_eq!(client.kick_count, 1);
}
