//! Linux systemd notify/fdstore helpers.
//!
//! This crate is the explicit unsafe boundary for systemd fdstore adoption:
//! systemd transfers inherited descriptors as raw fd numbers starting at 3.
//! The rest of chan consumes typed `OwnedFd` values.

#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "linux")]
use std::io::{IoSlice, Result};
#[cfg(target_os = "linux")]
use std::mem::MaybeUninit;
#[cfg(target_os = "linux")]
use std::os::fd::{BorrowedFd, FromRawFd, OwnedFd};
#[cfg(target_os = "linux")]
use std::os::linux::net::SocketAddrExt;
#[cfg(target_os = "linux")]
use std::os::unix::net::{SocketAddr, UnixDatagram};

#[cfg(target_os = "linux")]
use rustix::io::{fcntl_getfd, fcntl_setfd, FdFlags};
#[cfg(target_os = "linux")]
use rustix::net::{sendmsg, SendAncillaryBuffer, SendAncillaryMessage, SendFlags};

#[cfg(target_os = "linux")]
const LISTEN_FDS_START: i32 = 3;

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct NamedFd {
    pub name: String,
    pub fd: OwnedFd,
}

#[cfg(target_os = "linux")]
pub fn take_listen_fds() -> Vec<NamedFd> {
    let pid_ok = std::env::var("LISTEN_PID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .is_some_and(|pid| pid == std::process::id());
    let count = std::env::var("LISTEN_FDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let names = std::env::var("LISTEN_FDNAMES").unwrap_or_default();

    std::env::remove_var("LISTEN_PID");
    std::env::remove_var("LISTEN_FDS");
    std::env::remove_var("LISTEN_FDNAMES");

    if !pid_ok || count == 0 {
        return Vec::new();
    }

    let names: Vec<&str> = names.split(':').collect();
    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        let raw = LISTEN_FDS_START + idx as i32;
        let name = names.get(idx).copied().unwrap_or_default().to_string();
        // SAFETY: systemd's socket activation/fdstore protocol transfers
        // ownership of descriptors 3..3+LISTEN_FDS to this process when
        // LISTEN_PID matches our pid. We remove the env vars above so children
        // cannot accidentally adopt the same descriptors.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let _ = fcntl_getfd(&fd).and_then(|flags| fcntl_setfd(&fd, flags | FdFlags::CLOEXEC));
        out.push(NamedFd { name, fd });
    }
    out
}

#[cfg(target_os = "linux")]
pub fn notify_ready() -> Result<()> {
    notify("READY=1")
}

#[cfg(target_os = "linux")]
pub fn notify_status(status: &str) -> Result<()> {
    notify(&format!("STATUS={status}"))
}

#[cfg(target_os = "linux")]
pub fn fdstore(name: &str, fd: BorrowedFd<'_>) -> Result<()> {
    send_notify(
        Some(fd),
        &format!(
            "FDSTORE=1
FDNAME={name}
FDPOLL=0"
        ),
    )
}

#[cfg(target_os = "linux")]
pub fn fdstore_remove(name: &str) -> Result<()> {
    send_notify(
        None,
        &format!(
            "FDSTOREREMOVE=1
FDNAME={name}"
        ),
    )
}

#[cfg(target_os = "linux")]
pub fn fdstore_remove_many<'a>(names: impl IntoIterator<Item = &'a str>) {
    for name in names {
        let _ = fdstore_remove(name);
    }
}

#[cfg(target_os = "linux")]
fn notify(message: &str) -> Result<()> {
    send_notify(None, message)
}

#[cfg(target_os = "linux")]
fn send_notify(fd: Option<BorrowedFd<'_>>, message: &str) -> Result<()> {
    let Some(addr) = notify_socket_addr()? else {
        return Ok(());
    };
    let sock = UnixDatagram::unbound()?;
    sock.connect_addr(&addr)?;
    let iov = [IoSlice::new(message.as_bytes())];
    if let Some(fd) = fd {
        let fds = [fd];
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
        let mut ancillary = SendAncillaryBuffer::new(&mut space);
        if !ancillary.push(SendAncillaryMessage::ScmRights(&fds)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "ancillary buffer too small for fdstore message",
            ));
        }
        sendmsg(&sock, &iov, &mut ancillary, SendFlags::empty())?;
    } else {
        sock.send(message.as_bytes())?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn notify_socket_addr() -> Result<Option<SocketAddr>> {
    let Some(raw) = std::env::var_os("NOTIFY_SOCKET") else {
        return Ok(None);
    };
    if raw.is_empty() {
        return Ok(None);
    }
    let raw = raw.to_string_lossy();
    if let Some(name) = raw.strip_prefix('@') {
        return SocketAddr::from_abstract_name(name.as_bytes()).map(Some);
    }
    SocketAddr::from_pathname(raw.as_ref()).map(Some)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn fdstore_remove_is_noop_without_notify_socket() {
        std::env::remove_var("NOTIFY_SOCKET");
        fdstore_remove("chan.test").unwrap();
    }
}
