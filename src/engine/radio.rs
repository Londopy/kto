//! Live radio engine - stub.
//!
//! This is where the real `pcap` capture + frame-injection loop would run. In
//! this source tree the offensive methods return
//! [`EngineError::NotImplemented`], and `spawn` immediately emits an error event
//! explaining that the radio path is not built in, then stops.
//!
//! The whole rest of the application (UI, stats, exports, notifications,
//! Discord) is engine-agnostic, so supplying an authorized implementation here
//! is the only change required to go live. See `ABI_NOTE.md`.

use crossbeam_channel::unbounded;

use super::{Engine, EngineCommand, EngineError, EngineEvent, EngineHandle, RunParams};
use crate::net::frames;
use crate::util::MacAddr;

pub struct RadioEngine {
    pub params: RunParams,
}

impl RadioEngine {
    pub fn new(params: RunParams) -> Self {
        RadioEngine { params }
    }

    /// Stub: Would build and `pcap_sendpacket` a deauth frame (bidirectional
    /// plus optional broadcast). Returns [`EngineError::NotImplemented`].
    pub fn inject_deauth(
        &self,
        dst: &MacAddr,
        src: &MacAddr,
        bssid: &MacAddr,
        reason: u16,
    ) -> Result<(), EngineError> {
        // The frame builder is itself stubbed; constructing one here surfaces
        // the same not-implemented error rather than silently no-op'ing.
        let _frame = frames::build_deauth_frame(dst, src, bssid, reason)?;
        Err(EngineError::NotImplemented("deauth injection"))
    }

    /// Stub: Would run the monitor-mode capture loop, parsing beacons /
    /// probes / assoc requests and feeding the discovery tables.
    pub fn capture_loop(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented("pcap capture loop"))
    }
}

impl Engine for RadioEngine {
    fn spawn(self: Box<Self>) -> EngineHandle {
        let (ev_tx, ev_rx) = unbounded::<EngineEvent>();
        let (cmd_tx, _cmd_rx) = unbounded::<EngineCommand>();

        let join = std::thread::Builder::new()
            .name("engine-radio".into())
            .spawn(move || {
                let _ = ev_tx.send(EngineEvent::Error(
                    "radio engine is not implemented in this build - run with --simulate. \
                     See src/engine/ABI_NOTE.md."
                        .to_string(),
                ));
                let _ = ev_tx.send(EngineEvent::Stopped);
            })
            .expect("spawn engine-radio thread");

        EngineHandle { events: ev_rx, commands: cmd_tx, join }
    }
}
