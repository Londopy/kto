//! WPA2 4-way handshake capture - stub.
//!
//! The EAPOL state machine and `.hccapx` serializer are not
//! implemented in this source tree. The types and signatures are kept so the
//! surrounding code (events, exports, notifications) compiles against the real
//! shape. See `ABI_NOTE.md`.

use std::path::Path;

use super::EngineError;
use crate::util::MacAddr;

/// Progress through the 4-way handshake (M1->M2->M3->M4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandshakeState {
    #[default]
    Idle,
    GotM1,
    GotM2,
    GotM3,
    Complete,
}

/// Per-client EAPOL capture context.
#[derive(Debug, Default)]
pub struct HandshakeCapture {
    pub client: Option<MacAddr>,
    pub state: HandshakeState,
}

impl HandshakeCapture {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stub: Would feed an EAPOL frame into the state machine and advance
    /// it. Returns [`EngineError::NotImplemented`].
    pub fn feed(&mut self, _eapol_frame: &[u8]) -> Result<HandshakeState, EngineError> {
        Err(EngineError::NotImplemented("EAPOL handshake capture"))
    }

    /// Stub: Would serialize a completed handshake to hashcat `.hccapx`.
    pub fn write_hccapx(&self, _path: &Path) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented("hccapx serialization"))
    }

    pub fn is_complete(&self) -> bool {
        self.state == HandshakeState::Complete
    }
}
