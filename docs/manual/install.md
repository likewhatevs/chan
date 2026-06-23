# Installing Chan

Chan ships two ways. **Chan Desktop** is the native app for macOS, Linux, and Windows (requires [Git for Windows](https://gitforwindows.org/)) and is what most people want. Installing it also sets up the `chan` and `cs` command line, so there is nothing else to download.

The **standalone CLI** is just `chan` (plus a `cs` symlink) for the remote SSH workflow, or any headless server.

Pick your download from the [install page](https://chan.app/install/), then follow the steps below.

## Chan Desktop

### macOS (Apple Silicon)

- Download the DMG from the [install page](https://chan.app/install/).
- Open it and drag **Chan** into Applications.
- Launch Chan. On first run it installs `chan` and `cs` into `~/.local/bin` — make sure that directory is on your `PATH`.

### Linux (amd64 / aarch64)

Download the AppImage, `.deb`, or `.rpm` from the [install page](https://chan.app/install/), then install it:

- **AppImage:** `chmod +x Chan*.AppImage && ./Chan*.AppImage`
- **Debian/Ubuntu:** `sudo apt install ./chan-desktop_*.deb`
- **Fedora:** `sudo dnf install ./chan-desktop-*.rpm`

Launch Chan; first run installs the `chan` and `cs` commands into `~/.local/bin`.

## Standalone CLI, headless

The fastest way on macOS or Linux is the shell installer. It installs the `chan` CLI into `PREFIX/bin` (default `~/.local/bin`) plus a `cs` symlink for the control-socket client behind `cs terminal` and friends. It reads release metadata from `chan.app/dl/cli/` and downloads the matching asset:

```sh
curl -fsSL https://chan.app/install.sh | sh
```

Verify the install:

```sh
chan --version
```

Prefer a package or a manual download? Grab the `.tar.gz` (macOS), or the static binary, `.deb`, or `.rpm` (Linux) from the [install page](https://chan.app/install/).

### Installer options

Use `PREFIX` to choose the install prefix:

```sh
PREFIX=/usr/local sudo sh install.sh
```

Use `METADATA_URL` to test a local or mirrored metadata file:

```sh
METADATA_URL=/tmp/chan-cli-latest.json sh install.sh
```

Use `BASE` to point at another metadata directory. With `VERSION`, the installer reads `v<version>.json` from that directory:

```sh
BASE=https://mirror.example/dl/cli VERSION=X.Y.Z sh install.sh
```

Unsupported OS and architecture pairs fail explicitly. Read the script before piping it to a shell: [install.sh](https://chan.app/install.sh).

## First run

On first run, you get the best terminal:
![](./image.png#w=250)

Make sure `~/.local/bin` is in `$PATH` and create your first workspace:

```sh
chan open ~/path-to-my-project
```

When run from chan-desktop, it opens the workspace in a new window. On devserver, it registers the repo. On standalone client, it starts a loopback server and prints a URL with a per-launch bearer token, then opens your browser if there is one. Next: [Creating or opening a workspace](workspaces.md).

## Release metadata

The static metadata files published under `chan.app/dl/` describe complete releases after all GitHub Release assets exist.

- `/dl/releases.json` feeds the install page download links.
- `/dl/cli/` feeds the shell installer and `chan upgrade`.
- `/dl/desktop/` feeds the desktop updater.

Metadata points at concrete GitHub Release asset URLs. The site and installer do not guess URLs from tags or Cargo versions.
