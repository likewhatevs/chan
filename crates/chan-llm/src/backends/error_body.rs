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

use crate::session::LlmEventError;

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

/// Parsed error envelope from a vendor's HTTP body. Both Anthropic
/// and Gemini wrap errors in a JSON object with a category string
/// and a human-readable message; chan-llm extracts both so backends
/// can emit a clean `LlmEventError` instead of dumping raw JSON
/// into `on_error`.
///
/// Crate-private: only the classifier and its tests construct this;
/// callers above this module receive the fully-typed `LlmEventError`
/// the classifier produces, never the intermediate envelope.
pub(crate) struct VendorError {
    /// Short category from the vendor envelope. Anthropic supplies
    /// strings like `authentication_error`, `permission_error`,
    /// `not_found_error`, `invalid_request_error`, `rate_limit_error`,
    /// `api_error`, `overloaded_error`. Gemini supplies the
    /// uppercase `status` field: `UNAUTHENTICATED`, `PERMISSION_DENIED`,
    /// `INVALID_ARGUMENT`, `RESOURCE_EXHAUSTED`, `UNAVAILABLE`, etc.
    /// Lets the classifier promote an Anthropic 500 with
    /// `overloaded_error` to a 529-equivalent `Backend` event the
    /// host can render distinctly.
    pub kind: String,
    /// Human-readable message. Safe to surface to the user.
    pub message: String,
}

/// Try to extract `(kind, message)` from a vendor error envelope.
/// Returns `None` when the body isn't JSON or doesn't match either
/// shape; callers fall back to the raw body string in that case.
///
/// Anthropic shape:
///   `{"type":"error","error":{"type":"<kind>","message":"<msg>"}}`
///   or just `{"error":{"type":..., "message":...}}`.
///
/// Gemini shape (Google API standard):
///   `{"error":{"code":<n>,"message":"<msg>","status":"<KIND>"}}`.
///
/// Both have a top-level `error` object with at least a `message`.
/// Anthropic uses `type`; Gemini uses `status` (and may also include
/// `code` for the numeric HTTP status). We accept either field for
/// `kind`.
pub(crate) fn parse_vendor_error(body: &str) -> Option<VendorError> {
    let value: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
    let err = value.get("error")?;
    // Anthropic: error is an object. Gemini: error is an object.
    // Some older or wrapper APIs (e.g. some Bedrock paths) flatten
    // `error` to a string; treat that as the message with no kind.
    if let Some(s) = err.as_str() {
        return Some(VendorError {
            kind: String::new(),
            message: s.to_string(),
        });
    }
    let obj = err.as_object()?;
    let message = obj
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    if message.is_empty() {
        return None;
    }
    let kind = obj
        .get("type")
        .and_then(|t| t.as_str())
        .or_else(|| obj.get("status").and_then(|s| s.as_str()))
        .unwrap_or("")
        .to_string();
    Some(VendorError { kind, message })
}

/// Soft cap on the message we surface from a non-JSON error body.
/// 800 chars covers any reasonable single-line error; bigger bodies
/// are typically HTML pages from a proxy and the model / user can't
/// act on the full text anyway.
const RAW_BODY_MESSAGE_CAP_CHARS: usize = 800;

