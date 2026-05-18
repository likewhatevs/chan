//! Embedded SPA bundle and the fallback handler that serves it.
//!
//! `WebAssets` bakes `web/dist/` at compile time (release) or reads
//! from disk on each request (debug). The fallback handler returns
//! `index.html` for any path that isn't a baked asset and isn't an
//! `/api`/`/ws` route, so client-side routes work without server-side
//! awareness of them. The SPA shell gets `<meta name="chan-prefix">`
//! and (when set) `<meta name="chan-settings-disabled">` tags
//! injected so the frontend transport layer prepends the prefix to
//! fetch and WebSocket URLs and the Settings entry point can grey
//! itself out. The frontend also folds `window.location.origin` and
//! this injected prefix into its browser-storage keys for bearer
//! tokens and per-tab window ids, so independent `chan serve`
//! instances on the same host cannot reuse each other's session
//! routing state.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

use crate::state::AppState;

/// Frontend bundle baked at compile time. The path is relative to
/// this crate's manifest. In debug builds rust-embed reads files
/// from disk on each request (so `npm run build` updates take
/// effect without a cargo rebuild). In release builds the bundle
/// is embedded; build.rs emits cargo:rerun-if-changed for every
/// file under web/dist so a re-bundled frontend triggers a relink.
#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct WebAssets;

const SPA_CACHE_CONTROL: HeaderValue = HeaderValue::from_static("no-store");
const ASSET_CACHE_CONTROL: HeaderValue =
    HeaderValue::from_static("public, max-age=31536000, immutable");
const HOST_VARY: HeaderValue = HeaderValue::from_static("Host");

/// Single-page-app fallback: any path that doesn't match an /api or
/// /ws route, and doesn't correspond to a baked asset, returns
/// index.html so client-side routes work. For unknown /api paths
/// we return a real 404 instead of the SPA shell so callers don't
/// silently get HTML when they expected JSON.
pub async fn serve_static(State(state): State<Arc<AppState>>, uri: axum::http::Uri) -> Response {
    let path = uri.path();
    // Refuse to serve the SPA shell for /api or /ws misses; those
    // are programmatic surfaces, not browser navigation.
    if path.starts_with("/api") || path == "/ws" {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let candidate = path.trim_start_matches('/');
    let is_index = candidate.is_empty() || candidate == "index.html";
    let candidate = if candidate.is_empty() {
        "index.html"
    } else {
        candidate
    };
    let prefix = state.prefix.read().unwrap().clone();
    let settings_disabled = state.settings_disabled;
    if let Some(file) = WebAssets::get(candidate) {
        let body = if is_index {
            inject_chan_meta(&file.data, &prefix, settings_disabled)
        } else {
            file.data.into_owned()
        };
        return with_static_cache_headers(
            ([(header::CONTENT_TYPE, content_type_for(candidate))], body).into_response(),
            is_index,
        );
    }
    // SPA fallback: route paths the frontend handles client-side.
    if let Some(file) = WebAssets::get("index.html") {
        let body = inject_chan_meta(&file.data, &prefix, settings_disabled);
        return with_static_cache_headers(
            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body).into_response(),
            true,
        );
    }
    // No bundle baked / on disk yet (fresh clone, npm not run).
    (
        StatusCode::NOT_FOUND,
        "frontend bundle not built; run `cd web && npm install && npm run build`",
    )
        .into_response()
}

fn with_static_cache_headers(mut response: Response, spa_shell: bool) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        if spa_shell {
            SPA_CACHE_CONTROL
        } else {
            ASSET_CACHE_CONTROL
        },
    );
    headers.insert(header::VARY, HOST_VARY);
    response
}

