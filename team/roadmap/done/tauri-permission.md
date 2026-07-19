# Authenticated Exact-Origin Tauri Permissions

> Status: shipped in [v0.71.0](../../release/release-v0.71.0.md).

Status: analysis and implementation proposal. Grounded against `a27007f5` (`v0.70.3`) and checked against the `v0.70.1` tag on 2026-07-18.

## Summary

v0.70.1 already grants native IPC to self-hosted gateway windows at runtime. The desktop discovers the gateway's proxy apex, converts it to a wildcard such as `https://*.usr.devsrv.io`, and installs the full devserver-window capability with `tauri::Manager::add_capability`. Adding a second static exception for `*.usr.devsrv.io` would duplicate existing behavior and preserve the wrong trust boundary.

For v0.71, remove both gateway-wide grants:

- Delete the static `https://*.devserver.chan.app` capability.
- Stop minting `https://*.{discovered-proxy-apex}` when the account-level gateway connects.
- After an explicit devserver entry request authenticates and authorizes one `(owner, devserver_id)`, validate the returned exact proxy origin and grant IPC only to that origin.
- Grant owned devservers automatically. Require one launcher confirmation per shared `(gateway, owner, full devserver_id)` before native IPC is enabled.

This treats official and self-hosted gateways identically, supports both current `{owner}--{disc}.{apex}` hosts and bare-user hosts returned by other deployments, and prevents one authenticated devserver from conferring native IPC on every sibling under the gateway.

The selected policy is exact ACL only. Keep the existing command vocabulary intact. This proposal does not add command wrappers, a native bridge protocol, or a loopback content proxy.

## Current Behavior

### Two capability layers already ship

The desktop currently has two independent ways for a gateway window to receive IPC:

1. `desktop/src-tauri/capabilities/devserver-window.json` statically grants `lib-*` windows on `https://*.devserver.chan.app` the `workspace-window` permission set, upload picker, fullscreen, webview zoom, and opener permissions.
2. `desktop/src-tauri/src/runtime_capability.rs` builds the same capability for self-hosted gateways. `gateway_proxy_remote_urls("https://usr.devsrv.io")` returns only `https://*.usr.devsrv.io`, and `mint_gateway_grant` installs it once per wildcard and application run.

`desktop/src-tauri/src/gateway.rs::connect_gateway` calls `mint_gateway_grant` after the first account roster attempt unless that attempt returns `401`. A `200` or `304` proves the PAT can read the account roster. The current `Upstream` branch also reaches the mint because every non-`401` `RosterFetch` shares the same tail. That nuance does not change the v0.70.1 diagnosis, but v0.71 should move the mint out of account connect entirely.

The runtime module and call site in the `v0.70.1` tag are byte-identical to the v0.70.3 baseline. Its mock-runtime tests exercise the real generated Tauri ACL context and prove all of the following:

- A custom gateway origin is denied before the runtime mint.
- The same already-open `lib-*` window is allowed immediately after the mint.
- A later `lib-*` window is allowed.
- An unrelated origin, the proxy apex, a non-`lib-*` label, and a command outside the capability remain denied.
- Malformed capability JSON and unresolved permission names are panic paths, while the production builder parses and resolves successfully.

The current implementation also records the Tauri limitation that drives the lifecycle design: capabilities accumulate, duplicate additions grow the authority, and there is no runtime removal API.

### Current gateway entry is already the per-devserver authority

The account roster proves what the caller may see, but it does not create a browser session for a row. An explicit connect of a synthesized `gw:` row follows this path:

1. `desktop/src-tauri/src/main.rs::rostered_conn` resolves the current roster row and loads the gateway account PAT.
2. `desktop/src-tauri/src/devserver.rs::gateway_conn` posts `{owner, devserver_id, path}` to the discovery-advertised desktop entry endpoint.
3. `gateway/crates/identity/src/http.rs::desktop_devserver_entry` validates the PAT, resolves the requested owner and full devserver id, calls the existing per-devserver access decision, checks tunnel liveness, and returns `username`, `devserver_id`, `proxy_origin`, `entry_url`, and `expires_at`.
4. The desktop exchanges the entry URL for host-only `devserver_gate` and `devserver_csrf` cookies, then starts the devserver connection and window watcher.