/// Map an HTTP status + body to a typed `LlmEventError`. Used by HTTP
/// backends when `send_with_retry` returns a non-success response so
/// hosts can branch on the failure category (Auth, RateLimited,
/// BadRequest, Backend) without substring-matching on a free-form
/// string.
///
/// `body` is the (possibly truncated) response body; we try
/// `parse_vendor_error` first and fall back to the truncated raw
/// body when the upstream didn't emit a recognised envelope.
///
/// `retry_after_secs` is the upstream hint (when one was supplied);
/// callers usually pass `None` here because retry exhaustion already
/// consumed the value, but threading it through keeps the host UX
/// honest when present.
pub(crate) fn classify_http_error(
    backend: &str,
    status: u16,
    body: &str,
    retry_after_secs: Option<u64>,
) -> LlmEventError {
    let parsed = parse_vendor_error(body);
    let kind = parsed.as_ref().map(|v| v.kind.as_str()).unwrap_or("");
    let message = match &parsed {
        Some(v) => v.message.clone(),
        None => body.chars().take(RAW_BODY_MESSAGE_CAP_CHARS).collect(),
    };
    // Anthropic promotes capacity pressure with the `overloaded_error`
    // kind even when the HTTP status is 500. Map that pair to a
    // synthetic 529 so the host's "service overloaded" affordance
    // fires the same way it would for an explicit 529 reply.
    let effective_status = if kind == "overloaded_error" && (500..=599).contains(&status) {
        529
    } else {
        status
    };
    match effective_status {
        401 | 403 => LlmEventError::Auth {
            backend: backend.to_string(),
            message,
        },
        429 => LlmEventError::RateLimited {
            backend: backend.to_string(),
            retry_after_secs,
            message,
        },
        // 400 Bad Request, 404 Not Found (model), 405 Method Not
        // Allowed, 422 Unprocessable Entity. Everything in this set
        // is the caller's fault and won't fix itself.
        400 | 404 | 405 | 422 => LlmEventError::BadRequest {
            backend: backend.to_string(),
            message,
        },
        500..=599 => LlmEventError::Backend {
            backend: backend.to_string(),
            status: effective_status,
            message,
        },
        _ => LlmEventError::Other {
            backend: backend.to_string(),
            message,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_anthropic_envelope() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let v = parse_vendor_error(body).expect("parsed");
        assert_eq!(v.kind, "authentication_error");
        assert_eq!(v.message, "invalid x-api-key");
    }

    #[test]
    fn parses_gemini_envelope() {
        let body =
            r#"{"error":{"code":401,"message":"API key not valid","status":"UNAUTHENTICATED"}}"#;
        let v = parse_vendor_error(body).expect("parsed");
        assert_eq!(v.kind, "UNAUTHENTICATED");
        assert_eq!(v.message, "API key not valid");
    }

    #[test]
    fn parses_flat_error_string() {
        let body = r#"{"error":"something went wrong"}"#;
        let v = parse_vendor_error(body).expect("parsed");
        assert_eq!(v.kind, "");
        assert_eq!(v.message, "something went wrong");
    }

    #[test]
    fn returns_none_on_non_json() {
        assert!(parse_vendor_error("<html>500 Internal Server Error</html>").is_none());
        assert!(parse_vendor_error("").is_none());
    }

    #[test]
    fn returns_none_when_no_message() {
        let body = r#"{"error":{"type":"something"}}"#;
        assert!(parse_vendor_error(body).is_none());
    }

    #[test]
    fn ignores_unrelated_top_level_keys() {
        let body = r#"{"error":{"type":"rate_limit_error","message":"slow down"},"extra":42}"#;
        let v = parse_vendor_error(body).expect("parsed");
        assert_eq!(v.kind, "rate_limit_error");
        assert_eq!(v.message, "slow down");
    }

    #[test]
    fn classify_maps_401_to_auth() {
        let body = r#"{"error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let e = classify_http_error("anthropic", 401, body, None);
        assert!(matches!(e, LlmEventError::Auth { .. }));
        assert!(e.to_string().contains("invalid x-api-key"));
    }

    #[test]
    fn classify_maps_429_with_retry_after() {
        let body = r#"{"error":{"type":"rate_limit_error","message":"slow down"}}"#;
        let e = classify_http_error("anthropic", 429, body, Some(60));
        match e {
            LlmEventError::RateLimited {
                retry_after_secs, ..
            } => assert_eq!(retry_after_secs, Some(60)),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn classify_maps_400_to_bad_request() {
        let body = r#"{"error":{"type":"invalid_request_error","message":"model x not found"}}"#;
        let e = classify_http_error("anthropic", 400, body, None);
        assert!(matches!(e, LlmEventError::BadRequest { .. }));
    }

    #[test]
    fn classify_maps_503_to_backend() {
        // Use `api_error` (not `overloaded_error`) so the test stays
        // a pure 5xx-passthrough check; the overloaded_error promotion
        // is covered separately below.
        let body = r#"{"error":{"type":"api_error","message":"try again"}}"#;
        let e = classify_http_error("anthropic", 503, body, None);
        match e {
            LlmEventError::Backend { status, .. } => assert_eq!(status, 503),
            other => panic!("expected Backend, got {other:?}"),
        }
    }

    #[test]
    fn classify_promotes_overloaded_error_to_529() {
        // Anthropic occasionally emits an HTTP 500 with kind
        // `overloaded_error` instead of a clean 529. The classifier
        // promotes that pair to a synthetic 529 so the host renders
        // a "service overloaded" affordance distinctly from a
        // generic 500.
        let body = r#"{"error":{"type":"overloaded_error","message":"capacity pressure"}}"#;
        let e = classify_http_error("anthropic", 500, body, None);
        match e {
            LlmEventError::Backend { status, .. } => assert_eq!(status, 529),
            other => panic!("expected Backend{{ status: 529 }}, got {other:?}"),
        }
    }

    #[test]
    fn classify_falls_back_to_truncated_raw_body() {
        // No JSON envelope; classifier truncates the raw body to
        // RAW_BODY_MESSAGE_CAP_CHARS and uses it as the message.
        let body = "x".repeat(2_000);
        let e = classify_http_error("anthropic", 500, &body, None);
        match e {
            LlmEventError::Backend { message, .. } => {
                assert!(message.len() <= RAW_BODY_MESSAGE_CAP_CHARS);
            }
            other => panic!("expected Backend, got {other:?}"),
        }
    }
}
