# Creating Or Opening A Drive

A drive is a directory that contains markdown and related files. Chan treats
the drive root as the filesystem boundary for editing, search, graph, and
terminal work.

## Desktop

Open Chan Desktop and choose an existing folder or create a new drive from
the drives window. Track A will use this manual tree as the first-launch seed
for the default `Chan` drive.

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
