# Chan Manual

Chan is a minimalist and powerful IDE. What you get:

- A powerful terminal that is automatable from the command line and agent-friendly
- An IDE with Markdown editor, Index, and Graph your workspaces, which are git repos or any directory

You can seamlessly use Chan on your desktop, laptop, and on your remote server. It is supported on macOS and Linux.

What can you do?

- Drive any number of parallel agents, each in its own terminal session. You write Markdown.
- Agents coordinate with each other through `cs` tooling and the built-in MCP server, not an in-app chatbot.
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
