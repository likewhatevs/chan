//! The devserver management-API wire contract.
//!
//! A small, versioned HTTP/JSON surface a chan-desktop client drives over
//! the tunnel to list, mount, toggle, and forget workspaces on a headless
//! box. It
//! is the reserved-root namespace of [`crate::devserver`]: the management
//! router answers `/api/devserver/*`, and every workspace tenant mounts
//! under a non-empty, legible prefix below it.
//!
//! This module defines only the on-wire types and their pinned JSON; the
//! axum handlers, auth, and runtime live in [`crate::devserver`]. The
//! contract lives in its own module (like [`chan_shell::wire`]) so a field
//! or tag rename moves on both sides at once: the wire strings are the
//! serde field names, so a one-sided rename compiles green and breaks at
//! runtime. The `*_wire` tests below pin the exact bytes against that.
//!
//! Auth split:
//! - `GET /api/devserver/info` is unauthenticated. It carries no secret
//!   (version, protocol, host label), so the connecting client can poll it
//!   to detect that the devserver is up before it holds any token.
//! - Every other endpoint requires `Authorization: Bearer <token>` with the
//!   devserver-level token, which is distinct from the per-workspace tokens.

use serde::{Deserialize, Serialize};

/// Wire-protocol version of the management API. The client reads it from
/// `GET /api/devserver/info` and refuses a server whose value it does not
/// understand, rather than best-effort-decoding an unknown shape. It is
/// independent of [`crate::devserver_handoff::PROTOCOL_VERSION`] (the
/// serve-to-devserver registration RPC) and of the per-workspace tokens.
pub const DEVSERVER_API_PROTOCOL: u32 = 1;

/// Response of `GET /api/devserver/info`, the unauthenticated health
/// probe. It carries no secret so a client can poll it before it holds a
/// token: version, protocol, and a human label for the box.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevserverInfo {
    /// `CARGO_PKG_VERSION` of the running devserver.
    pub devserver_version: String,
    /// [`DEVSERVER_API_PROTOCOL`]; the client refuses a value it does not
    /// understand.
    pub protocol: u32,
    /// Human label for the box, shown to group its workspaces in a client.
    pub host_label: String,
}

/// One element of `GET /api/devserver/workspaces`, the box's workspace
/// list as a client sees it. A client assembles each workspace URL itself
/// (`http://127.0.0.1:{local_port}{prefix}/index.html?t={token}`, the
/// loopback or `ssh -L` making the devserver's port reachable locally), so
/// it never allocates the prefix or mints the token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceEntry {
    /// Non-empty, legible route prefix the tenant is mounted at, e.g.
    /// `/api/notes-1a2b3c`. The devserver allocates it.
    pub prefix: String,
    /// Absolute workspace root on the box.
    pub path: String,
    /// Display name (the last path segment).
    pub label: String,
    /// Whether the workspace is mounted right now.
    pub on: bool,
    /// Per-workspace bearer token, minted devserver-side.
    pub token: String,
}

/// Body of `POST /api/devserver/workspaces`: mount the workspace rooted at
/// `path`. The call is idempotent, so an already-mounted root returns its
/// existing prefix with 200 rather than an error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWorkspaceRequest {
    /// Workspace root to mount, resolvable on the box.
    pub path: String,
}

/// Response of `POST /api/devserver/workspaces`: the prefix the new or
/// existing workspace tenant is mounted at. A client builds the tenant URL
/// from this plus the token it already holds from `GET workspaces`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MountedPrefix {
    /// Route prefix the tenant is mounted at, e.g. `/api/notes-1a2b3c`.
    pub prefix: String,
}

/// Body of `POST /api/devserver/workspaces/{prefix}/on`: set whether the
/// registered workspace at `{prefix}` is mounted right now. This is
/// **distinct from `DELETE`** (= Forget): toggling `on:false` unmounts the
/// workspace (releasing its per-workspace flock) but keeps it registered and
/// remembered as off, so the row stays in `GET workspaces` and re-mounts at
/// the **same** prefix on `on:true`. The handler answers `200` with the
/// updated [`WorkspaceEntry`] (a fresh `token` when `on:true`; `token:""`
/// when off), or `404` when `{prefix}` is not a registered workspace. The
/// call is idempotent in both directions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetWorkspaceOnRequest {
    /// Target mount state: `true` mounts the workspace (minting a fresh
    /// per-mount token), `false` unmounts it but keeps it registered.
    pub on: bool,
}

