//! The chan library: the authoritative window / terminal / workspace
//! lifecycle.
//!
//! This crate is the source of truth a chan deployment owns — the
//! per-library standalone terminals and the persisted window/terminal
//! lifecycle. Clients (native desktop windows, browser tabs, `cs`) are views
//! that reconcile to its state; `chan-server` is the HTTP route layer over it.
//!
//! It houses the terminal-session registry plus the terminal config and
//! unix-time helpers the registry needs; `chan-server`'s route layer
//! re-exports them so its handlers reach them unqualified.

#![forbid(unsafe_code)]

pub mod config;
pub mod terminal_sessions;
pub mod time;

pub use config::{
    TerminalConfig, TerminalFontChoice, TERMINAL_SCROLLBACK_MB_MAX, TERMINAL_SCROLLBACK_MB_MIN,
};
