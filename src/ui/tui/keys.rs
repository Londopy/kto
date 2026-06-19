//! Key mapping helpers for the TUI, including the Konami detector adapter.

use crossterm::event::KeyCode;

use crate::fun::easter_eggs::KonamiKey;

/// Translate a crossterm key into the abstract [`KonamiKey`] alphabet.
pub fn to_konami(code: KeyCode) -> KonamiKey {
    match code {
        KeyCode::Up => KonamiKey::Up,
        KeyCode::Down => KonamiKey::Down,
        KeyCode::Left => KonamiKey::Left,
        KeyCode::Right => KonamiKey::Right,
        KeyCode::Char('b') | KeyCode::Char('B') => KonamiKey::B,
        KeyCode::Char('a') | KeyCode::Char('A') => KonamiKey::A,
        _ => KonamiKey::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_arrows_and_letters() {
        assert_eq!(to_konami(KeyCode::Up), KonamiKey::Up);
        assert_eq!(to_konami(KeyCode::Char('B')), KonamiKey::B);
        assert_eq!(to_konami(KeyCode::Char('z')), KonamiKey::Other);
    }
}
