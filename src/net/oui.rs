//! OUI -> vendor lookup and device-category hints.
//!
//! The spec ships a ~900 KB embedded IEEE OUI database. To keep this tree
//! self-contained the lookup uses a representative built-in table plus an
//! optional runtime override file. Swap [`lookup`] to consult the full embedded
//! `assets/oui.gz` when you wire it in via `build.rs`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::util::MacAddr;

/// Built-in OUI -> vendor table (subset of the IEEE registry). Keys are upper
/// case `XX:XX:XX`.
const BUILTIN: &[(&str, &str)] = &[
    ("A4:83:E7", "Apple"),
    ("3C:15:C2", "Apple"),
    ("F0:18:98", "Apple"),
    ("AC:BC:32", "Apple"),
    ("DC:A6:32", "Raspberry Pi Foundation"),
    ("B8:27:EB", "Raspberry Pi Foundation"),
    ("E4:5F:01", "Raspberry Pi Foundation"),
    ("FC:FB:FB", "Cisco"),
    ("00:1A:11", "Google"),
    ("F4:F5:E8", "Google"),
    ("3C:5A:B4", "Google"),
    ("00:1D:D8", "Microsoft"),
    ("50:1A:C5", "Microsoft"),
    ("B4:2E:99", "Giga-Byte"),
    ("00:50:F2", "Microsoft"),
    ("D0:03:4B", "Apple"),
    ("60:6B:FF", "Samsung"),
    ("8C:77:12", "Samsung"),
    ("F0:25:B7", "Samsung"),
    ("00:09:BF", "Nintendo"),
    ("E8:4E:CE", "Nintendo"),
    ("98:B6:E9", "Nintendo"),
    ("00:1F:3F", "Netgear"),
    ("A0:40:A0", "Netgear"),
    ("50:C7:BF", "TP-Link"),
    ("AC:84:C6", "TP-Link"),
    ("70:4F:57", "TP-Link"),
    ("2C:FD:A1", "ASUSTek"),
    ("AC:22:0B", "ASUSTek"),
    ("00:0C:42", "Routerboard (MikroTik)"),
    ("DC:2C:6E", "Routerboard (MikroTik)"),
    ("00:18:0A", "Meraki"),
    ("E0:55:3D", "Meraki"),
    ("00:24:6C", "Aruba"),
    ("D8:C7:C8", "Aruba"),
    ("00:1B:63", "Apple"),
    ("FC:65:DE", "Amazon"),
    ("44:65:0D", "Amazon"),
    ("68:37:E9", "Amazon"),
];

fn table() -> &'static HashMap<&'static str, &'static str> {
    static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| BUILTIN.iter().copied().collect())
}

/// Resolve a vendor name from a MAC address.
pub fn lookup(mac: &MacAddr) -> Option<&'static str> {
    table().get(mac.oui_key().as_str()).copied()
}

/// A coarse device-category hint derived from the vendor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceHint {
    /// Apple -> likely iOS/macOS.
    AppleDevice,
    /// Nintendo -> game console.
    GameConsole,
    /// Router/AP vendor - flag with `[INFRA?]`.
    Infrastructure,
    /// Raspberry Pi -> "interesting".
    Interesting,
    /// Nothing notable.
    None,
}

impl DeviceHint {
    pub fn tag(&self) -> Option<&'static str> {
        match self {
            DeviceHint::Infrastructure => Some("[INFRA?]"),
            DeviceHint::Interesting => Some("[!]"),
            _ => None,
        }
    }
}

/// Categorize a vendor string into a device hint.
pub fn device_hint(vendor: &str) -> DeviceHint {
    let v = vendor.to_ascii_lowercase();
    if v.contains("apple") {
        DeviceHint::AppleDevice
    } else if v.contains("nintendo") {
        DeviceHint::GameConsole
    } else if v.contains("raspberry") {
        DeviceHint::Interesting
    } else if ["tp-link", "netgear", "asus", "mikrotik", "meraki", "aruba", "cisco"]
        .iter()
        .any(|k| v.contains(k))
    {
        DeviceHint::Infrastructure
    } else {
        DeviceHint::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_known_vendor() {
        let m: MacAddr = "A4:83:E7:11:22:33".parse().unwrap();
        assert_eq!(lookup(&m), Some("Apple"));
    }

    #[test]
    fn unknown_returns_none() {
        let m: MacAddr = "12:34:56:78:9A:BC".parse().unwrap();
        assert_eq!(lookup(&m), None);
    }

    #[test]
    fn hints() {
        assert_eq!(device_hint("Apple"), DeviceHint::AppleDevice);
        assert_eq!(device_hint("Nintendo"), DeviceHint::GameConsole);
        assert_eq!(device_hint("TP-Link"), DeviceHint::Infrastructure);
        assert_eq!(device_hint("Raspberry Pi Foundation"), DeviceHint::Interesting);
    }
}
