# Terminal

Chan embeds a real terminal next to your files. Each terminal tab is a PTY rooted at the workspace, so shell work, builds, and AI agents all run in the tree you are editing. One terminal owns one agent: start an agent CLI (claude, codex, gemini) in a tab, and that tab is how you, and other agents, reach it.

## What a terminal exports

Every terminal session exports a few environment variables so scripts and agents know where they are:

```text
CHAN_TAB_NAME    the tab's name (also the cs selector for this session)
CHAN_TAB_GROUP   the broadcast group (default "default")
CHAN_MCP_*       MCP discovery, when the server's MCP bridge is up (below)
```

## The `cs terminal` command family

`cs` is Chan's in-terminal CLI; `cs terminal` drives the live terminal sessions in the current window. Prefix matching applies, so `cs t n`, `cs t w`, and `cs t l` resolve to `new`, `write`, and `list`.

Sessions are selected by `--tab-name <name>` (one session, the tab's CHAN_TAB_NAME) or `--tab-group <name>` (every session in a broadcast group). `write`, `restart`, and `survey` require at least one selector.

- `new [PATH]` opens a terminal tab. `--tab-name` sets its CHAN_TAB_NAME, `--tab-group` its group (default "default"); PATH defaults to the workspace root.
- `list` lists live sessions grouped by group; `--json` for machine output.
- `write` writes raw bytes to the selected session(s). No newline is appended (`cs terminal write $'ls\n'` runs a command; `cs terminal write ls` only types it); `--stdin` streams the bytes instead, and `--submit` appends an agent's submit chord (see Pokes).
- `restart` restarts the selected session(s), preserving each one's spawn command and env so an agent relaunches in place.
- `scrollback` dumps one session's scrollback ring to stdout. It takes `--tab-name` only (no group axis, since it reads a single terminal).
- `team new|load` creates or loads a Team Work team from a `config.toml` (the CLI form of the Cmd+P team dialog); `--script` emits the bootstrap as a runnable shell script instead of writing anything.

## The `cs window` command family

`cs window` inspects and manages the chan windows themselves — the OS windows, not the terminal tabs inside them. Prefix matching applies (`cs w l`, `cs w n`, `cs w o`); use `cs w hi` for `hide`, since a bare `h` is `help`. `cs window list` works anywhere; the rest drive the desktop app and report an error under a standalone `chan open`, which has no windows to manage.

- `list` shows every window chan knows about, with its id, kind (terminal or workspace), title, and status (`open` and/or `saved`). `--json` for machine output. The id is the handle the other verbs take.
- `new` opens a window. From a standalone terminal it opens another standalone terminal window; from a workspace it opens another window of that workspace. It prints the new window id.
- `open <id>` focuses a window, un-hiding it if it was hidden. A workspace window that was closed but still has a saved layout reopens when its workspace is running.
- `hide <id>` hides a window — the same as its title-bar close button (see [Chan Desktop](desktop.md)).
- `rm <id>` removes a window for good — it does not come back — and drops its saved layout. When the window still has running terminals it asks for confirmation first; `--force` skips the prompt.
- `title <id> <title>` sets a custom window title; an empty title resets it to the default. A title another open window already shows is rejected, so window names stay unambiguous.

## Pokes

A poke is how one agent hands work to another. `cs terminal write --submit <agent>` writes bytes into another tab AND submits them, so a running agent receives the input hands-free:

```sh
cs terminal write --tab-name=@@Worker --submit claude 'read tasks/next.md'
```

The submit chord differs per agent, because each agent CLI reads a different key as "submit":

- `claude`: the Cmd+Enter chord (an xterm modifyOtherKeys sequence).
- `codex` / `gemini`: a plain carriage return.

Omit `--submit` to write pure bytes: the text then lands in the agent's compose box unsubmitted (a bare newline is just a newline to an agent, not a submit), which is rarely what you want for a poke.

## Rich Prompt

Rich Prompt is a floating markdown input over the bottom of the active terminal. Toggle it with Cmd+Shift+P (or the terminal right-click "Show/Hide Rich Prompt" entry); one bubble per window follows whichever terminal is active. Type markdown freely: Enter inserts a newline, and Cmd+Enter submits.

Each terminal's bubble is backed by a real draft on disk, a `Drafts/<name>/draft.md` in the workspace, so the prompt text is an ordinary file. Pasting an image works like the editor: the image is written into the same draft folder and referenced with `![](path)`, so an agent reads the picture as a file (no base64), whichever agent it is. Submitting clears the draft text but keeps the folder and any pasted media, so the agent can still read them; the whole draft folder is discarded when the terminal closes.

Submit sends the text to the active terminal's agent through the same write queue the CLI uses, so a prompt and a `cs terminal write` poke share one ordered path.

## The write queue

Every write to a terminal session, from `cs terminal write` or from Rich Prompt, goes through a per-session FIFO queue. The queue serializes deliveries so chained messages never interleave into one compose buffer and submit one after another: the drainer delivers the next queued message only once the target agent has gone idle (its previous turn's output has quiesced), each with the right submit chord. A free target drains immediately; a busy one enqueues and drains as it frees.

The queue holds up to 100 messages per session and is dropped when the session is recycled (restart or close). It detects that the agent is generating, not that it has a half-typed but unsubmitted compose buffer, so a message can still land mid-buffer in the rare case where text was left typed and paused; routing all input through the queue is what keeps that case rare.

## Survey

`cs terminal survey` asks a question and blocks until it is answered. It raises a survey over the SPA window that owns the target tab and prints the chosen option to stdout (or, with `--followup`, the path of a follow-up file the UI writes):

```sh
cs terminal survey --tab-name=@@Host --title "Cut the release?" \
    --option "Yes" --option "Not yet" "v0.24.0 is green."
```

Options are numbered `[1]`..`[4]` in the UI, and `--stdin` reads a multi-line markdown body.

The constraint to know: survey needs a live SPA window that owns the tab. A terminal running as a bare PTY with no window attached matches nothing, and the command fails with `no live terminal session matched`. Open the tab in a Chan window before surveying it.

## Team Work

Team Work runs a set of agents as a team across terminal tabs. The lead is an ordinary terminal, with the same Rich Prompt and survey as any other and no special composer: on spawn it receives its identity prompt through the same write queue, then runs like any other terminal. You bootstrap a team with Cmd+P (the team setup/load dialog) or `cs terminal team new|load`, which writes or reads the team's `config.toml` and the generated bootstrap. The team then coordinates through the same `cs terminal` tools (pokes, survey) this page covers.

## MCP discovery

When the server's MCP bridge is available, each terminal session exports a descriptor for Chan's in-process MCP server:

```text
CHAN_MCP_SERVER_NAME=chan
CHAN_MCP_SOCKET=...
CHAN_MCP_COMMAND=...
CHAN_MCP_COMMAND_JSON=...
CHAN_MCP_SERVER_JSON=...
```

An external agent CLI launched from the terminal translates that `CHAN_` descriptor into its own MCP configuration shape, which is how it picks up Chan's workspace tools. Chan exposes its tools through MCP for external agents only; it ships no in-app chat or assistant HTTP API.
