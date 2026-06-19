//! Simulation engine - the default, hardware-free [`Engine`].
//!
//! `SimEngine` produces a realistic stream of [`EngineEvent`]s on a background
//! thread: it "discovers" a target, surfaces a handful of synthetic clients
//! (drawn from real OUIs so vendor lookup and fingerprinting light up), and
//! "kicks" them on a loop while respecting pause / resume / whitelist / stop
//! commands. It lets the entire UI/stats/export/notify pipeline be exercised
//! with no radio and no privileges.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, TryRecvError};
use rand::seq::SliceRandom;
use rand::Rng;

use super::{Engine, EngineCommand, EngineEvent, EngineHandle, RunParams};
use crate::app::state::RunStatus;
use crate::net::fingerprint::Signals;
use crate::net::Encryption;
use crate::util::MacAddr;

/// A synthetic client template.
struct SimClient {
    mac: MacAddr,
    probe_ssids: Vec<String>,
    signals: Signals,
}

fn synthetic_clients() -> Vec<SimClient> {
    let mk = |s: &str| s.parse::<MacAddr>().unwrap();
    vec![
        SimClient {
            mac: mk("A4:83:E7:11:22:66"),
            probe_ssids: vec!["CorpNet".into(), "Starbucks".into(), "Home".into()],
            signals: Signals { probe_empty_first: true, vht_present: true, ..Default::default() },
        },
        SimClient {
            mac: mk("60:6B:FF:33:44:77"),
            probe_ssids: vec!["CorpNet".into(), "AndroidAP".into()],
            signals: Signals { rsn_ccmp_and_tkip: true, ht_sgi_ldpc: true, ..Default::default() },
        },
        SimClient {
            mac: mk("00:1D:D8:55:66:88"),
            probe_ssids: vec!["CorpNet".into(), "eduroam".into()],
            signals: Signals { windows_rate_pattern: true, ..Default::default() },
        },
        SimClient {
            mac: mk("00:09:BF:99:AA:BB"),
            probe_ssids: vec!["CorpNet".into()],
            signals: Signals::default(),
        },
        SimClient {
            mac: mk("B8:27:EB:CC:DD:EE"),
            probe_ssids: vec!["CorpNet".into(), "labwifi".into()],
            signals: Signals { ht_sgi_ldpc: true, ..Default::default() },
        },
        SimClient {
            mac: mk("F4:F5:E8:00:11:22"),
            probe_ssids: vec!["CorpNet".into(), "GoogleGuest".into()],
            signals: Signals { vht_present: true, ..Default::default() },
        },
    ]
}

pub struct SimEngine {
    params: RunParams,
}

impl SimEngine {
    pub fn new(params: RunParams) -> Self {
        SimEngine { params }
    }
}

