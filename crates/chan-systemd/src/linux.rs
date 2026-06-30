use std::io::{IoSlice, Read, Result};
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixDatagram, UnixStream};
use std::time::Duration;

use rustix::fs::{fstat, fstatfs};
use rustix::io::{fcntl_getfd, fcntl_setfd, FdFlags};
use rustix::net::{sendmsg, SendAncillaryBuffer, SendAncillaryMessage, SendFlags};
use rustix::process::{pidfd_open, Pid, PidfdFlags};

const LISTEN_FDS_START: i32 = 3;
const PID_FS_MAGIC: u64 = 0x5049_4446;

#[derive(Debug)]
pub struct NamedFd {
    pub name: String,
    pub fd: OwnedFd,
}

pub fn take_listen_fds() -> Vec<NamedFd> {
    let pid_ok = std::env::var("LISTEN_PID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .is_some_and(|pid| pid == std::process::id());
    let pidfdid = std::env::var("LISTEN_PIDFDID").ok();
    let pidfdid_ok = listen_pidfdid_ok(pidfdid.as_deref(), own_pidfdid());
    let count = std::env::var("LISTEN_FDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let names = std::env::var("LISTEN_FDNAMES").unwrap_or_default();

    std::env::remove_var("LISTEN_PID");
    std::env::remove_var("LISTEN_PIDFDID");
    std::env::remove_var("LISTEN_FDS");
    std::env::remove_var("LISTEN_FDNAMES");

    if !pid_ok || !pidfdid_ok || count == 0 {
        return Vec::new();
    }

    let names: Vec<&str> = names.split(':').collect();
    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        let raw = LISTEN_FDS_START + idx as i32;
        let name = names.get(idx).copied().unwrap_or_default().to_string();
        // SAFETY: systemd's socket activation/fdstore protocol transfers
        // ownership of descriptors 3..3+LISTEN_FDS to this process when
        // LISTEN_PID matches our pid. When systemd supplies LISTEN_PIDFDID and
        // this kernel can expose a pidfs-backed pidfd inode, we verify that too.
        // We remove the env vars above so children cannot accidentally adopt
        // the same descriptors.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let _ = fcntl_getfd(&fd).and_then(|flags| fcntl_setfd(&fd, flags | FdFlags::CLOEXEC));
        out.push(NamedFd { name, fd });
    }
    out
}

pub fn notify_ready() -> Result<()> {
    notify("READY=1")
}

pub fn notify_barrier(timeout: Duration) -> Result<()> {
    if notify_socket_addr()?.is_none() {
        return Ok(());
    }

    let (mut reader, writer) = UnixStream::pair()?;
    reader.set_read_timeout(Some(timeout))?;
    send_notify(Some(writer.as_fd()), "BARRIER=1")?;
    drop(writer);

    let mut buf = [0u8; 1];
    match reader.read(&mut buf) {
        Ok(0) => Ok(()),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "systemd barrier fd became readable before close",
        )),
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "timed out waiting for systemd notify barrier",
            ))
        }
        Err(e) => Err(e),
    }
}

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

pub fn fdstore_remove_many<'a>(names: impl IntoIterator<Item = &'a str>) {
    for name in names {
        let _ = fdstore_remove(name);
    }
}

fn fdstore_remove(name: &str) -> Result<()> {
    send_notify(
        None,
        &format!(
            "FDSTOREREMOVE=1
FDNAME={name}"
        ),
    )
}

fn notify(message: &str) -> Result<()> {
    send_notify(None, message)
}

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
                "ancillary buffer too small for systemd notify message",
            ));
        }
        sendmsg(&sock, &iov, &mut ancillary, SendFlags::empty())?;
    } else {
        sock.send(message.as_bytes())?;
    }
    Ok(())
}

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

fn listen_pidfdid_ok(raw: Option<&str>, own: Option<u64>) -> bool {
    let Some(raw) = raw else {
        return true;
    };
    let Ok(expected) = raw.parse::<u64>() else {
        return false;
    };
    own.is_some_and(|actual| actual == expected)
}

fn own_pidfdid() -> Option<u64> {
    let raw_pid = i32::try_from(std::process::id()).ok()?;
    let pid = Pid::from_raw(raw_pid)?;
    let pidfd = pidfd_open(pid, PidfdFlags::empty()).ok()?;
    let statfs = fstatfs(&pidfd).ok()?;
    if statfs.f_type as u64 != PID_FS_MAGIC {
        return None;
    }
    let stat = fstat(&pidfd).ok()?;
    if std::mem::size_of_val(&stat.st_ino) < 8 {
        return None;
    }
    Some(stat.st_ino as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fdstore_remove_is_noop_without_notify_socket() {
        std::env::remove_var("NOTIFY_SOCKET");
        fdstore_remove("chan.test").unwrap();
    }

    #[test]
    fn notify_barrier_is_noop_without_notify_socket() {
        std::env::remove_var("NOTIFY_SOCKET");
        notify_barrier(Duration::from_millis(1)).unwrap();
    }

    #[test]
    fn listen_pidfdid_is_checked_when_available() {
        assert!(listen_pidfdid_ok(None, Some(42)));
        assert!(listen_pidfdid_ok(Some("42"), Some(42)));
        assert!(!listen_pidfdid_ok(Some("43"), Some(42)));
        assert!(!listen_pidfdid_ok(Some("nope"), Some(42)));
    }

    #[test]
    fn listen_pidfdid_is_rejected_when_kernel_cannot_report_own_id() {
        assert!(!listen_pidfdid_ok(Some("42"), None));
    }

    #[test]
    fn take_listen_fds_clears_systemd_activation_env_on_pid_mismatch() {
        std::env::set_var("LISTEN_PID", "0");
        std::env::set_var("LISTEN_PIDFDID", "123");
        std::env::set_var("LISTEN_FDS", "1");
        std::env::set_var("LISTEN_FDNAMES", "chan.pty.test");

        assert!(take_listen_fds().is_empty());
        assert!(std::env::var_os("LISTEN_PID").is_none());
        assert!(std::env::var_os("LISTEN_PIDFDID").is_none());
        assert!(std::env::var_os("LISTEN_FDS").is_none());
        assert!(std::env::var_os("LISTEN_FDNAMES").is_none());
    }
}
