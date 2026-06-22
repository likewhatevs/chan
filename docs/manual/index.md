# Chan Manual

Chan is an AI-native IDE for the modern engineer. You point it at a folder on disk (a workspace) and drive your project in Markdown (design docs, specs, tasks), with AI agents working alongside you in the editor and terminal. Edit, search, graph, run terminals, and coordinate agents over that tree, through the desktop app or the standalone `chan open` command.

Start here:

- [Installing chan](install.md)
- [Creating or opening a workspace](workspaces.md)
- [Editing markdown](editing-markdown.md)
- [Wiki-links](wiki-links.md)
- [Search and graph basics](search-and-graph.md)
- [Terminal](terminal.md)
- [Tunnel basics](tunnel.md)
- [Chan Desktop and remote workspaces](desktop.md)
- [Devserver (many workspaces on one box)](devserver.md)
- [Chan Gateway (self-hosted tunnel)](gateway.md)
- [Upgrade and troubleshooting](upgrade-and-troubleshooting.md)

## What stays on disk

Your markdown files remain ordinary files under the workspace root. Chan reads, writes, searches, and watches that tree through the workspace layer. You can edit the same files with another editor, commit them to git, or sync them with a file sync tool.
