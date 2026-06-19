//! 802.11 frame handling.
//!
//! ## Receive path (implemented, benign)
//! Parsers for the management frames KTO needs for *discovery*: reading frame
//! control, addresses, and the SSID tagged parameter from beacons / probe
//! responses / probe requests. These only read bytes off the wire.
//!
//! ## Transmit path (stub)
//! [`build_deauth_frame`] is not implemented in this source
//! tree. It returns [`BuildError`]. See `src/engine/ABI_NOTE.md`.

use crate::util::MacAddr;

/// Management frame subtypes (type 0) used by the discovery path.
pub mod subtype {
    pub const ASSOC_REQ: u8 = 0;
    pub const REASSOC_REQ: u8 = 2;
    pub const PROBE_REQ: u8 = 4;
    pub const PROBE_RESP: u8 = 5;
    pub const BEACON: u8 = 8;
    pub const AUTH: u8 = 11;
    pub const DEAUTH: u8 = 12;
}

/// Frame type 0 = management.
pub const FRAME_TYPE_MGMT: u8 = 0;

/// A parsed 802.11 MAC header (the fields KTO cares about).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacHeader {
    pub frame_type: u8,
    pub subtype: u8,
    pub addr1: MacAddr, // receiver / destination
    pub addr2: MacAddr, // transmitter / source
    pub addr3: MacAddr, // BSSID (for the frames we parse)
}

/// Strip a RadioTap header, returning the inner 802.11 frame slice.
///
/// RadioTap: `[version:1][pad:1][len:2 LE][...]`. Returns `None` if the buffer
/// is too short or the declared length is inconsistent.
pub fn strip_radiotap(buf: &[u8]) -> Option<&[u8]> {
    if buf.len() < 4 {
        return None;
    }
    let len = u16::from_le_bytes([buf[2], buf[3]]) as usize;
    if len < 4 || len > buf.len() {
        return None;
    }
    Some(&buf[len..])
}

/// Parse the 24-byte management MAC header from an 802.11 frame (no RadioTap).
pub fn parse_mac_header(frame: &[u8]) -> Option<MacHeader> {
    if frame.len() < 24 {
        return None;
    }
    let fc = frame[0];
    let frame_type = (fc >> 2) & 0x03;
    let subtype = (fc >> 4) & 0x0F;
    let addr1 = read_mac(frame, 4)?;
    let addr2 = read_mac(frame, 10)?;
    let addr3 = read_mac(frame, 16)?;
    Some(MacHeader { frame_type, subtype, addr1, addr2, addr3 })
}

fn read_mac(buf: &[u8], off: usize) -> Option<MacAddr> {
    let bytes = buf.get(off..off + 6)?;
    let mut o = [0u8; 6];
    o.copy_from_slice(bytes);
    Some(MacAddr(o))
}

/// Extract the SSID from a management frame's tagged parameters.
///
/// `fixed_len` is the number of fixed-parameter bytes between the 24-byte
/// header and the tagged parameters: 12 for beacon/probe-response, 0 for probe
/// request. Returns `Some("")` for a wildcard (zero-length) SSID, `None` if
/// absent or malformed.
pub fn parse_ssid(frame: &[u8], fixed_len: usize) -> Option<String> {
    let start = 24 + fixed_len;
    let tags = frame.get(start..)?;
    let mut i = 0;
    while i + 2 <= tags.len() {
        let tag_id = tags[i];
        let tag_len = tags[i + 1] as usize;
        let val_start = i + 2;
        let val_end = val_start.checked_add(tag_len)?;
        if val_end > tags.len() {
            break;
        }
        if tag_id == 0 {
            // SSID element
            let raw = &tags[val_start..val_end];
            return Some(String::from_utf8_lossy(raw).into_owned());
        }
        i = val_end;
    }
    None
}

/// Convenience: parse a (possibly RadioTap-prefixed) frame into its header.
pub fn parse_frame(buf: &[u8]) -> Option<MacHeader> {
    let frame = strip_radiotap(buf).unwrap_or(buf);
    parse_mac_header(frame)
}

// ---------------------------------------------------------------------------
// Transmit path - STUB
// ---------------------------------------------------------------------------

/// Error returned by the unimplemented frame builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("deauth frame construction is not implemented in this build (see src/engine/ABI_NOTE.md)")]
pub struct BuildError;

/// Stub: Would assemble a raw 802.11 deauthentication management frame
/// (RadioTap + MAC header + reason code) for injection.
///
/// returns [`BuildError`] in this source tree. The signature is
/// kept stable so the call site in [`crate::engine::radio`] compiles unchanged
/// once an authorized implementation is supplied.
pub fn build_deauth_frame(
    _dst: &MacAddr,
    _src: &MacAddr,
    _bssid: &MacAddr,
    _reason: u16,
) -> Result<Vec<u8>, BuildError> {
    Err(BuildError)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic beacon frame (no RadioTap) advertising `ssid`.
    fn synthetic_beacon(ssid: &str) -> Vec<u8> {
        let mut f = Vec::new();
        // frame control: type=mgmt(0), subtype=beacon(8) -> 0b1000_00_00 = 0x80
        f.push(0x80);
        f.push(0x00); // flags
        f.extend_from_slice(&[0, 0]); // duration
        f.extend_from_slice(&[0xFF; 6]); // addr1 broadcast
        f.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]); // addr2
        f.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]); // addr3 bssid
        f.extend_from_slice(&[0, 0]); // seq
        f.extend_from_slice(&[0u8; 12]); // fixed params (timestamp/interval/cap)
        // SSID tag
        f.push(0x00);
        f.push(ssid.len() as u8);
        f.extend_from_slice(ssid.as_bytes());
        f
    }

    #[test]
    fn parses_beacon_header_and_ssid() {
        let f = synthetic_beacon("CorpNet");
        let h = parse_mac_header(&f).unwrap();
        assert_eq!(h.frame_type, FRAME_TYPE_MGMT);
        assert_eq!(h.subtype, subtype::BEACON);
        assert_eq!(h.addr3, "AA:BB:CC:DD:EE:FF".parse().unwrap());
        assert_eq!(parse_ssid(&f, 12).as_deref(), Some("CorpNet"));
    }

    #[test]
    fn radiotap_strip() {
        let mut buf = vec![0x00, 0x00, 0x08, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];
        buf.extend_from_slice(&[0x11, 0x22]);
        let inner = strip_radiotap(&buf).unwrap();
        assert_eq!(inner, &[0x11, 0x22]);
    }

    #[test]
    fn short_buffers_dont_panic() {
        assert!(parse_mac_header(&[0u8; 10]).is_none());
        assert!(strip_radiotap(&[0u8; 2]).is_none());
        assert!(parse_ssid(&[0u8; 10], 12).is_none());
    }

    #[test]
    fn builder_is_stubbed() {
        let m: MacAddr = "11:22:33:44:55:66".parse().unwrap();
        assert_eq!(build_deauth_frame(&m, &m, &m, 7), Err(BuildError));
    }
}
