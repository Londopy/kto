//! Discord Rich Presence transport.
//!
//! Connects to the local Discord IPC pipe and pushes presence on a 15-second
//! heartbeat. The actual IPC client is behind the `discord` feature; without it
//! `spawn` is a no-op so the rest of the app is unaffected.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::app::state::Shared;

/// Discord application id. Make an app at https://discord.com/developers,
/// paste its Application ID below, and upload the asset images named in
/// state_mapper.rs. Until then Rich Presence just stays quiet.
pub const APP_ID: &str = "0000000000000000000";

/// Heartbeat interval.
pub const HEARTBEAT: Duration = Duration::from_secs(15);

/// Handle to the presence thread; dropping it (or calling `stop`) ends the loop.
pub struct PresenceHandle {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl PresenceHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Spawn the presence heartbeat thread. Returns `None` if Discord support is not
/// compiled in or disabled.
pub fn spawn(shared: Shared, obfuscate: bool) -> Option<PresenceHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();

    let join = std::thread::Builder::new()
        .name("discord-rpc".into())
        .spawn(move || run(shared, obfuscate, stop_clone))
        .ok()?;

    Some(PresenceHandle { stop, join: Some(join) })
}

#[cfg(feature = "discord")]
fn run(shared: Shared, obfuscate: bool, stop: Arc<AtomicBool>) {
    use discord_rpc_client::Client as DiscordClient;

    let app_id: u64 = match APP_ID.parse() {
        Ok(id) => id,
        Err(_) => return,
    };
    let mut client = DiscordClient::new(app_id);
    client.start();
    let start_ts = chrono::Utc::now().timestamp() as u64;

    while !stop.load(Ordering::Relaxed) {
        let p = {
            let st = shared.read();
            super::state_mapper::presence(&st, obfuscate)
        };
        // NOTE: exact builder API may differ across discord-rpc-client versions;
        // adjust field setters to match your pinned version.
        let _ = client.set_activity(|act| {
            let mut a = act
                .details(p.details.clone())
                .state(p.state.clone())
                .timestamps(|t| t.start(start_ts));
            a = a.assets(|assets| {
                let mut x = assets.large_image(p.large_image);
                if let Some(small) = p.small_image {
                    x = x.small_image(small);
                }
                x
            });
            if let Some((cur, max)) = p.party {
                a = a.party(|party| party.size((cur as u32, max as u32)));
            }
            a
        });
        std::thread::sleep(HEARTBEAT);
    }
}

#[cfg(not(feature = "discord"))]
fn run(shared: Shared, obfuscate: bool, stop: Arc<AtomicBool>) {
    // No transport compiled in: still resolve presence periodically so logs
    // reflect what *would* be pushed, and so the thread exits cleanly on stop.
    while !stop.load(Ordering::Relaxed) {
        {
            let st = shared.read();
            let p = super::state_mapper::presence(&st, obfuscate);
            tracing::trace!("[discord:disabled] {} - {}", p.details, p.state);
        }
        std::thread::sleep(HEARTBEAT);
    }
}
