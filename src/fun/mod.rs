//! The "fun" layer: XP, kill streaks, achievements, and easter eggs.
//!
//! These systems are pure and side-effect free (aside from the achievement
//! store's own persistence) so they're easy to test and never touch a
//! radio.

pub mod achievements;
pub mod easter_eggs;
pub mod streaks;
pub mod xp;