The gateway response is already sufficient for exact-origin ACLs and needs no API-version bump. The desktop currently deserializes only `proxy_origin` and `entry_url`, ignores the returned owner and devserver id, accepts an empty proxy origin by falling back to discovery, and does not bind later entry refreshes to the original origin. Those are the validation gaps v0.71 must close.

The gateway currently returns an exact host formed by `Config::devserver_host_for`: `{owner}--{first-12-id-hex}.{devserver_wildcard_suffix}`. The desktop already uses the returned origin rather than reconstructing that format when the field is non-empty.

### Existing native vocabulary

The remote devserver capability grants the same IPC vocabulary as a loopback `lib-*` window, except for `read_dropped_paths`, which remains local-only:

- Window lifecycle, reload, DevTools, zoom, URL probe, and platform detection.
- Clipboard text, image, and HTML reads and writes.
- Save-to-downloads.
- The native upload picker, which reads the files the user selects.
- Native fullscreen and webview zoom.
- Opening URLs in the system browser.

Exact-origin scoping does not reduce this set. The decision is to narrow who receives the existing vocabulary, not to introduce per-command grants in this release.

## Why `alice.usr.devsrv.io` Should Work on v0.70.1

If discovery advertises `devserver_proxy_origin: "https://usr.devsrv.io"`, the current runtime builder mints `remote.urls: ["https://*.usr.devsrv.io"]`. A `lib-*` webview loaded from `https://alice.usr.devsrv.io` therefore matches the grant. The current `{owner}--{disc}` form is also one label and matches the same wildcard.

A static `https://*.usr.devsrv.io` addition cannot fix a correctly executed v0.70.1 runtime path because it grants the same origin set. Diagnose which input or call path differs from that model:

1. **Binary provenance.** Reproduce with the exact v0.70.1 release artifact, confirm the version in About, and launch that bundle's executable directly when collecting logs. Eliminate an older installed app, an unrefreshed shadow bundle, and a stale process before changing policy.
2. **Discovery.** Fetch `https://<configured-identity-origin>/.well-known/chan-gateway`. Confirm `kind: "chan-gateway"`, `api_version: 1`, the configured identity origin, and `devserver_proxy_origin: "https://usr.devsrv.io"` with the expected scheme and port. An advertised tenant host would incorrectly produce a wildcard below that tenant.
3. **Registration path.** Confirm the address is configured on the Gateways screen and the failing machine is a synthesized gateway roster row. A plain devserver row uses the raw-dial path in `connect_devserver_impl_inner`; it does not run `connect_gateway` and therefore does not mint a gateway capability. The one-shot backstop only identifies this mistake after a raw connect fails.
4. **Account connect.** Confirm the gateway reached the account connect tail and did not receive `401`. A mint error raises the launcher notice titled `Gateway windows limited`; the desktop also logs `gateway window grant failed`.
5. **Actual webview origin.** In the failing window's DevTools, record `location.origin`. Compare its scheme, host, and effective port with the discovery-derived wildcard. The apex itself, another suffix, `http` instead of `https`, or a non-default port not advertised by discovery is outside the grant.
6. **Window class.** Confirm the native window label is `lib-*`. An `outbound-*` window deliberately receives no gateway IPC even if it loads the same URL.
7. **Failure shape.** An error such as `not allowed by ACL` points at capability resolution. Gateway `401`, `403`, `404`, cookie, CSRF, or watcher errors point at entry/session/transport handling instead.
8. **Source proof.** On a v0.70.1 checkout, `cargo test -p chan-desktop runtime_capability` exercises the custom-origin mint against the generated ACL context. A failure in the deployed app with this test green makes discovery, origin drift, call-path selection, grant installation, or artifact provenance the next targets.

For desktop stderr on macOS, use the documented direct-launch shape:

```sh
CHAN_LOG=chan_desktop=debug,warn /Applications/Chan.app/Contents/MacOS/chan-desktop
```

Do not add a static custom-domain exception until this checklist identifies a different concrete failure. It broadens authority without explaining why the existing equivalent runtime grant did not apply.

## Threat Boundary

The design separates four facts that the current wildcard grant conflates:

