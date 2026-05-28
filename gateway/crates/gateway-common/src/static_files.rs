//! Generic SPA-fallback handler for the embedded frontends.
//!
//! Each consumer keeps its own `#[derive(rust_embed::Embed)]` struct
//! (rust_embed resolves `#[folder = "web/dist/"]` relative to the
//! deriving crate) and supplies a const "frontend not built" banner.
//! This module owns the path resolution, MIME guessing, SPA route
//! fallback, and 404 logic.

use axum::body::Body;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// Resolve `uri` against the embedded asset set `R`.
///
/// Order:
///   1. Direct hit on the requested path -> 200 + correct MIME
///   2. No extension on the path (likely an SPA client route) ->
///      serve `index.html` so the client router takes over
///   3. Bundle missing entirely -> serve `banner` so the developer
///      sees a clear "run npm run build" page instead of a 404
///   4. Anything else -> 404
pub async fn serve<R: RustEmbed>(uri: Uri, banner: &'static [u8]) -> Response {
    let raw = uri.path().trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };

    if let Some(res) = serve_embedded::<R>(path) {
        return res;
    }

    if std::path::Path::new(path)
        .extension()
        .is_none_or(|e| e.is_empty())
    {
        return serve_embedded::<R>("index.html").unwrap_or_else(|| not_built(banner));
    }

    StatusCode::NOT_FOUND.into_response()
}

fn serve_embedded<R: RustEmbed>(path: &str) -> Option<Response> {
    let file = R::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(file.data.into_owned()))
            .expect("static asset response is valid"),
    )
}

fn not_built(banner: &'static [u8]) -> Response {
    // 503 (Service Unavailable) so a missing bundle in prod surfaces
    // to monitoring instead of silently rendering as a healthy 200.
    // In dev the banner still renders normally; browsers display the
    // HTML body regardless of the 5xx status.
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(banner))
        .expect("not-built banner response is valid")
}
