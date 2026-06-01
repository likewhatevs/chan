# Chan Manual

Chan is an AI-native IDE for the modern engineer. You point it at a
folder on disk (a workspace) and drive your project in Markdown (design
docs, specs, tasks), with AI agents working alongside you in the editor
and terminal. Edit, search, graph, run terminals, and coordinate agents
over that tree, through the desktop app or the standalone `chan serve`
command.

Start here:

- [Install choices](install.md)
- [Creating or opening a workspace](workspaces.md)
- [Editing markdown](editing-markdown.md)
- [Wiki-links](wiki-links.md)
- [Search and graph basics](search-and-graph.md)
- [Terminal and MCP discovery](terminal-and-mcp.md)
- [Tunnel basics](tunnel.md)
- [Upgrade and troubleshooting](upgrade-and-troubleshooting.md)

## What stays on disk

Your markdown files remain ordinary files under the workspace root. Chan reads,
writes, searches, and watches that tree through the workspace layer. You can edit
the same files with another editor, commit them to git, or sync them with a
file sync tool.

## What is local

Local use does not require an account. The standalone server binds to
loopback by default and prints a per-launch URL with a bearer token. Tunnel
mode is opt-in.
