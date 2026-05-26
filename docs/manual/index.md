# Chan Manual

Chan works with plain markdown drives. A drive is a folder on disk that
Chan opens through the desktop app or through the standalone `chan serve`
command.

Start here:

- [Install choices](install.md)
- [Creating or opening a drive](drives.md)
- [Editing markdown](editing-markdown.md)
- [Wiki-links](wiki-links.md)
- [Search and graph basics](search-and-graph.md)
- [Terminal and MCP discovery](terminal-and-mcp.md)
- [Tunnel basics](tunnel.md)
- [Upgrade and troubleshooting](upgrade-and-troubleshooting.md)

## What stays on disk

Your markdown files remain ordinary files under the drive root. Chan reads,
writes, searches, and watches that tree through the drive layer. You can edit
the same files with another editor, commit them to git, or sync them with a
file sync tool.

## What is local

Local use does not require an account. The standalone server binds to
loopback by default and prints a per-launch URL with a bearer token. Tunnel
mode is opt-in.
