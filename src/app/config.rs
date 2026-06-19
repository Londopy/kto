//! Configuration: the on-disk TOML schema (), with load/save and
//! platform-correct path resolution.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub interface: InterfaceCfg,
    pub scan: ScanCfg,
    pub deauth: DeauthCfg,
    pub output: OutputCfg,
    pub notifications: NotificationsCfg,
    pub discord: DiscordCfg,
    pub update: UpdateCfg,
    pub ui: UiCfg,
    pub fun: FunCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceCfg {
    pub default_iface: String,
    pub auto_monitor: bool,
}
impl Default for InterfaceCfg {
    fn default() -> Self {
        Self { default_iface: "wlan0mon".into(), auto_monitor: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanCfg {
    pub duration: f64,
    pub sweep_interval: f64,
    pub channel_hop: bool,
    pub hop_dwell_ms: u64,
}
impl Default for ScanCfg {
    fn default() -> Self {
        Self { duration: 8.0, sweep_interval: 5.0, channel_hop: false, hop_dwell_ms: 100 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeauthCfg {
    pub count: u32,
    pub delay: f64,
    pub broadcast: bool,
    pub use_aireplay: bool,
    pub reason_code: u16,
    pub aggressive: bool,
}
impl Default for DeauthCfg {
    fn default() -> Self {
        Self {
            count: 5,
            delay: 0.1,
            broadcast: false,
            use_aireplay: false,
            reason_code: 7,
            aggressive: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputCfg {
    pub log_file: String,
    pub auto_export_json: bool,
    pub auto_export_html: bool,
    pub export_dir: String,
}
impl Default for OutputCfg {
    fn default() -> Self {
        Self {
            log_file: String::new(),
            auto_export_json: false,
            auto_export_html: false,
            export_dir: "~/.local/share/kto/sessions".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationsCfg {
    pub enabled: bool,
    pub new_client: bool,
    pub client_kicked: bool,
    pub handshake: bool,
    pub rogue_ap: bool,
    pub update_available: bool,
    pub kill_streak: bool,
}
impl Default for NotificationsCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            new_client: true,
            client_kicked: false, // off by default - too noisy
            handshake: true,
            rogue_ap: true,
            update_available: true,
            kill_streak: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DiscordCfg {
    pub enabled: bool,
    pub obfuscate_ssid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateCfg {
    pub check_on_startup: bool,
    pub skip_version: String,
}
impl Default for UpdateCfg {
    fn default() -> Self {
        Self { check_on_startup: true, skip_version: String::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiCfg {
    pub default_mode: String, // "cli" | "tui" | "gui"
    pub theme: String,
    pub live_table: bool,
}
impl Default for UiCfg {
    fn default() -> Self {
        Self { default_mode: "tui".into(), theme: "dark".into(), live_table: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FunCfg {
    pub kill_streaks: bool,
    pub xp_system: bool,
    pub sounds: bool,
    pub boss_mode_key: String,
}
impl Default for FunCfg {
    fn default() -> Self {
        Self {
            kill_streaks: true,
            xp_system: true,
            sounds: false,
            boss_mode_key: "F12".into(),
        }
    }
}

impl Config {
    /// Default config-file path: `~/.config/kto/config.toml` on Unix,
    /// `%APPDATA%\kto\config.toml` on Windows.
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kto")
            .join("config.toml")
    }

    /// Load from a path. Missing file -> defaults (not an error).
    pub fn load(path: &Path) -> anyhow::Result<Config> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let raw = fs::read_to_string(path)?;
        let cfg = toml::from_str(&raw)?;
        Ok(cfg)
    }

    /// Load from the default path.
    pub fn load_default() -> anyhow::Result<Config> {
        Config::load(&Config::default_path())
    }

    /// Serialize to a path, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml = toml::to_string_pretty(self)?;
        fs::write(path, toml)?;
        Ok(())
    }
}

/// Expand a leading `~` in a path string to the user's home directory.
pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(stripped) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_defaults() {
        let cfg = Config::default();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(back.deauth.reason_code, 7);
        assert_eq!(back.ui.default_mode, "tui");
        assert!(!back.notifications.client_kicked);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let toml = r#"
            [deauth]
            count = 9
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.deauth.count, 9);
        // untouched fields fall back to defaults
        assert_eq!(cfg.deauth.reason_code, 7);
        assert_eq!(cfg.scan.duration, 8.0);
    }

    #[test]
    fn tilde_expands() {
        let p = expand_tilde("~/foo");
        assert!(!p.to_string_lossy().starts_with('~'));
    }
}
