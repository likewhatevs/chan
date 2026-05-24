# Install Choices

Use Chan Desktop for normal local editing. Use the standalone CLI when you
want a terminal-first server, an SSH workflow, or explicit `chan serve`
control.

## Desktop

The public install page links the active desktop artifacts:

- macOS DMG
- Linux AppImage
- Linux deb

Desktop packages are built by the desktop release workflow. The shell
installer does not install desktop packages.

## Standalone CLI

The POSIX installer installs only the `chan` CLI. The script is served from
`chan.app`, but release tarballs are downloaded from GitHub Releases:

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

Use `BASE` to point at another directory containing release assets:

```sh
BASE=https://github.com/fiorix/chan/releases/download/chan-v0.14.0 sh install.sh
```

Unsupported OS and architecture pairs fail explicitly.
