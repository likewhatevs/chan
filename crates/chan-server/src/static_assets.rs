//! Embedded SPA bundle and the fallback handler that serves it.
//!
//! `WebAssets` bakes `web/dist/` at compile time (release) or reads
//! from disk on each request (debug). The fallback handler returns
//! `index.html` for any path that isn't a baked asset and isn't an
//! `/api`/`/ws` route, so client-side routes work without server-side
//! awareness of them. The SPA shell gets a `<meta name="chan-prefix">`
//! tag injected so the frontend transport layer prepends the prefix
//! to fetch and WebSocket URLs.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, StatusCode};
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
    if let Some(file) = WebAssets::get(candidate) {
        let body = if is_index {
            inject_chan_prefix(&file.data, &prefix)
        } else {
            file.data.into_owned()
        };
        return ([(header::CONTENT_TYPE, content_type_for(candidate))], body).into_response();
    }
    // SPA fallback: route paths the frontend handles client-side.
    if let Some(file) = WebAssets::get("index.html") {
        let body = inject_chan_prefix(&file.data, &prefix);
        return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body).into_response();
    }
    // No bundle baked / on disk yet (fresh clone, npm not run).
    (
        StatusCode::NOT_FOUND,
        "frontend bundle not built; run `cd web && npm install && npm run build`",
    )
        .into_response()
}

/// Inject `<meta name="chan-prefix" content="<prefix>">` after the
/// opening `<head>` tag of the SPA shell so the frontend transport
/// layer can read it at boot and prepend the prefix to fetch and
/// WebSocket URLs.
///
/// No-op when `prefix` is empty (the meta tag isn't needed; the
/// frontend defaults to "" when absent). When `<head>` isn't found,
/// returns the original bytes unchanged.
pub fn inject_chan_prefix(html: &[u8], prefix: &str) -> Vec<u8> {
    if prefix.is_empty() {
        return html.to_vec();
    }
    let needle = b"<head>";
    let Some(pos) = html.windows(needle.len()).position(|w| w == needle) else {
        return html.to_vec();
    };
    // Prefix is canonical (`/seg[/seg...]` with `[A-Za-z0-9-]+`
    // segments) so it cannot contain HTML-attribute-special bytes.
    let insert = format!("<meta name=\"chan-prefix\" content=\"{prefix}\">");
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
    fn inject_chan_prefix_inserts_meta_after_head() {
        let html = b"<!doctype html><html><head><title>x</title></head></html>";
        let out = inject_chan_prefix(html, "/foo");
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("<head><meta name=\"chan-prefix\" content=\"/foo\"><title>"));
    }

    #[test]
    fn inject_chan_prefix_noop_on_empty_prefix() {
        let html = b"<head></head>";
        let out = inject_chan_prefix(html, "");
        assert_eq!(out, html);
    }

    #[test]
    fn inject_chan_prefix_noop_when_head_missing() {
        let html = b"<html></html>";
        let out = inject_chan_prefix(html, "/foo");
        assert_eq!(out, html);
    }
}
