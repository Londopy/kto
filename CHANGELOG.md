# Changelog

All notable changes to KTO are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/) and the project uses
[Semantic Versioning](https://semver.org/).

## [3.0.0] - unreleased

### Added
- Ground-up Rust rewrite of the Python v2 tool.
- Three UI front-ends behind feature flags: CLI (always), TUI (`ratatui`),
  GUI (`egui`).
- Shared `AppState` with thread-safe access via `parking_lot::RwLock`.
- Client intelligence: OUI vendor lookup, passive OS/device fingerprinting,
  signal history ring buffer, probe-SSID history, persistent nicknames and a
  known-device database.
- Session outputs: append-mode text log, JSON / CSV / self-contained HTML
  reports.
- Notifications (OS toast), Discord Rich Presence, and a GitHub release update
  checker with version-skip support.
- "Fun" layer: XP/leveling, kill streaks, achievements, Boss Mode, Matrix-rain
  Konami egg, and the hidden `--swordfish` / `--404` flags.
- Build matrix and feature flags for Linux/macOS/Windows.

- Windows GUI control panel (`gui-cpp/`) that drives the CLI, with a
  "run at Windows startup" toggle.
- App icon embedded in the Windows binary, the installer, and all shortcuts.
- Inno Setup installer (MIT license page, optional PATH + desktop shortcut,
  GitHub links) and a Scoop manifest.
- SHA-256 checksums published for every release asset.
- Discord Rich Presence is on by default (activate with `--discord`).

### Not included in this tree
- The live radio attack path (raw deauth frame injection, EAPOL 4-way
  handshake capture, PMKID extraction) ships as typed, non-functional stubs
  behind the `radio` feature. See `src/engine/ABI_NOTE.md`.

[3.0.0]: https://github.com/Londopy/kto/releases/tag/v3.0.0