impl Engine for SimEngine {
    fn spawn(self: Box<Self>) -> EngineHandle {
        let (ev_tx, ev_rx) = unbounded::<EngineEvent>();
        let (cmd_tx, cmd_rx) = unbounded::<EngineCommand>();
        let params = self.params;

        let join = std::thread::Builder::new()
            .name("engine-sim".into())
            .spawn(move || {
                let mut rng = rand::thread_rng();
                let send = |e: EngineEvent| {
                    let _ = ev_tx.send(e);
                };

                let ssid = params.targets.first().cloned().unwrap_or_else(|| "CorpNet".into());

                // --- scan phase -------------------------------------------
                send(EngineEvent::StatusChanged(RunStatus::Scanning));
                send(EngineEvent::Notice(format!("Scanning for SSID: {ssid}")));
                std::thread::sleep(Duration::from_millis(1500));
                send(EngineEvent::TargetFound {
                    ssid: ssid.clone(),
                    bssid: "AA:BB:CC:DD:EE:FF".parse().unwrap(),
                    channel: params.channel.unwrap_or(6),
                    encryption: Encryption::Wpa2,
                    pmf: false,
                });

                let clients = synthetic_clients();
                let mut discovered: Vec<usize> = Vec::new();
                let mut whitelisted: HashSet<MacAddr> = HashSet::new();
                let mut burst: u64 = 0;
                let mut sweep_n: u64 = 0;
                let mut paused = false;
                let mut aggressive = params.aggressive;
                let mut rogue_announced = false;
                let mut last_sweep = Instant::now();
                let sweep_every = Duration::from_secs_f64(params.sweep_interval.clamp(1.0, 10.0));

                if !params.scan_only {
                    send(EngineEvent::StatusChanged(RunStatus::Deauthing));
                } else {
                    send(EngineEvent::StatusChanged(RunStatus::ScanOnly));
                }

                loop {
                    // --- drain commands ----------------------------------
                    loop {
                        match cmd_rx.try_recv() {
                            Ok(EngineCommand::Stop) => {
                                send(EngineEvent::Notice("Stopping…".into()));
                                send(EngineEvent::Stopped);
                                return;
                            }
                            Ok(EngineCommand::Pause) => {
                                paused = true;
                                send(EngineEvent::StatusChanged(RunStatus::Paused));
                            }
                            Ok(EngineCommand::Resume) => {
                                paused = false;
                                send(EngineEvent::StatusChanged(if params.scan_only {
                                    RunStatus::ScanOnly
                                } else {
                                    RunStatus::Deauthing
                                }));
                            }
                            Ok(EngineCommand::SetAggressive(v)) => aggressive = v,
                            Ok(EngineCommand::SetBroadcast(_)) => {}
                            Ok(EngineCommand::ForceSweep) => last_sweep = Instant::now()
                                .checked_sub(sweep_every)
                                .unwrap_or_else(Instant::now),
                            Ok(EngineCommand::Whitelist(mac)) => {
                                whitelisted.insert(mac);
                                send(EngineEvent::Notice(format!("Whitelisted {mac}")));
                            }
                            Ok(EngineCommand::KickNow(mac)) => {
                                if !whitelisted.contains(&mac) {
                                    burst += 1;
                                    send(EngineEvent::ClientKicked { mac, burst });
                                }
                            }
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => {
                                send(EngineEvent::Stopped);
                                return;
                            }
                        }
                    }

                    // --- periodic sweep: maybe discover a new client -----
                    if last_sweep.elapsed() >= sweep_every {
                        sweep_n += 1;
                        send(EngineEvent::SweepStarted { n: sweep_n });
                        if discovered.len() < clients.len() && rng.gen_bool(0.85) {
                            let idx = discovered.len();
                            discovered.push(idx);
                            let c = &clients[idx];
                            let rssi = rng.gen_range(-78..=-48);
                            send(EngineEvent::ClientSeen {
                                mac: c.mac,
                                rssi,
                                probe_ssids: c.probe_ssids.clone(),
                                signals: c.signals.clone(),
                            });
                        }
                        send(EngineEvent::SweepCompleted {
                            n: sweep_n,
                            clients_found: discovered.len(),
                        });

                        // one-time rogue AP demo around the third sweep
                        if !rogue_announced && sweep_n == 3 {
                            rogue_announced = true;
                            send(EngineEvent::RogueAp {
                                bssid: "00:11:22:44:55:66".parse().unwrap(),
                                ssid: ssid.clone(),
                                reasons: vec![
                                    crate::net::rogue::RogueReason::DifferentVendor,
                                    crate::net::rogue::RogueReason::StrongerSignal,
                                ],
                            });
                        }
                        last_sweep = Instant::now();
                    }

                    // --- kick loop ---------------------------------------
                    if !paused && !params.scan_only && !discovered.is_empty() {
                        let active: Vec<usize> = discovered
                            .iter()
                            .copied()
                            .filter(|&i| !whitelisted.contains(&clients[i].mac))
                            .collect();
                        if let Some(&i) = active.choose(&mut rng) {
                            burst += 1;
                            send(EngineEvent::ClientKicked { mac: clients[i].mac, burst });
                            // refresh signal occasionally
                            if rng.gen_bool(0.3) {
                                let rssi = rng.gen_range(-80..=-45);
                                send(EngineEvent::ClientSeen {
                                    mac: clients[i].mac,
                                    rssi,
                                    probe_ssids: clients[i].probe_ssids.clone(),
                                    signals: clients[i].signals.clone(),
                                });
                            }
                        }
                    }

                    // aggressive mode kicks faster
                    let tick = if aggressive { 250 } else { 700 };
                    std::thread::sleep(Duration::from_millis(tick));
                }
            })
            .expect("spawn engine-sim thread");

        EngineHandle { events: ev_rx, commands: cmd_tx, join }
    }
}
