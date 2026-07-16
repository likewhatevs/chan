//! Pins for granting gateway windows IPC through a runtime capability.
//!
//! Self-hosted gateway proxy origins are unknown at build time, so the
//! static `capabilities/devserver-window.json` grant (scoped to
//! `https://*.devserver.chan.app`) cannot cover them; covering a foreign
//! origin takes a capability added at runtime via
//! `Manager::add_capability`. These tests pin the tauri behavior such a
//! grant relies on:
//!
//! - IPC access is resolved against the shared `RuntimeAuthority` on every
//!   invoke (`Webview::on_message`), never snapshotted per window: a
//!   `lib-*` window that is ALREADY OPEN when the capability is added
//!   gains the grant on its next invoke, and windows created after the
//!   mint are covered equally.
//! - The grant stays scoped: origins outside the capability's
//!   `remote.urls`, window labels outside `lib-*`, and commands outside
//!   the granted permission set all remain denied.
//! - `add_capability`'s JSON-string form PANICS on malformed JSON
//!   (`RuntimeCapability::build` expect) and on permissions missing from
//!   the build-time ACL manifests (`Resolved::resolve` unwrap), aborting
//!   the app. The JSON shape minted for gateway windows is pinned here to
//!   parse and resolve cleanly so both panic paths stay unreachable.
//!
//! Two further tauri facts constrain what a minted capability may carry:
//!
//! - Deny entries are ORIGIN-BLIND in `resolve_access` (the origin-match
//!   result of a denied command is discarded): any deny entry would deny
//!   the command on EVERY origin, so a minted capability carries NO deny
//!   entries - the same absolute rule as no scoped permissions (runtime
//!   scope ids collide with build-time ids).
//! - Re-adding a capability ACCUMULATES duplicate resolved-command
//!   entries (no dedup on the identifier), so minting must stay
//!   once-per-(origin, app run); the duplicates are harmless to
//!   resolution but grow the authority without bound.
//!
//! What unit tests cannot prove on a headless host: a real OS webview
//! delivering an invoke from a remote https page. That end matters only
//! once per platform and is covered by the desktop hand-smoke; the static
//! devserver-window capability shipping in production pins the same
//! remote-IPC delivery path.

use std::panic::{catch_unwind, AssertUnwindSafe};

use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::utils::acl::capability::CapabilityFile;
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Stub for the `platform_os` app command (granted to `lib-*` windows via
/// the `workspace-window` permission set), so an allowed invoke has a
/// handler to reach and returns a recognizable body.
#[tauri::command]
fn platform_os() -> &'static str {
    "stub-os"
}

/// Stub for `read_dropped_paths` - the one command a loopback workspace
/// window holds that `lib-*` windows never get, on any origin. Registered
/// so the out-of-set denial pin cannot pass vacuously: if a capability
/// ever leaked this command to lib windows, the invoke would reach this
/// handler and return Ok, failing the pin (an unregistered command is
/// rejected with the same Err shape as an ACL denial).
#[tauri::command]
fn read_dropped_paths() -> &'static str {
    "leaked"
}

/// The capability JSON minted for a gateway proxy origin: the exact
/// permission list of `capabilities/devserver-window.json`, `lib-*`
/// windows only, `remote.urls` swapped to the gateway's proxy wildcard.
/// No scoped permissions: runtime scope ids collide with build-time ids.
fn gateway_capability_json(origin_pattern: &str) -> String {
    serde_json::json!({
        "identifier": "gateway-window-test",
        "description": "runtime grant for gateway-served lib windows",
        "remote": { "urls": [origin_pattern] },
        "windows": ["lib-*"],
        "permissions": [
            "workspace-window",
            "allow-pick-upload-files",
            "core:webview:allow-set-webview-zoom",
            "core:window:allow-set-fullscreen",
            "opener:default",
            "opener:allow-open-url",
        ],
    })
    .to_string()
}

/// A mock-runtime app built from the REAL generated context: the actual
/// ACL manifests and static capabilities of chan-desktop, no display
/// server required.
fn mock_desktop_app() -> tauri::App<tauri::test::MockRuntime> {
    mock_builder()
        .invoke_handler(tauri::generate_handler![platform_os, read_dropped_paths])
        .build(tauri::generate_context!())
        .expect("mock app builds from the real context")
}

fn lib_window(
    app: &tauri::App<tauri::test::MockRuntime>,
    label: &str,
    url: &str,
) -> tauri::WebviewWindow<tauri::test::MockRuntime> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::External(url.parse().unwrap()))
        .build()
        .expect("mock webview window")
}

/// Drives `Webview::on_message` with `cmd` as if a page at `url` sent it:
/// the same per-invoke path (origin derivation + live authority lookup)
/// production IPC takes.
fn invoke_from(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    url: &str,
    cmd: &str,
) -> Result<String, serde_json::Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::default(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<String>().expect("string response"))
}

const GATEWAY_ORIGIN_PATTERN: &str = "https://*.proxy.gw-test.example";
const GATEWAY_PAGE: &str = "https://ws1.proxy.gw-test.example/";
const OTHER_REMOTE_PAGE: &str = "https://ws1.unrelated.example/";
const STATIC_DEVSERVER_PAGE: &str = "https://0a1b2c3d4e5f.devserver.chan.app/";

