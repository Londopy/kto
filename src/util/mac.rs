//! `MacAddr` newtype: parsing, normalization, validation, and vendor-prefix
//! extraction. Serializes to/from the canonical upper-case colon form.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A 48-bit IEEE 802 MAC address.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MacAddr(pub [u8; 6]);

/// Error returned when a string cannot be parsed as a MAC address.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid MAC address: {0:?}")]
pub struct ParseMacError(pub String);

impl MacAddr {
    /// The all-ones broadcast address `FF:FF:FF:FF:FF:FF`.
    pub const BROADCAST: MacAddr = MacAddr([0xFF; 6]);

    /// The all-zero address.
    pub const ZERO: MacAddr = MacAddr([0x00; 6]);

    /// Raw six octets.
    #[inline]
    pub fn octets(&self) -> [u8; 6] {
        self.0
    }

    /// The 24-bit OUI (first three octets) as an upper-case hex key, e.g.
    /// `"A4:83:E7"`. Used for vendor lookups.
    pub fn oui_key(&self) -> String {
        format!("{:02X}:{:02X}:{:02X}", self.0[0], self.0[1], self.0[2])
    }

    /// True for the broadcast address.
    pub fn is_broadcast(&self) -> bool {
        *self == MacAddr::BROADCAST
    }

    /// True for group/multicast addresses (low bit of first octet set).
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    /// True for locally-administered addresses (bit 1 of first octet set).
    /// These are common on randomized-MAC phones.
    pub fn is_locally_administered(&self) -> bool {
        self.0[0] & 0x02 != 0
    }

    /// Compact display used in tight TUI columns, e.g. `11:22:…:66`.
    pub fn short(&self) -> String {
        format!(
            "{:02X}:{:02X}:…:{:02X}",
            self.0[0], self.0[1], self.0[5]
        )
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Debug for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MacAddr({self})")
    }
}

impl FromStr for MacAddr {
    type Err = ParseMacError;

    /// Accepts colon- or hyphen-separated forms (`AA:BB:CC:DD:EE:FF`,
    /// `aa-bb-cc-dd-ee-ff`) as well as the bare 12-hex-digit form.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleaned: String = s
            .chars()
            .filter(|c| !matches!(c, ':' | '-' | '.' | ' '))
            .collect();
        if cleaned.len() != 12 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ParseMacError(s.to_string()));
        }
        let mut octets = [0u8; 6];
        for (i, octet) in octets.iter_mut().enumerate() {
            let byte = &cleaned[i * 2..i * 2 + 2];
            *octet = u8::from_str_radix(byte, 16).map_err(|_| ParseMacError(s.to_string()))?;
        }
        Ok(MacAddr(octets))
    }
}

impl Serialize for MacAddr {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MacAddr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Parse a comma-separated list of MACs, ignoring blanks. Returns the first
/// parse error encountered.
pub fn parse_mac_list(raw: &str) -> Result<Vec<MacAddr>, ParseMacError> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(MacAddr::from_str)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colon_form() {
        let m: MacAddr = "AA:BB:CC:DD:EE:FF".parse().unwrap();
        assert_eq!(m.octets(), [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn parses_hyphen_and_bare_forms() {
        let a: MacAddr = "aa-bb-cc-dd-ee-ff".parse().unwrap();
        let b: MacAddr = "aabbccddeeff".parse().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn normalizes_to_upper_colon() {
        let m: MacAddr = "aa:bb:cc:dd:ee:ff".parse().unwrap();
        assert_eq!(m.to_string(), "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn rejects_malformed() {
        assert!("zz:zz".parse::<MacAddr>().is_err());
        assert!("AA:BB:CC:DD:EE".parse::<MacAddr>().is_err());
        assert!("".parse::<MacAddr>().is_err());
    }

    #[test]
    fn oui_key_is_first_three_octets() {
        let m: MacAddr = "A4:83:E7:11:22:33".parse().unwrap();
        assert_eq!(m.oui_key(), "A4:83:E7");
    }

    #[test]
    fn broadcast_and_flags() {
        assert!(MacAddr::BROADCAST.is_broadcast());
        assert!(MacAddr::BROADCAST.is_multicast());
        let local: MacAddr = "02:00:00:00:00:01".parse().unwrap();
        assert!(local.is_locally_administered());
    }

    #[test]
    fn list_parsing() {
        let list = parse_mac_list("AA:BB:CC:DD:EE:FF, 11:22:33:44:55:66").unwrap();
        assert_eq!(list.len(), 2);
    }
}
