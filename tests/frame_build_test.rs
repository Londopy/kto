//! Integration: frame TX is stubbed, frame RX parsing works.

use kto::net::frames::{self, subtype, FRAME_TYPE_MGMT};
use kto::util::MacAddr;

fn synthetic_beacon(ssid: &str) -> Vec<u8> {
    let mut f = Vec::new();
    f.push(0x80); // mgmt + beacon
    f.push(0x00);
    f.extend_from_slice(&[0, 0]);
    f.extend_from_slice(&[0xFF; 6]);
    f.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    f.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    f.extend_from_slice(&[0, 0]);
    f.extend_from_slice(&[0u8; 12]);
    f.push(0x00);
    f.push(ssid.len() as u8);
    f.extend_from_slice(ssid.as_bytes());
    f
}

#[test]
fn beacon_round_trips_through_parser() {
    let frame = synthetic_beacon("CorpNet");
    let header = frames::parse_mac_header(&frame).expect("header parses");
    assert_eq!(header.frame_type, FRAME_TYPE_MGMT);
    assert_eq!(header.subtype, subtype::BEACON);
    assert_eq!(header.addr3, "AA:BB:CC:DD:EE:FF".parse().unwrap());
    assert_eq!(frames::parse_ssid(&frame, 12).as_deref(), Some("CorpNet"));
}

#[test]
fn deauth_builder_is_not_implemented() {
    let m: MacAddr = "11:22:33:44:55:66".parse().unwrap();
    assert!(frames::build_deauth_frame(&m, &m, &m, 7).is_err());
}
