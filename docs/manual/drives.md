# Creating Or Opening A Drive

A drive is a directory that contains markdown and related files. Chan treats
the drive root as the filesystem boundary for editing, search, graph, and
terminal work.

## Desktop

On a fresh install, Chan Desktop creates a `Chan` drive under your Documents
folder and seeds it with this manual. From the drives window you can also
open an existing folder, attach a running `chan serve` URL, or receive a
remote drive through the listener.

## CLI

Start a local server for a folder:

```sh
chan serve ~/notes
```

The command prints a loopback URL with a bearer token. Open that URL in a
browser on the same machine.

## Drive contents

Chan watches the drive tree for external edits. The files are still yours:
edit them with another program, commit them to git, or move the folder as a
normal directory.