- Gateway discovery binds the configured identity origin to its entry, roster, and proxy namespace endpoints.
- Gateway account authentication proves the desktop account to the gateway and permits an account roster read.
- The authenticated entry endpoint additionally authorizes the account to one explicit `(owner, devserver_id)` and returns the origin that will host it.
- Tauri IPC authorizes web content to act on the local machine. That authority must be bound to the authorized entry target, not to every hostname the gateway can serve.

An owned devserver is the user's own PAT-backed machine and receives native access automatically. A shared devserver is controlled by another user. Its web content can request clipboard reads and writes, read user-selected local files, save downloads, control Chan windows, and open system-browser URLs. Gateway share authorization is necessary to view that content, but it is not consent to give that content native desktop authority.

The security invariants are:

- No official-domain special case. Official and self-hosted gateways take the same authenticated runtime path.
- No capability is minted from the account roster alone.
- One successful entry authorization can grant only the response's validated exact origin and only to `lib-*` windows.
- A response cannot redirect the entry token or the later connection to an origin outside the discovery-advertised proxy namespace.
- A shared row cannot start native windows without persisted consent for its exact owner and full devserver id.
- Roster removal or consent revocation prevents any asynchronous connect already in flight from starting a watcher.
- Capability mint failure fails only the explicit devserver connect. The account-level gateway and its roster poll remain connected.
- Revocation closes every window contributed by the row and prevents managed reconnection. Because Tauri grants are additive, the exact origin remains in the process authority until Chan Desktop quits.

The final invariant is a limitation, not a complete revocation claim. The desktop treats the grant as dormant after closing the row's windows and blocking its watcher, but the process authority would still match any future `lib-*` webview loaded at that exact origin. A full ACL purge requires application restart.

## Decision

### Recommend exact-origin runtime capabilities

Mint the existing capability lazily after the entry endpoint authorizes a concrete target. This is the smallest design that aligns native IPC with the gateway's existing per-devserver authorization boundary.

Benefits:

- Removes sibling-host authority from both official and custom gateways.
- Uses authenticated data the gateway already returns.
- Avoids encoding the gateway's hostname construction in the desktop.
- Preserves all existing native behavior for trusted devservers.
- Requires no gateway wire change and no local content-proxy subsystem.

### Reject hardcoded domain policy

Adding `*.usr.devsrv.io`, retaining `*.devserver.chan.app`, or maintaining an official-domain allowlist has the same structural defect: account-level trust in a gateway becomes native trust in every devserver origin below it. It also makes custom gateway support a release-by-release domain list.

### Defer a local-shell proxy

A stronger redesign could keep every Tauri-enabled webview on a loopback shell and proxy or message-bridge untrusted remote content through it. That would enable revocable per-command policy, but it would also introduce a new authenticated content proxy or native bridge, cookie and CSRF forwarding, WebSocket and streaming behavior, navigation policy, and a second command protocol. Exact-origin grants close the identified wildcard boundary without that subsystem. Revisit the shell design only if immediate in-process ACL removal or per-command mediation becomes a requirement.

## Proposed Design

### Persist shared native trust under the gateway

Extend `desktop/src-tauri/src/config.rs::Gateway` with a default-empty list:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeDevserverTrust {
    pub owner: String,
    pub devserver_id: String,
}

