# Creating Or Opening A Workspace

A workspace is a directory that contains markdown and related files. Chan treats
the workspace root as the filesystem boundary for editing, search, graph, and
terminal work.

## Desktop

On a fresh install, Chan Desktop creates a `Chan` workspace under your Documents
folder and seeds it with this manual. From the workspaces window you can also
open an existing folder, attach a running `chan serve` URL, or receive a
remote workspace through the listener.

## CLI

Start a local server for a folder:

```sh
chan serve ~/notes
```

The command prints a loopback URL with a bearer token. Open that URL in a
browser on the same machine.

## Workspace contents

Chan watches the workspace tree for external edits. The files are still yours:
edit them with another program, commit them to git, or move the folder as a
normal directory.

## File transfers

File Browser inspectors expose Upload and Download for selected files and
directories. Graph inspectors expose the same actions for file and directory
nodes where the action applies.

- For a file, Upload replaces the selected file. Text-class paths reject
  uploaded bytes that are not valid UTF-8.
- For a directory, Upload adds the selected files inside that directory.
  Existing target paths are refused.
- Download retrieves the selected file as-is. Downloading a directory
  retrieves a tar archive rooted at that directory name.
