# Chan Manual

Chan is an AI-native IDE for the modern engineer. You point it at a folder on disk (a workspace) and drive your project in Markdown (design docs, specs, tasks), with AI agents working alongside you in the editor and terminal. Edit, search, graph, run terminals, and coordinate agents over that tree, through the desktop app or the standalone `chan open` command.

- Drive any number of parallel agents, each in its own terminal session.
- Each agent a different specialist — Claude, Codex, Gemini — matched to the task.
- Agents coordinate with each other through `cs` tooling and the built-in MCP server, not an in-app chatbot.
- A Markdown workspace your agents read and write directly, as ordinary files.
- Scriptable end to end, from one `cs` command line.

Start here:

- [Installing chan](./install.md)
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

## Chan stays out of the way

When you point chan at a workspace, it does not store any files in it. All of chan's files are stored in `~/.chan` keyed to the workspace directories.
You can seamlessly work on local and remote workspaces using [chan-desktop](install.md) and [chan devserver](devserver.md).