pub struct Gateway {
    // existing fields
    #[serde(default)]
    pub native_trust: Vec<NativeDevserverTrust>,
}
```

The identity key is `(gateway.id, owner, full devserver_id)`. Do not key trust by label, 12-character discriminator, proxy origin, or gateway apex:

- Labels are mutable display data.
- A discriminator is not the full gateway authorization key.
- Bare-user deployments may reuse an owner origin after devserver-id rotation.
- A gateway URL can be removed and later re-added as a new configured trust boundary.

`Gateway` ownership naturally removes its trust records when the configured gateway is removed. Gateway disconnect and restart retain trust. Existing configs deserialize with no trust, so shared devservers are untrusted after upgrade while owned rows need no migration.

Add `native_trust_required: bool` to `chan_library::DevserverEntry` and its TypeScript mirror. Projection rules:

- Plain configured devserver: `false`.
- Owned gateway roster row: `false`.
- Shared gateway roster row with an exact persisted trust tuple: `false`.
- Shared gateway roster row without that tuple: `true`.

Keep the backend authoritative. A direct `POST .../connect` must fail for an untrusted shared row even if a modified launcher skips the confirmation.

### Validate and freeze the entry origin

Expand `desktop/src-tauri/src/devserver.rs::GatewayEntryResponse` to consume the fields already on the wire:

```rust
#[derive(Deserialize)]
struct GatewayEntryResponse {
    username: String,
    devserver_id: String,
    proxy_origin: String,
    entry_url: String,
}
```

Centralize validation in one helper used for the initial entry, session refresh, and per-window navigation entry. It receives discovery, the requested `(owner, full devserver_id)`, an optional already-pinned connection origin, and the response. It returns canonical exact `proxy_origin` and parsed `entry_url` only when all checks pass.

Required checks:

1. `username` equals the requested owner and `devserver_id` equals the requested full id. Do not accept the gateway's fallback selection for an explicit roster connect.
2. `proxy_origin` is non-empty and parses as an HTTP(S) URL with a host.
3. It is an origin only: no credentials, query, fragment, or non-root path. Canonicalize it with the URL origin serialization before storing or comparing it.
4. It uses HTTPS outside the existing loopback-development allowlist.
5. Its scheme and effective port equal the discovery-advertised `devserver_proxy_origin` scheme and effective port.
6. Its host is exactly one DNS label below the discovery-advertised proxy apex. After removing `.<apex>` from the canonical host, the remaining prefix must be non-empty and contain no dot. Reject the apex, nested descendants, unrelated suffixes, and suffix lookalikes.
7. `entry_url` has no credentials and its canonical origin equals the validated `proxy_origin` exactly. Its path and entry-token query remain allowed.
8. When the connection already has a pinned origin, the newly returned `proxy_origin` and `entry_url` origin both equal it. A refresh cannot move the connection.

Compare effective ports, so omitted `:443` and explicit `:443` are equivalent before the origin is canonicalized. Preserve non-default ports in the canonical exact origin and therefore in `remote.urls`.

Remove the current empty-`proxy_origin` fallback. A gateway supporting the existing desktop entry contract already returns the field; accepting the apex fallback would recreate ambiguity exactly where native authority is minted.

Use the returned origin as the source of truth. Do not construct `{owner}.{domain}` or `{owner}--{disc}.{domain}` in the desktop. Exact response use supports both forms without coupling Chan Desktop to gateway routing policy.

### Connect sequence

Change the synthesized-row connect path to this order:

1. Resolve the current configured gateway, validated discovery, and exact roster row. Determine owned versus shared from the authenticated roster and evaluate persisted native trust.
2. Reject a shared untrusted row with a stable `native_trust_required` error. The launcher handles that state before calling connect, but the backend check is mandatory.
3. Load the account PAT and request an entry for the exact roster `(owner, full devserver_id)`.
4. Validate response owner, devserver id, proxy origin, and entry URL against discovery and the requested target.
5. Exchange the entry token through the existing no-redirect client for the gateway session cookies.
6. Mint the existing Tauri permission vocabulary with `remote.urls` containing only the canonical exact `proxy_origin`.
7. Re-read roster membership, ownership/shared status, persisted trust, and the row's lifecycle generation. If the row disappeared, became untrusted, or was revoked while entry/session/grant work was in flight, fail without registering the connection or starting a watcher.
8. Fetch the devserver rows and other connection seeds, register the connection, and start the window and color watchers. No native devserver window may exist before the exact grant succeeds and the final policy recheck passes.

The entry and capability operations are unavoidably additive and not transactional. A failure after step 6 can leave a dormant exact grant, but no `lib-*` window on that connection is opened. This is strictly narrower than minting the gateway wildcard at account connect.

If capability minting returns an error, clear the row's connecting state, tear down any partial devserver state, surface a devserver-scoped error, and leave `GatewayRuntime` plus its roster poll intact. Do not retain the current nonfatal `Gateway windows limited` behavior for an explicit connect, because a connected window without its required native vocabulary is a partial and misleading success.

### Runtime capability shape

Refactor `desktop/src-tauri/src/runtime_capability.rs` from a proxy-apex wildcard builder to an exact-origin builder:

- Input is the already validated canonical exact origin.
- `remote.urls` is a one-element list containing that origin verbatim.
- `windows` remains exactly `lib-*`.
- `permissions` remains exactly the current `devserver-window.json` list.
- No scoped permission and no deny entry may be emitted. The current Tauri collision and origin-blind-deny warnings still apply.
- Track each canonical exact origin once per process to avoid duplicate authority entries.
- Parse the generated `CapabilityFile` before `add_capability` and retain generated-context resolution tests so malformed and unknown-permission panic paths remain unreachable.

Delete `desktop/src-tauri/capabilities/devserver-window.json` and remove its static-shape pins. Update the origin-aware ACL walk in `desktop/src-tauri/src/serve.rs` to inject only exact runtime capabilities for authenticated test origins. Add a repository pin that no tracked static or runtime capability contains a `*.chan.app` remote URL.

Remove the `mint_gateway_grant` call from `connect_gateway`. Account connect should discover, authenticate, fetch the roster, and poll it. It should not change Tauri authority.

### Launcher consent and trust routes

Add desktop-only launcher routes:

```text
PUT    /api/library/devservers/{synthesized_id}/native-trust
DELETE /api/library/devservers/{synthesized_id}/native-trust
```

Place them in the devserver subrouter behind the existing launcher bearer, `require_local_mutation`, and desktop bridge. Validate that the id parses as a synthesized `gw:` row, its gateway is still configured and connected, and the current authenticated roster still contains the exact shared `(owner, devserver_id)`. Plain rows and owned rows do not accept trust mutations.

Model these as new `DesktopWindowOp` variants rather than expanding the generic `DevserverRegistry` CRUD contract. Trust persistence needs `AppState` gateway state, and revocation must invoke `teardown_devserver_connection`; both are desktop integration concerns, like connect and disconnect.

PUT is idempotent. It persists the exact tuple and signals the library feed before returning. The launcher then re-lists the row and calls connect only after PUT succeeds.

DELETE is also idempotent for a current shared row. It invalidates that row's lifecycle generation, removes persisted consent, tears down the live connection and every window through the existing full teardown, then signals the feed. The endpoint does not return success until persistence and teardown have completed.

On the Connect button for `native_trust_required: true`, use the existing in-SPA confirmation dialog. The confirmation must state:

> This shared devserver controls the web content in its Chan windows. Native access can read and write your clipboard, read files you select, save downloads, control Chan windows, and open links in your system browser. Grant access only if you trust its owner.

Use `Grant native access` as the confirm action. Cancel performs no PUT, no connect, and no pending-state transition. Confirm awaits PUT, refreshes the row, then starts the ordinary connect. A trusted shared row exposes a `Revoke native access` action that calls DELETE.

### Roster and revocation lifecycle

The current roster poll computes a `RosterDiff`, replaces `GatewayRuntime.roster`, and then discards the diff except for a changed boolean. v0.71 must return the affected row keys to the caller so teardown and trust pruning happen outside the runtime-map lock.

On every fresh roster:

- A removed shared row is immediately disconnected, all its windows are closed, and its persisted trust tuple is pruned.
- A changed row is re-evaluated. If an owned row becomes shared and has no exact trust, disconnect it. If a shared row becomes owned, native access becomes automatic and any stale trust tuple is pruned.
- A devserver-id rotation is a remove plus add. The old id is disconnected and pruned; the new full id requires fresh shared consent.
- An upstream roster failure retains the last-known roster, connection state, and trust, matching current degraded-mode semantics. It is not evidence of revocation.

Gateway user disconnect, gateway removal, and roster `401` continue to run the existing cascade that closes every contributed connection and window. User disconnect retains persisted shared trust. Gateway removal deletes it with the `Gateway` record. A `401` retains it until a later authenticated roster can confirm membership or removal.

Use a per-row lifecycle generation, or an equivalent serialized policy token, for connect-versus-revoke safety:

- Capture it before asynchronous entry work.
- Increment it when trust is revoked, a relevant roster row changes or disappears, or the gateway cascades.
- Check it with roster membership and trust immediately before registering the connection and watcher.

A stale connect may already have minted an exact capability, which cannot be removed, but it must not open a target window or remain connected. This makes the residual grant dormant under the desktop-managed lifecycle and makes revocation behavior deterministic. It does not turn the Tauri authority entry into a revoked grant.

## Implementation Surface

Expected files and responsibilities:

- `desktop/src-tauri/src/runtime_capability.rs`: exact-origin builder, per-origin tracking, generated-context invoke tests.
- `desktop/src-tauri/capabilities/devserver-window.json`: delete the static official-domain grant.
- `desktop/src-tauri/src/serve.rs`: update static capability inventory, origin-aware parity classes, and no-wildcard pins.
- `desktop/src-tauri/src/devserver.rs`: deserialize all entry identity fields, centralize entry/origin validation, pin origin across refresh and navigation.
- `desktop/src-tauri/src/main.rs`: enforce trust in `rostered_conn`, mint before watcher setup, make mint failure fatal to the row connect, and perform the final policy-generation recheck.
- `desktop/src-tauri/src/gateway.rs`: remove account-connect minting, return actionable fresh-roster diffs, and trigger row teardown/pruning without holding the runtime lock.
- `desktop/src-tauri/src/config.rs`: persist per-gateway native trust and project `native_trust_required`.
- `crates/chan-library/src/devserver_registry.rs`: add the wire field with a default.
- `crates/chan-library/src/desktop_window_ops.rs`, `crates/chan-server/src/routes/library.rs`, and `desktop/src-tauri/src/window_ops.rs`: wire guarded trust PUT/DELETE operations through the desktop bridge.
- `web/packages/launcher/src/api/library.ts`, `state/library.svelte.ts`, and `components/Library.svelte`: mirror the field, add trust methods, confirmation, ordered PUT-then-connect, and revoke UI.
- Desktop, chan-library, chan-server, launcher mock, demo, and fixture construction sites: populate the new required field consistently.
- `desktop/design.md`: replace the static/wildcard description with authenticated exact-origin behavior and document shared consent plus the restart-only ACL purge.

No gateway implementation change is required. Keep the existing identity entry response and API version. Strengthen gateway tests only if needed to keep `username`, full `devserver_id`, exact `proxy_origin`, and same-origin `entry_url` pinned as the desktop dependency.

## Verification

### Baseline proof

- Check out `v0.70.1` and run `cargo test -p chan-desktop runtime_capability`.
- Pin that `runtime_capability.rs` and the `connect_gateway` mint call are present in v0.70.1, not introduced after it.
- Preserve a test that a custom self-hosted gateway wildcard is allowed under the v0.70.1 implementation. This guards the corrected diagnosis independently of the v0.71 redesign.

### Exact capability tests

Drive the production `on_message` dispatch through the mock runtime and real generated ACL context, as the current tests do:

- The exact authorized origin on a `lib-*` label can invoke every expected app and plugin command.
- A sibling owner/devserver origin is denied.
- The discovery proxy apex is denied.
- An unrelated hostname is denied.
- The same hostname on the wrong effective port is denied.
- The exact origin on a non-`lib-*` label is denied.
- `read_dropped_paths` remains denied to every `lib-*` origin.
- Re-minting the same exact origin is suppressed.
- No tracked capability contains `https://*.devserver.chan.app`, another `*.chan.app` remote URL, or a discovery-apex wildcard.
- Production generated JSON parses and every permission resolves. The tests that demonstrate malformed and unresolved inputs panic remain quarantined to `catch_unwind`; no production path can reach them.