/// Response of `POST /api/devserver/terminals`: the prefix and per-tenant
/// token of the new standalone terminal tenant. Terminals are not carried
/// by `GET workspaces`, so the token comes back inline here; this is the
/// one place a client can capture it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MountedTerminal {
    /// Route prefix the terminal tenant is mounted at.
    pub prefix: String,
    /// Per-tenant bearer token, minted devserver-side.
    pub token: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // `to_value` asserts the exact on-wire JSON: a field or tag rename the
    // client did not agree to fails the build instead of production.

    #[test]
    fn devserver_info_wire() {
        let info = DevserverInfo {
            devserver_version: "0.38.0".into(),
            protocol: DEVSERVER_API_PROTOCOL,
            host_label: "build-box".into(),
        };
        let v = serde_json::to_value(&info).unwrap();
        assert_eq!(
            v,
            json!({
                "devserver_version": "0.38.0",
                "protocol": 1,
                "host_label": "build-box",
            })
        );
        assert_eq!(info, serde_json::from_value(v).unwrap());
    }

    #[test]
    fn workspace_entry_wire() {
        let entry = WorkspaceEntry {
            prefix: "/api/notes-1a2b3c".into(),
            path: "/home/u/notes".into(),
            label: "notes".into(),
            on: true,
            token: "tok_abc".into(),
        };
        let v = serde_json::to_value(&entry).unwrap();
        assert_eq!(
            v,
            json!({
                "prefix": "/api/notes-1a2b3c",
                "path": "/home/u/notes",
                "label": "notes",
                "on": true,
                "token": "tok_abc",
            })
        );
        assert_eq!(entry, serde_json::from_value(v).unwrap());
    }

    #[test]
    fn workspace_list_wire() {
        // The list endpoint is a bare JSON array of WorkspaceEntry.
        let list = vec![WorkspaceEntry {
            prefix: "/api/a-0000".into(),
            path: "/a".into(),
            label: "a".into(),
            on: false,
            token: String::new(),
        }];
        let v = serde_json::to_value(&list).unwrap();
        assert_eq!(
            v,
            json!([{
                "prefix": "/api/a-0000",
                "path": "/a",
                "label": "a",
                "on": false,
                "token": "",
            }])
        );
    }

    #[test]
    fn open_workspace_request_wire() {
        let req = OpenWorkspaceRequest {
            path: "/home/u/notes".into(),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v, json!({ "path": "/home/u/notes" }));
        assert_eq!(req, serde_json::from_value(v).unwrap());
    }

    #[test]
    fn mounted_prefix_wire() {
        let resp = MountedPrefix {
            prefix: "/api/notes-1a2b3c".into(),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v, json!({ "prefix": "/api/notes-1a2b3c" }));
        assert_eq!(resp, serde_json::from_value(v).unwrap());
    }

    #[test]
    fn mounted_terminal_wire() {
        let resp = MountedTerminal {
            prefix: "/api/term-1a2b3c".into(),
            token: "tok_t".into(),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v, json!({ "prefix": "/api/term-1a2b3c", "token": "tok_t" }));
        assert_eq!(resp, serde_json::from_value(v).unwrap());
    }

    #[test]
    fn set_workspace_on_request_wire() {
        // The toggle body is a single `{ "on": bool }`. The client posts this
        // exact shape to `.../{prefix}/on`; pin both directions so a rename
        // fails the build instead of silently no-op-ing the toggle.
        let off = SetWorkspaceOnRequest { on: false };
        let v = serde_json::to_value(&off).unwrap();
        assert_eq!(v, json!({ "on": false }));
        assert_eq!(off, serde_json::from_value(v).unwrap());

        let on = SetWorkspaceOnRequest { on: true };
        assert_eq!(serde_json::to_value(&on).unwrap(), json!({ "on": true }));
    }

    #[test]
    fn protocol_is_one() {
        // A client pins this value; guard a silent bump.
        assert_eq!(DEVSERVER_API_PROTOCOL, 1);
    }
}
