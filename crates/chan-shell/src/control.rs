//! Client side of the control socket: resolve the chan-terminal
//! environment ($CHAN_WINDOW_ID / $CHAN_CONTROL_SOCKET), make paths
//! absolute, and round-trip a [`ControlRequest`] over the Unix-domain
//! socket to the chan-server the terminal belongs to.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

// ControlResponse is only decoded on the unix transport path; importing
// it unconditionally would warn on a non-unix build (where the stub
// below never reads a reply).
use crate::wire::ControlRequest;
#[cfg(unix)]
use crate::wire::ControlResponse;

/// The chan-terminal environment a window-targeting action needs: which
/// window to act on and which server socket to reach it through.
#[derive(Debug)]
pub struct OpenEnv {
    pub window_id: String,
    pub control_socket: PathBuf,
}

/// Build an [`OpenEnv`] from explicit values (the env-var lookups live in
/// [`open_env`]; this split keeps the validation unit-testable without
/// touching the process environment).
pub fn open_env_from(window_id: Option<String>, control_socket: Option<String>) -> Result<OpenEnv> {
    let window_id = window_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("not running inside a chan session; this needs $CHAN_WINDOW_ID")
        })?;
    let control_socket = control_socket
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("not running inside a chan session; this needs $CHAN_CONTROL_SOCKET")
        })?;
    Ok(OpenEnv {
        window_id,
        control_socket: PathBuf::from(control_socket),
    })
}

/// Resolve the full chan-terminal environment from the process env, for
/// category-1 actions that target a specific window.
pub fn open_env() -> Result<OpenEnv> {
    open_env_from(
        std::env::var("CHAN_WINDOW_ID").ok(),
        std::env::var("CHAN_CONTROL_SOCKET").ok(),
    )
}

/// Resolve just the control socket, for category-2 actions (`cs terminal
/// write` / `terminal list` / `search`) that act on the server's live
/// sessions and so do not need a window to target.
pub fn control_socket_env() -> Result<PathBuf> {
    let socket = std::env::var("CHAN_CONTROL_SOCKET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("not running inside a chan terminal; this needs $CHAN_CONTROL_SOCKET")
        })?;
    Ok(PathBuf::from(socket))
}

/// Make a path absolute against the shell's current working directory.
/// Relative `cs open` / `cs terminal new` paths resolve where the user
/// typed them, not where the server runs.
pub fn absolutize(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()
            .context("resolving current directory")?
            .join(path))
    }
}

/// Connect to the control socket, write one JSON request line, and return
/// the server's reply message (or its error, surfaced as an `Err`).
#[cfg(unix)]
pub async fn send_control_request(socket: &Path, request: ControlRequest) -> Result<String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let stream = UnixStream::connect(socket)
        .await
        .with_context(|| format!("connecting to chan control socket {}", socket.display()))?;
    let (read, mut write) = stream.into_split();
    let mut payload = serde_json::to_vec(&request).context("encoding control request")?;
    payload.push(b'\n');
    write
        .write_all(&payload)
        .await
        .context("writing control request")?;
    write.shutdown().await.context("closing control request")?;

    let mut line = String::new();
    BufReader::new(read)
        .read_line(&mut line)
        .await
        .context("reading control response")?;
    let response: ControlResponse =
        serde_json::from_str(&line).context("decoding control response")?;
    match response {
        ControlResponse::Ok { message } => Ok(message),
        ControlResponse::Error { message } => anyhow::bail!("{message}"),
    }
}

/// Non-unix builds have no Unix-domain control socket; the `cs` surface
/// fails fast instead of half-working.
#[cfg(not(unix))]
pub async fn send_control_request(_socket: &Path, _request: ControlRequest) -> Result<String> {
    anyhow::bail!("chan shell requires unix-domain sockets on this build");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_env_requires_window_id_and_control_socket() {
        let err = open_env_from(None, Some("/tmp/chan-control.sock".into())).unwrap_err();
        assert!(err.to_string().contains("CHAN_WINDOW_ID"));

        let err = open_env_from(Some("win".into()), None).unwrap_err();
        assert!(err.to_string().contains("CHAN_CONTROL_SOCKET"));

        let env = open_env_from(
            Some(" win ".into()),
            Some(" /tmp/chan-control.sock ".into()),
        )
        .unwrap();
        assert_eq!(env.window_id, "win");
        assert_eq!(env.control_socket, PathBuf::from("/tmp/chan-control.sock"));
    }
}
