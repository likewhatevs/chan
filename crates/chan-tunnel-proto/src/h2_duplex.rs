//! Wrap an h2 `(SendStream<Bytes>, RecvStream)` pair into a single
//! `tokio::io::AsyncRead + AsyncWrite + Unpin` so the rest of the
//! tunnel code (handshake, yamux) can stay generic over an opaque
//! duplex.
//!
//! The pairing is symmetric: on the server side, `RecvStream` is
//! the request body and `SendStream` is the response body; on the
//! client side, `SendStream` is the request body and `RecvStream`
//! is the response body. Either way the wrapper is the same.
//!
//! Flow-control windows are released eagerly: every chunk the
//! reader pulls from `RecvStream` is followed by an immediate
//! `release_capacity` call so the peer can keep sending. The
//! writer requests capacity per `poll_write` and only re-requests
//! when the granted capacity has been fully consumed.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use h2::{RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct H2Duplex {
    send: SendStream<Bytes>,
    recv: RecvStream,
    /// Bytes pulled from `recv` but not yet handed to the
    /// `AsyncRead` caller. h2 delivers full DATA frames; the read
    /// side consumes them piecemeal into the caller's buffer.
    pending: Bytes,
    /// True once `recv` has returned `None` (clean EOF).
    eof: bool,
    /// True once we've issued the half-close DATA frame on
    /// `shutdown`. Subsequent shutdown calls are no-ops.
    write_closed: bool,
}

impl H2Duplex {
    pub fn new(send: SendStream<Bytes>, recv: RecvStream) -> Self {
        Self {
            send,
            recv,
            pending: Bytes::new(),
            eof: false,
            write_closed: false,
        }
    }
}

fn h2_to_io(e: h2::Error) -> io::Error {
    io::Error::other(e)
}

impl AsyncRead for H2Duplex {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.pending.is_empty() {
            if self.eof {
                return Poll::Ready(Ok(()));
            }
            match self.recv.poll_data(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    let len = chunk.len();
                    self.pending = chunk;
                    // Best-effort flow-control release. If the peer
                    // already reset the stream this errors and we
                    // ignore it; the next read will surface the
                    // failure.
                    let _ = self.recv.flow_control().release_capacity(len);
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(h2_to_io(e))),
                Poll::Ready(None) => {
                    self.eof = true;
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        let n = std::cmp::min(self.pending.len(), buf.remaining());
        let chunk = self.pending.split_to(n);
        buf.put_slice(&chunk);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for H2Duplex {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if self.write_closed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "h2 write half closed",
            )));
        }
        if self.send.capacity() == 0 {
            self.send.reserve_capacity(buf.len());
            match self.send.poll_capacity(cx) {
                Poll::Ready(Some(Ok(cap))) => {
                    if cap == 0 {
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(h2_to_io(e))),
                Poll::Ready(None) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "h2 stream closed",
                    )))
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        let n = std::cmp::min(self.send.capacity(), buf.len());
        let chunk = Bytes::copy_from_slice(&buf[..n]);
        match self.send.send_data(chunk, false) {
            Ok(()) => Poll::Ready(Ok(n)),
            Err(e) => Poll::Ready(Err(h2_to_io(e))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // h2 has no explicit flush; data is on-the-wire as soon as
        // send_data returns.
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.write_closed {
            return Poll::Ready(Ok(()));
        }
        self.write_closed = true;
        match self.send.send_data(Bytes::new(), true) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(h2_to_io(e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use http::{Method, Request, Response};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Drive an h2 connection to completion in a background task.
    /// h2 connections are passive: nothing happens until somebody
    /// awaits them. The test sides each spawn one of these so the
    /// frames actually move while the H2Duplex halves are exercised.
    async fn run_h2_pair() -> (H2Duplex, H2Duplex) {
        let (a, b) = tokio::io::duplex(64 * 1024);

        let server_handle = tokio::spawn(async move {
            let mut conn = h2::server::handshake(b).await.expect("server handshake");
            let (req, mut respond) = conn.accept().await.expect("first stream").expect("not eof");
            let (_parts, body) = req.into_parts();
            let response = Response::builder().status(200).body(()).unwrap();
            let send = respond
                .send_response(response, false)
                .expect("send response");
            // Keep accepting (always Pending here) to drive
            // connection-level I/O while the duplex is in use. The
            // task ends when the peer closes the connection.
            tokio::spawn(async move { while conn.accept().await.is_some() {} });
            H2Duplex::new(send, body)
        });

        let client_handle = tokio::spawn(async move {
            let (mut client, conn) = h2::client::handshake(a).await.expect("client handshake");
            tokio::spawn(async move {
                let _ = conn.await;
            });
            let req = Request::builder()
                .method(Method::POST)
                .uri("https://test.invalid/v1/tunnel")
                .body(())
                .unwrap();
            let (resp_fut, send) = client.send_request(req, false).expect("send_request");
            let resp = resp_fut.await.expect("response");
            let recv = resp.into_body();
            H2Duplex::new(send, recv)
        });

        let server_dup = server_handle.await.expect("server task");
        let client_dup = client_handle.await.expect("client task");
        (server_dup, client_dup)
    }

    #[tokio::test]
    async fn bidirectional_byte_flow() {
        let (mut server_dup, mut client_dup) = run_h2_pair().await;

        // Client -> server.
        client_dup.write_all(b"ping").await.expect("client write");
        client_dup.flush().await.expect("client flush");
        let mut buf = [0u8; 4];
        server_dup.read_exact(&mut buf).await.expect("server read");
        assert_eq!(&buf, b"ping");

        // Server -> client.
        server_dup.write_all(b"pong").await.expect("server write");
        server_dup.flush().await.expect("server flush");
        let mut buf = [0u8; 4];
        client_dup.read_exact(&mut buf).await.expect("client read");
        assert_eq!(&buf, b"pong");
    }

    #[tokio::test]
    async fn shutdown_signals_eof_to_peer() {
        let (mut server_dup, mut client_dup) = run_h2_pair().await;

        client_dup.write_all(b"hi").await.expect("write");
        client_dup.shutdown().await.expect("shutdown");

        let mut sink = Vec::new();
        server_dup
            .read_to_end(&mut sink)
            .await
            .expect("read to end");
        assert_eq!(sink, b"hi");
    }
}
