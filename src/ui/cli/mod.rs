//! Plain-CLI front-end. Streams the activity log to stdout, or renders an
//! in-place client table with `--live-table`.

pub mod printer;
pub mod table;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::app::state::Shared;
use crate::cli::Args;
use crate::engine::{EngineCommand, EngineEvent, EngineHandle};
use crate::runtime::Runtime;

/// Run the CLI event loop until the engine stops or the user presses Enter.
pub fn run(shared: Shared, handle: EngineHandle, mut runtime: Runtime, args: &Args) {
    let no_color = args.no_color;

    // Header.
    let mode = if args.scan_only {
        "SCAN ONLY"
    } else if args.aggressive {
        "AGGRESSIVE - threaded"
    } else {
        "standard"
    };
    printer::info_line("Interface", args.interface.as_deref().unwrap_or("?"), no_color);
    printer::info_line("Target SSID", &args.target.join(", "), no_color);
    printer::info_line("Mode", mode, no_color);
    printer::info_line(
        "Burst",
        &format!("{} frames / direction  (reason {})", args.count, args.reason),
        no_color,
    );
    printer::info_line(
        "Scan dur.",
        &format!("{} s   Sweep interval: {} s", args.scan_duration, args.sleep),
        no_color,
    );
    if !no_color {
        println!();
    }
    eprintln!("(press Enter to stop and export)\n");

    // Watch stdin so the user can stop cleanly (and trigger exports).
    let quit = Arc::new(AtomicBool::new(false));
    spawn_stdin_watcher(quit.clone(), handle.commands.clone());

    let mut last_seq: u64 = 0;
    let mut stop_sent = false;
    loop {
        match handle.events.recv_timeout(Duration::from_millis(200)) {
            Ok(ev) => {
                let stopping = matches!(ev, EngineEvent::Stopped);
                runtime.apply(&shared, ev);
                if args.live_table {
                    let st = shared.read();
                    table::draw(&st, no_color);
                } else {
                    last_seq = drain_activity(&shared, last_seq, no_color);
                }
                if stopping {
                    break;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if args.live_table {
                    let st = shared.read();
                    table::draw(&st, no_color);
                }
                if quit.load(Ordering::Relaxed) && !stop_sent {
                    let _ = handle.commands.send(EngineCommand::Stop);
                    stop_sent = true;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }

    // Final flush of any remaining lines.
    if !args.live_table {
        drain_activity(&shared, last_seq, no_color);
    }

    super::finalize(&shared, args);
    let _ = handle.join.join();
}

/// Print all activity entries with `seq >= from_seq`; return the next seq.
fn drain_activity(shared: &Shared, from_seq: u64, no_color: bool) -> u64 {
    let st = shared.read();
    let mut next = from_seq;
    for entry in st.activity.iter().filter(|e| e.seq >= from_seq) {
        printer::print_activity(entry, no_color);
        next = entry.seq + 1;
    }
    next
}

fn spawn_stdin_watcher(quit: Arc<AtomicBool>, commands: crossbeam_channel::Sender<EngineCommand>) {
    std::thread::spawn(move || {
        let mut line = String::new();
        // A single read blocks until Enter or EOF - enough to request shutdown.
        let _ = std::io::stdin().read_line(&mut line);
        quit.store(true, Ordering::Relaxed);
        let _ = commands.send(EngineCommand::Stop);
    });
}
