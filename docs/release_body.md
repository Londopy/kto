**KTO - Kick Them Out** is a WiFi security-assessment tool for **authorized testing only**.

This build ships the CLI, TUI, and a Windows GUI. The live radio path is stubbed - run it
with `--simulate` to exercise the full pipeline safely. See the README for the full feature list.

## Install (Windows)

Pick the download that matches your CPU:

- **x64** - 64-bit Intel or AMD (almost everyone)
- **x86** - 32-bit Intel or AMD
- **arm64** - Windows on ARM (Snapdragon / Surface Pro X)

**Installer (recommended):** download `kto-__VERSION__-setup-<arch>.exe` and run it. It adds
KTO to your PATH and installs the GUI plus a "run at Windows startup" toggle.

**Scoop:**

    scoop install https://raw.githubusercontent.com/Londopy/kto/main/scoop/kto.json

**Portable zip:** download `kto-__VERSION__-windows-<arch>.zip`, verify it (below), then run
`kto-gui.exe` (GUI) or `kto.exe` (terminal).

## Verify your download

In PowerShell, hash the file and compare it to the matching line in the checksums:

    Get-FileHash .\kto-__VERSION__-windows-x64.zip -Algorithm SHA256

## SHA-256 checksums

__CHECKSUMS__

---

*Authorized use only. Deauthenticating networks you do not own, or lack written permission to
test, is illegal in most places - you are responsible for how you use this.*
