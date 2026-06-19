//! TUI interaction state and key handling.

use std::time::Duration;

use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::matrix_rain::MatrixRain;
use crate::app::state::{ActivityKind, ClientStatus, Shared};
use crate::cli::Args;
use crate::engine::{EngineCommand, EngineHandle};
use crate::fun::easter_eggs::{self, KonamiDetector};
use crate::ui;
use crate::util::MacAddr;

/// Which pane currently has focus (for `Tab` cycling / scroll).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Clients,
    Log,
    Stats,
}

impl Pane {
    fn next(self) -> Pane {
        match self {
            Pane::Clients => Pane::Log,
            Pane::Log => Pane::Stats,
            Pane::Stats => Pane::Clients,
        }
    }
}

/// Local input mode (used for the nickname editor).
#[derive(Debug, Clone)]
pub enum InputMode {
    Normal,
    Nickname(String),
}

/// Per-frame TUI state that isn't part of the shared `AppState`.
pub struct TuiApp {
    pub selected: usize,
    pub focus: Pane,
    pub show_detail: bool,
    pub show_help: bool,
    pub input: InputMode,
    pub konami: KonamiDetector,
    pub rain: Option<MatrixRain>,
    pub status_flash: Option<String>,
    /// set once the 60-minute Mr. Robot line has fired
    pub mr_robot_shown: bool,
}

impl Default for TuiApp {
    fn default() -> Self {
        TuiApp {
            selected: 0,
            focus: Pane::Clients,
            show_detail: false,
            show_help: false,
            input: InputMode::Normal,
            konami: KonamiDetector::new(),
            rain: None,
            status_flash: None,
            mr_robot_shown: false,
        }
    }
}

impl TuiApp {
    /// Stable, sorted list of client MACs for the current target.
    pub fn client_macs(&self, shared: &Shared) -> Vec<MacAddr> {
        let st = shared.read();
        let mut macs: Vec<MacAddr> = st
            .target()
            .map(|t| t.clients.keys().copied().collect())
            .unwrap_or_default();
        macs.sort_by_key(|m| m.octets());
        macs
    }

    /// The currently selected MAC, if any.
    pub fn selected_mac(&self, shared: &Shared) -> Option<MacAddr> {
        let macs = self.client_macs(shared);
        macs.get(self.selected).copied()
    }

    /// Handle a key event. Returns `true` if the app should quit.
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        shared: &Shared,
        handle: &EngineHandle,
        args: &Args,
    ) -> bool {
        // Ctrl+C always quits.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }

        // Nickname editor captures all keys.
        if let InputMode::Nickname(buf) = &mut self.input {
            match key.code {
                KeyCode::Char(c) => buf.push(c),
                KeyCode::Backspace => {
                    buf.pop();
                }
                KeyCode::Enter => {
                    let name = buf.clone();
                    self.commit_nickname(shared, name);
                    self.input = InputMode::Normal;
                }
                KeyCode::Esc => self.input = InputMode::Normal,
                _ => {}
            }
            return false;
        }

        // Feed the Konami detector before normal handling.
        if self.konami.feed(super::keys::to_konami(key.code)) {
            self.trigger_konami(shared);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => return true,
            KeyCode::Esc => {
                if self.show_detail {
                    self.show_detail = false;
                } else if self.show_help {
                    self.show_help = false;
                }
            }
            KeyCode::Char('p') => {
                let paused = shared.read().paused;
                let _ = handle.commands.send(if paused {
                    EngineCommand::Resume
                } else {
                    EngineCommand::Pause
                });
            }
            KeyCode::Char('s') => {
                let _ = handle.commands.send(EngineCommand::ForceSweep);
            }
            KeyCode::Char('a') => {
                let mut st = shared.write();
                st.config.deauth.aggressive = !st.config.deauth.aggressive;
                let on = st.config.deauth.aggressive;
                drop(st);
                let _ = handle.commands.send(EngineCommand::SetAggressive(on));
            }
            KeyCode::Char('b') => {
                let mut st = shared.write();
                st.config.deauth.broadcast = !st.config.deauth.broadcast;
                let on = st.config.deauth.broadcast;
                drop(st);
                let _ = handle.commands.send(EngineCommand::SetBroadcast(on));
            }
            KeyCode::Char('w') => {
                if let Some(mac) = self.selected_mac(shared) {
                    {
                        let mut st = shared.write();
                        if let Some(t) = st.target_mut() {
                            if let Some(c) = t.clients.get_mut(&mac) {
                                c.status = ClientStatus::Whitelisted;
                            }
                        }
                        st.log(ActivityKind::Info, format!("Whitelisted {mac}"));
                    }
                    let _ = handle.commands.send(EngineCommand::Whitelist(mac));
                }
            }
            KeyCode::Char('n') => {
                if self.selected_mac(shared).is_some() {
                    self.input = InputMode::Nickname(String::new());
                }
            }
            KeyCode::Char('e') => {
                ui::finalize(shared, args);
                shared.write().log(ActivityKind::Good, "Exported session");
            }
            KeyCode::Char('c') => {
                shared.write().activity.clear();
            }
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::F(12) => {
                let mut st = shared.write();
                st.boss_mode = !st.boss_mode;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let n = self.client_macs(shared).len();
                if n > 0 {
                    self.selected = (self.selected + 1).min(n - 1);
                }
            }
            KeyCode::Enter => {
                if self.selected_mac(shared).is_some() {
                    self.show_detail = !self.show_detail;
                }
            }
            KeyCode::Tab => self.focus = self.focus.next(),
            _ => {}
        }
        false
    }

    fn commit_nickname(&self, shared: &Shared, name: String) {
        if let Some(mac) = self.selected_mac(shared) {
            let mut st = shared.write();
            if let Some(t) = st.target_mut() {
                if let Some(c) = t.clients.get_mut(&mac) {
                    c.nickname = if name.is_empty() { None } else { Some(name.clone()) };
                }
            }
            st.log(ActivityKind::Info, format!("Set nickname for {mac}: {name}"));
        }
    }

    fn trigger_konami(&mut self, shared: &Shared) {
        self.rain = Some(MatrixRain::new(120, Duration::from_secs(3)));
        let mut st = shared.write();
        st.stats.cheat_codes_used += 1;
        st.xp_multiplier_until = Some(Utc::now() + chrono::Duration::minutes(5));
        if st.achievements.unlock("konami") {
            let _ = st.achievements.save_default();
        }
        st.xp.add(easter_eggs::KONAMI_BONUS.as_secs()); // small flavor XP
        for line in easter_eggs::konami_banner().lines() {
            st.log(ActivityKind::Good, line.to_string());
        }
    }
}
