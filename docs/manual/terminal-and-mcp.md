# Terminal And MCP Discovery

Terminal tabs start at the drive root. They are intended for shell work that
belongs next to the files you are editing.

## MCP environment

When the server MCP bridge is available, Chan exports discovery variables into
terminal sessions:

```text
CHAN_MCP_SERVER_NAME=chan
CHAN_MCP_SOCKET=...
CHAN_MCP_COMMAND=...
CHAN_MCP_COMMAND_JSON=...
CHAN_MCP_SERVER_JSON=...
```

External agent CLIs launched from that terminal can translate the `CHAN_`
descriptor into their own MCP configuration shape.

## External agents only

Chan exposes its drive tools through MCP for external agents. It does not
ship in-app chat or assistant HTTP APIs.
