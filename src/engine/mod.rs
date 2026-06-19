//! Engine abstraction.
//!
//! The UI and stats layers talk to an [`Engine`] via two channels - an event
//! stream out and a command stream in - and never touch a radio directly. Two
//! implementations exist:
//!
//! * [`sim::SimEngine`] - the default, deterministic simulation (no hardware).
//! * [`radio::RadioEngine`] - the live pcap path, **stubbed** in this tree.

pub mod channel;
pub mod handshake;
pub mod pmkid;
pub mod radio;
pub mod sim;

use crossbeam_channel::{Receiver, Sender};

use crate::app::state::RunStatus;
use crate::net::fingerprint::Signals;
use crate::net::rogue::RogueReason;
use crate::net::Encryption;
use crate::util::MacAddr;

/// Errors the engine can raise.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("not implemented in this build: {0}")]
    NotImplemented(&'static str),

    #[error("interface error: {0}")]
    Interface(String),

    #[error(transparent)]
    FrameBuild(#[from] crate::net::frames::BuildError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// An event emitted by the engine for the runtime to fold into `AppState`.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    TargetFound {
        ssid: String,
        bssid: MacAddr,
        channel: u8,
        encryption: Encryption,
        pmf: bool,
    },
    SweepStarted {
        n: u64,
    },
    SweepCompleted {
        n: u64,
        clients_found: usize,
    },
    ClientSeen {
        mac: MacAddr,
        rssi: i8,
        probe_ssids: Vec<String>,
        signals: Signals,
    },
    ClientKicked {
        mac: MacAddr,
        burst: u64,
    },
    ClientGone {
        mac: MacAddr,
    },
    RogueAp {
        bssid: MacAddr,
        ssid: String,
        reasons: Vec<RogueReason>,
    },
    HandshakeCaptured {
        mac: MacAddr,
    },
    PmkidCaptured {
        mac: MacAddr,
    },
    StatusChanged(RunStatus),
    Notice(String),
    Error(String),
    Stopped,
}

/// A command sent into the engine from the UI / control loop.
#[derive(Debug, Clone)]
pub enum EngineCommand {
    Pause,
    Resume,
    ForceSweep,
    SetAggressive(bool),
    SetBroadcast(bool),
    Whitelist(MacAddr),
    KickNow(MacAddr),
    Stop,
}

/// Parameters for an engine run, derived from CLI args + config.
#[derive(Debug, Clone)]
pub struct RunParams {
    pub interface: String,
    pub targets: Vec<String>,
    pub channel: Option<u8>,
    pub scan_duration: f64,
    pub sweep_interval: f64,
    pub aggressive: bool,
    pub broadcast: bool,
    pub reason: u16,
    pub burst_count: u32,
    pub scan_only: bool,
    pub hop: bool,
    pub capture_hs: bool,
    pub pmkid: bool,
}

impl Default for RunParams {
    fn default() -> Self {
        RunParams {
            interface: "wlan0mon".into(),
            targets: Vec::new(),
            channel: None,
            scan_duration: 8.0,
            sweep_interval: 5.0,
            aggressive: false,
            broadcast: false,
            reason: 7,
            burst_count: 5,
            scan_only: false,
            hop: false,
            capture_hs: false,
            pmkid: false,
        }
    }
}

/// Handle returned after spawning an engine.
pub struct EngineHandle {
    pub events: Receiver<EngineEvent>,
    pub commands: Sender<EngineCommand>,
    pub join: std::thread::JoinHandle<()>,
}

/// The engine contract: consume `self`, spawn a worker thread, and return the
/// channels to talk to it.
pub trait Engine {
    fn spawn(self: Box<Self>) -> EngineHandle;
}
