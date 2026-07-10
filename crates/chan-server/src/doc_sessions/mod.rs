//! Live document sessions: the server-side authority for collaborative
//! editing over `@codemirror/collab`'s update-log model.
//!
//! `changes` is the pure ChangeSet layer (the pinned wire grammar, the
//! UTF-16 applier, whole-document diff); the session registry and the
//! disk-integration tasks build on top of it.

// Inert: nothing routes into this module until the doc WebSocket route
// mounts and the registry lands on AppState. Drop this allow then.
#![allow(dead_code)]

pub mod changes;
