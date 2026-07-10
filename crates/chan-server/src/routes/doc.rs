//! Live co-editing document sessions: the `/api/doc/ws` WebSocket route.
//!
//! One duplex socket per (editor mount, document) attachment, modeled on
//! the terminal route rather than the tenant-wide `/ws` broadcast: doc
//! traffic is keystroke-scale and per-path, and a lost or reordered frame
//! permanently desyncs a peer, so every attachment gets its own lossless
//! per-socket FIFO. chan-server is the central authority in the
//! `@codemirror/collab` update-log model: clients push `{version,
//! updates}`, the authority accepts a push only at a matching version
//! (a stale push rebases client-side and retries), and never transforms.
//!
//! The frame enums below ARE the wire contract the SPA's docSync layer
//! builds against; the serde tests in this module pin every tag, field
//! name, and shape. Change a pin only together with the client.

// Referenced only by the wire-contract pin tests until the ws route glue
// lands and constructs these frames.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Query parameters for `GET /api/doc/ws`.
#[derive(Debug, Deserialize)]
struct DocQuery {
    /// Workspace-relative POSIX path of the document to attach.
    path: String,
    /// The attaching window's `window_id`. Presence only: it labels this
    /// attachment's cursor via the session roster. Collab identity is the
    /// per-attachment `clientID` inside `push` frames, never `w` (two
    /// panes of one window may attach the same doc).
    w: String,
    /// Reconnect hint: the client's confirmed version. At or above the
    /// session's log base it earns an incremental `updates` catch-up;
    /// below the base or absent it earns a fresh `snapshot`.
    version: Option<u64>,
}

/// One collab update on the wire: `{clientID, changes}`.
///
/// `changes` is the CodeMirror `ChangeSet.toJSON()` value, carried
/// opaquely here; the section grammar is validated where the update is
/// applied, not at the frame layer. A client-sent `effects` field is
/// tolerated on input and dropped: this struct does not hold it, so
/// echoes and rebroadcasts never carry it.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DocUpdate {
    #[serde(rename = "clientID")]
    pub(crate) client_id: String,
    pub(crate) changes: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientFrame {
    /// Submit local updates on top of `version` (the client's confirmed
    /// version). Accepted only when it matches the authority's current
    /// version. ONE push in flight per client at a time.
    #[serde(rename = "push")]
    Push {
        version: u64,
        updates: Vec<DocUpdate>,
    },
    /// Explicit catch-up request from `version`.
    #[serde(rename = "pull")]
    Pull { version: u64 },
    /// Local selection moved. UTF-16 doc offsets, client-throttled; the
    /// server clamps, stamps the current version, and fans to the other
    /// attachments without rebasing.
    #[serde(rename = "cursor")]
    Cursor { anchor: u64, head: u64 },
}

