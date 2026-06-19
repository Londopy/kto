//! PMKID capture - stub.
//!
//! PMKID derivation from the RSNE / EAPOL message 1 is not
//! implemented in this source tree. The function shape is preserved. See
//! `ABI_NOTE.md`.

use super::EngineError;
use crate::util::MacAddr;

/// A captured PMKID record in the hashcat-friendly shape
/// `pmkid*bssid*clientmac*ssid_hex`.
#[derive(Debug, Clone)]
pub struct PmkidRecord {
    pub pmkid: [u8; 16],
    pub ap: MacAddr,
    pub station: MacAddr,
    pub ssid: String,
}

impl PmkidRecord {
    /// Render in hashcat `16800` style: `pmkid*bssid*clientmac*ssid_hex`.
    pub fn to_hashcat(&self) -> String {
        format!(
            "{}*{}*{}*{}",
            hex::encode(self.pmkid),
            hex::encode(self.ap.octets()),
            hex::encode(self.station.octets()),
            hex::encode(self.ssid.as_bytes()),
        )
    }
}

/// Stub: Would extract a PMKID from an EAPOL message-1 frame. Returns
/// [`EngineError::NotImplemented`].
pub fn extract_pmkid(_eapol_m1: &[u8]) -> Result<PmkidRecord, EngineError> {
    Err(EngineError::NotImplemented("PMKID extraction"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashcat_format_shape() {
        let rec = PmkidRecord {
            pmkid: [0xAB; 16],
            ap: "AA:BB:CC:DD:EE:FF".parse().unwrap(),
            station: "11:22:33:44:55:66".parse().unwrap(),
            ssid: "CorpNet".into(),
        };
        let s = rec.to_hashcat();
        // four star-separated fields
        assert_eq!(s.split('*').count(), 4);
        assert!(s.starts_with(&"ab".repeat(16)));
    }

    #[test]
    fn extraction_is_stubbed() {
        assert!(matches!(extract_pmkid(&[]), Err(EngineError::NotImplemented(_))));
    }
}
