//! Discord Rich Presence integration.

pub mod rpc;
pub mod state_mapper;

pub use rpc::{spawn, PresenceHandle};
pub use state_mapper::{presence, Presence};
