# backsystacean-6: tab-rename to env propagation memo

Owner: @@Backsystacean
Status: REVIEW (decision: option a, 2026-05-18 by Alex)

## Decision (Alex 2026-05-18)

* **Option (a): spawn-time-only contract.** UI title renames
  immediately. `$CHAN_TAB_NAME` inside the running shell stays at
  the inherited value until the user clicks Restart on the
  terminal (Restart spawns a fresh PTY with refreshed env; see
  `web/src/components/TerminalTab.svelte:383`).
* **Rename prompt + stale-env warning**: on rename commit with an
  active PTY session, the frontend prompts inline with `Restart
  now` and `Later (keep stale env)`. Restart re-spawns the PTY
  with refreshed env via the existing `restart()` flow. Picking
  Later leaves a small stale-env badge near the title until the
  user restarts. UI work tracked in
  [frontend-2](./frontend-2.md).
* **Documentation**: add a one-paragraph note to
  `crates/chan-drive/design.md` (or chan-server's docs, whichever
  fits) stating the spawn-time-only contract and the Restart-to-
  refresh-env mechanism. Honest, no surprises.
* **Out of scope this phase**: opt-in shell integration. If
  runtime refresh demand surfaces later, option (b) from the memo
  is the clean implementation path.

## Goal

Decide and document the mechanism for propagating a tab rename
into the embedded terminal's environment. [backsystacean-1](./backsystacean-1.md)
flagged this as a product decision after observing that POSIX
shells have no general way to mutate the environment of an
already-running process.

## Relevant links

* Request: [request.md](./request.md) - "When we change the tab
  name, the ENV does not change... how can we fix that?".
* Prior work: [backsystacean-1](./backsystacean-1.md) (set
  `CHAN_TAB_NAME` at PTY spawn).
* Design memo: [architect-2.md](./architect-2.md).

## Investigation prompts

Three plausible mechanisms; pick one or argue for a different
shape. Each line below is a starting question, not a verdict.

1. **OSC title escape sequences (no env change).** Chan can emit
   `\x1b]0;<name>\x07` to update the terminal title. Easy to
   implement; does not actually change `$CHAN_TAB_NAME` inside the
   running shell, so anything that reads the env on demand still
   sees the old name. Works for users who care about the title bar
   only.

2. **Shell integration script.** Chan ships a small `chan.sh`
   sourced by the user's bashrc / zshrc that reads from a fifo or
   from a sentinel env file when prompted. On tab rename, chan
   writes the new value; on each PROMPT_COMMAND, the shell re-reads
   it. Full env mutation, but requires user opt-in to source the
   script.

3. **Injected `export` command.** On tab rename, chan injects
   `export CHAN_TAB_NAME=<new>\n` into the PTY stdin. Works
   immediately, but pollutes the shell command history and runs
   in whatever process is currently in the foreground (probably
   the wrong one if a TUI is running).

4. **Status quo + accept the trade-off.** Tab rename updates the
   title via OSC and the next fresh PTY (Reload, new tab) picks
   up the new value. Document this in the help.

## Deliverable

A short memo in this task file with:

* Recommended mechanism (with reasoning).
* Trade-offs of the rejected mechanisms.
* Implementation sketch if mechanism (1), (3), or (4) is chosen.
* Open question for Alex if (2) is the right path (shell integration
  is opt-in and crosses the bash / zsh boundary).

## Out of scope

* Implementation work is gated on Alex's decision after reading the
  memo. Open a follow-up task once the path is picked.

## Acceptance criteria

* Memo drafted in this file.
* @@Architect reads and flags for Alex's decision.

## Tests

* Not applicable until implementation lands.

## Memo

Recommendation: keep `CHAN_TAB_NAME` as a spawn-time environment
variable, update Chan's own tab title immediately on rename, and do
not inject commands into the PTY. If Alex wants `$CHAN_TAB_NAME` to
change inside an already-running interactive shell, the correct
mechanism is an opt-in shell integration script. There is no general
PTY or POSIX API that lets the parent process mutate the environment
of an existing shell or foreground child after `exec`.

The honest product contract is:

* Fresh terminal, Reload, or any new PTY: `CHAN_TAB_NAME` is correct
  at spawn.
* Existing PTY without shell integration: the UI title can change,
  but `$CHAN_TAB_NAME` inside the shell remains the old inherited
  value.
* Existing PTY with shell integration: the shell can refresh an
  exported `CHAN_TAB_NAME` at prompt boundaries. Foreground programs
  still keep the environment they inherited when they were launched;
  they cannot be retroactively patched.

Rejected mechanism: injected `export CHAN_TAB_NAME=...`. It is
unsafe as a default because Chan cannot know whether the foreground
process is a shell, a REPL, `vim`, `ssh`, `codex`, or `claude`.
Writing an `export` line to PTY stdin would run in the wrong program,
pollute command history in normal shells, and can corrupt a TUI
session. This should not ship.

Rejected mechanism as a "fix": OSC title escapes. Sending
`ESC ] 0 ; name BEL` is fine as a cosmetic terminal-title update, but
it does not modify environment variables. It can be part of the UI
polish, not the answer to the env requirement.

Recommended implementation if shell integration is approved:

1. At PTY spawn, create a per-session state file under chan-managed
   runtime state, e.g. `<runtime>/terminal/<session-id>/env`.
   Write `CHAN_TAB_NAME=<shell-quoted-value>` and keep the path in a
   spawn env var such as `CHAN_TERMINAL_ENV_FILE`.
2. Ship `chan shell-integration` or a static `chan.sh` snippet for
   bash and zsh. When sourced, it installs a prompt hook
   (`PROMPT_COMMAND` for bash, `precmd` for zsh) that reads
   `CHAN_TERMINAL_ENV_FILE` if present and exports supported keys.
3. On tab rename, the server updates the state file atomically.
   The next shell prompt picks up the new value. No bytes are written
   into the user's PTY input stream.
4. Keep the key allowlist tight at first: only `CHAN_TAB_NAME`.
   Do not create a generic "write arbitrary env" channel.
5. Document the limit clearly: already-running child commands do not
   see the update until they exit and a new command is launched from
   the refreshed shell.

Open question for Alex: is runtime `$CHAN_TAB_NAME` important enough
to justify opt-in shell integration, or is spawn-time env plus UI
rename enough for now? My recommendation is to avoid implementing a
partial invisible workaround. Choose either "document spawn-time only"
or "ship explicit shell integration"; do not inject commands.

## Progress notes

* Confirmed the current implementation sets `CHAN_TAB_NAME` at PTY
  spawn only.
* Drafted the decision memo above.

## Completion notes

Ready for @@Architect / Alex decision. No implementation or tests
apply until the product path is chosen.
