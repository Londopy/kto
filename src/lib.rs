//! KTO v3 - library root.
//!
//! All functionality lives in the library so it's reachable from both the `kto`
//! binary and the integration tests under `tests/`. See `docs/ARCHITECTURE.md`
//! for the module map, and `src/engine/ABI_NOTE.md` for the scope of the
//! stubbed radio path.

pub mod app;
pub mod banner;
pub mod cli;
pub mod discord;
pub mod engine;
pub mod error;
pub mod fun;
pub mod net;
pub mod notify;
pub mod runtime;
pub mod session;
pub mod ui;
pub mod update;
pub mod util;

use std::path::PathBuf;
use std::time::Duration;

use app::config::Config;
use app::state::AppState;
use cli::Args;
use engine::{Engine, RunParams};
use notify::Notifier;
use runtime::Runtime;

/// Process entry point used by `main`.
pub fn run() {
    init_tracing();
    let args = Args::parse_args();
    run_with_args(args);
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
}

/// The real entry logic, split out so it can be driven from tests if needed.
pub fn run_with_args(args: Args) {
    // ----- hidden easter-egg flags ----------------------------------------
    if args.swordfish {
        println!("{}", fun::easter_eggs::swordfish_text());
        return;
    }
    if args.four_oh_four {
        fake_scan_then_404();
        return;
    }

    if args.list_networks {
        print_networks();
        return;
    }

    // ----- config ----------------------------------------------------------
    let cfg_path = args
        .config
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(Config::default_path);
    let mut config = Config::load(&cfg_path).unwrap_or_else(|e| {
        eprintln!("Failed to load config {}: {e}", cfg_path.display());
        Config::default()
    });
    merge_args_into_config(&mut config, &args);

    if args.check_update {
        run_check_update(&config);
        return;
    }

    if args.save_config {
        match config.save(&cfg_path) {
            Ok(()) => println!("Saved config → {}", cfg_path.display()),
            Err(e) => eprintln!("Failed to save config: {e}"),
        }
        if !args.wants_run() {
            return;
        }
    }

    if args.target.is_empty() {
        eprintln!("error: at least one --target <SSID> is required (try --help).");
        std::process::exit(2);
    }
    let interface = args
        .interface
        .clone()
        .unwrap_or_else(|| config.interface.default_iface.clone());

    if !args.simulate {
        eprintln!(
            "note: the live radio engine is stubbed in this build. Run with --simulate to \
             exercise the full pipeline. See src/engine/ABI_NOTE.md."
        );
    }

    let shared = AppState::new(config.clone()).shared();

    if config.update.check_on_startup && !args.no_update_check {
        spawn_update_check(shared.clone(), config.clone());
    }

    let discord_handle = if args.discord || config.discord.enabled {
        discord::spawn(shared.clone(), config.discord.obfuscate_ssid)
    } else {
        None
    };

    let params = RunParams {
        interface,
        targets: args.target.clone(),
        channel: args.channel,
        scan_duration: args.scan_duration,
        sweep_interval: args.sleep,
        aggressive: config.deauth.aggressive,
        broadcast: config.deauth.broadcast,
        reason: config.deauth.reason_code,
        burst_count: config.deauth.count,
        scan_only: args.scan_only,
        hop: args.hop,
        capture_hs: args.capture_hs,
        pmkid: args.pmkid,
    };

    let engine: Box<dyn Engine> = if args.simulate {
        Box::new(engine::sim::SimEngine::new(params))
    } else {
        Box::new(engine::radio::RadioEngine::new(params))
    };
    let handle = engine.spawn();

    let notifier = Notifier::new(config.notifications.clone());
    let logger = args.log.as_ref().and_then(|p| {
        match session::log::SessionLogger::open(
            std::path::Path::new(p),
            args.interface.as_deref().unwrap_or("?"),
            &args.target.join(","),
            None,
        ) {
            Ok(l) => Some(l),
            Err(e) => {
                eprintln!("Failed to open log file {p}: {e}");
                None
            }
        }
    });
    let runtime = Runtime::new(notifier, logger);

    banner::print_banner(args.no_color, args.reason);
    launch_ui(shared, handle, runtime, &args, &config);

    if let Some(d) = discord_handle {
        d.stop();
    }
}

