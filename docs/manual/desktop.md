# Chan Desktop

Chan Desktop is the native version of the IDE for macOS and Linux. It hosts the same workspace surface as `chan serve` in a native window (no browser chrome), and it can attach to a workspace running on another machine.

## Install

Download the desktop package for your platform from the [install page](/install/). Desktop packages are release artifacts; the shell installer (`curl -fsSL https://chan.app/install.sh | sh`) installs the standalone `chan` CLI, not the desktop app.

## Local workspaces

Open a folder on disk and Chan Desktop launches a local `chan serve` for it and mounts the editor in the window. This is the same single-user, single-machine model as the CLI; your files stay ordinary files under the workspace root.

## Windows

Each workspace or terminal you open gets its own native window, and a running workspace can have several. Closing a window with its title-bar button hides it rather than destroying it: its terminals keep running and its layout stays warm, and the Window menu lists hidden windows to bring back (the "Hidden Windows" header shows how many are kept warm). Hidden windows do not count against the per-workspace window cap.

From inside a terminal, the `cs window` family manages these windows directly — list them, open and hide them, remove them for good, and rename them. See [Terminal](terminal.md) for the command reference.

## Remote workspaces

A remote workspace is a `chan serve` running on another machine (a VM on your laptop, a box on your LAN, a VPS). The remote listens; Chan Desktop connects to it over HTTP/2.

1. On the remote machine, install chan and start a server:

```sh
# On the remote host (e.g. a Linux VM):
curl -fsSL https://chan.app/install.sh | sh
git clone https://github.com/fiorix/chan
chan serve ./chan
```

   Copy the URL `chan serve` prints (it carries the per-launch bearer token).

2. In Chan Desktop: New -> Remote, and paste that URL. The workspace opens in a native window and feels local.

If the remote port is not directly reachable, forward it over SSH first, then paste the resulting `http://localhost:<port>/...` URL:

```sh
ssh user@host -L 8787:localhost:8787
# then, in that session on the remote host:
#   curl -fsSL https://chan.app/install.sh | sh && chan serve ./repo
# paste the printed localhost URL into New -> Remote
```

A Lima VM on a Mac is a convenient remote: it is a real Linux host reachable from the Mac, so the outbound flow exercises the full remote path locally.

## Devservers

A devserver is a `chan devserver` running on a box that hosts many workspaces behind one port (see [workspaces](workspaces.md)). Chan Desktop attaches to it and lists its workspaces in their own launcher group. For running the devserver on Linux across logout (`--systemd`) and reaching it from the desktop at `localhost` through a lima VM, an sdme container, or `ssh -L`, see [Devserver](devserver.md).

1. New -> Devserver, and fill in the host and port. An optional script runs to bring the devserver up before the desktop dials it, for example `ssh user@box -L 8787:localhost:8787 chan devserver --bind 127.0.0.1 --port 8787`. Adding a devserver connects to it right away.

2. The group header carries the lifecycle controls:

   - **Connect / Disconnect** the devserver. While connected, the group lists its live workspace rows.
   - **New Terminal** (connected only) opens another standalone terminal on the devserver.
   - The caret menu holds **Edit** (disconnected only, so a live connection's recipe cannot change under it) and **Forget** (drops the saved devserver).

3. Each workspace row has **Open** (opens that workspace in a native window) and **Forget** (unmounts it from the devserver).

If a scripted connect fails, the desktop keeps the control terminal open showing why and offers to retry, edit the recipe, or abandon the devserver.
