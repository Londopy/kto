//! Command-line interface (), defined with `clap`'s derive API.
//!
//! `Interface` and `Target` are modelled as optional here even though the spec
//! marks them required, because several flags (`--check-update`, `--swordfish`,
//! `--404`, `--save-config`) are valid without them. `main` enforces that a
//! normal run has both.

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    Dark,
    Light,
    Matrix,
    Blood,
    Dracula,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Matrix => "matrix",
            Theme::Blood => "blood",
            Theme::Dracula => "dracula",
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "kto",
    version,
    about = "KTO - WiFi deauthentication tool for authorized pen-testing",
    long_about = "Authorized use only. See README for the legal disclaimer.",
    after_help = "Run against the built-in simulator with --simulate (no interface touched)."
)]
pub struct Args {
    // ----- REQUIRED (enforced in main for live/sim runs) ------------------
    /// Wireless interface (monitor mode, or use --auto-monitor).
    #[arg(short, long)]
    pub interface: Option<String>,

    /// Target WiFi SSID. May be given multiple times for multi-target mode.
    #[arg(short, long)]
    pub target: Vec<String>,

    // ----- SCAN OPTIONS ----------------------------------------------------
    /// Seconds per passive sweep.
    #[arg(long, default_value_t = 8.0)]
    pub scan_duration: f64,

    /// Lock to a specific channel.
    #[arg(long)]
    pub channel: Option<u8>,

    /// Auto-select the strongest AP in mesh / multi-AP SSIDs.
    #[arg(long)]
    pub auto_bssid: bool,

    /// Enable channel hopping during scan.
    #[arg(long)]
    pub hop: bool,

    /// Milliseconds per channel while hopping.
    #[arg(long, default_value_t = 100)]
    pub hop_dwell: u64,

    /// Passive scan only; no active probe injection.
    #[arg(long)]
    pub passive: bool,

    // ----- DEAUTH OPTIONS --------------------------------------------------
    /// Deauth frames per burst per direction.
    #[arg(short = 'n', long, default_value_t = 5)]
    pub count: u32,

    /// Sweep interval in non-aggressive mode (seconds).
    #[arg(short = 's', long, default_value_t = 5.0)]
    pub sleep: f64,

    /// Per-client delay in aggressive loop (seconds).
    #[arg(long, default_value_t = 0.1)]
    pub delay: f64,

    /// Threaded: deauth runs parallel to scan.
    #[arg(long)]
    pub aggressive: bool,

    /// Add ff:ff:ff:ff:ff:ff broadcast deauth.
    #[arg(long)]
    pub broadcast: bool,

    /// Use aireplay-ng instead of native inject.
    #[arg(long)]
    pub aireplay: bool,

    /// 802.11 deauth reason code.
    #[arg(long, default_value_t = 7)]
    pub reason: u16,

    /// Passive discovery only, no deauth.
    #[arg(long)]
    pub scan_only: bool,

    // ----- WHITELIST -------------------------------------------------------
    /// Comma-separated MACs to spare.
    #[arg(short, long)]
    pub whitelist: Option<String>,

    /// File of MACs, one per line (# = comment).
    #[arg(long)]
    pub whitelist_file: Option<String>,

    // ----- HANDSHAKE -------------------------------------------------------
    /// Deauth clients and capture WPA2 4-way handshake. (stub in this build)
    #[arg(long)]
    pub capture_hs: bool,

    /// Attempt PMKID capture from EAPOL frames. (stub in this build)
    #[arg(long)]
    pub pmkid: bool,

    /// Write captured handshake(s) to.hccapx file.
    #[arg(long)]
    pub hs_out: Option<String>,

    // ----- OUTPUT ----------------------------------------------------------
    /// Append timestamped kick log.
    #[arg(long)]
    pub log: Option<String>,

    /// Export session summary as JSON on exit.
    #[arg(long)]
    pub export_json: Option<String>,

    /// Export session summary as CSV on exit.
    #[arg(long)]
    pub export_csv: Option<String>,

    /// Export a styled HTML session report on exit.
    #[arg(long)]
    pub export_html: Option<String>,

    // ----- UI MODES --------------------------------------------------------
    /// Launch TUI (default if compiled with the tui feature).
    #[arg(long)]
    pub tui: bool,

    /// Launch GUI (requires the gui feature).
    #[arg(long)]
    pub gui: bool,

    /// Force plain CLI even if the tui feature is present.
    #[arg(long)]
    pub no_tui: bool,

    /// Refresh an in-place client table (CLI only).
    #[arg(long)]
    pub live_table: bool,

    // ----- SYSTEM ----------------------------------------------------------
    /// Run airmon-ng start automatically.
    #[arg(long)]
    pub auto_monitor: bool,

    /// Enable OS toast notifications.
    #[arg(long)]
    pub notify: bool,

    /// Enable Discord Rich Presence.
    #[arg(long)]
    pub discord: bool,

    /// Force an update check and exit.
    #[arg(long)]
    pub check_update: bool,

    /// Load config from a TOML file.
    #[arg(long)]
    pub config: Option<String>,

    /// Write current flags to the config file.
    #[arg(long)]
    pub save_config: bool,

    /// Disable the background update check.
    #[arg(long)]
    pub no_update_check: bool,

    /// Run against the built-in simulator (no interface is touched). The
    /// offensive radio path is stubbed in this build, so this is the way to
    /// exercise the full UI/stat/export pipeline.
    #[arg(long)]
    pub simulate: bool,

    // ----- STYLE -----------------------------------------------------------
    /// Color theme.
    #[arg(long, value_enum)]
    pub theme: Option<Theme>,

    /// Disable ANSI colors.
    #[arg(long)]
    pub no_color: bool,

    // ----- MISC ------------------------------------------------------------
    /// Display spreadsheet decoy (press F12 to toggle).
    #[arg(long)]
    pub boss_mode: bool,

    // ----- HIDDEN ----------------------------------------------------------
    /// Easter egg (hidden).
    #[arg(long, hide = true)]
    pub swordfish: bool,

    /// Easter egg (hidden).
    #[arg(long = "404", hide = true)]
    pub four_oh_four: bool,
}

impl Args {
    pub fn parse_args() -> Args {
        Args::parse()
    }

    /// Whether any UI run (sim or live) is being requested, vs a one-shot flag.
    pub fn wants_run(&self) -> bool {
        !(self.check_update || self.swordfish || self.four_oh_four || self.save_config)
    }
}