/// A peer cursor as other attachments see it. `id` is the server attach
/// id, NOT the window id (two panes of one window may attach the same
/// doc); `w` is the owning window, resolved to a display name through the
/// session roster; `version` is the doc version the offsets were stamped
/// against.
#[derive(Debug, Serialize)]
pub(crate) struct PeerCursor {
    pub(crate) id: u64,
    pub(crate) w: String,
    pub(crate) anchor: u64,
    pub(crate) head: u64,
    pub(crate) version: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum ServerFrame {
    /// Full document state: answers an attach without a usable
    /// `?version=`, and any hard resync. `mtime_ns` is the
    /// flushed-to-disk CAS token as a decimal string (the `/api/files`
    /// convention: nanosecond epoch timestamps overflow JS number
    /// precision), null when the disk state is unknown.
    #[serde(rename = "snapshot")]
    Snapshot {
        path: String,
        version: u64,
        doc: String,
        dirty: bool,
        mtime_ns: Option<String>,
        cursors: Vec<PeerCursor>,
    },
    /// Committed updates, broadcast to every attachment INCLUDING the
    /// sender: the own-clientID echo is the sender's confirmation
    /// (standard `@codemirror/collab` pattern). `version` is the base the
    /// first update applies to; each socket sees strictly increasing,
    /// gapless versions.
    #[serde(rename = "updates")]
    Updates {
        version: u64,
        updates: Vec<DocUpdate>,
    },
    /// The in-flight push committed; `version` is the authority version
    /// after the commit.
    #[serde(rename = "push-ok")]
    PushOk { version: u64 },
    /// The in-flight push's base version did not match; `version` is the
    /// authority's current version. The missed `updates` frames are
    /// already in flight on this same socket: rebase through them and
    /// re-push.
    #[serde(rename = "push-stale")]
    PushStale { version: u64 },
    /// A peer's cursor moved. Same fields as a `snapshot.cursors` entry.
    #[serde(rename = "cursor")]
    Cursor {
        id: u64,
        w: String,
        anchor: u64,
        head: u64,
        version: u64,
    },
    /// A peer detached; drop its cursor.
    #[serde(rename = "cursor-gone")]
    CursorGone { id: u64 },
    /// The authority flushed to disk (`dirty: false` plus the fresh
    /// `mtime_ns` token) or repeatedly failed to (an `error` message; the
    /// session stays live and the content is safe in memory and in every
    /// client). Clients stamp `savedMtimeNs` from `mtime_ns`: that token
    /// is what keeps the classic autosave CAS correct after a
    /// degradation.
    #[serde(rename = "flush")]
    Flush {
        dirty: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        mtime_ns: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// The file vanished on disk. The client routes into its
    /// missing-file machinery; the server keeps the session but stops
    /// flushing (a deliberate delete is never resurrected by a flush).
    #[serde(rename = "removed")]
    Removed,
    /// Protocol error (malformed frame, bad changeset, oversized doc),
    /// followed by the server closing this attachment: a garbled frame
    /// means a desynced peer, so this route errors loudly, unlike `/ws`'s
    /// silent drop.
    #[serde(rename = "error")]
    Error {
        message: String,
        reason: &'static str,
    },
    /// Registry-initiated teardown (storage reset, shutdown).
    #[serde(rename = "closed")]
    Closed { reason: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(frame: &ServerFrame) -> String {
        serde_json::to_string(frame).expect("serialize server frame")
    }

    // ---- client -> server ----------------------------------------------

    #[test]
    fn client_push_decodes_the_pinned_shape() {
        // { type: "push", version, updates: [{clientID, changes}] }. The
        // `clientID` casing is CodeMirror's `Update.clientID`; a Rust-side
        // rename would break every SPA push with a green build, so pin the
        // decode.
        let frame: ClientFrame = serde_json::from_str(
            r#"{"type":"push","version":7,"updates":[
                {"clientID":"a-1","changes":[1,[0,"x"],3]},
                {"clientID":"a-1","changes":[[5]]}
            ]}"#,
        )
        .unwrap();
        match frame {
            ClientFrame::Push { version, updates } => {
                assert_eq!(version, 7);
                assert_eq!(updates.len(), 2);
                assert_eq!(updates[0].client_id, "a-1");
                assert_eq!(updates[0].changes, serde_json::json!([1, [0, "x"], 3]));
                assert_eq!(updates[1].changes, serde_json::json!([[5]]));
            }
            other => panic!("expected Push, got {other:?}"),
        }
    }

    #[test]
    fn client_push_requires_the_client_id_casing() {
        // snake_case / lowercase spellings are NOT the contract; reject
        // them so a drifted client fails loudly instead of pushing
        // anonymous updates.
        for bad in [
            r#"{"type":"push","version":0,"updates":[{"client_id":"a","changes":[]}]}"#,
            r#"{"type":"push","version":0,"updates":[{"clientid":"a","changes":[]}]}"#,
        ] {
            assert!(
                serde_json::from_str::<ClientFrame>(bad).is_err(),
                "must reject: {bad}"
            );
        }
    }

    #[test]
    fn client_push_tolerates_and_drops_effects() {
        // CM's `Update.toJSON()` may carry `effects`; the contract
        // tolerates the field on input and never rebroadcasts it.
        let frame: ClientFrame = serde_json::from_str(
            r#"{"type":"push","version":1,"updates":[{"clientID":"a","changes":[[0,"hi"]],"effects":["e"]}]}"#,
        )
        .unwrap();
        let ClientFrame::Push { version, updates } = frame else {
            panic!("expected Push");
        };
        let echo = enc(&ServerFrame::Updates { version, updates });
        assert!(!echo.contains("effects"), "echo must drop effects: {echo}");
    }

    #[test]
    fn client_pull_and_cursor_decode() {
        let pull: ClientFrame = serde_json::from_str(r#"{"type":"pull","version":42}"#).unwrap();
        match pull {
            ClientFrame::Pull { version } => assert_eq!(version, 42),
            other => panic!("expected Pull, got {other:?}"),
        }
        let cursor: ClientFrame =
            serde_json::from_str(r#"{"type":"cursor","anchor":3,"head":9}"#).unwrap();
        match cursor {
            ClientFrame::Cursor { anchor, head } => {
                assert_eq!(anchor, 3);
                assert_eq!(head, 9);
            }
            other => panic!("expected Cursor, got {other:?}"),
        }
    }

    #[test]
    fn client_unknown_or_malformed_frames_reject() {
        // Unknown tags and missing fields are protocol errors (the route
        // answers `error` + close, never a silent drop), so the decode
        // itself must fail.
        for bad in [
            r#"{"type":"nope"}"#,
            r#"{"version":1}"#,
            r#"{"type":"push","updates":[]}"#,
            r#"{"type":"pull"}"#,
            r#"{"type":"cursor","anchor":1}"#,
        ] {
            assert!(
                serde_json::from_str::<ClientFrame>(bad).is_err(),
                "must reject: {bad}"
            );
        }
    }

    // ---- server -> client ----------------------------------------------

    #[test]
    fn server_snapshot_pins_the_wire_shape() {
        let full = ServerFrame::Snapshot {
            path: "notes/a.md".into(),
            version: 3,
            doc: "hi".into(),
            dirty: false,
            mtime_ns: Some("1751234567890123456".into()),
            cursors: vec![PeerCursor {
                id: 7,
                w: "win-1".into(),
                anchor: 2,
                head: 5,
                version: 3,
            }],
        };
        assert_eq!(
            enc(&full),
            r#"{"type":"snapshot","path":"notes/a.md","version":3,"doc":"hi","dirty":false,"mtime_ns":"1751234567890123456","cursors":[{"id":7,"w":"win-1","anchor":2,"head":5,"version":3}]}"#
        );
        // Disk state unknown (e.g. after `removed`): mtime_ns is null, not
        // omitted; the client stamps savedMtimeNs unconditionally.
        let unknown = ServerFrame::Snapshot {
            path: "a".into(),
            version: 0,
            doc: String::new(),
            dirty: true,
            mtime_ns: None,
            cursors: vec![],
        };
        assert_eq!(
            enc(&unknown),
            r#"{"type":"snapshot","path":"a","version":0,"doc":"","dirty":true,"mtime_ns":null,"cursors":[]}"#
        );
    }

    #[test]
    fn server_updates_and_push_acks_pin_the_wire_shape() {
        let updates = ServerFrame::Updates {
            version: 4,
            updates: vec![DocUpdate {
                client_id: "$disk".into(),
                changes: serde_json::json!([2, [1, "z"]]),
            }],
        };
        assert_eq!(
            enc(&updates),
            r#"{"type":"updates","version":4,"updates":[{"clientID":"$disk","changes":[2,[1,"z"]]}]}"#
        );
        assert_eq!(
            enc(&ServerFrame::PushOk { version: 5 }),
            r#"{"type":"push-ok","version":5}"#
        );
        assert_eq!(
            enc(&ServerFrame::PushStale { version: 9 }),
            r#"{"type":"push-stale","version":9}"#
        );
    }

    #[test]
    fn server_cursor_frames_pin_the_wire_shape() {
        let moved = ServerFrame::Cursor {
            id: 7,
            w: "win-1".into(),
            anchor: 10,
            head: 10,
            version: 6,
        };
        assert_eq!(
            enc(&moved),
            r#"{"type":"cursor","id":7,"w":"win-1","anchor":10,"head":10,"version":6}"#
        );
        assert_eq!(
            enc(&ServerFrame::CursorGone { id: 7 }),
            r#"{"type":"cursor-gone","id":7}"#
        );
    }

    #[test]
    fn server_flush_pins_success_and_failure_shapes() {
        let clean = ServerFrame::Flush {
            dirty: false,
            mtime_ns: Some("1751234567890123456".into()),
            error: None,
        };
        assert_eq!(
            enc(&clean),
            r#"{"type":"flush","dirty":false,"mtime_ns":"1751234567890123456"}"#
        );
        let failed = ServerFrame::Flush {
            dirty: true,
            mtime_ns: None,
            error: Some("write failed".into()),
        };
        assert_eq!(
            enc(&failed),
            r#"{"type":"flush","dirty":true,"error":"write failed"}"#
        );
    }

    #[test]
    fn server_lifecycle_frames_pin_the_wire_shape() {
        assert_eq!(enc(&ServerFrame::Removed), r#"{"type":"removed"}"#);
        assert_eq!(
            enc(&ServerFrame::Error {
                message: "bad changeset".into(),
                reason: "bad-changeset",
            }),
            r#"{"type":"error","message":"bad changeset","reason":"bad-changeset"}"#
        );
        assert_eq!(
            enc(&ServerFrame::Closed { reason: "reset" }),
            r#"{"type":"closed","reason":"reset"}"#
        );
    }

    #[test]
    fn doc_query_decodes_path_w_and_optional_version() {
        // Axum's Query extractor deserializes through serde exactly like
        // this; pin the parameter names the SPA puts in the URL.
        let q: DocQuery =
            serde_json::from_str(r#"{"path":"notes/a.md","w":"win-1","version":12}"#).unwrap();
        assert_eq!(q.path, "notes/a.md");
        assert_eq!(q.w, "win-1");
        assert_eq!(q.version, Some(12));
        let q: DocQuery = serde_json::from_str(r#"{"path":"a","w":"win-1"}"#).unwrap();
        assert_eq!(q.version, None);
        // Both path and w are required; a client that omits either never
        // gets a socket.
        assert!(serde_json::from_str::<DocQuery>(r#"{"path":"a"}"#).is_err());
        assert!(serde_json::from_str::<DocQuery>(r#"{"w":"win-1"}"#).is_err());
    }
}
