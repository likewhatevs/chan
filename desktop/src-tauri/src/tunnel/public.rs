//! Per-tenant local public listener.
//!
//! Each unique tenant label (= `Validated.username` returned by
//! `LocalValidator`, which equals the bearer token verbatim) gets
//! its own axum listener bound on `127.0.0.1:0`. Visitor URL is
//! `http://127.0.0.1:<port>/<workspace>/...`, matching chan-server's
//! `chan-prefix=/<workspace>` so the SPA's relative fetches resolve.
//!
//! The audited `chan_tunnel_server::public_router` only registers
//! two-segment routes (`/:user/:workspace/...`). To present a
//! one-segment URL while reusing that router unchanged, we wrap it
//! in a tower `Layer` that prepends `/<label>` to every incoming
//! request URI before routing. The prepended segment is captured at
//! listener-construction time from the desktop-side tenant string —
//! it is NEVER derived from the request — so a malicious client
//! cannot influence which tenant the request routes under.
//!
//! Subtlety: `axum::Router::layer` applies a layer per matched
//! route (after routing), so a layer attached via `.layer(...)`
//! would never affect which route was picked. To run BEFORE
//! routing, the wrapped service is mounted as the fallback of an
//! empty outer router. With no other routes, every request flows
//! through the layer and into the inner router, whose `/:user/:workspace`
//! match then sees the rewritten path.
//!
//! Per-tenant origins are also a security feature: the browser's
//! same-origin policy treats `http://127.0.0.1:A/` and
//! `http://127.0.0.1:B/` as different origins, so JS served by
//! tenant A cannot fetch from tenant B. This is the localhost
//! analogue of `*.workspace.chan.app` subdomain isolation in prod.

use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::Router;
use chan_tunnel_server::{public_router, Registry};
use http::uri::PathAndQuery;
use http::{Request, Uri};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower::{Layer, Service};

/// Tower `Layer` that prepends a fixed path segment to every
/// incoming request's URI path-and-query.
///
/// Invariants:
///
/// * `prefix` is constructed once at listener bring-up from a
///   server-controlled tenant label. The layer never reads any
///   byte of the inbound request to compute it.
/// * `prefix` always starts with `/` and never ends with `/`. The
///   incoming path-and-query always starts with `/`, so the
///   concatenation `{prefix}{old}` is a well-formed path; no extra
///   slash logic needed.
/// * URI reconstruction goes through `http::uri::Parts` +
///   `PathAndQuery::try_from`, never through `uri.to_string()`,
///   so scheme/authority survive intact for any proxy-shaped
///   request that may arrive (axum normalizes most of these away
///   on the server side, but defending here costs nothing).
#[derive(Clone)]
pub struct PrependPathLayer {
    prefix: Arc<str>,
}

impl PrependPathLayer {
    /// `label` must be a syntactically valid username
    /// (`chan_tunnel_proto::is_valid_username`); the caller has
    /// already enforced that for any label that exists in the
    /// registry (the handshake refuses unsafe ones before
    /// registration). Re-checking here would be belt-and-braces
    /// but matches the existing defense-in-depth pattern in
    /// `handshake_validated`.
    pub fn new(label: &str) -> Self {
        debug_assert!(
            chan_tunnel_proto::is_valid_username(label),
            "PrependPathLayer label must be a valid username; got {label:?}",
        );
        Self {
            prefix: Arc::from(format!("/{label}")),
        }
    }
}

impl<S> Layer<S> for PrependPathLayer {
    type Service = PrependPath<S>;
    fn layer(&self, inner: S) -> Self::Service {
        PrependPath {
            inner,
            prefix: self.prefix.clone(),
        }
    }
}

#[derive(Clone)]
pub struct PrependPath<S> {
    inner: S,
    prefix: Arc<str>,
}

impl<S, B> Service<Request<B>> for PrependPath<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        if let Some(new_uri) = prepend_path(req.uri(), &self.prefix) {
            *req.uri_mut() = new_uri;
        }
        // If reconstruction fails (interior NULs etc., impossible
        // for well-formed HTTP requests but defended against), the
        // request passes through unmodified. The inner router will
        // not match it and will return 404 — the correct failure
        // mode: never silently route to a different tenant.
        self.inner.call(req)
    }
}

fn prepend_path(uri: &Uri, prefix: &str) -> Option<Uri> {
    let mut parts = uri.clone().into_parts();
    let old = parts
        .path_and_query
        .as_ref()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let combined = format!("{prefix}{old}");
    parts.path_and_query = Some(PathAndQuery::try_from(combined).ok()?);
    Uri::from_parts(parts).ok()
}

