# Radio engine - implementation note

The live radio path is **not implemented in this source tree.**

`RadioEngine` (in `radio.rs`) and the `handshake.rs` / `pmkid.rs` helpers expose
the exact type signatures the rest of the application expects, but their bodies
return `EngineError::NotImplemented`. Specifically, the following are stubs:

| Function                                  | What it would do                              |
|-------------------------------------------|-----------------------------------------------|
| `frames::build_deauth_frame`              | assemble a raw 802.11 deauth management frame |
| `RadioEngine::inject_deauth`              | `pcap` `sendpacket` of the above              |
| `RadioEngine::capture_loop`               | live monitor-mode capture                     |
| `handshake::HandshakeCapture::feed`       | EAPOL M1-M4 state machine                     |
| `handshake::HandshakeCapture::write_hccapx` | hashcat `.hccapx` serialization             |
| `pmkid::extract_pmkid`                    | PMKID derivation from RSNE / EAPOL M1         |

What **is** implemented and usable:

* the `Engine` trait, its event/command channels, and the default `SimEngine`;
* all 802.11 frame *parsing* used for read-only discovery (beacon/probe/assoc),
  which is non-offensive;
* channel planning, OUI lookup, fingerprint heuristics, rogue-AP detection;
* every UI, stats, export, notification, Discord, and update subsystem.

If you are operating under written authorization and your jurisdiction permits
it, the stubs are where you would add the platform-specific injection/capture
code. Keeping them isolated behind one trait and one Cargo feature (`radio`)
means the rest of the codebase needs no changes.
