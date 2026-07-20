#![forbid(unsafe_code)]

mod actor;
mod config;
mod state;

pub use actor::{spawn_controller, ActorError, ControllerHandle, ProxyControlSession};
pub use config::Config;
pub use state::{
    ProxyStatus, ProxyView, SessionIncarnation, StateError, TunnelView, ADMISSION_CLAIM_TTL,
    CONVERGENCE_WINDOW, HEARTBEAT_INTERVAL, SESSION_DEAD_AFTER,
};

pub(crate) use state::{ControllerState, Effect, SessionKey};