/// Inject the SPA's runtime hints as `<meta>` tags right after the
/// opening `<head>` so the frontend can read them synchronously at
/// boot:
///
///   - `<meta name="chan-prefix" content="<prefix>">` when `prefix`
///     is non-empty. The transport layer prepends it to fetch and
///     WebSocket URLs.
///   - `<meta name="chan-settings-disabled" content="1">` when
///     `settings_disabled` is true. Greys out the Settings entry
///     point in the SPA.
///
/// No-op when neither hint applies, or when `<head>` isn't found in
/// the document (returns the original bytes unchanged).
pub fn inject_chan_meta(html: &[u8], prefix: &str, settings_disabled: bool) -> Vec<u8> {
    if prefix.is_empty() && !settings_disabled {
        return html.to_vec();
    }
    let needle = b"<head>";
    let Some(pos) = html.windows(needle.len()).position(|w| w == needle) else {
        return html.to_vec();
    };
    let mut insert = String::new();
    if !prefix.is_empty() {
        // Prefix is canonical (`/seg[/seg...]` with `[A-Za-z0-9-]+`
        // segments) so it cannot contain HTML-attribute-special bytes.
        insert.push_str(&format!("<meta name=\"chan-prefix\" content=\"{prefix}\">"));
    }
    if settings_disabled {
        insert.push_str("<meta name=\"chan-settings-disabled\" content=\"1\">");
    }
    let mut out = Vec::with_capacity(html.len() + insert.len());
    let after_head = pos + needle.len();
    out.extend_from_slice(&html[..after_head]);
    out.extend_from_slice(insert.as_bytes());
    out.extend_from_slice(&html[after_head..]);
    out
}

/// Conservative MIME map for the file types the SPA bundle ships:
/// hashed JS / CSS, source maps, fonts, images, and a couple of
/// well-known toplevel files. Falls back to
/// `application/octet-stream` so unknown extensions never get the
/// wrong type assigned.
pub fn content_type_for(path: &str) -> &'static str {
    let ext = match path.rsplit_once('.') {
        Some((_, e)) => e.to_ascii_lowercase(),
        None => return "application/octet-stream",
    };
    match ext.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_chan_meta_inserts_prefix_after_head() {
        let html = b"<!doctype html><html><head><title>x</title></head></html>";
        let out = inject_chan_meta(html, "/foo", false);
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("<head><meta name=\"chan-prefix\" content=\"/foo\"><title>"));
        assert!(!s.contains("chan-settings-disabled"));
    }

    #[test]
    fn inject_chan_meta_inserts_settings_disabled_after_head() {
        let html = b"<head><title>x</title></head>";
        let out = inject_chan_meta(html, "", true);
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("<head><meta name=\"chan-settings-disabled\" content=\"1\"><title>"));
        assert!(!s.contains("chan-prefix"));
    }

    #[test]
    fn inject_chan_meta_combines_both_tags() {
        let html = b"<head><title>x</title></head>";
        let out = inject_chan_meta(html, "/foo", true);
        let s = std::str::from_utf8(&out).unwrap();
        // Prefix is injected first, settings-disabled second; both
        // sit immediately after the opening <head>.
        assert!(s.contains(
            "<head><meta name=\"chan-prefix\" content=\"/foo\">\
             <meta name=\"chan-settings-disabled\" content=\"1\"><title>"
        ));
    }

    #[test]
    fn inject_chan_meta_noop_when_nothing_set() {
        let html = b"<head></head>";
        let out = inject_chan_meta(html, "", false);
        assert_eq!(out, html);
    }

    #[test]
    fn inject_chan_meta_noop_when_head_missing() {
        let html = b"<html></html>";
        let out = inject_chan_meta(html, "/foo", true);
        assert_eq!(out, html);
    }

    #[test]
    fn static_cache_headers_do_not_store_spa_shell() {
        let response = with_static_cache_headers("ok".into_response(), true);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&SPA_CACHE_CONTROL)
        );
        assert_eq!(response.headers().get(header::VARY), Some(&HOST_VARY));
    }

    #[test]
    fn static_cache_headers_allow_immutable_assets() {
        let response = with_static_cache_headers("ok".into_response(), false);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&ASSET_CACHE_CONTROL)
        );
        assert_eq!(response.headers().get(header::VARY), Some(&HOST_VARY));
    }
}
