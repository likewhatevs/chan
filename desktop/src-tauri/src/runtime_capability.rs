//! The runtime-minted IPC grant for gateway windows.
//!
//! Self-hosted gateway proxy origins are unknown at build time, so the
//! static `capabilities/devserver-window.json` grant (scoped to
//! `https://*.devserver.chan.app`) cannot cover them. At the first
//! successful connect of a gateway whose proxy origin falls outside that
//! static scope, the desktop mints the same grant for that origin via
//! `Manager::add_capability`: [`gateway_capability_json`] builds the
//! capability (the devserver-window permission list, `lib-*` windows,
//! `remote.urls` = the gateway's proxy wildcard, nothing wider), and
//! [`mint_gateway_grant`] installs it once per (origin, app run).
//!
//! Rules the mint must never break, each pinned by a test below:
//!
//! - NO scoped permissions: runtime scope ids collide with build-time ids.
//! - NO deny entries: deny entries are ORIGIN-BLIND in tauri's
//!   `resolve_access` (the origin-match result of a denied command is
//!   discarded), so any deny entry would kill the command on EVERY origin.
//! - Once per (origin, run): re-adding a capability ACCUMULATES duplicate
//!   resolved-command entries (no dedup on the identifier); duplicates are
//!   harmless to resolution but grow the authority without bound. There is
//!   no remove_capability: a removed gateway's grant persists until the
//!   app restarts, which is why the grant stays scoped to the exact proxy
//!   host.
//! - The minted JSON must parse and every permission must resolve:
//!   `add_capability`'s string form PANICS on malformed JSON
//!   (`RuntimeCapability::build` expect) and on permissions missing from
//!   the build-time ACL manifests (`Resolved::resolve` unwrap), aborting
//!   the app. [`mint_gateway_grant`] parses as a guard before handing the
//!   string over, and the pins keep the resolution path green.
//!
//! The tests drive the production `on_message` dispatch through the mock
//! runtime against the app's real generated ACL context: IPC access is
//! resolved against the shared `RuntimeAuthority` on every invoke, never
//! snapshotted per window, so a `lib-*` window that is ALREADY OPEN when
//! the capability is added gains the grant on its next invoke. What unit
//! tests cannot prove on a headless host - a real OS webview delivering an
//! invoke from a remote https page - is covered by the desktop hand-smoke;
//! the static devserver-window capability shipping in production pins the
//! same remote-IPC delivery path.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use tauri::utils::acl::capability::CapabilityFile;
use tauri::Manager;

/// The `remote.urls` scope of the static devserver-window grant. A gateway
/// whose proxy wildcard equals this is already covered at build time and
/// mints nothing; the pin test keeps this constant equal to the shipped
/// capability file's pattern.
const STATIC_GRANT_PATTERN: &str = "https://*.devserver.chan.app";

/// The `remote.urls` pattern for a gateway's proxy origin: the subdomain
/// wildcard (every devserver's window origin is a label under the proxy
/// host), scoped to the gateway's exact host and carrying its explicit
/// port when one is present. Deliberately NOTHING wider: no window class
/// serves from the apex origin itself, so the runtime grant surface stays
/// exactly the wildcard shape the static grant uses for chan.app.
pub fn gateway_proxy_remote_urls(proxy_origin: &str) -> Result<Vec<String>, String> {
    let parsed = url::Url::parse(proxy_origin).map_err(|e| format!("invalid proxy origin: {e}"))?;
    let scheme = parsed.scheme();
    if !matches!(scheme, "http" | "https") {
        return Err(format!("proxy origin {proxy_origin:?} must be http(s)"));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("proxy origin {proxy_origin:?} has no host"))?;
    let port = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
    Ok(vec![format!("{scheme}://*.{host}{port}")])
}

