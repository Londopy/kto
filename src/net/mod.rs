//! Network layer: frame parsing (RX), vendor/fingerprint intelligence, and
//! rogue-AP detection. Frame *injection* lives here too but is a stub - see
//! [`frames::build_deauth_frame`].

pub mod fingerprint;
pub mod frames;
pub mod oui;
pub mod rogue;

use std::fmt;

use crate::util::MacAddr;

/// WiFi encryption type, ordered weakest -> strongest for comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Encryption {
    Open,
    Wep,
    Wpa,
    Wpa2,
    Wpa3,
    Unknown,
}

impl Encryption {
    /// A rough strength rank (higher = stronger). `Unknown` sorts mid.
    pub fn strength(&self) -> u8 {
        match self {
            Encryption::Open => 0,
            Encryption::Wep => 1,
            Encryption::Wpa => 2,
            Encryption::Wpa2 => 3,
            Encryption::Wpa3 => 4,
            Encryption::Unknown => 2,
        }
    }

    pub fn parse(s: &str) -> Encryption {
        match s.to_ascii_uppercase().as_str() {
            "OPEN" | "NONE" => Encryption::Open,
            "WEP" => Encryption::Wep,
            "WPA" => Encryption::Wpa,
            "WPA2" => Encryption::Wpa2,
            "WPA3" => Encryption::Wpa3,
            _ => Encryption::Unknown,
        }
    }
}

impl fmt::Display for Encryption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Encryption::Open => "Open",
            Encryption::Wep => "WEP",
            Encryption::Wpa => "WPA",
            Encryption::Wpa2 => "WPA2",
            Encryption::Wpa3 => "WPA3",
            Encryption::Unknown => "?",
        };
        f.write_str(s)
    }
}

/// A discovered access point, built from beacon/probe-response frames.
#[derive(Debug, Clone)]
pub struct ApInfo {
    pub ssid: String,
    pub bssid: MacAddr,
    pub channel: u8,
    pub rssi: i8,
    pub encryption: Encryption,
    /// Management Frame Protection required (802.11w).
    pub pmf_required: bool,
    pub seen_count: u32,
}

impl ApInfo {
    pub fn vendor(&self) -> Option<&'static str> {
        oui::lookup(&self.bssid)
    }
}
