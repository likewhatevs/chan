# Phase 43 -- v0.53.0: leader presence, the self-managed devserver daemon, and terminal scrollback resume

Branch `v0.53.0`, cut from post-v0.52.0 GA `main`. An EXECUTION round -- the first feature round since the v0.52.0 hygiene sweep -- run by a six-member delivery team: a lead plus five file-local worker lanes building concurrently. Unlike v0.52.0 this round IS behavior change: eight asks landed as atomic, gate-green commits, with a light design gate in front of the three hard lanes; six rc2 bugs rolled forward from the v0.52.0-rc2 investigation, each absorbed by the lane owning its files; and a context-dependent terminology sweep. The dev host can build neither the Tauri/GTK desktop nor the cross-OS daemon, so those validate on the CI dry-run build plus the host's devices; local proofs covered the cross-platform Rust, the gateway workspace, and the web and marketing surfaces, gated green by an integrated `make pre-push`.

## Theme

Multi-client collaboration and a hardened devserver. The session gains a leader/followers model with `cs session`; the `chan devserver --service` path becomes a real self-managed cross-OS daemon; the terminal resumes scrollback from a client cache instead of full-replaying on reload; the editor persists the caret and links local files; and the chan-desktop launcher is regrouped into a Library tree with correct spinners.

## Design gates

The three heavy lanes shipped a design/analysis doc first; the lead consolidated them into one host survey and the host approved all recommended defaults. Load-bearing rulings: the terminal protocol builds **option A** (a SerializeAddon snapshot base plus a generation-guarded contiguous byte-delta) and DECLINES the independently-addressable-blocks reading (that naive form was shipped and reverted before -- it corrupts the parser across chunk boundaries); the `chan` daemon runs FOREGROUND on every OS with no detached walk-away; `--stop` / bind-port-mismatch is `--force`-only with no interactive prompt; presence defers the `cs window new` from-a-devserver stretch.

## What landed (by lane)

### Service -- `chan devserver --service` daemon (ask 1) + the close handoff

- `--service` becomes an enum `none|chan|systemd|launchd` (`none` = per-OS pick). The `chan` backend runs a single-instance FOREGROUND daemon on every OS (`8875684b`), fixing the Windows fork-instead-of-foreground bug and deleting `devserver_windows.rs`; the pidfile + flock + stale-takeover generalize from a new cross-OS `daemon_lock` primitive (`3a30ee7e`). `--status` / `--stop` / `--restart` / `--force`, a `daemon.json` record superseding `service.json`, the `-v` path listing, the `$APPIMAGE` -> `current_exe` -> `~/.local/bin/chan` binary resolution (`9f9ac3c7`), and tunnel-under-`=chan` (`f7fac11e`). Reattach is a healthz watchdog for ALL backends -- the journalctl / launchd log-follow is dropped (`=chan` `4eea6791`; systemd/launchd unified `ac2c5606`). `chan close` / `chan workspace rm` hands off to chan-desktop (`b3e8ff14`, with Launcher's desktop arm).

### Presence -- leader/followers + `cs session` (ask 5)

- A new per-tenant session-presence registry keyed by `window_id` with a live/disconnecting/disconnected/gone lifecycle and a reaper (`2b196a77`, `37b460ab`): the first ws client is leader; when a leader goes "gone" the longest-connected live participant is auto-promoted. `cs session list` / `self --name=` / `handover` / `takeover` over a survey-style handover bus plus a leader-gated reply route (`ac5b8717`, `5f436d66`), and a downloads-style handover-prompt overlay (`20f04bf1`). `takeover --force` seizes a live leader.

### Terminal -- scrollback resume + `cs` threading (asks 6 + 8) + the black bar

- A per-PTY-life generation epoch on the session and the attach prelude gates the since-cursor (`16c25b08`); the client persists a SerializeAddon snapshot plus the last seq to localStorage and resumes via a since-delta on reload instead of a full ring replay, generation-guarded with a full-replay fallback (`ad3e49de`). `cs terminal list` traces window -> pane -> tab (`fbe140ba` server side; the lead-owned CLI columns in `9785b67c`). The xterm-viewport black bar is painted out (`adec0705`).

### Editor -- caret persist + empty-discard + inline links (asks 2/3/4)

- A per-file caret index persists across a clean reload, PARKS at the top while a large file streams in, and is dropped when the file disappears (`520aef0b`), built on a caret-command reset so an explicit open lands at row 1 (`65d6292b`). An empty editable file auto-discards on close (`5f98eddc`). An inline code span that resolves to a real workspace file renders a clickable link bubble (detect + open; the in-place fs-autocomplete edit is a follow-up) (`aa665369`). The URL-slot link autocomplete offers the link itself first (`b7a98688`).

### Launcher -- launcher redesign + spinner reconcile (ask 7)

- The launcher regroups into a "Library" tree (LOCAL plus per-devserver groups, per-row controls, host-label click-to-copy lowercased; `dfa0d444`); the spinner reconcile resyncs against the authoritative server status on a feed-socket drop and on launcher re-show (`d6990391`); the control-terminal EYE slow-flashes yellow on inner-process exit (`7ba6a66a`). Desktop startup restores only the actually-mounted set, and the disconnect-overlay Abandon plus the cmd+backtick passthrough land (`5175b66b`); the bug-2 desktop CloseWorkspace arm rides here (`7ac14aec`).

### Lead -- the cs-wire seam, the integrated gate, the terminology sweep, the close

- The `cs session` wire variants + clap surface and the `cs terminal list` pane/tab columns (`ec083254`, `9785b67c`). The terminology sweep: `chan serve` -> `chan open` (local) / `chan devserver` (tunnel), a three-way context-dependent rewrite across docs, code comments, and user-facing strings (`970d91ee`). The rc bump (`35b6a498`).

## Notes

- **Known limitation (backlogged).** A markdown link whose label carries balanced brackets (`` [[foo] bar](path) ``) renders as plain text -- an upstream `@lezer/markdown` greedy shortcut-ref that trips the no-nested-links rule, not editor code; the real fix is a link-parser fork. Workaround: escape the inner brackets.
- **Deferred to follow-ups.** The `cs window new`-from-a-devserver stretch (presence); the server ring-shrink "minimum scrollback" knob and the independently-addressable block protocol (terminal, the declined option C / option B); the inline-code change-via-fs-autocomplete affordance (the editor ask-4 second half); a light/dark switch on the survey overlay.
- **Adversarial review.** The five lanes' commits were reviewed for correctness, no-archaeology, and lane discipline. The daemon review caught a Unix `--stop` / takeover pid-reuse hazard (a `kill -9`-leaked pidfile plus a reused pid could signal an innocent process -- now flock-gated) and stale docs describing the deleted detached Windows backend (rewritten to the foreground model); both fixed before the cut.
- **Host-validated build.** The cross-OS `--service=chan` daemon (linux / macOS / windows start/status/stop/restart plus watchdog reattach, and systemd + launchd), the chan-desktop launcher / presence / terminal surfaces, and the macOS sign/notarize path are proven on the non-publishing `release.yml` dry-run build and the host's own devices -- the dev host has no GTK and cannot run the app.
