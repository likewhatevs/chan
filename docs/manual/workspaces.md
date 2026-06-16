# Creating Or Opening A Workspace

A workspace is a directory that contains markdown and related files. Chan treats the workspace root as the filesystem boundary for editing, search, graph, and terminal work.

## Desktop

On launch, Chan Desktop opens an empty workspaces window and a standalone terminal; it never creates a workspace for you (no `Chan` folder under Documents, no seeded manual). From the workspaces window, open an existing folder, attach a running `chan serve` URL, or receive a remote workspace through the listener.

## CLI

Start a local server for a folder:

```sh
chan serve ~/notes
```

The command prints a loopback URL with a bearer token. Open that URL in a browser on the same machine.

The workspace path is required: `chan serve` with no path exits with an error asking you to pass one (for example `chan serve .`).

`chan add PATH` registers a workspace without serving it; `chan list` and `chan remove` manage the registry.

Useful `chan serve` flags:

- `-4` / `-6`: force IPv4 / IPv6 loopback (default 127.0.0.1).
- `--host`, `--port`: bind elsewhere. No TLS; loud warning when binding off-loopback.
- `--prefix /seg`: mount under a URL prefix so a reverse proxy can multiplex many `chan serve` instances under one host.
- `--timeout 30s` / `5m` / `1h`: graceful shutdown after an idle window with no HTTP / WebSocket activity. Designed for systemd socket-activation.
- `--no-token`: skip the bearer-token gate (loopback bind only).

`chan --help` documents the full flag surface.

## Workspace contents

Chan watches the workspace tree for external edits. The files are still yours: edit them with another program, commit them to git, or move the folder as a normal directory.

## File transfers

File Browser inspectors expose Upload and Download for selected files and directories. Graph inspectors expose the same actions for file and directory nodes where the action applies.

- For a file, Upload replaces the selected file. Text-class paths reject uploaded bytes that are not valid UTF-8.
- For a directory, Upload adds the selected files inside that directory. Existing target paths are refused.
- Download retrieves the selected file as-is. Downloading a directory retrieves a tar archive rooted at that directory name.
