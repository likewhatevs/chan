//! Shared helpers for backends that parse NDJSON over a subprocess
//! stdout pipe (ClaudeCli, GeminiCli). The defaults here are the
//! resilience knobs that section 13.1 of `crates/chan-llm/design.md`
//! ("Bite A: correctness and resilience") refers to. Centralising
//! them means a future tweak (e.g. raising the line cap for a new
//! CLI that ships richer events) only has to touch one place.

use std::io;
use std::time::Duration;

use tokio::io::{AsyncBufRead, AsyncBufReadExt};

/// Hard cap on a single NDJSON line emitted by a backend subprocess.
/// A well-behaved CLI emits one JSON object per line, typically under
/// 64 KiB. The cap is generous (4 MiB), but bounded: a buggy or
/// malicious child cannot exhaust the host's memory by emitting one
/// multi-gigabyte line before the per-turn assistant text cap fires.
/// When the cap is hit we abort the stream and surface a structured
/// error to the listener; partial state already emitted via on_delta
/// stays.
pub(crate) const NDJSON_LINE_CAP_BYTES: usize = 4 * 1024 * 1024;

/// Maximum distinct parse-error emissions we forward to the listener
/// in a single turn. Past this threshold further parse failures are
/// counted silently, with the count surfaced once at the end of the
/// turn. This prevents a single misbehaving line shape from flooding
/// the WebSocket / native callback channel with thousands of error
/// frames.
pub(crate) const PARSE_ERROR_EMIT_LIMIT: usize = 5;

/// Default inactivity timeout between consecutive stdout lines from
/// a backend subprocess. 300 seconds covers slow first-token latency
/// on cold-start models and large prompts; longer real silences
/// nearly always mean the child is wedged. The session-level cancel
/// flag still works on top of this, so an impatient user can stop
/// sooner.
pub(crate) const DEFAULT_STREAM_INACTIVITY_SECS: u64 = 300;

/// Read one '\n'-terminated line into `buf`, returning `Ok(true)`
/// when a line was read, `Ok(false)` on clean EOF (buf empty), or
/// `Err` on I/O failure or when the accumulated line would exceed
/// `cap`. The trailing newline is included in `buf`; callers strip
/// it when they need a clean string.
///
/// Why not `BufReader::lines()` / `next_line()`: tokio's `read_line`
/// has no length cap on the internal accumulator. A 1 GiB line from
/// a wedged child would be buffered in full before our per-turn
/// text cap could fire. This helper enforces the cap during the
/// read so the worst-case memory we hold is `cap` bytes per stream.
pub(crate) async fn read_line_capped<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    cap: usize,
) -> io::Result<bool> {
    buf.clear();
    loop {
        let avail = reader.fill_buf().await?;
        if avail.is_empty() {
            // Clean EOF. If we'd already buffered a partial line
            // without a trailing newline, surface it as-if it had
            // one; the caller's parser will reject malformed JSON.
            // Returning Ok(false) only when buf is empty matches the
            // semantics of `next_line()`.
            return Ok(!buf.is_empty());
        }
        let consumed = match avail.iter().position(|&b| b == b'\n') {
            Some(i) => {
                let total = buf.len() + i + 1;
                if total > cap {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("line exceeds {cap}-byte cap"),
                    ));
                }
                buf.extend_from_slice(&avail[..=i]);
                i + 1
            }
            None => {
                if buf.len() + avail.len() > cap {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("line exceeds {cap}-byte cap"),
                    ));
                }
                buf.extend_from_slice(avail);
                avail.len()
            }
        };
        reader.consume(consumed);
        if buf.last() == Some(&b'\n') {
            return Ok(true);
        }
    }
}

/// Resolve the per-call inactivity timeout. `None` means "use the
/// chan-llm default" (`DEFAULT_STREAM_INACTIVITY_SECS`). Zero is
/// rejected by `LlmConfig` validation and never reaches here; a
/// silently-zero value would disable the timeout entirely, which is
/// not a knob we expose.
pub(crate) fn resolve_inactivity_timeout(secs: Option<u32>) -> Duration {
    Duration::from_secs(
        secs.map(u64::from)
            .unwrap_or(DEFAULT_STREAM_INACTIVITY_SECS),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn reads_one_line_and_returns_eof_after() {
        let data = b"hello\n";
        let mut reader = BufReader::new(&data[..]);
        let mut buf = Vec::new();
        assert!(read_line_capped(&mut reader, &mut buf, 1024).await.unwrap());
        assert_eq!(buf, b"hello\n");
        assert!(!read_line_capped(&mut reader, &mut buf, 1024).await.unwrap());
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn reads_unterminated_trailing_line() {
        // Some subprocesses flush a final chunk without a newline. We
        // surface it as one line so the parser gets a chance.
        let data = b"first\nsecond";
        let mut reader = BufReader::new(&data[..]);
        let mut buf = Vec::new();
        assert!(read_line_capped(&mut reader, &mut buf, 1024).await.unwrap());
        assert_eq!(buf, b"first\n");
        assert!(read_line_capped(&mut reader, &mut buf, 1024).await.unwrap());
        assert_eq!(buf, b"second");
        assert!(!read_line_capped(&mut reader, &mut buf, 1024).await.unwrap());
    }

    #[tokio::test]
    async fn rejects_oversize_line_before_buffering_unbounded() {
        // 1 KiB cap with a 4 KiB single line, no newline. Must error
        // out instead of growing the buffer past the cap.
        let big = vec![b'x'; 4096];
        let mut reader = BufReader::new(&big[..]);
        let mut buf = Vec::new();
        let err = read_line_capped(&mut reader, &mut buf, 1024)
            .await
            .expect_err("expected cap error");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        // The partial buf may carry up to one fill_buf() worth of
        // data, but never more than `cap` bytes by the time we
        // returned: the check fires before we extend past the cap.
        assert!(buf.len() <= 1024);
    }

    #[tokio::test]
    async fn cap_applies_across_multiple_fills() {
        // The cap must consider the cumulative buffer, not just the
        // current fill_buf chunk. Construct a reader whose internal
        // chunks each fit under the cap but together overflow.
        let half = vec![b'x'; 800];
        let mut combined = Vec::new();
        combined.extend_from_slice(&half);
        combined.extend_from_slice(&half);
        let mut reader = BufReader::new(&combined[..]);
        let mut buf = Vec::new();
        let err = read_line_capped(&mut reader, &mut buf, 1024)
            .await
            .expect_err("expected cap error across multiple fills");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn inactivity_timeout_falls_back_to_default() {
        assert_eq!(
            resolve_inactivity_timeout(None),
            Duration::from_secs(DEFAULT_STREAM_INACTIVITY_SECS)
        );
        assert_eq!(
            resolve_inactivity_timeout(Some(10)),
            Duration::from_secs(10)
        );
    }
}
