# KTO v3 - Kick Them Out

> **Authorized use only.** KTO is a WiFi security-assessment tool. Deauthentication
> activity against networks you do not own or lack written permission to test is
> illegal in most jurisdictions. You are responsible for how you use it.

KTO v3 is a Rust rewrite of the Python v2 tool. It provides client discovery,
session reporting, and a polished CLI/TUI/GUI front-end for wireless
assessments, with optional OS notifications, Discord Rich Presence, and an
update checker.

## Scope of this source tree

This repository contains the **complete application framework**: argument
parsing, shared state, configuration, client intelligence (vendor lookup,
passive fingerprinting, signal/probe history), the XP/streak/achievement layer,
all session exports, notifications, Discord presence, the update checker, and
the CLI + TUI front-ends.

The **live radio attack path is not implemented here.** The
functions that would inject raw 802.11 deauthentication frames and capture
EAPOL/PMKID material are present as typed stubs that return
`EngineError::NotImplemented`. They sit behind the `radio` Cargo feature and are
documented in [`src/engine/ABI_NOTE.md`](src/engine/ABI_NOTE.md). Everything in
the binary compiles and runs against a **simulation engine** so you can exercise
the full UI/stats/export pipeline without touching a radio.

## Install

**Windows - installer:** grab `kto-<ver>-setup-x64.exe` from the
[releases page](https://github.com/Londopy/kto/releases) and run it (adds `kto`
to your PATH). It also installs **KTO GUI**, a small
control-panel window (with a "run at Windows startup" toggle) that drives the
tool. Source is in [`gui-cpp/`](gui-cpp/).

**Windows - Scoop:**

```powershell
scoop install https://raw.githubusercontent.com/Londopy/kto/main/scoop/kto.json
```

**Linux (x64):** download the archive from the releases page,
verify it, and drop `kto` on your PATH:

```bash
sha256sum -c kto-<ver>-SHA256SUMS.txt
tar xzf kto-<ver>-x86_64-unknown-linux-gnu.tar.gz
```

**macOS / other platforms:** no prebuilt binaries, but the CLI/TUI builds from
source - install Rust and run `cargo build --release` (the GUI is Windows-only).

Every release ships SHA-256 checksums (`kto-<ver>-SHA256SUMS.txt`). See
[`docs/RELEASING.md`](docs/RELEASING.md) for how releases are built.

## Build

```bash
# Default: CLI + TUI + notifications + Discord (the radio path is stubbed)
cargo build --release

# Windows GUI (separate C++ app - needs CMake + MSVC)
cmake -S gui-cpp -B gui-cpp/build -A x64 && cmake --build gui-cpp/build --config Release

# Minimal: plain CLI only
cargo build --release --no-default-features

# Enable the (stubbed) radio path so the call sites are compiled in
cargo build --release --features radio
```

Building with `--features radio` requires libpcap headers
(`libpcap-dev` on Debian/Ubuntu, Npcap SDK on Windows).

## Quick start

```bash
# Run against the built-in simulation engine (safe; no interface touched)
kto -i wlan0mon -t CorpNet --simulate

# Plain CLI output
kto -i wlan0mon -t CorpNet --simulate --no-tui

# Force an update check and exit
kto --check-update

# Write current flags to the config file
kto -i wlan0mon -t CorpNet --aggressive --save-config
```

Run `kto --help` for the full flag reference (§4 of the spec).

## Configuration

Config lives at `~/.config/kto/config.toml` (Linux/macOS) or
`%APPDATA%\kto\config.toml` (Windows). Generate it with `--save-config`. See
[`docs/config.example.toml`](docs/config.example.toml) for the full schema.

## Layout

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the module map and thread
model.

## License

MIT © Londopy
