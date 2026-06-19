//! Passive OS / device fingerprinting.
//!
//! These are *read-only* heuristics over information elements observed in probe
//! and association requests. They classify; they do not attack.

/// Observed signals extracted from a client's management frames. All fields are
/// optional - fingerprinting is rough.
#[derive(Debug, Clone, Default)]
pub struct Signals {
    /// Supported-rates set looks like the classic Windows ordering.
    pub windows_rate_pattern: bool,
    /// RSN IE advertises both AES-CCMP and TKIP.
    pub rsn_ccmp_and_tkip: bool,
    /// Probe SSID list begins with an empty (wildcard) SSID.
    pub probe_empty_first: bool,
    /// Vendor-specific IE OUI == 00:50:F2 (WPS-capable / Microsoft).
    pub vendor_wps: bool,
    /// HT Capabilities present with SGI + LDPC.
    pub ht_sgi_ldpc: bool,
    /// VHT Capabilities present.
    pub vht_present: bool,
}

/// A rough OS / capability guess.
pub fn guess_os(s: &Signals) -> Option<String> {
    // Order matters: the more specific OS signals win over capability tags.
    if s.probe_empty_first {
        return Some("iOS".into());
    }
    if s.rsn_ccmp_and_tkip {
        return Some("Android likely".into());
    }
    if s.windows_rate_pattern {
        return Some("Windows likely".into());
    }
    if s.vht_present {
        return Some("802.11ac client".into());
    }
    if s.ht_sgi_ldpc {
        return Some("802.11n client".into());
    }
    if s.vendor_wps {
        return Some("WPS-capable".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ios_from_empty_probe() {
        let s = Signals { probe_empty_first: true, ..Default::default() };
        assert_eq!(guess_os(&s).as_deref(), Some("iOS"));
    }

    #[test]
    fn android_from_rsn() {
        let s = Signals { rsn_ccmp_and_tkip: true, ..Default::default() };
        assert_eq!(guess_os(&s).as_deref(), Some("Android likely"));
    }

    #[test]
    fn windows_from_rates() {
        let s = Signals { windows_rate_pattern: true, ..Default::default() };
        assert_eq!(guess_os(&s).as_deref(), Some("Windows likely"));
    }

    #[test]
    fn capability_fallbacks() {
        let s = Signals { vht_present: true, ..Default::default() };
        assert_eq!(guess_os(&s).as_deref(), Some("802.11ac client"));
        let s = Signals { ht_sgi_ldpc: true, ..Default::default() };
        assert_eq!(guess_os(&s).as_deref(), Some("802.11n client"));
    }

    #[test]
    fn nothing_known() {
        assert_eq!(guess_os(&Signals::default()), None);
    }
}