/// Control: the shipped static capability resolves through this harness,
/// so a denial elsewhere means "no grant", not a broken rig.
#[test]
fn static_devserver_grant_resolves_through_invoke_path() {
    let app = mock_desktop_app();
    let webview = lib_window(&app, "lib-static", STATIC_DEVSERVER_PAGE);
    assert_eq!(
        invoke_from(&webview, STATIC_DEVSERVER_PAGE, "platform_os"),
        Ok("stub-os".into())
    );
}

/// The core pin: a foreign-origin invoke is denied before the mint, the
/// SAME already-open window is allowed right after it, and a window
/// created after the mint is covered too.
#[test]
fn runtime_grant_reaches_already_open_and_later_windows() {
    let app = mock_desktop_app();

    let open_before_mint = lib_window(&app, "lib-before", GATEWAY_PAGE);
    assert!(
        invoke_from(&open_before_mint, GATEWAY_PAGE, "platform_os").is_err(),
        "foreign origin must be denied before the mint"
    );

    app.add_capability(gateway_capability_json(GATEWAY_ORIGIN_PATTERN))
        .expect("add_capability returned Ok");

    assert_eq!(
        invoke_from(&open_before_mint, GATEWAY_PAGE, "platform_os"),
        Ok("stub-os".into()),
        "an already-open window gains the grant on its next invoke"
    );

    let opened_after_mint = lib_window(&app, "lib-after", GATEWAY_PAGE);
    assert_eq!(
        invoke_from(&opened_after_mint, GATEWAY_PAGE, "platform_os"),
        Ok("stub-os".into()),
        "windows created after the mint are covered"
    );
}

/// The grant must not leak: wrong origin, wrong window label, or a
/// command outside the granted set all stay denied after the mint.
#[test]
fn runtime_grant_stays_scoped() {
    let app = mock_desktop_app();
    app.add_capability(gateway_capability_json(GATEWAY_ORIGIN_PATTERN))
        .expect("add_capability returned Ok");

    let lib = lib_window(&app, "lib-scoped", GATEWAY_PAGE);
    assert!(
        invoke_from(&lib, OTHER_REMOTE_PAGE, "platform_os").is_err(),
        "origins outside remote.urls stay denied"
    );
    assert!(
        invoke_from(&lib, GATEWAY_PAGE, "read_dropped_paths").is_err(),
        "commands outside the granted set stay denied"
    );

    let non_lib = lib_window(&app, "settings-scoped", GATEWAY_PAGE);
    assert!(
        invoke_from(&non_lib, GATEWAY_PAGE, "platform_os").is_err(),
        "window labels outside lib-* stay denied"
    );
}

/// Both add_capability panic paths, pinned unreachable for the JSON we
/// mint: it parses as a CapabilityFile and every named permission
/// resolves against the app's build-time manifests.
#[test]
fn minted_capability_parses_and_resolves() {
    let json = gateway_capability_json(GATEWAY_ORIGIN_PATTERN);
    json.parse::<CapabilityFile>()
        .expect("minted JSON parses as a capability");

    // A clean return proves resolution: an unresolvable set panics inside
    // add_capability before an Err is ever reachable.
    let app = mock_desktop_app();
    app.add_capability(json)
        .expect("add_capability returned Ok");
}

/// Re-adding the same capability accumulates duplicate grants rather than
/// erroring or replacing: resolution still allows the command, which is
/// why the mint must stay once-per-(origin, app run) rather than
/// re-issuing on every connect.
#[test]
fn re_minting_accumulates_without_breaking_resolution() {
    let app = mock_desktop_app();
    let json = gateway_capability_json(GATEWAY_ORIGIN_PATTERN);
    app.add_capability(json.clone())
        .expect("add_capability returned Ok");
    app.add_capability(json).expect("re-add returned Ok");
    let webview = lib_window(&app, "lib-remint", GATEWAY_PAGE);
    assert_eq!(
        invoke_from(&webview, GATEWAY_PAGE, "platform_os"),
        Ok("stub-os".into())
    );
}

/// The hazard the pin above guards: malformed JSON and unknown
/// permissions don't error, they PANIC (and abort the app outside
/// catch_unwind). Documents why the minted shape must stay test-pinned.
#[test]
fn malformed_or_unresolvable_capability_panics() {
    let app = mock_desktop_app();
    assert!(
        catch_unwind(AssertUnwindSafe(|| app.add_capability("{not json"))).is_err(),
        "malformed JSON panics"
    );
    drop(app);

    let app = mock_desktop_app();
    let unresolvable = serde_json::json!({
        "identifier": "gateway-window-bad",
        "description": "",
        "remote": { "urls": [GATEWAY_ORIGIN_PATTERN] },
        "windows": ["lib-*"],
        "permissions": ["no-such-permission"],
    })
    .to_string();
    assert!(
        catch_unwind(AssertUnwindSafe(|| app.add_capability(unresolvable))).is_err(),
        "unknown permission panics"
    );
}
