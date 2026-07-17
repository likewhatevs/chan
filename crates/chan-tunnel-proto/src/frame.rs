//! Length-prefixed JSON framing for the two control messages that
//! precede yamux. After the Hello/HelloAck pair the byte stream
//! belongs to yamux; this codec is not used again.

use bytes::{Buf, BufMut, BytesMut};
use serde::{de::DeserializeOwned, Serialize};

use crate::MAX_CONTROL_FRAME_BYTES;

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("control frame too large: {0} bytes")]
    TooLarge(u32),

    #[error("control frame truncated; need {need} more bytes")]
    Incomplete { need: usize },

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Encode `value` as `[u32 BE length][json bytes]`. The length
/// prefix is on the wire so the decoder can size a buffer before
/// allocating.
pub fn encode_frame<T: Serialize>(value: &T, out: &mut BytesMut) -> Result<(), FrameError> {
    let json = serde_json::to_vec(value)?;
    let len = u32::try_from(json.len()).map_err(|_| FrameError::TooLarge(u32::MAX))?;
    if json.len() > MAX_CONTROL_FRAME_BYTES {
        return Err(FrameError::TooLarge(len));
    }
    out.reserve(4 + json.len());
    out.put_u32(len);
    out.extend_from_slice(&json);
    Ok(())
}

/// Try to decode one frame from `buf`. On success, `buf` is
/// advanced past the frame and the decoded value is returned.
/// On `Incomplete`, `buf` is left untouched; the caller should
/// read more bytes and try again.
pub fn decode_frame<T: DeserializeOwned>(buf: &mut BytesMut) -> Result<T, FrameError> {
    if buf.len() < 4 {
        return Err(FrameError::Incomplete {
            need: 4 - buf.len(),
        });
    }
    let len = u32::from_be_bytes(buf[..4].try_into().expect("checked above"));
    if (len as usize) > MAX_CONTROL_FRAME_BYTES {
        return Err(FrameError::TooLarge(len));
    }
    let total = 4 + len as usize;
    if buf.len() < total {
        return Err(FrameError::Incomplete {
            need: total - buf.len(),
        });
    }
    buf.advance(4);
    let json = buf.split_to(len as usize);
    let value = serde_json::from_slice(&json)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hello, ProtocolVersion};

    #[test]
    fn roundtrip_hello() {
        let h = Hello {
            protocol: ProtocolVersion::V1,
            client_version: "chan/0.4.0".into(),
            workspace: "notes".into(),
            name: None,
        };
        let mut buf = BytesMut::new();
        encode_frame(&h, &mut buf).unwrap();
        let got: Hello = decode_frame(&mut buf).unwrap();
        assert_eq!(got.protocol, h.protocol);
        assert_eq!(got.client_version, h.client_version);
        assert_eq!(got.workspace, h.workspace);
        assert!(buf.is_empty());
    }

    #[test]
    fn incomplete_returns_need() {
        let h = Hello {
            protocol: ProtocolVersion::V1,
            client_version: "chan/0.4.0".into(),
            workspace: "notes".into(),
            name: None,
        };
        let mut full = BytesMut::new();
        encode_frame(&h, &mut full).unwrap();
        // Truncate to first 3 bytes (less than the 4-byte length prefix).
        let mut partial = BytesMut::from(&full[..3]);
        match decode_frame::<Hello>(&mut partial) {
            Err(FrameError::Incomplete { need }) => assert_eq!(need, 1),
            other => panic!("expected Incomplete, got {other:?}"),
        }
    }

    #[test]
    fn rejects_oversize_length_prefix() {
        let mut buf = BytesMut::new();
        buf.put_u32(MAX_CONTROL_FRAME_BYTES as u32 + 1);
        match decode_frame::<Hello>(&mut buf) {
            Err(FrameError::TooLarge(_)) => {}
            other => panic!("expected TooLarge, got {other:?}"),
        }
    }
}
