//! Per-tunnel driver task.
//!
//! Owns a `yamux::Connection` for the lifetime of one registered
//! tunnel. Three things share its attention:
//!
//! 1. `OpenRequest` messages from the fronting proxy (via
//!    `TunnelHandle::open`) asking for a new outbound substream. Each
//!    message carries a oneshot reply channel.
//! 2. Inbound substreams from the peer. The protocol does not use
//!    these in v0; we drop them, which yamux turns into a RST on
//!    the next poll.
//! 3. The shutdown signal: the registry's `oneshot::Sender<()>`
//!    being dropped (either because the entry was evicted, or
//!    because deregistration ran). Receiver wakes with an error.
//!
//! All three are merged into one `poll_fn` body so the two
//! reborrows of `&mut conn` happen sequentially within a single
//! poll invocation; `select!` over multiple `poll_fn`s holding
//! `&mut conn` would conflict on the borrow.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use futures::AsyncRead as FutAsyncRead;
use futures::AsyncWrite as FutAsyncWrite;
use tokio::sync::{mpsc, oneshot};
use yamux::Connection as YamuxConnection;

use crate::registry::{OpenRequest, Registry, TunnelHandle};

/// Run the driver to completion. Returns when the yamux connection
/// closes, errors, or the shutdown signal fires.
///
/// `handle` is kept alive only so the driver can call
/// `Registry::deregister_if_owner` once it exits. Cloning the
/// handle into the registry beforehand and dropping it here would
/// also remove the entry, but only after the request channel
/// receiver this task owns goes away, which already implies
/// "tunnel is gone" to the public side.
pub(crate) async fn workspace_tunnel<S>(
    mut conn: YamuxConnection<S>,
    mut open_rx: mpsc::Receiver<OpenRequest>,
    mut shutdown_rx: oneshot::Receiver<()>,
    registry: Arc<Registry>,
    handle: TunnelHandle,
) where
    S: FutAsyncRead + FutAsyncWrite + Unpin + Send + 'static,
{
    let mut pending: VecDeque<OpenRequest> = VecDeque::new();
    let mut request_channel_open = true;

    enum Step {
        Inbound,  // peer opened a substream we don't expect
        Shutdown, // shutdown signal or yamux error
    }

    loop {
        let step = futures::future::poll_fn(|cx| {
            // Shutdown takes priority over anything else. Either an
            // explicit `()` send, or the sender being dropped (which
            // is how `Registry::register` evicts the previous tunnel),
            // resolves the receiver.
            if Pin::new(&mut shutdown_rx).poll(cx).is_ready() {
                return Poll::Ready(Step::Shutdown);
            }

            if request_channel_open {
                loop {
                    match open_rx.poll_recv(cx) {
                        Poll::Ready(Some(reply)) => pending.push_back(reply),
                        Poll::Ready(None) => {
                            request_channel_open = false;
                            break;
                        }
                        Poll::Pending => break,
                    }
                }
            }

            while pending.front().is_some() {
                match Pin::new(&mut conn).poll_new_outbound(cx) {
                    Poll::Ready(Ok(stream)) => {
                        let reply = pending.pop_front().expect("front() said Some");
                        let _ = reply.send(Ok(stream));
                    }
                    Poll::Ready(Err(_)) => return Poll::Ready(Step::Shutdown),
                    Poll::Pending => break,
                }
            }

            match Pin::new(&mut conn).poll_next_inbound(cx) {
                Poll::Ready(Some(Ok(_unexpected))) => Poll::Ready(Step::Inbound),
                Poll::Ready(Some(Err(_))) | Poll::Ready(None) => Poll::Ready(Step::Shutdown),
                Poll::Pending => Poll::Pending,
            }
        })
        .await;

        match step {
            Step::Inbound => {
                // Drop the unexpected stream. yamux will RST it on the
                // next poll cycle. We continue serving outbound opens.
                tracing::warn!("client opened unexpected substream; dropping");
                continue;
            }
            Step::Shutdown => break,
        }
    }

    // Best-effort yamux close. Errors here only affect the peer's
    // log; we're done either way.
    let _ = futures::future::poll_fn(|cx| Pin::new(&mut conn).poll_close(cx)).await;

    // Tell any open() callers still waiting that we're gone.
    while let Some(reply) = pending.pop_front() {
        let _ = reply.send(Err(crate::registry::OpenError::Disconnected));
    }

    registry.deregister_if_owner(&handle);
}