fn launch_ui(
    shared: app::state::Shared,
    handle: engine::EngineHandle,
    runtime: Runtime,
    args: &Args,
    config: &Config,
) {
    if args.gui {
        eprintln!("note: GUI is not built in this tree - falling back to TUI/CLI.");
    }

    let want_tui = !args.no_tui && (args.tui || config.ui.default_mode != "cli");

    #[cfg(feature = "tui")]
    {
        if want_tui {
            if let Err(e) = ui::tui::run(shared, handle, runtime, args) {
                eprintln!("TUI error: {e}");
            }
            return;
        }
    }
    #[cfg(not(feature = "tui"))]
    let _ = want_tui;

    ui::cli::run(shared, handle, runtime, args);
}

/// Apply CLI overrides onto the loaded config (booleans OR in; scalars win).
pub fn merge_args_into_config(config: &mut Config, args: &Args) {
    if let Some(theme) = args.theme {
        config.ui.theme = theme.as_str().to_string();
    }
    config.interface.auto_monitor |= args.auto_monitor;
    config.scan.duration = args.scan_duration;
    config.scan.sweep_interval = args.sleep;
    config.scan.channel_hop |= args.hop;
    config.scan.hop_dwell_ms = args.hop_dwell;
    config.deauth.count = args.count;
    config.deauth.delay = args.delay;
    config.deauth.reason_code = args.reason;
    config.deauth.aggressive |= args.aggressive;
    config.deauth.broadcast |= args.broadcast;
    config.deauth.use_aireplay |= args.aireplay;
    config.discord.enabled |= args.discord;
    if args.notify {
        config.notifications.enabled = true;
    }
    if args.no_update_check {
        config.update.check_on_startup = false;
    }
    if let Some(iface) = &args.interface {
        config.interface.default_iface = iface.clone();
    }
}

fn run_check_update(config: &Config) {
    let current = update::checker::current_version();
    match update::checker::check(current, &config.update.skip_version) {
        Ok(Some(info)) => {
            println!("Update available: v{} → {}", info.version, info.url);
            if let Some(notes) = info.changelog_snippet {
                println!("\n{notes}");
            }
        }
        Ok(None) => println!("KTO is up to date (v{current})."),
        Err(e) => eprintln!("Update check failed: {e}"),
    }
}

fn spawn_update_check(shared: app::state::Shared, config: Config) {
    std::thread::spawn(move || {
        let current = update::checker::current_version();
        if let Ok(Some(info)) = update::checker::check(current, &config.update.skip_version) {
            let version = info.version.clone();
            shared.write().update_available = Some(info);
            let mut notifier = Notifier::new(config.notifications.clone());
            notifier.handle(&notify::NotificationEvent::UpdateAvailable { version });
        }
    });
}

fn print_networks() {
    // Sample networks for the GUI picker (SSID, BSSID, channel, RSSI). Real
    // scanning lives behind the stubbed radio path, so this is demo data.
    let nets = [
        ("CorpNet", "AA:BB:CC:DD:EE:FF", 6, -52),
        ("CorpNet-Guest", "AA:BB:CC:DD:EE:F0", 6, -58),
        ("eduroam", "11:22:33:44:55:66", 11, -67),
        ("Starbucks WiFi", "66:77:88:99:AA:BB", 1, -71),
        ("NETGEAR47", "C0:FF:EE:12:34:56", 3, -74),
        ("Hidden", "00:11:22:33:44:55", 9, -80),
    ];
    for (ssid, bssid, ch, rssi) in nets {
        println!("{ssid}\t{bssid}\t{ch}\t{rssi}");
    }
}

fn fake_scan_then_404() {
    use std::io::Write;
    print!("[*] scanning");
    let _ = std::io::stdout().flush();
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(500));
        print!(" .");
        let _ = std::io::stdout().flush();
    }
    println!("\n");
    println!("{}", fun::easter_eggs::not_found_text());
}