/// The capability JSON minted for a gateway's window origins: the exact
/// permission list of `capabilities/devserver-window.json`, `lib-*`
/// windows only, `remote.urls` swapped to the gateway's proxy wildcard.
/// No scoped permissions, no deny entries (see the module doc for why
/// both rules are absolute).
pub fn gateway_capability_json(remote_urls: &[String]) -> String {
    serde_json::json!({
        "identifier": "gateway-window",
        "description": "runtime grant for gateway-served lib windows",
        "remote": { "urls": remote_urls },
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

fn minted_origins() -> &'static Mutex<HashSet<String>> {
    static MINTED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    MINTED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Grant gateway windows on `proxy_origin` their IPC vocabulary, once per
/// (origin, app run). Returns `Ok(true)` when a capability was added,
/// `Ok(false)` when the origin is covered by the static grant or already
/// minted this run. Already-open windows on the origin gain the grant on
/// their next invoke; there is nothing to re-issue.
pub fn mint_gateway_grant<R: tauri::Runtime>(
    manager: &impl Manager<R>,
    proxy_origin: &str,
) -> Result<bool, String> {
    let urls = gateway_proxy_remote_urls(proxy_origin)?;
    if urls[0] == STATIC_GRANT_PATTERN {
        return Ok(false);
    }
    // Poison-tolerant: an unwind inside add_capability (its panic paths
    // are pinned unreachable for our JSON, but pins are not proofs) must
    // not wedge every later mint into a panic on this lock; recovering a
    // possibly-stale set risks at most one duplicate re-mint, which
    // resolution tolerates.
    let mut minted = minted_origins().lock().unwrap_or_else(|e| e.into_inner());
    if minted.contains(&urls[0]) {
        return Ok(false);
    }
    let json = gateway_capability_json(&urls);
    // Parse first: the string form of add_capability ABORTS on malformed
    // JSON, so guard with the fallible parse before handing the string
    // over (the unresolvable-permission abort stays covered by the pins).
    json.parse::<CapabilityFile>()
        .map_err(|e| format!("minted capability does not parse: {e}"))?;
    manager
        .add_capability(json)
        .map_err(|e| format!("adding gateway capability: {e}"))?;
    minted.insert(urls.into_iter().next().expect("urls is non-empty"));
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::panic::{catch_unwind, AssertUnwindSafe};

    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
    use tauri::webview::InvokeRequest;
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    /// Stub for the `platform_os` app command (granted to `lib-*` windows
    /// via the `workspace-window` permission set), so an allowed invoke
    /// has a handler to reach and returns a recognizable body.
    #[tauri::command]
    fn platform_os() -> &'static str {
        "stub-os"
    }

    /// Stub for `read_dropped_paths` - the one command a loopback
    /// workspace window holds that `lib-*` windows never get, on any
    /// origin. Registered so the out-of-set denial pin cannot pass
    /// vacuously: if a capability ever leaked this command to lib
    /// windows, the invoke would reach this handler and return Ok,
    /// failing the pin (an unregistered command is rejected with the same
    /// Err shape as an ACL denial).
    #[tauri::command]
    fn read_dropped_paths() -> &'static str {
        "leaked"
    }

    /// A mock-runtime app built from the REAL generated context: the
    /// actual ACL manifests and static capabilities of chan-desktop, no
    /// display server required.
    fn mock_desktop_app() -> tauri::App<tauri::test::MockRuntime> {
        mock_builder()
            .invoke_handler(tauri::generate_handler![platform_os, read_dropped_paths])
            .build(crate::app_context())
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

    /// Drives `Webview::on_message` with `cmd` as if a page at `url` sent
    /// it: the same per-invoke path (origin derivation + live authority
    /// lookup) production IPC takes.
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

    const GATEWAY_PROXY_ORIGIN: &str = "https://proxy.gw-test.example";
    const GATEWAY_PAGE: &str = "https://ws1.proxy.gw-test.example/";
    const OTHER_REMOTE_PAGE: &str = "https://ws1.unrelated.example/";
    const STATIC_DEVSERVER_PAGE: &str = "https://0a1b2c3d4e5f.devserver.chan.app/";

    fn production_json() -> String {
        gateway_capability_json(&gateway_proxy_remote_urls(GATEWAY_PROXY_ORIGIN).unwrap())
    }

    #[test]
    fn proxy_remote_urls_are_the_wildcard_alone() {
        assert_eq!(
            gateway_proxy_remote_urls("https://proxy.gw-test.example").unwrap(),
            vec!["https://*.proxy.gw-test.example".to_string()]
        );
        assert_eq!(
            gateway_proxy_remote_urls("http://127.0.0.1:7002").unwrap(),
            vec!["http://*.127.0.0.1:7002".to_string()]
        );
        assert!(gateway_proxy_remote_urls("ftp://x").is_err());
        assert!(gateway_proxy_remote_urls("not a url").is_err());
    }

    #[test]
    fn static_grant_pattern_matches_the_shipped_capability_file() {
        // The skip rule compares against this constant; it must stay equal
        // to what capabilities/devserver-window.json actually scopes, or
        // chan.app gateways would double-mint (or foreign ones skip).
        let file: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/devserver-window.json"))
                .expect("static capability parses");
        assert_eq!(
            file["remote"]["urls"][0].as_str(),
            Some(STATIC_GRANT_PATTERN)
        );
    }

    /// Control: the shipped static capability resolves through this
    /// harness, so a denial elsewhere means "no grant", not a broken rig.
    #[test]
    fn static_devserver_grant_resolves_through_invoke_path() {
        let app = mock_desktop_app();
        let webview = lib_window(&app, "lib-static", STATIC_DEVSERVER_PAGE);
        assert_eq!(
            invoke_from(&webview, STATIC_DEVSERVER_PAGE, "platform_os"),
            Ok("stub-os".into())
        );
    }

    /// The core pin: a foreign-origin invoke is denied before the mint,
    /// the SAME already-open window is allowed right after it, and a
    /// window created after the mint is covered too. Consumes the
    /// PRODUCTION mint path end to end. This test is the sole
    /// mint_gateway_grant caller for GATEWAY_PROXY_ORIGIN: the once-guard
    /// is process-global, so a second caller would read Ok(false)
    /// depending on test order.
    #[test]
    fn runtime_grant_reaches_already_open_and_later_windows() {
        let app = mock_desktop_app();

        let open_before_mint = lib_window(&app, "lib-before", GATEWAY_PAGE);
        assert!(
            invoke_from(&open_before_mint, GATEWAY_PAGE, "platform_os").is_err(),
            "foreign origin must be denied before the mint"
        );

        assert_eq!(
            mint_gateway_grant(&app, GATEWAY_PROXY_ORIGIN),
            Ok(true),
            "first mint for the origin installs the capability"
        );

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

        // The once-per-(origin, run) guard: a reconnect does not
        // accumulate duplicate grants.
        assert_eq!(mint_gateway_grant(&app, GATEWAY_PROXY_ORIGIN), Ok(false));
    }

    /// The grant must not leak: wrong origin, wrong window label, or a
    /// command outside the granted set all stay denied after the mint.
    #[test]
    fn runtime_grant_stays_scoped() {
        let app = mock_desktop_app();
        app.add_capability(production_json())
            .expect("add_capability returned Ok");

        let lib = lib_window(&app, "lib-scoped", GATEWAY_PAGE);
        assert!(
            invoke_from(&lib, OTHER_REMOTE_PAGE, "platform_os").is_err(),
            "origins outside remote.urls stay denied"
        );
        assert!(
            invoke_from(&lib, "https://proxy.gw-test.example/", "platform_os").is_err(),
            "the apex origin is outside the grant: the wildcard needs a label"
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

    /// Origins the static grant covers mint nothing: the wildcard equals
    /// the shipped pattern, so a chan.app gateway adds no capability.
    #[test]
    fn static_scope_origins_skip_the_mint() {
        let app = mock_desktop_app();
        assert_eq!(
            mint_gateway_grant(&app, "https://devserver.chan.app"),
            Ok(false)
        );
    }

    /// Both add_capability panic paths, pinned unreachable for the JSON
    /// the mint produces: it parses as a CapabilityFile and every named
    /// permission resolves against the app's build-time manifests.
    #[test]
    fn minted_capability_parses_and_resolves() {
        let json = production_json();
        json.parse::<CapabilityFile>()
            .expect("minted JSON parses as a capability");

        // A clean return proves resolution: an unresolvable set panics
        // inside add_capability before an Err is ever reachable.
        let app = mock_desktop_app();
        app.add_capability(json)
            .expect("add_capability returned Ok");
    }

    /// Re-adding the same capability accumulates duplicate grants rather
    /// than erroring or replacing: resolution still allows the command,
    /// which is why mint_gateway_grant keeps its once-per-(origin, run)
    /// guard rather than re-issuing on every connect.
    #[test]
    fn re_minting_accumulates_without_breaking_resolution() {
        let app = mock_desktop_app();
        let json = production_json();
        app.add_capability(json.clone())
            .expect("add_capability returned Ok");
        app.add_capability(json).expect("re-add returned Ok");
        let webview = lib_window(&app, "lib-remint", GATEWAY_PAGE);
        assert_eq!(
            invoke_from(&webview, GATEWAY_PAGE, "platform_os"),
            Ok("stub-os".into())
        );
    }

    /// The hazard the pins above guard: malformed JSON and unknown
    /// permissions don't error, they PANIC (and abort the app outside
    /// catch_unwind). Documents why the minted shape must stay pinned.
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
            "remote": { "urls": ["https://*.proxy.gw-test.example"] },
            "windows": ["lib-*"],
            "permissions": ["no-such-permission"],
        })
        .to_string();
        assert!(
            catch_unwind(AssertUnwindSafe(|| app.add_capability(unresolvable))).is_err(),
            "unknown permission panics"
        );
    }
}