Keep the SPA invoke-vocabulary parity walk. Replace its generic `gateway lib window` wildcard fixture with at least two exact origins so the expected origin passes and its sibling fails.

### Entry validation tests

Cover the validator as a pure function where possible:

- Requested owner mismatch.
- Requested full devserver-id mismatch, including a matching 12-character prefix with a different remainder.
- Empty or malformed proxy origin.
- Credentials, query, fragment, or non-root path in proxy origin.
- HTTP outside loopback.
- Unsupported scheme.
- Discovery scheme mismatch.
- Discovery effective-port mismatch.
- Proxy apex instead of a child.
- Nested child instead of a one-label child.
- Unrelated host and suffix-lookalike namespace escape.
- Entry URL origin differs from proxy origin by scheme, host, or effective port.
- Entry URL contains credentials.
- A later session refresh or window entry returns a different origin from the pinned connection origin.
- Current `{owner}--{disc}` and bare-user one-label hosts both validate when returned by the gateway.

At the HTTP boundary, verify validation happens before the desktop requests `entry_url`, so a malicious cross-origin entry URL receives no entry token exchange.

### Trust and lifecycle tests

- Owned row connects without a trust record and receives exact-origin IPC.
- Shared untrusted row is listed with `native_trust_required: true` and backend connect refuses it.
- PUT for a current shared row persists `(owner, full devserver_id)` under the correct gateway and flips the projected field.
- Trust survives launcher reload, gateway disconnect/reconnect, and desktop restart.
- Cancel persists nothing and starts no connect.
- DELETE disconnects, closes every eligible window, removes trust, and restores `native_trust_required: true`.
- A devserver-id rotation does not inherit trust from the old id.
- Fresh-roster removal disconnects and prunes immediately.
- Gateway disconnect, removal, and PAT rejection close all contributed windows.
- Connect-versus-DELETE and connect-versus-roster-removal races cannot register a connection or start a watcher after the revocation generation changes.
- A grant minted before a losing race starts no target watcher or window and managed reconnect is blocked; the authority entry still requires application restart for a hard purge.

