//! Bounded read of an HTTP error-response body for logging.
//!
//! `reqwest::Response::text()` and `bytes()` are unbounded: a
//! malicious or buggy upstream returning a multi-gigabyte body
//! would force us to allocate the full thing before we get a
//! chance to truncate. Backends only want a snippet for logs, so
//! they go through `read_capped_text` instead.
//!
//! Reads at most `cap_bytes` from the streaming body, then
//! converts to a UTF-8 string with replacement characters for
//! any invalid sequences. Stops as soon as the cap is hit even
//! if the server keeps streaming.

use futures_util::StreamExt;
use reqwest::Response;

/// Default byte cap when callers don't care. Covers Anthropic's
/// and Gemini's structured error envelopes (a few KB at most)
/// with margin; deliberately small so a runaway HTML error page
/// can't dominate logs.
pub const DEFAULT_BODY_CAP_BYTES: usize = 16 * 1024;

/// Drain up to `cap_bytes` of `resp`'s body and return it as a
/// (possibly lossy) UTF-8 string. Trailing bytes past the cap are
/// dropped; the returned `bool` is true when the body exceeded
/// the cap so callers can mark the log line as truncated.
pub async fn read_capped_text(resp: Response, cap_bytes: usize) -> (String, bool) {
    let mut buf: Vec<u8> = Vec::with_capacity(cap_bytes.min(4096));
    let mut truncated = false;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(b) => {
                if buf.len() >= cap_bytes {
                    truncated = true;
                    break;
                }
                let remaining = cap_bytes - buf.len();
                if b.len() <= remaining {
                    buf.extend_from_slice(&b);
                } else {
                    buf.extend_from_slice(&b[..remaining]);
                    truncated = true;
                    break;
                }
            }
            // A mid-stream read error on an already-failed response
            // is not interesting; we log whatever we got.
            Err(_) => break,
        }
    }
    (String::from_utf8_lossy(&buf).into_owned(), truncated)
}
