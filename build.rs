//! Build script. Exposes a couple of compile-time facts, and on Windows it
//! attaches the app icon + version strings to the .exe so Explorer, shortcuts,
//! and the taskbar all show the KTO icon.

use std::process::Command;

fn main() {
    // Short git hash -> KTO_GIT_HASH (falls back to "unknown").
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=KTO_GIT_HASH={git_hash}");

    // Build timestamp for the banner.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=KTO_BUILD_EPOCH={ts}");

    // Attach the icon + version info when building for Windows. The host doing
    // the build needs a resource compiler (the MSVC toolchain on the CI runner
    // has one); if it's missing we just warn and ship without the embedded icon.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "KTO");
        res.set("FileDescription", "KTO - WiFi deauth tool for authorized testing");
        res.set("LegalCopyright", "MIT licensed");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows icon/resources: {e}");
        }
    }

    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
}