### Launcher tests

- The confirmation text names clipboard access, selected-file reads, downloads, window control, and system-browser opening.
- Confirm sends PUT and waits for success before POST connect.
- PUT failure reports the error and does not connect.
- Cancel sends neither request and creates no pending spinner.
- A trusted shared row skips the prompt on later connects.
- Revoke sends DELETE and the refreshed row returns to the trust-required state with no windows.
- Plain and owned rows retain their existing one-click connect behavior.

### Manual smoke

Smoke both the official gateway and a custom gateway such as `usr.devsrv.io`:

1. Connect the account gateway and confirm that doing so alone grants no devserver origin.
2. Connect an owned row and inspect that the capability origin equals the exact webview `location.origin`.
3. Exercise clipboard text/image/HTML behavior, native upload selection, save-to-downloads, zoom, fullscreen, and external-link opener behavior.
4. Connect a shared row, cancel once, confirm it remains disconnected, then grant consent and repeat the native vocabulary smoke.
5. Disconnect/reconnect and restart the desktop to confirm trust persistence.
6. Revoke trust and confirm all shared windows close and cannot reconnect.
7. Before restart, confirm the old exact grant is dormant under the managed window lifecycle. After restart, confirm it is absent from the new process authority.
8. Repeat against current `{owner}--{disc}` routing and, where available, a bare-user routing deployment.

