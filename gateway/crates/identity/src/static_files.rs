//! Embed identity-service's Svelte SPA at compile time and serve it
//! via the shared SPA-fallback handler.
//!
//! `web/dist/` is the output of `npm run build` in `crates/identity/web/`.
//! On a fresh checkout the directory may not exist yet; the shared
//! handler returns the "frontend not built" banner so developers see
//! a clear next step instead of a blank 404.

use axum::http::Uri;
use axum::response::Response;

#[derive(rust_embed::Embed)]
#[folder = "web/dist/"]
struct Assets;

const NOT_BUILT_BANNER: &[u8] = b"<!doctype html><meta charset=utf-8><title>identity</title>\
<style>body{font:14px/1.4 -apple-system,BlinkMacSystemFont,sans-serif;\
background:#1c1c1e;color:#e8e8ea;padding:2rem;max-width:640px;margin:0 auto}\
code{background:#2a2a2c;padding:.1em .35em;border-radius:3px}</style>\
<h1>identity-service</h1><p>Frontend bundle is missing. Build it once:\
<pre><code>cd web &amp;&amp; npm install &amp;&amp; npm run build</code></pre>\
<p>Then re-run this binary; the SPA will be embedded.";

pub async fn handler(uri: Uri) -> Response {
    gateway_common::static_files::serve::<Assets>(uri, NOT_BUILT_BANNER).await
}
