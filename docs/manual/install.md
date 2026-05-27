# Install Choices

Use Chan Desktop for normal local editing. Use the standalone CLI when you
want a terminal-first server, an SSH workflow, or explicit `chan serve`
control.

## Desktop

The public install page reads release metadata from `chan.app/dl/` and links
the active desktop artifacts from that metadata:

- macOS DMG
- Linux AppImage
- Linux deb

Desktop packages are built by the desktop release workflow. The shell
installer does not install desktop packages. If metadata is unavailable, the
install page falls back to the GitHub Releases page.

## Standalone CLI

The POSIX installer installs only the `chan` CLI. The script is served from
`chan.app`, reads complete-release metadata from `chan.app/dl/cli/`, and
downloads the matching asset URL named by that metadata:

```sh
curl -fsSL https://chan.app/install.sh | sh
```

The active CLI release targets are:

- Linux x86_64
- Linux aarch64
- macOS aarch64

Use `PREFIX` to choose the install prefix:

```sh
PREFIX=/usr/local sudo sh install.sh
```

Use `METADATA_URL` to test a local or mirrored metadata file:

```sh
METADATA_URL=/tmp/chan-cli-latest.json sh install.sh
```

Use `BASE` to point at another metadata directory. With `VERSION`, the
installer reads `v<version>.json` from that directory:

```sh
BASE=https://mirror.example/dl/cli VERSION=0.14.0 sh install.sh
```

Unsupported OS and architecture pairs fail explicitly.

## Release Metadata

The static metadata files published under `chan.app/dl/` describe complete
releases after all GitHub Release assets exist.

- `/dl/releases.json` feeds the install page download links.
- `/dl/cli/latest.json` and `/dl/cli/vX.Y.Z.json` feed the shell installer
  and `chan upgrade`.
- `/dl/desktop/latest.json` and `/dl/desktop/vX.Y.Z.json` feed the desktop
  updater.

Metadata points at concrete GitHub Release asset URLs. The site and installer
do not guess URLs from tags or Cargo versions.