Run the focused gates after implementation:

```sh
cargo test -p chan-library devserver_registry
cargo test -p chan-server library
cargo test -p chan-desktop runtime_capability
cargo test -p chan-desktop gateway
cd web && npm run test -w @chan/launcher
cd desktop && make build
```

Finish with the full project pre-push gate because the new `DevserverEntry` field crosses the root Rust workspace, desktop construction sites, and TypeScript fixtures. The gateway workspace needs only its focused existing entry tests unless its source or wire docs change.

## Acceptance Criteria

- No static official-domain exception remains.
- Account-level gateway connect mints no Tauri devserver capability.
- Every new remote devserver capability contains one canonical exact origin and targets only `lib-*`.
- Entry owner, full id, namespace, origin, and refresh stability are validated before any native window starts.
- Owned rows connect without prompts.
- Shared rows require exact persisted consent, and cancel is a no-op.
- Revocation and fresh-roster removal close windows and defeat in-flight connects.
- Capability mint failure fails the row connect without disconnecting the gateway account.
- The existing full native command vocabulary works on trusted official and self-hosted devservers.
- The UI and documentation state that a runtime grant can only be purged by quitting Chan Desktop.

## Assumptions

- v0.70.3 is the implementation baseline; the relevant runtime capability and gateway-connect behavior is unchanged from v0.70.1.
- The current gateway entry response remains the contract: owner username, full devserver id, exact proxy origin, entry URL, and expiry. No gateway API-version bump is required.
- Existing shared devservers default to untrusted after upgrade. Owned devservers require no migration.
- Shared trust is local desktop configuration, not gateway share state and not synchronized across desktops.
- No native bridge wrappers, loopback content proxy, or per-command consent model is included.
