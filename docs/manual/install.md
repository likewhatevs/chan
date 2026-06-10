# Install Choices

Use Chan Desktop for normal local editing. Use the standalone CLI when you
want a terminal-first server, an SSH workflow, or explicit `chan serve`
control.

## Install page

The [install page](/install/) lists every released package — Chan Desktop,
the standalone `chan` CLI, and the Chan Gateway components — for macOS and
Linux, with links resolved from the published release metadata. Desktop
packages are built by the desktop release workflow; the shell installer below
does not install them. If metadata is unavailable, the install page falls back
to the GitHub Releases page.

## Standalone CLI

The POSIX installer installs the `chan` CLI into `PREFIX/bin` (default
`~/.local/bin`) plus a `cs` symlink to it — the control-socket client behind
`cs terminal`, `cs poke`, and friends. It does not install the desktop
packages. The script is served from `chan.app`, reads release metadata from
`chan.app/dl/cli/`, and downloads the matching asset URL named by that
metadata:

```sh
curl -fsSL https://chan.app/install.sh | sh
```

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
BASE=https://mirror.example/dl/cli VERSION=X.Y.Z sh install.sh
```

Unsupported OS and architecture pairs fail explicitly.

## Release Metadata

The static metadata files published under `chan.app/dl/` describe complete
releases after all GitHub Release assets exist.

- `/dl/releases.json` feeds the install page download links.
- `/dl/cli/` feeds the shell installer and `chan upgrade`.
- `/dl/desktop/` feeds the desktop updater.

Metadata points at concrete GitHub Release asset URLs. The site and installer
do not guess URLs from tags or Cargo versions.