/// Bind a loopback listener for `label` and spawn a graceful axum
/// server that proxies into the shared `registry` via
/// `chan_tunnel_server::public_router`, wrapped by the path-prepend
/// layer. Returns the bound port and the cancellation token that
/// shuts the server down.
///
/// 127.0.0.1 is hard-coded. There is no config knob to change the
/// bind host. Any external sharing of a tunneled workspace goes through
/// the SSH tunnel or a future gateway integration, never through
/// rebinding this listener.
pub async fn spawn_tenant_listener(
    label: String,
    registry: Arc<Registry>,
) -> std::io::Result<(u16, CancellationToken)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let cancel = CancellationToken::new();
    let cancel_child = cancel.clone();

    // We must wrap the audited router from the OUTSIDE so the
    // path rewrite happens before routing. `Router::layer` would
    // attach the middleware to every Route post-match, which is too
    // late — we want the inner router to see the rewritten path
    // when picking `/:user/:workspace/*rest`. Hence: mount the inner
    // router as the fallback service of an empty outer router, with
    // the PrependPath layer wrapped around it. The outer router has
    // no routes of its own, so every request flows through the
    // layer + into the inner router.
    let inner: Router = public_router(registry);
    let prepended: PrependPath<Router> = PrependPathLayer::new(&label).layer(inner);
    let app: Router = Router::new().fallback_service(prepended);

    // ConnectInfo<SocketAddr> is required so the inner handlers'
    // `Option<ConnectInfo<SocketAddr>>` extractor populates. On
    // loopback every peer is 127.0.0.1; the value is still used by
    // the upstream's X-Forwarded-For rewrite and is harmless here.
    let make_svc = app.into_make_service_with_connect_info::<SocketAddr>();

    tokio::spawn(async move {
        let server = axum::serve(listener, make_svc)
            .with_graceful_shutdown(async move { cancel_child.cancelled().await });
        if let Err(e) = server.await {
            tracing::warn!(error = %e, "per-tenant listener exited with error");
        }
    });

    Ok((port, cancel))
}

#[cfg(test)]
mod tests {
    use super::{prepend_path, PrependPath, PrependPathLayer};
    use http::{Request, Uri};
    use std::convert::Infallible;
    use std::task::{Context, Poll};
    use tower::{Layer, Service};

    /// Stub inner service that records the URI it sees on each
    /// `call` so tests can assert the rewrite landed correctly.
    #[derive(Clone, Default)]
    struct UriCapture {
        last: std::sync::Arc<std::sync::Mutex<Option<Uri>>>,
    }

    impl Service<Request<()>> for UriCapture {
        type Response = ();
        type Error = Infallible;
        type Future = std::future::Ready<Result<(), Infallible>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: Request<()>) -> Self::Future {
            *self.last.lock().unwrap() = Some(req.uri().clone());
            std::future::ready(Ok(()))
        }
    }

    fn run(layer: &PrependPathLayer, path: &str) -> Uri {
        let capture = UriCapture::default();
        let captured = capture.last.clone();
        let mut svc: PrependPath<UriCapture> = layer.layer(capture);
        let req = Request::builder().uri(path).body(()).unwrap();
        let _ = futures_block_on(<PrependPath<UriCapture> as Service<Request<()>>>::call(
            &mut svc, req,
        ));
        let uri = captured.lock().unwrap().clone();
        uri.expect("inner called")
    }

    /// Minimal block-on for the test's `Ready` future without
    /// pulling tokio in as a dev-dep for this file.
    fn futures_block_on<F: std::future::Future>(mut f: F) -> F::Output {
        use std::pin::Pin;
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        const NOOP: RawWakerVTable = RawWakerVTable::new(
            |_| RawWaker::new(std::ptr::null(), &NOOP),
            |_| {},
            |_| {},
            |_| {},
        );
        let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &NOOP)) };
        let mut cx = Context::from_waker(&waker);
        // SAFETY: pinned to the stack, never moved.
        let mut pinned = unsafe { Pin::new_unchecked(&mut f) };
        loop {
            if let Poll::Ready(v) = pinned.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    #[test]
    fn root_path_gets_label_prepended() {
        let layer = PrependPathLayer::new("alex-laptop");
        let uri = run(&layer, "/notes/");
        assert_eq!(uri.path(), "/alex-laptop/notes/");
        assert!(uri.query().is_none());
    }

    #[test]
    fn deep_path_preserved() {
        let layer = PrependPathLayer::new("alex-laptop");
        let uri = run(&layer, "/notes/api/files/read");
        assert_eq!(uri.path(), "/alex-laptop/notes/api/files/read");
    }

    #[test]
    fn query_string_preserved() {
        let layer = PrependPathLayer::new("alex-laptop");
        let uri = run(&layer, "/notes/api/search?q=hello&k=2");
        assert_eq!(uri.path(), "/alex-laptop/notes/api/search");
        assert_eq!(uri.query(), Some("q=hello&k=2"));
    }

    #[test]
    fn bare_slash_yields_label_slash() {
        let layer = PrependPathLayer::new("alex-laptop");
        let uri = run(&layer, "/");
        assert_eq!(uri.path(), "/alex-laptop/");
    }

    #[test]
    fn label_pinned_at_construction() {
        // Two layer instances must produce distinct prefixes even
        // when serving identical incoming paths; the prefix is
        // server-controlled per listener.
        let alex = PrependPathLayer::new("alex-laptop");
        let bob = PrependPathLayer::new("bob-workmac");
        assert_eq!(run(&alex, "/notes/").path(), "/alex-laptop/notes/");
        assert_eq!(run(&bob, "/notes/").path(), "/bob-workmac/notes/");
    }

    #[test]
    fn prepend_path_helper_handles_missing_path_and_query() {
        // `Uri::from_static("*")` has no path_and_query. We default
        // to "/" so the result is still routable.
        let uri = Uri::from_static("*");
        // Authority-form / asterisk-form URIs aren't supposed to
        // reach the router in practice, but the helper should not
        // panic on them. The result either succeeds with a sensible
        // path or returns None; both are acceptable. We only assert
        // no panic.
        let _ = prepend_path(&uri, "/alex");
    }
}
