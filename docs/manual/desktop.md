# Chan Desktop

Chan Desktop is the native version of the IDE for macOS and Linux. It hosts
the same workspace surface as `chan serve` in a native window (no browser
chrome), and it can attach to workspaces running on other machines, both
outbound and inbound.

## Install

Download the desktop package for your platform from the
[install page](/install/). Desktop packages are release artifacts; the shell
installer (`curl -fsSL https://chan.app/install.sh | sh`) installs the
standalone `chan` CLI, not the desktop app.

## Local workspaces

Open a folder on disk and Chan Desktop launches a local `chan serve` for it
and mounts the editor in the window. This is the same single-user,
single-machine model as the CLI; your files stay ordinary files under the
workspace root.

## Remote workspaces

A remote workspace is a `chan serve` running on another machine (a VM on your
laptop, a box on your LAN, a VPS). Chan Desktop reaches it two ways.

### Outbound: you dial the remote

The remote listens; the desktop connects to it over HTTP/2.

1. On the remote machine, install chan and start a server:

   ```sh
   # On the remote host (e.g. a Linux VM):
   curl -fsSL https://chan.app/install.sh | sh
   git clone https://github.com/fiorix/chan
   chan serve ./chan
   ```

   Copy the URL `chan serve` prints (it carries the per-launch bearer token).

2. In Chan Desktop: New -> Remote -> Outbound, and paste that URL. The
   workspace opens in a native window and feels local.

If the remote port is not directly reachable, forward it over SSH first, then
paste the resulting `http://localhost:<port>/...` URL:

```sh
ssh user@host -L 8787:localhost:8787
# then, in that session on the remote host:
#   curl -fsSL https://chan.app/install.sh | sh && chan serve ./repo
# paste the printed localhost URL into New -> Remote -> Outbound
```

A Lima VM on a Mac is a convenient remote: it is a real Linux host reachable
from the Mac, so the outbound flow exercises the full remote path locally.

### Inbound: the remote dials you

When you cannot listen on the remote machine, Chan Desktop can accept a
reverse tunnel: the desktop listens, and the remote `chan serve` dials back
to it.

1. In Chan Desktop: New -> Remote -> Inbound, and pick a port to listen on
   (or `0` to let the OS choose).
2. Copy the `chan serve ... --tunnel-url=<desktop-listener>` command the
   dialog shows, and run it on the remote machine. The workspace then appears
   in the desktop window.

## Verification status

- `chan serve` (local and on a remote host) is verified.
- The in-app New -> Remote -> Outbound / Inbound click-paths run in the
  desktop webview (WKWebView on macOS), which the automated tooling cannot
  drive. They are PENDING a hand-smoke by the maintainer before this page is
  presented as fully verified.
