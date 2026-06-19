//! Rogue / evil-twin AP detection.
//!
//! Compares a candidate AP advertising the target SSID against the known-good
//! AP and flags suspicious differences. Purely analytical - defensive tooling.

use super::{ApInfo, Encryption};

/// How much stronger (dB) a same-SSID AP must be than the legit one before we
/// consider the signal "unusually high".
const SUSPICIOUS_RSSI_DELTA: i16 = 12;

/// A reason a candidate AP looks like a rogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RogueReason {
    /// Different vendor OUI than the legitimate AP.
    DifferentVendor,
    /// Much stronger signal than the known AP.
    StrongerSignal,
    /// Open or weaker encryption than the real AP.
    WeakerEncryption,
}

impl RogueReason {
    pub fn describe(&self) -> &'static str {
        match self {
            RogueReason::DifferentVendor => "different vendor OUI than the legitimate AP",
            RogueReason::StrongerSignal => "unusually strong signal vs. the known AP",
            RogueReason::WeakerEncryption => "weaker encryption than the real AP",
        }
    }
}

/// Evaluate `candidate` (which shares `legit`'s SSID but has a different BSSID)
/// and return all rogue indicators found. An empty vec means "looks benign".
pub fn evaluate(legit: &ApInfo, candidate: &ApInfo) -> Vec<RogueReason> {
    let mut reasons = Vec::new();
    if legit.ssid != candidate.ssid || legit.bssid == candidate.bssid {
        return reasons; // only compare same-SSID, different-BSSID pairs
    }

    let legit_oui = &legit.bssid.octets()[..3];
    let cand_oui = &candidate.bssid.octets()[..3];
    if legit_oui != cand_oui {
        reasons.push(RogueReason::DifferentVendor);
    }

    if (candidate.rssi as i16 - legit.rssi as i16) >= SUSPICIOUS_RSSI_DELTA {
        reasons.push(RogueReason::StrongerSignal);
    }

    if candidate.encryption.strength() < legit.encryption.strength()
        || candidate.encryption == Encryption::Open
    {
        reasons.push(RogueReason::WeakerEncryption);
    }

    reasons
}

/// True if any rogue indicator is present.
pub fn is_rogue(legit: &ApInfo, candidate: &ApInfo) -> bool {
    !evaluate(legit, candidate).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::MacAddr;

    fn ap(bssid: &str, rssi: i8, enc: Encryption) -> ApInfo {
        ApInfo {
            ssid: "CorpNet".into(),
            bssid: bssid.parse::<MacAddr>().unwrap(),
            channel: 6,
            rssi,
            encryption: enc,
            pmf_required: false,
            seen_count: 1,
        }
    }

    #[test]
    fn flags_evil_twin() {
        let legit = ap("AA:BB:CC:11:22:33", -60, Encryption::Wpa2);
        let twin = ap("00:11:22:44:55:66", -40, Encryption::Open);
        let reasons = evaluate(&legit, &twin);
        assert!(reasons.contains(&RogueReason::DifferentVendor));
        assert!(reasons.contains(&RogueReason::StrongerSignal));
        assert!(reasons.contains(&RogueReason::WeakerEncryption));
        assert!(is_rogue(&legit, &twin));
    }

    #[test]
    fn benign_same_vendor_same_strength() {
        let legit = ap("AA:BB:CC:11:22:33", -60, Encryption::Wpa2);
        let sibling = ap("AA:BB:CC:99:88:77", -62, Encryption::Wpa2);
        assert!(!is_rogue(&legit, &sibling));
    }

    #[test]
    fn ignores_different_ssid() {
        let legit = ap("AA:BB:CC:11:22:33", -60, Encryption::Wpa2);
        let mut other = ap("00:11:22:44:55:66", -40, Encryption::Open);
        other.ssid = "GuestNet".into();
        assert!(evaluate(&legit, &other).is_empty());
    }
}
