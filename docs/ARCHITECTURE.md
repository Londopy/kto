# KTO Architecture

## Module map

```
src/
├── main.rs            entry point: parse args → load config → launch UI
├── cli.rs             clap argument definitions (Args struct)
├── banner.rs          startup ASCII banner + theme-aware coloring
├── error.rs           top-level error type
│
├── app/
│   ├── state.rs       AppState (shared, RwLock-guarded), Client, Target, Stats
│   ├── config.rs      Config struct, TOML load/save, path resolution
│   └── achievements.rs persistent achievement store
│
├── util/
│   ├── mac.rs         MacAddr newtype (parse / normalize / validate / vendor key)
│   ├── time.rs        timestamp + duration formatting
│   └── signal.rs      RSSI → bar string + tier color
│
├── net/
│   ├── oui.rs         OUI → vendor lookup (embedded table, pluggable)
│   ├── fingerprint.rs passive OS/device heuristics
│   ├── rogue.rs       rogue/evil-twin AP detection
│   └── frames.rs      802.11 frame *parsing* (RX) + injection stubs (TX)
│
├── engine/
│   ├── mod.rs         Engine trait, EngineEvent, EngineError, EngineCommand
│   ├── sim.rs         SimEngine - deterministic simulation (default)
│   ├── radio.rs       RadioEngine - live pcap path (stubbed, `radio` feature)
│   ├── handshake.rs   EAPOL state machine (stub)
│   ├── pmkid.rs       PMKID extraction (stub)
│   └── channel.rs     channel plan + hop scheduler
│
├── fun/
│   ├── xp.rs          XP accrual, level curve, level titles
│   ├── streaks.rs     kill-streak tracker + tier names
│   ├── achievements.rs unlock conditions
│   └── easter_eggs.rs --swordfish / --404 / konami / mr-robot
│
├── session/
│   ├── log.rs         append-mode text logger
│   ├── export_json.rs
│   ├── export_csv.rs
│   └── export_html.rs self-contained HTML report
│
├── notify/
│   ├── mod.rs         Notifier trait + dispatch
│   ├── events.rs      NotificationEvent enum + rendering
│   └── debounce.rs    per-client rate limiting
│
├── discord/
│   ├── rpc.rs         connect / heartbeat / push presence (feature-gated)
│   └── state_mapper.rs AppState → presence fields
│
├── update/
│   ├── checker.rs     GitHub latest-release fetch + semver compare
│   └── info.rs        UpdateInfo + skip-version logic
│
└── ui/
    ├── cli/           printer + live table
    └── tui/           ratatui app, panes, keys, boss mode, matrix rain
```

## Engine abstraction

The UI and stats layers never talk to a radio directly. They talk to an
[`Engine`](../src/engine/mod.rs):

```text
        EngineCommand  ──────────────►  ┌─────────────┐
  UI / control loop                     │   Engine    │
        ◄──────────────  EngineEvent    └─────────────┘
                                          ▲         ▲
                                     SimEngine   RadioEngine
                                   (default)    (radio feature, stubbed)
```

* `SimEngine` produces a realistic stream of `EngineEvent`s (clients appear,
  associate, get "kicked", drop) on a background thread. It is the default and
  needs no hardware or privileges.
* `RadioEngine` is where the real pcap capture/inject loop would live. In this
  tree its frame-injection and handshake/PMKID methods return
  `EngineError::NotImplemented`. See `engine/ABI_NOTE.md`.

Swapping engines is a single match in `main.rs`, so the entire UI/stat/export/
notification/discord pipeline is identical in simulation and live modes.

## Thread model

| Thread        | Purpose                                            | Channel |
|---------------|----------------------------------------------------|---------|
| `main`        | UI event loop (TUI) or blocking CLI wait           | AppState RwLock |
| `engine`      | Sim or radio loop; emits `EngineEvent`             | `crossbeam-channel` |
| `notify`      | Debounced OS notification dispatch                 | `mpsc` rx |
| `discord_rpc` | 15s heartbeat + presence push                      | AppState read |
| `update`      | one-shot startup release check                     | AppState write |

## Windows GUI

`gui-cpp/` is a standalone Win32 control panel (C++). It doesn't link the Rust
code - it launches `kto.exe`, streams its stdout into a log box, and stops it by
writing a newline to its stdin (what the CLI watches for). It also owns the
"run at Windows startup" toggle (the `HKCU\...\Run` key).
