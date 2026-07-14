//! Live Excalidraw scene sessions: the server-side authority for
//! element-level collaborative drawing.
//!
//! Mirrors the doc_sessions architecture (registry keyed by
//! workspace-relative path, attach handles with lossless per-attachment
//! outboxes, a debounced flusher, a detach-grace reaper, and a watcher
//! reconciler) with a scene-typed state: instead of a CodeMirror
//! update log, [`scene`] holds a map of opaque elements merged by
//! Excalidraw's element-level last-writer-wins rule.

pub mod scene;
