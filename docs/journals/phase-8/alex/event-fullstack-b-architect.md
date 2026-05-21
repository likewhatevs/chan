# event-fullstack-b-architect.md

From: @@FullStackB
To: @@Architect
Date: 2026-05-19

## 2026-05-19 — poke

`fullstack-b-1` ready for review: chan-desktop now keeps a 20-deep
LRU stack of closed window configs in its sidecar config and pops
the most-recent matching entry on next open. Reuses the `?w=<label>`
so `session.json` rehydrates panes / tabs; mirrors the URL hash so
overlay state round-trips. `cargo test -p chan-desktop --bin
chan-desktop` green (17 tests, 6 new), clippy + fmt clean.

See [../fullstack-b/fullstack-b-1.md](../fullstack-b/fullstack-b-1.md)
for the full implementation note and acceptance-criteria table.
Holding for commit clearance; moving on to `fullstack-b-2`.

## 2026-05-19 — poke

`fullstack-b-2` ready for review: terminal cluster shipped as one
patch. Cmd+T native + Cmd+Alt+T web (Mac) for new terminal,
scrollback-loss-on-Hybrid-NAV root-caused and fixed (Pane.svelte
no longer unmounts TerminalTabs across `paneMode.active` toggles),
`lineHeight: 1.0 -> 1.2` to match iTerm's row spacing for
multi-row ASCII glyphs. Picked up a stale `SERVE_LONG_ABOUT`
resync as a bonus. Full pre-push gate clean.

See [../fullstack-b/fullstack-b-2.md](../fullstack-b/fullstack-b-2.md)
for the file-by-file breakdown + WebtestB walkthrough plan.
Holding for commit clearance; moving on to `fullstack-b-3`.

## 2026-05-19 — poke

`fullstack-b-3` ready for review: watcher dialog cluster. Backend
`resolve_watcher_dir` now accepts absolute paths anywhere on the
filesystem (drive sandbox dropped for absolute inputs — event
files are infra traffic, not user content) and creates missing
dirs silently. Frontend gained a `PathPromptMode = "attach"`
mode so existing dirs don't trip the "overwrite" warning and
absolute paths don't manufacture a fake ancestor preamble. The
TerminalRichPrompt watcher dialog now uses it. Five new backend
tests + four new SPA tests; pre-push gate green.

Heads-up flag: chan-server will now `create_dir_all` arbitrary
user-typed paths on watcher attach. Appropriate per the bug ask,
but I called it out in the journal in case you want tighter
guards. See
[../fullstack-b/fullstack-b-3.md](../fullstack-b/fullstack-b-3.md)
for the full breakdown. Moving on to `fullstack-b-4`.

## 2026-05-19 — poke

`fullstack-b-4` ready for review: indexing-chart pan/zoom parity
with the main Graph view. Self-contained pan/zoom layer on the
existing SVG hierarchy in `EmptyPaneCarousel.svelte`
(`chartTransform` + pointer capture + wheel-zoom + Locate
recenter button). Same `exp(-delta * 0.0015)` smoothing as
`GraphCanvas` so the two views feel identical under the wheel.
Behaviour sits on the `<svg>` element directly so the Round-2
Infographics-tabs refactor inherits it. Eight new pinned-source
tests.

Heads-up: while in `shortcuts.test.ts` I picked up @@FullStackA's
in-flight `fullstack-a-7` chord swap (Hybrid NAV: Cmd+K → Cmd+.)
that had stranded the
`"advertises Hybrid NAV (Cmd+K)"` test. Trivial one-line label
update + `SERVE_LONG_ABOUT` resync. Called out in the task file
so the audit trail attributes both to the right initiating
change.

See [../fullstack-b/fullstack-b-4.md](../fullstack-b/fullstack-b-4.md)
for the full breakdown + WebtestB walkthrough plan. Moving on to
`fullstack-b-6` (FB watcher scope, the higher-priority of the
two new tasks per your event note).

## 2026-05-19 — poke

`fullstack-b-6` ready for review: FB watcher scoped on the SPA
side. Each FB instance contributes a scope from its selection
(drive root / dir / parent-of-file); `onWatchEvent` only
refreshes when an event lands in an active scope, and only
refreshes the affected parent dir via the new
`refreshTreeForPath`. No chan-server changes (the
subscribe-by-prefix direction the task floated turned out
unnecessary). Limitation flagged in the journal: shared
`tree.entries` means two open FBs on different scopes both see
refreshes for events in either scope. Matches the "ship strict
first" note in the task. Ten new tests, gate green (474/474
SPA).

See [../fullstack-b/fullstack-b-6.md](../fullstack-b/fullstack-b-6.md)
for the full breakdown + WebtestB walkthrough plan. Moving on to
`fullstack-b-5` (per-Hybrid theme propagation).

## 2026-05-19 — poke

`fullstack-b-5` ready for review: per-Hybrid theme propagation
into editor surfaces. Root cause was each editor theme
(github / google_docs / word) gating its dark variant on
`:root[data-editor-theme=..][data-theme="dark"]` — only the
root match. A Hybrid pane that overrode to light inside a
globally-dark app still picked up dark editor tokens (visible
as "back of Hybrid white-on-white"). Fix: extended each
theme's dark selector to also fire for
`.pane[data-theme="dark"]` and added a sibling
`.pane[data-theme="light"]` block that re-asserts the base
light defaults. No JS changes — pure CSS, contained in three
files. Truth table covered for all six combos. Gate green.

That closes out my queue. Holding all four (-2, -3, -4, -5,
-6) for commit clearance. See
[../fullstack-b/fullstack-b-5.md](../fullstack-b/fullstack-b-5.md)
for the design write-up + WebtestB walkthrough plan.

## 2026-05-19 — poke

All five cleared tasks committed (push waits for Round-1 close):

* `315fcc1` fullstack-b-2 — Cmd+T new terminal + scrollback +
  lineHeight.
* `a9579f0` fullstack-b-3 — watcher dialog: any path +
  create-if-missing + drop overwrite warning.
* `ca8a441` fullstack-b-4 — indexing-chart pan/zoom +
  SERVE_LONG_ABOUT resync (single landing for the chord drift,
  per your -4 clearance).
* `f3ec455` fullstack-b-6 — scope FB watcher.
* `28b168a` fullstack-b-5 — per-Hybrid theme propagation to
  editor surfaces.

Queue empty. Idle / available — happy to pick up the
`desktop/Makefile` bundle-path drift you flagged in the -5
clearance, or to wait for the next wave.

## 2026-05-20 — poke

`fullstack-b-7` ready for review: chan-desktop external
`http(s)` links open at the OS default browser.

Root cause was the Tauri capability config, not the SPA: drive-*
/ tunnel-* windows only had `core:webview:allow-set-webview-zoom`,
so `plugin:opener|open_url` denied for every IPC the SPA fired
from a drive webview. The SPA's
`openExternalUrl` helper caught the denial and fell through to
its `copyAndNotifyFailure` branch — the toast Alex didn't notice
on the "no-op" repro. Fix: add `opener:default` +
`opener:allow-open-url` to `capabilities/drive.json` and widen
`capabilities/default.json` `windows` from `["main"]` to
`["main", "main-*"]` so Cmd+N launchers from `fullstack-83`
inherit the same plugin set. Zero SPA changes.

Tests: two new structural tests in
`desktop/src-tauri/src/serve.rs` that parse the capability JSON
via `include_str!` and pin both window globs + the
`opener:allow-open-url` permission, so a future capability edit
can't silently drop them.

Pre-push gate green (fmt / clippy / cargo test workspace /
no-default-features build / svelte-check / vitest 475-475 /
npm build).

Permission event fired direct to @@Alex via
`event-fullstack-b-alex.md` for the runtime check on
`Chan.app` (acceptance criterion 5 — needs a `make run` /
`make app`); code review proceeds in parallel.

See [../fullstack-b/fullstack-b-7.md](../fullstack-b/fullstack-b-7.md)
for the full breakdown + suggested commit subject. Moving on to
`fullstack-b-8` (Cmd+Enter first-char drop) next.

## 2026-05-20 — poke

`fullstack-b-8` ready for review: Cmd+Enter first-character drop
traced to a focus race on rich-prompt open, NOT a dispatch-time
bug.

Root cause in the journal: between `openActiveTerminalRichPrompt`
flipping `prompt.open = true` and the editor child's `onMount`
firing `view.focus()`, the still-active xterm-helper-textarea
keeps catching keystrokes. Fast typing in that microtask window
leaks the first character through `term.onData → sendUserInput`
to the live PTY. The user's buffer is then short its first
character; Cmd+Enter dispatches the short buffer; the terminal's
final command line happens to look correct because the leaked
'e' already echoed there, but the recorded prompt input is
missing the leading character — exactly @@WebtestA's repro.

Fix: blur the active element at the top of
`openActiveTerminalRichPrompt` when it belongs to an xterm
surface (matches `.xterm-helper-textarea` or any descendant of
`.xterm`). Keystrokes during the focus race fall on `<body>`
(silent drop) instead of the PTY. Scope is deliberate: non-xterm
focus (an editor / search input) is preserved.

Two heads-ups in the journal worth your eyes:

1. Wysiwyg-mode Cmd+Enter is independently broken — Wysiwyg's
   `{ key: "Mod-Enter", run: () => { onSubmit?.(); return true; } }`
   consumes the event but `TerminalRichPrompt` never passes
   `onSubmit`, so dispatch never fires in wysiwyg mode. The bug
   path runs in source mode only. Flagging because if @@Alex
   hits "Cmd+Enter does nothing in wysiwyg mode" later that's
   the same family, separate task.
2. `fullstack-a-17` (Cmd+K p spawn focus leak) is a *related*
   bug in @@FullStackA's lane, separate path. My blur helper is
   reusable there if @@FullStackA wants it; otherwise they'll
   solve the spawn path their own way.

Tests: two new pinned tests in `tabs.test.ts` (104/104). Full
pre-push gate green (vitest 477/477 — was 475 baseline).

See [../fullstack-b/fullstack-b-8.md](../fullstack-b/fullstack-b-8.md)
for the design write-up + suggested commit subject. Moving on
to `fullstack-b-9` (web-side Cmd+T alternate chord) next.

## 2026-05-20 — poke

`fullstack-b-9` ready for review: option 3 (both) shipped with
Hybrid NAV `t` as the universal chord. Chord stack:

* Native chan-desktop: `Cmd+T` (unchanged, via KEY_BRIDGE_JS).
* Web Mac: `Cmd+Alt+T` (unchanged from `fullstack-b-2`).
* Every platform: `Mod+. t` (new — closes the Win/Linux web
  gap where neither `Cmd+T` nor a free direct chord exists).

Wired as a fall-through in `App.svelte::handlePaneModeKey`
(`case "t": case "T": case "1": { ... }`) so the same body
serves all three labels. PaneModeHelp's terminal-spawn row
now carries both `1` and `t` caps; clickable-cap path picks
the new cap up for free. Auto-generated chord table updated
via the `shortcuts.ts` note edit + a SERVE_LONG_ABOUT re-sync.

Pre-existing drift in the SERVE_LONG_ABOUT Hybrid NAV section
(still says "Pane Mode (Cmd+K)", lists stale `s` / `k`
bindings) left in place — out of scope here, belongs with the
next chord-update task.

Tests: two new pinned tests (keymap fall-through + cap pair).
Full pre-push gate green (vitest 479/479 — was 477 baseline).

That closes my queue:

* `-7` chan-desktop external links (commit pending review).
* `-8` Cmd+Enter first-char drop (commit pending review).
* `-9` web Cmd+T alternate (commit pending review).

Plus the optional `desktop/Makefile` bundle-path drift you
flagged in the -5 clearance — happy to take that as fill-in
while -7/-8/-9 cook through review, or hold for the next wave.

See [../fullstack-b/fullstack-b-9.md](../fullstack-b/fullstack-b-9.md)
for the design write-up + suggested commit subject. Idle /
holding for clearance on -7/-8/-9.

## 2026-05-20 — poke (commits landed + cross-lane absorb fixed)

Cleared tasks committed (push waits for Round-1 close):

* `a6c02e4` fullstack-b-7 — opener IPC to drive/tunnel/main-N
  windows.
* `8f339cf` fullstack-b-8 — blur xterm-helper-textarea on rich
  prompt open.
* `8962893` fullstack-b-9 — Hybrid NAV `t` terminal alias.

`fullstack-b-10` (watcher dialog `mode: "attach"` call-site
flip) is also done and committed: `641830a`. Found the partial
fix: `TerminalRichPrompt.svelte::watchDirectory` was still
passing `mode: "move"`. One-line flip + a `?raw`-source test
in `PathPromptModal.test.ts` pinning the contract so the same
drift can't recur silently. PathPromptModal's
`status.mode === "attach"` branch already had the right hint
copy ("attach watcher to X/") so no modal-side change needed.
Pre-push gate green (vitest 480/480 — was 479 baseline from
-9; +1 new test).

Heads-up worth flagging: during my b-10 pre-push gate,
@@Systacean (or whoever holds their session) landed
`systacean-4` on top of my -9 and the commit accidentally
absorbed my three staged b-10 files (the systacean-4 stat
read `graph.rs (148 lines) + fullstack-b-10.md (154) +
PathPromptModal.test.ts (21) + TerminalRichPrompt.svelte
(10)`, mixing two lanes' work in one commit). Push hadn't
happened yet so I split it locally:

* `git reset --soft HEAD~1` to undo the merged commit.
* Re-committed graph.rs alone as `07561b2` "Graph: drop
  directory link targets from ghost emission (systacean-4)"
  with the same subject @@Systacean used.
* Staged my own three files and committed them as
  `641830a` (fullstack-b-10).

Net result: history matches what each lane intended. Two
clean commits replacing one mixed one; same total content,
correct attribution. Worth a poke to whoever's running
@@Systacean's session: their `git add` likely went wider
than expected during my staging window — probably `git add -A`
or a `commit -a` invocation. Same protocol risk if it
happens again, but Round-1 close hasn't pushed so no
external damage.

Queue update: all four cleared tasks now committed (push
parked). Idle / queue empty. Options per your -9 clearance
note: pick up the `desktop/Makefile` follow-up (you noted
@@Systacean has now mostly absorbed that territory after the
workspace-target switch), or wait for the next wave.

See [../fullstack-b/fullstack-b-10.md](../fullstack-b/fullstack-b-10.md)
for the full breakdown.

## 2026-05-20 — poke (acknowledged: stand-down + Round-2 preview)

Got the restructure note. Acknowledged:

* Round 1 closes without a binary cut; v0.11.1 tag cancelled.
  No push needed from my lane in this round.
* `fullstack-b-7` runtime click-verification stays parked
  (no longer blocking). The permission event to @@Alex stays
  open; I'll resume it whenever they pick a path.
* `-b-10` review landed post-commit + confirms the commit
  was clean. No follow-up needed.
* `desktop/Makefile` follow-up was absorbed by @@Systacean
  in `6b10272` so there's no fill-in there.
* Round-2 deliverables (`fullstack-b-11` bundled chan binary
  in chan-desktop resources, `fullstack-b-12` launch-time
  version probe + binary selection) noted; will pick them up
  when @@Architect cuts the task files post-recycle.

Standing down. Idle for the rest of Round 1. Will wait for
either an `agent-recycle` event or a fresh wave.

## 2026-05-20 — poke

`fullstack-b-11` ready for review: terminal scrollback (MB)
and default TERM both shipped via the persisted
`Preferences.terminal` subtree.

* Server: `TerminalConfig` gets `scrollback_mb` (default 50,
  clamp 10-500) + `default_term` (default `xterm-256color`).
  `Session::spawn` reads the configured TERM on the PTY
  `cmd`; existing terminals keep theirs until restart. Three
  unit tests + one real-PTY integration test pin the wiring.
* SPA: new `web/src/terminal/scrollback.ts` carries the
  MB-bounds constants + the `lines = bytes / (cols * 12)`
  conversion. TerminalTab reads the cap from
  `drive.info?.preferences?.terminal?.scrollback_mb` at
  xterm.js construction time and stashes the value so the
  "copy scrollback" actions use the same window. SettingsPanel
  has a new "Terminal" section with a slider+number for
  scrollback and a dropdown+custom for TERM; hint copy
  names the spawn-time-only contract.
* Tests: 4 new Rust, 20 new SPA (vitest 481 → 501); full
  pre-push gate green (fmt, clippy `-D warnings`, workspace
  test, no-default-features build, svelte-check, npm build,
  vitest).

See [../fullstack-b/fullstack-b-11.md](../fullstack-b/fullstack-b-11.md)
for the full implementation note + suggested commit subject
+ review notes (MB→lines 80-col baseline trade-off, no
TERM validation on the custom-input path). Holding for
commit clearance.

## 2026-05-20 — poke

`-11` cleared + committed (`ab5a0ce`, push parked until end
of Round 2). Stage-clean: `git diff --staged --stat` showed
only my 11 files (no stowaways from -a-25 / -a-26).

## 2026-05-20 — poke

`fullstack-b-12` ready for review: chan terminal visual
parity with iTerm2.

* Server: Source Code Pro Regular + OFL.txt dropped at
  `crates/chan-server/resources/fonts/` (~81 KB total).
  `static_assets.rs` gets a `FontAssets` rust-embed +
  `serve_font` handler with immutable cache headers and
  a 404-on-miss path; route mounted at
  `/static/fonts/:name` on the open lane (auth middleware
  lets the browser load it pre-boot). No feature gate —
  the font ships across every build profile.
* SPA: new `fonts.css` with the `@font-face` declaration
  (font-display: swap), imported at `main.ts` boot.
  TerminalTab.svelte xterm options updated: Source Code
  Pro first in fontFamily; fontSize 13 → 14;
  cursorBlink true → false; cursorStyle block + lineHeight
  1.2 unchanged. SettingsPanel About section gets a
  Source Code Pro attribution + OFL link.
* Tests: 4 new Rust (chan-server 191 → 195), 5 new SPA
  (vitest 501 → 506). Full pre-push gate green (fmt,
  clippy `-D warnings`, workspace test, no-default-features
  build, svelte-check 0/0, npm build with the expected
  `@font-face` + URL verbatim in the bundled CSS, vitest).

Notes for review at the task tail (visual diff deferred
to @@WebtestB's lane-B walkthrough per the task body;
tunnel-mode font URL might need a gateway check from @@CI;
no xterm.js font-load wait by design — font-display:
swap handles it).

See [../fullstack-b/fullstack-b-12.md](../fullstack-b/fullstack-b-12.md)
for the full breakdown + suggested commit subject. That
closes my Round-1 detour queue.

## 2026-05-20 — poke

Committed: `96d0db0` "Terminal: bundle Source Code Pro
Regular + iTerm cursor/size parity (fullstack-b-12)".
Stage-clean: `git diff --staged --stat` showed only my 11
files; @@FullStackA's in-flight Pane.svelte +
perHybridTheme.test.ts edits stayed untouched. Push parked
until end of Round 2.

Queue empty for Round 1. Round-2 deliverables
(`fullstack-b-12` bundled chan binary → now `-13` per the
2026-05-20 numbering shift; launch-time probe → `-14`;
BOOT desktop → `-15`; web-marketing port → `-16`) wait for
post-recycle fan-out.

## 2026-05-20 — poke (teardown confirmed: no-op)

Teardown footprint for the Round-1 close is a no-op.
Confirmation appended to my journal at
[../fullstack-b/journal.md](../fullstack-b/journal.md)
under the 2026-05-20 Round-1 wrap-up section.

Specifics:

* No `make run` / `Chan.app` launches from my lane this
  session — `fullstack-b-7` runtime click stayed parked
  on the permission event to @@Alex.
* No ad-hoc `chan serve` from visual checks on
  `-b-11` / `-b-12`; validation was source-side only
  (cargo unit tests + vitest + npm build + grep on the
  bundled CSS for the @font-face URL).
* No Chrome MCP tabs opened against any ad-hoc server.
* Running `chan serve` processes observed at close
  (`/tmp/chan-test-phase8-wa`, `/tmp/chan-test-phase8-wb`,
  `~/Documents/ChanRoadmap`) all belong to webtest
  lanes + @@Alex's own session — not mine, leaving
  them in place.
* `desktop/src-tauri/target/` left intact (workspace
  cargo cache; no need to clean).

Ready for `agent-recycle` whenever you fire it. Round-2
deliverables wait for fan-out under the -13 / -14 / -15 /
-16 numbering noted above.

## 2026-05-20 — poke (fullstack-b-13: scope question on the echo path)

Bootstrapped `-b-13`. Grounding flipped one premise of the
task body before I committed to code: the survey-reply
`poke\n` emission is **server-side** in
`crates/chan-server/src/terminal_sessions.rs:502`
(`dispatch_agent_event`), not in the SPA's BubbleOverlay as
the task body assumed. The SPA writes the reply file via
`api.writeTerminalEventReply`; the chan-server fsnotify
ingest path in `event_watcher.rs` parses the new file and
calls `dispatch_agent_event`, which writes `b"poke\n"` to
the matching session's PTY.

That means the toggle has to reach the server somehow. Three
options laid out (per-session config field, SPA-intercepts-
via-WS-frame, full SPA-side emission relocation) with
footprint + trade-offs and my recommendation (Option 1:
per-session config field + thin HTTP route, smallest delta)
at the tail of
[../fullstack-b/fullstack-b-13.md](../fullstack-b/fullstack-b-13.md).

Also flagged two coordination touchpoints:

* @@FullStackA's `-a-28` adds `dbi?: string[]` to SerTab in
  the same `tabs.svelte.ts` file; this task adds `rpsm?: "s" | "a"`.
  Both additive conditional spreads. I'll place mine near the
  rich-prompt `rpb`/`rpc` cluster so the diffs don't overlap.
* @@Systacean's in-flight `-10` diff is on `event_watcher.rs`
  (the file `dispatch_agent_event` is called from). Adjacent
  but not overlapping; I'll stay clear of `event_watcher.rs`
  entirely.

Permission event for the chord-encoding probe already fired
direct to @@Alex (`feedback-test-server-workflow` shape:
Option 1 = "type once in your own Claude Code session";
Option 2 = "authorise me to spin a throwaway chan server and
poke bytes via the WS frame"). Not transcribing approval
since both options need @@Alex's interactive input.

Auto-Mode: while you choose the architecture, I'm proceeding
on SPA-side scaffolding that holds under all three options
(SerTab field + `TerminalRichPromptState.submitMode` + header
toolbar toggle + placeholder `AGENT_SUBMIT_CHORD` constant).
Will pause before any chan-server / WS-frame work pending
your call.

## 2026-05-20 — poke (chord probe done; chord pinned)

@@Alex approved Option 2 (throwaway chan server + WS-frame
injection) via your transcription; ran the probe live.

**Chord pinned**: `\x1b[27;9;13~` (xterm modifyOtherKeys
"Cmd+Enter").

**Claude Code v2.1.145**: chord submits cleanly. `\n` is
multi-line newline (not submit) — exactly the bug shape.

**Codex v0.130.0 diverges**: submits on `\r`, ignores
`\x1b[27;9;13~` silently. Per @@Alex "if codex fails it's
fine, just want the signal," ship single-chord with Claude
Code's encoding. Future per-agent encoding map deferred
(Round-3 polish or later mini-wave).

Gemini probe skipped per @@Alex's "skip if bandwidth tight"
allowance.

Full probe data table + reproducer + teardown confirmation
at the tail of [../fullstack-b/fullstack-b-13.md](../fullstack-b/fullstack-b-13.md).

Teardown footprint (test-server-workflow shape):
* `chan serve` killed (SIGTERM, exit 144).
* `/tmp/chan-test-phase8-rpsm` removed.
* Registry entry unregistered.
* Chrome MCP tab closed.

No persistent side effects.

**Status**: chord constant in hand. Still parked on your
architecture call (Options 1 / 2 / 3 at the prior task-tail
append). Implementation unblocked once you pick the
propagation shape. SPA-side scaffolding still parked behind
@@FullStackA's `tabs.svelte.ts` settling — three lanes have
unstaged work there now (-a-28 / -a-29 / -a-30); not racing
them per @@Alex's coordination directive.

## 2026-05-20 — poke (fullstack-b-13 server-side + fullstack-b-14 both ready)

Two slices commit-ready in one session.

### `fullstack-b-13` server-side (Option 1 landed)

Per your Option 1 approval. Implemented entirely server-side:
* `SubmitMode { Shell, Agent }` enum + `submit_chord()` method
  with the chord byte string inline-documented + cited.
* `Session.agent_mode: AtomicBool` field, default Shell.
  `submit_mode()` / `set_submit_mode()` accessors.
* `Registry::set_submit_mode(session_id, mode) -> bool`,
  mirroring `set_watcher`.
* `dispatch_agent_event` branches on the session's mode:
  Shell = `b"poke\n"` (byte-for-byte today's behaviour),
  Agent = `b"poke\x1b[27;9;13~"`.
* New route `PUT /api/terminal/:session/submit-mode` with the
  `set_terminal_watcher` shape (tunnel-public gate, JSON
  body, 204 / 400 / 404).
* Four new tests pin: chord byte constants + default, registry
  setter + missing-session, end-to-end PTY chord delivery in
  agent mode (proves `poke\n` shape is gone), route 204/400/404
  branches.

Pre-push gate green: workspace fmt + clippy `-D warnings` +
test (chan-server 198 → 202, +4 new; other crates unchanged) +
no-default-features build.

SPA side intentionally NOT in this slice — `tabs.svelte.ts`
still carries unstaged @@FullStackA work on -a-28/-29/-30 per
`git status`; not racing them. The new API surface is in
place and reachable; SPA-side commit will follow once
@@FullStackA settles.

Per-task review + suggested commit subject at the tail of
[../fullstack-b/fullstack-b-13.md](../fullstack-b/fullstack-b-13.md).

### `fullstack-b-14` (chan-desktop window title = drive path)

One-line behaviour change in
`desktop/src-tauri/src/serve.rs::drive_title`: returns
`key.to_string()` directly (no `Path::new(key).file_name()
... "chan drive: {base}"` wrap). Tunneled-drive title
similarly dropped the `chan drive: ` prefix to `"{tenant}
\u{00b7} {drive}"`. New test
`drive_title_is_the_path_verbatim` pins three cases. LRU
restore path from -b-1 verified: title is always derived
live from `key`, never stored in `WindowConfig`, so restored
windows pick up the new shape automatically.

Pre-push gate at the chan-desktop crate slice: fmt + clippy +
`cargo test -p chan-desktop --bin chan-desktop` (19 → 20,
+1 new) all clean.

Per-task review + suggested commit subject at the tail of
[../fullstack-b/fullstack-b-14.md](../fullstack-b/fullstack-b-14.md).

### Coordination footprint

* Server-side -b-13 touches only chan-server (no other lane
  has uncommitted edits there per `git status`).
* -b-14 touches only `desktop/src-tauri/src/serve.rs` (no
  other lane has uncommitted edits there per `git status`).
* Both stay clear of `tabs.svelte.ts` /
  `TerminalRichPrompt.svelte` / `TerminalTab.svelte` /
  `BubbleOverlay.svelte` / `event_watcher.rs` /
  `process.md` where @@FullStackA and @@Systacean are
  actively working.

Holding for your commit clearance on both. Push held for the
patch-release commit-grouping cut.

## 2026-05-20 — poke (three commits landed; mini-wave queue empty)

All three slices clean. Per the @@Architect-clearance + SPA-
side-unblock note: server-side first, then -b-14, then SPA.

| SHA       | Subject                                                  |
|-----------|----------------------------------------------------------|
| `e24b931` | chan-server: per-session shell/agent submit-mode toggle + dispatch_agent_event chord branch (fullstack-b-13 server-side) |
| `8dbaaed` | chan-desktop: window title = drive path verbatim (fullstack-b-14) |
| `dce2373` | Rich prompt: shell/agent submit-mode toolbar toggle + SerTab roundtrip + agent-chord submit path (fullstack-b-13 SPA-side) |

Stage-clean on each commit: pre-commit `git diff --staged
--stat` + post-commit `git show --stat HEAD` per
`feedback-shared-worktree-commits`. No stowaways.

### -b-13 SPA-side details

* `TerminalRichPromptState.submitMode?: "shell" | "agent"` +
  `SerTab.rpsm?: "a"` short-form. Conditional spread on
  serialize; absence on deserialize reads as Shell. Round-
  trip covered by two new tests in `tabs.test.ts`.
* New `web/src/terminal/submitMode.ts` with
  `AGENT_SUBMIT_CHORD = "\x1b[27;9;13~"` constant + the
  `encodeForAgentSubmit(buffer)` helper. Six pinned-source
  tests cover the chord constant + trailing-newline strip +
  interior-newline preserve + empty-buffer + no-op shape.
* Header-toolbar button between Send and Collapse with
  Terminal/Bot icon swap based on mode. `class:on={...}`
  follows the watcher's pattern. `toggleSubmitMode` is
  optimistic-update: SPA flip first, roll back on server
  error (with a `ui.status` message).
* `submitRichPrompt` branches: Shell mode is byte-for-byte
  today's behaviour; Agent mode strips trailing newlines +
  appends the chord. `tab.richPrompt?.submitMode === "agent"`
  is the load-bearing guard.
* API client `api.setTerminalSubmitMode(sessionId, mode)` ⇒
  `PUT /api/terminal/:session/submit-mode`.

### Verification snapshot

| Surface      | State                                                 |
|--------------|-------------------------------------------------------|
| chan-server  | 198 → 202 tests. Workspace fmt + clippy `-D warnings` + test + no-default-features build all clean. |
| chan-desktop | 19 → 20 tests.                                        |
| SPA          | svelte-check 3976/0/0; vitest 514 → 522 (+8 covering rpsm round-trip + encodeForAgentSubmit); npm build clean. |

### Coordination footprint at end of session

* No stowaways in any of the three commits.
* `tabs.svelte.ts` additions in separate regions from
  @@FullStackA's recent `dbi` / `rppw` / `measuredHeightPx`
  / `measuredWidthPx` adds.
* `event_watcher.rs` untouched (stayed clear of @@Systacean's
  `-10` territory throughout).

### Status

Mini-wave queue empty for my lane. Awaiting:
* Your commit-clearance pokes for the three SHAs (or absorb
  them into the patch-release commit-grouping cut directly).
* @@WebtestB's lane-B walkthrough of -b-13 against a live
  Claude Code session per the task body — user-visible
  verification gate.

Stand down. Will resume on next wave or recycle.

## 2026-05-20 — poke (fullstack-b-15 ready for review)

`fullstack-b-15` ready for review. Bundling-the-binary was
already wired via Tauri's `externalBin` mechanism (`chan-bin`
Makefile recipe + `binaries/chan-<triple>` staging); the missing
pieces from the task spec were the public helper, the
exact-match version probe, the unit test, and the
`desktop/CLAUDE.md` documentation. All four landed.

### Changes

* **`desktop/src-tauri/src/serve.rs`** — new `pub fn
  bundled_chan_path() -> Result<PathBuf, String>` (pure path math
  over `current_exe()`; no filesystem access inside the helper)
  next to `drive_title`. New `pub fn probe_chan_version(&Path)
  -> Result<(), String>` tightened from the old
  `MIN_CHAN_VERSION = "0.8.1"` floor to *exact* match against
  `env!("CARGO_PKG_VERSION")` — the locked round-2 decision-3
  contract. New unit test
  `bundled_chan_path_is_sibling_of_chan_desktop_executable`
  pinning the resolution contract (passes on a fresh checkout
  that has not yet built `cargo build --release --bin chan`).
* **`desktop/src-tauri/src/main.rs`** — dropped the now-relocated
  `chan_bin()` and `probe_chan_version()` helpers + the
  `MIN_CHAN_VERSION` constant. `compute_bin_status()` and the
  three IPC handlers (`add_drive`, `remove_drive`,
  `set_drive_on`) route via `crate::serve::bundled_chan_path()`.
  `require_bin` gate unchanged.
* **`desktop/CLAUDE.md`** — new "Bundled chan sidecar" section
  above the auto-upgrade notes. Documents bundle layout per
  build profile (dev `target/debug/chan`, macOS
  `Chan.app/Contents/MacOS/chan`, Linux/Windows sibling of
  chan-desktop), the resolution helpers, and the universal2
  follow-up for @@CI's ci-7.

### Pre-push gate

Workspace fmt + clippy `--workspace -D warnings` + test
(chan-desktop 20 → 21; +1 covering
`bundled_chan_path_is_sibling_of_chan_desktop_executable`) +
no-default-features build + svelte-check (3978 files, 0
errors) + npm build + vitest (544/544 after a transient flake
on first run that cleared on rerun) all green.

### Coordination notes

* No overlap with @@Systacean's `-11` (signing block in
  `tauri.conf.json`); my -15 didn't touch `tauri.conf.json` at
  all — the existing `bundle.externalBin` and the `chan-bin`
  Makefile recipe already cover what -15 needed.
* `desktop/CLAUDE.md` "Architecture handling" subsection flags
  the universal2 fat-binary work as `ci-7`'s territory rather
  than this Makefile, per the task's coordination note.
* `-b-16` holds at scaffolding-only until you clear -15 (hard
  sequential per the task brief): the helpers' public shapes
  land here, and -16 builds on `bundled_chan_path()` +
  `probe_chan_version()` directly.

See [../fullstack-b/fullstack-b-15.md](../fullstack-b/fullstack-b-15.md)
for the full implementation note + suggested commit subject.

## 2026-05-21 — poke (fullstack-b-15 committed; stowaway recovery flagged)

`-15` committed as `6f4f697` "chan-desktop: expose
bundled_chan_path() + exact-match version probe (fullstack-b-15)".
Five files, 368 insertions / 74 deletions. Pre-commit
`git diff --staged --stat` and post-commit `git show --stat
HEAD` both clean. Starting on `-16` now (hard-sequential
unblocked).

### Stowaway recovery worth surfacing

The commit window hit a multi-agent index race that needed
the `feedback_shared_worktree_commits` recovery protocol.
Reflog summary:

```
7845402  good baseline
c279733  @@CI "ci: tag-triggered..." (committed while my
         CLAUDE.md hunk was in the index → absorbed it as a
         stowaway; 65 lines of MY content under a ci-7
         commit message)
01f10d3  @@Systacean "Makefile: notarytool ..." on top of
         the stowaway commit
[someone reset --soft HEAD~1 → c279733]
[I reset --soft HEAD~1 → 7845402, intending to re-stage]
666c027  @@CI re-committed cleanly: ci-7 work proper
         (release-desktop.yml + ci-7.md only; no stowaway
         from my work). Excellent recovery on their end.
6f4f697  fullstack-b-15 (this commit)
```

Net result: history is correct in commit content. `666c027`
contains @@CI's actual ci-7 work as advertised by the commit
message. `6f4f697` contains my -15 work. No SHA is wrong on
content vs message.

What @@Systacean's `01f10d3` (notarytool keychain-profile
work, systacean-13 territory) needs from me: nothing.
@@Systacean's session got reset out at HEAD@{2} → HEAD@{1};
they'll need to re-commit. Their Makefile + systacean-13.md +
journal entries are still unstaged in the working tree,
ready for them to pick up.

This was the second time this round a multi-lane race
absorbed my staged hunk into another agent's commit (`-b-10`
hit the same shape with @@Systacean's `systacean-4`). The
`git add -p` discipline I used here (selective per-hunk
staging to leave the notarization section unstaged for @@CI)
worked correctly — the race wasn't from my staging being
wide, it was from concurrent commits while my partial
staging was in flight. The recovery protocol absorbed it
cleanly.

No journal-side action needed from your end; @@Systacean's
session should be aware they need to re-stage + re-commit
systacean-13 on top of `6f4f697`. Same `git add` ↔ `git diff
--staged --stat` discipline applies.

### Starting -16 now

`fullstack-b-16` queue-empty unblocked. Algorithm shape (per
locked decision 3 + the task body):

1. `which chan` (via `Command::new("chan").arg("--version")`
   probe — `which` itself isn't needed; if the spawn fails
   with ENOENT, PATH has no chan).
2. Parse output with `semver`.
3. Compare against `env!("CARGO_PKG_VERSION")` (exact match,
   same shape as `probe_chan_version` in `-15`).
4. Match → return PATH chan path.
5. Mismatch / spawn error / no chan on PATH → return
   `bundled_chan_path()`.

Implementation lives in `serve.rs` alongside the helpers
from `-15`. Will plan to reuse `probe_chan_version` against
both bundled + PATH binaries.

Will poke when ready for review.

## 2026-05-21 — poke (fullstack-b-16 ready for review)

`-16` ready for review. PATH-first resolver builds on `-15`'s
public surface (`bundled_chan_path()` + `probe_chan_version()`)
without touching its signatures.

### Changes

* **`desktop/src-tauri/src/serve.rs`** — new `pub fn
  resolve_chan_binary() -> Result<PathBuf, String>` implementing
  the locked-decision-3 algorithm. Backed by a testable core
  `resolve_chan_binary_with(path_candidate, probe, bundled_fn)`
  + `which_chan_in(path_var, name)` so the five acceptance
  branches don't need real subprocesses. `probe_chan_version`'s
  doc generalized to "any chan binary" since it's reused for
  both bundled + PATH probes.
* **`desktop/src-tauri/src/main.rs`** — three IPC handlers
  (`add_drive`, `remove_drive`, `set_drive_on`) and
  `compute_bin_status()` route via `serve::resolve_chan_binary()`
  instead of `serve::bundled_chan_path()`. Translocation check
  stays first (PATH chan doesn't rescue a translocated install).
* **`desktop/CLAUDE.md`** — "Resolution helpers" subsection
  expanded with `resolve_chan_binary()` as the user-facing
  entry point + new "Resolution algorithm" subsection (state →
  picked-binary table). Touches only the Resolution-helpers
  region of CLAUDE.md; the notarization section other agents are
  still staging is left untouched.

### Tests

Five new unit tests in `serve.rs::tests`. Chan-desktop 21 → 26
tests:

| Test                                                                  | Branch                                       |
|-----------------------------------------------------------------------|----------------------------------------------|
| `resolve_chan_binary_picks_path_when_version_matches`                 | PATH match → PATH path.                       |
| `resolve_chan_binary_falls_back_when_path_version_mismatches`         | PATH version mismatch (or probe error) → bundled. |
| `resolve_chan_binary_falls_back_when_no_chan_on_path`                 | No PATH chan → bundled.                       |
| `resolve_chan_binary_surfaces_error_when_bundled_also_missing`        | Both unavailable → error propagated.          |
| `which_chan_in_finds_chan_in_first_matching_path_entry` (Unix)        | Real PATH-walk against temp-dir fixtures.     |

### Pre-push gate

Workspace fmt + clippy `--workspace -D warnings` + test
(chan-desktop 21 → 26; chan-server 202; full workspace clean) +
no-default-features build + svelte-check (3978 files, 0 errors)
all green.

### Notes for review

* **One extra `chan --version` subprocess at boot** (only when
  PATH actually has a chan binary). Measurable; deferred to
  @@WebtestB's lane-B walkthrough as cold-start latency
  verification per the task's acceptance criteria. Standing
  permission covers debug-build runtime checks on my end if a
  more rigorous measurement is needed before clearance.
* **PATH-resolution edge cases** — first-match-wins on multiple
  PATH entries (`which_chan_in` test covers it explicitly).
  Non-executable files at a matching name are rejected on Unix
  (test covers it explicitly). The Windows branch accepts any
  matching `chan.exe` file via `.is_file()` because PATHEXT
  semantics are baked into the filename match itself.
* **Translocation interaction** — `compute_bin_status` still
  short-circuits on translocation before calling the resolver.
  A PATH chan doesn't rescue a translocated install since the
  broader runtime environment (config dir, file watchers, etc.)
  is also affected. Documented inline.

### Coordination footprint

* No overlap with @@CI's ci-7 (`.github/workflows/`,
  `docs/journals/phase-8/ci/`).
* No overlap with @@Systacean's systacean-11 (signing block in
  `tauri.conf.json`) or systacean-13 (Makefile notarytool path).
  CLAUDE.md edit is in the Resolution-helpers subsection only;
  notarization section is untouched.
* No overlap with `tabs.svelte.ts` / chan-server route table —
  this is pure chan-desktop / Tauri-side work.

After commit clearance, queue empty for Wave-1. Standby until
Wave-2 fan-out.

See [../fullstack-b/fullstack-b-16.md](../fullstack-b/fullstack-b-16.md)
for the full implementation note + suggested commit subject.

## 2026-05-21 — poke (v0.11.2 wave: -17 / -18 / -19 all ready)

All three v0.11.2 mini-wave tasks ship-ready in one session.
Workspace pre-push gate green (workspace fmt + clippy
`--workspace -D warnings` + test + no-default-features build +
svelte-check + npm build + vitest). chan-desktop test count
21 → 33 across the wave (+4 in -17, +4 in -19, +0 Rust /
+3 vitest in -18). vitest 555 → 568 (+10 net).

### -b-17: tab right-click Reload + Open Inspector (DEV META-BLOCKER)

* **`desktop/src-tauri/Cargo.toml`** — `devtools` feature added
  to the `tauri` workspace dep so release builds carry the
  inspector affordance (Tauri 2 dropped the v1 `app.devTools`
  JSON key for this compile-time flag).
* **`main.rs`** — `reload_window(window: WebviewWindow) ->
  Result<(), String>` evals `window.location.reload()` (Tauri 2
  removed `WebviewWindow::reload()`; eval is the supported
  path). `open_devtools(window)` calls
  `WebviewWindow::open_devtools()` directly. Both registered
  in `generate_handler!`.
* **`serve.rs`** — KEY_BRIDGE_JS extended with `Cmd+R` →
  `invokeIpc('reload_window')` and `Cmd+Opt+I` /
  `Ctrl+Alt+I` → `invokeIpc('open_devtools')`. New
  `invokeIpc(e, cmd)` helper calls
  `window.__TAURI__.core.invoke(cmd)` directly. Bypasses the
  SPA event bus so a frozen Svelte runtime can't lock the
  dev affordances away.

IPC contract for @@FullStackA's `-a-36`:
* `__TAURI__.core.invoke('reload_window')` — no args.
* `__TAURI__.core.invoke('open_devtools')` — no args.

### -b-18: submit-mode reload persistence + tooltip

SPA-only. Server-side untouched.

* **`tabs.svelte.ts`** — after richPromptFromSer resolves
  during tab restore, if the deserialized tab has
  `submitMode === "agent"` AND a `terminalSessionId`, fire
  `api.setTerminalSubmitMode(sessionId, "agent")` as
  fire-and-forget. Server's `Session.agent_mode` defaults
  false on every spawn; this re-sync closes the desync that
  the bug entry's reproducer (toolbar says Agent, server
  emits Shell chord) was hitting.
* **`TerminalRichPrompt.svelte`** — shell-mode tooltip copy
  changed from `"Submit mode: shell (Cmd+Enter sends a
  trailing newline)"` to `"Submit mode: shell (default;
  Cmd+Enter submits the buffer verbatim)"`. The
  agent-mode tooltip stayed accurate (chord IS appended).
* 3 new vitest pins covering the re-sync (agent fires PUT,
  shell skips PUT, no-session skips PUT).

### -b-19: chan-desktop Cmd+= / Cmd+- / Cmd+0 zoom

* **`config.rs`** — `WindowConfig.zoom_level: f64` with
  `#[serde(default = "default_zoom")]` (returns 1.0).
  Backward compat with existing `config.json` files pinned.
* **`main.rs`** — `AppState.live_window_zooms:
  Mutex<HashMap<String, f64>>`. Three IPC handlers
  `zoom_in` / `zoom_out` / `zoom_reset` with shared
  `current_zoom` + `apply_zoom` helpers. Step 10 %, clamp
  [0.25, 5.0]. Registered in `generate_handler!`.
* **`serve.rs`** — KEY_BRIDGE_JS routes `Equal` /
  `NumpadAdd` → `zoom_in`, `Minus` / `NumpadSubtract` →
  `zoom_out`, `Digit0` / `Numpad0` → `zoom_reset`. Routed
  BEFORE the shift branch so `Cmd+=` and `Cmd+Shift+=`
  (= `Cmd++`) both zoom in. `build_drive_window` accepts a
  `zoom_seed` parameter and applies it via
  `WebviewWindow::set_zoom` after build. The close handler
  drains the live zoom into `WindowConfig.zoom_level` so
  the `-b-1` LRU restore picks it up on the next open.
* Tauri's `zoom_hotkeys_enabled(true)` polyfill stays on as
  a mousewheel + trackpad pinch fallback (chord overlap is
  harmless — capture-phase listener pre-empts the polyfill).
* 4 new tests (2 config + 2 serve).

### Coordination footprint

* No overlap with @@FullStackA's `-a-36` (paired SPA work for
  -17) — they consume the IPC names I've locked here.
* No overlap with @@CI's ci-7 (release-desktop.yml) or
  @@Systacean's `-11` / `-13` (signing block /
  Makefile notarytool) — different files.
* Same shared-worktree commit discipline as `-15` / `-16`:
  per-file `git add`, pre-commit `git diff --staged --stat`,
  post-commit `git show --stat HEAD`. Tauri Cargo.toml
  feature flag change is in shared file space; will stage
  explicitly and verify.

### Suggested commit subjects (one per task)

* `chan-desktop: reload_window + open_devtools IPC + Cmd+R /
  Cmd+Opt+I accelerators (fullstack-b-17)`
* `Rich prompt: re-sync submit-mode on tab restore + tooltip
  copy fix (fullstack-b-18)`
* `chan-desktop: Cmd+= / Cmd+- / Cmd+0 zoom chords + per-window
  persistence (fullstack-b-19)`

### Recommended commit order

1. `-b-17` first (unblocks @@FullStackA's `-a-36` SPA dispatch).
2. `-b-19` second (extends `-b-17`'s `KEY_BRIDGE_JS` +
   `invoke_handler!` + adds the `WindowConfig.zoom_level`
   field; touches main.rs's AppState).
3. `-b-18` third (SPA-only; no Rust dependency on -17 / -19).

Holding for commit clearance. Per-task implementation notes +
verification tables at the tails of
[`../fullstack-b/fullstack-b-17.md`](../fullstack-b/fullstack-b-17.md),
[`../fullstack-b/fullstack-b-18.md`](../fullstack-b/fullstack-b-18.md),
[`../fullstack-b/fullstack-b-19.md`](../fullstack-b/fullstack-b-19.md).

## 2026-05-21 — poke (v0.11.2 wave closeout: 3 SHAs landed)

All three v0.11.2 wave tasks committed per your recommended
order:

| SHA       | Subject                                                                                          |
|-----------|--------------------------------------------------------------------------------------------------|
| `5850750` | chan-desktop: reload_window + open_devtools IPC + Cmd+R / Cmd+Opt+I accelerators (fullstack-b-17) |
| `59f5688` | chan-desktop: Cmd+= / Cmd+- / Cmd+0 zoom chords + per-window persistence (fullstack-b-19)        |
| `f6d498c` | Rich prompt: re-sync submit-mode on tab restore + tooltip copy fix (fullstack-b-18)              |

Interleaved cleanly with @@FullStackA's `-a-37` through `-a-41`
landings (HEAD has `-b-17` → `-a-38` → `-a-39` → `-a-40` →
`-b-19` → `-a-41` → `-b-18`). Pre-commit
`git diff --staged --stat` + post-commit `git show --stat HEAD`
confirmed each landing matched the intended file set + no
stowaways.

### Multi-agent stowaway race survived

`-b-17`'s first commit attempt absorbed @@FullStackA's `-a-37`
work as a stowaway: between my `git add` (of my 5 chan-desktop
files) and `git commit -F`, @@FullStackA's session ran their own
`git add` for `FileEditorTab.svelte` + `missingFileRecovery.test.ts`
+ `store.svelte.ts` + `tabs.svelte.ts`, replacing my index
entries. My `commit -F` then committed THEIR 4 files under my
"fullstack-b-17" message (commit `3abb6e3` — wrong content,
wrong message).

Recovery per `feedback_shared_worktree_commits`:
* `git reset --soft HEAD~1` — undid `3abb6e3`. HEAD back to
  `f5b0122` (ci-9). Index had FA's 4 files staged.
* `git restore --staged` for FA's 4 files — moved them back to
  unstaged in the working tree.
* `git add` for MY -17 files explicitly.
* Verified staged stat matched my expected -17 content.
* `git commit -F` — landed as `5850750`.

@@FullStackA's session subsequently re-committed `-a-37` cleanly
under their own SHA (`6f1850a`). No work lost; the
volatile-SHA-during-multi-agent-rebase principle from
`feedback_shared_worktree_commits` held — both lanes have the
right commits in HEAD now.

### -b-19 / -b-18 commit shape

After the -17 fix, the -19 + -18 commits chained without
incident. I used `&&`-chained `git add` + `git diff --staged
--stat` + `git commit -F` blocks to minimise the race window.

### Final gate state

* Workspace fmt + clippy `--workspace -D warnings` + test
  (chan-desktop 26 → 33 across the wave; workspace total 862
  passing) + no-default-features build all clean at every
  commit boundary.
* SPA svelte-check 3983 files / 0 errors; vitest 583 → 586
  (3 new tests for -b-18 resync).

### Standing down

Queue empty per your "After all 3 commit" note. Standby for
v0.11.2 walkthrough verdicts from @@WebtestA / @@WebtestB +
ci-8 dry-run support if @@CI needs hands.

Push waits for @@Systacean's `chan-v0.11.2` cut per the
commit-plan.

## 2026-05-21 — poke (fullstack-b-20 committed; ci-8 dry-run #3 unblocked)

`-b-20` committed as `602d0cb` "chan-desktop: aarch64-only DMG
via bundle.macOS.files + main.rs unused-app rename". 4 files,
175 insertions / 21 deletions. Pre-commit
`git diff --staged --stat` + post-commit
`git show --stat HEAD` clean.

### Empirical findings beyond the task body

* **Option (iii) doesn't work.** Tested locally: setting
  `bundle.externalBin = ["binaries/chan-aarch64-apple-darwin"]`
  caused tauri-build to look for
  `binaries/chan-aarch64-apple-darwin-aarch64-apple-darwin`
  (Tauri 2 appends the triple unconditionally; no
  detect-existing-triple short-circuit). Confirmed via
  `cargo check`.
* **Option (i) with `bundle.macOS.files`** is the cleanest
  workable path. Destinations are relative to
  `Chan.app/Contents/`, NOT the bundle root — first attempt
  with `"Contents/MacOS/chan"` produced
  `Chan.app/Contents/Contents/MacOS/chan` (two levels).
  Corrected to `"MacOS/chan"` and end-to-end verified:
  `cargo tauri build --bundles app` produces
  `Chan.app/Contents/MacOS/chan` (26 MB, executable, signed,
  `chan --version` → 0.11.1).
* **Bug #2's task-body "1-char rename" is incomplete.** The
  closure parameter `app` IS used at line 932 inside
  `#[cfg(target_os = "macos")]`. The naive rename to `_app`
  makes the body's `app` reference fall back to the outer
  `tauri::App` binding (wrong type for `show_window(&AppHandle,
  ...)`); macOS build fails to compile. Fix: rename the param
  AND update the line 932 reference to `_app`. `_app` is still
  a usable binding; only the unused-warning is suppressed.

### Trade-offs documented in CLAUDE.md

Dropping `bundle.externalBin` has two scoped regressions, both
documented in `desktop/CLAUDE.md`'s new "v0.11.2 hotfix:
aarch64-only DMG, no externalBin" subsection:

1. **Dev-mode auto-copy** via externalBin is gone. `cargo tauri
   dev` no longer drops chan into `target/debug/`. Contributors
   with `cargo install --path crates/chan` chan on PATH get the
   `-b-16` resolver path automatically; without it, dev mode
   reports `BinStatus::missing`.
2. **Linux/Windows bundling** no longer ships chan
   (`bundle.macOS.files` is macOS-only). Users on those
   platforms install chan separately and rely on `-b-16`'s
   PATH resolver.

Both restored as a post-v0.11.2 `ci-N` follow-up that pairs
universal2 / lipo work with multi-platform
`bundle.<linux|windows>.files` restoration.

### Hand-off

ci-8 dry-run #3 can fire against `602d0cb`. If green →
@@WebtestB second-Mac verify → @@Alex "cut it" → @@Systacean
cuts `chan-v0.11.2`. Standing by for verdicts.

See [../fullstack-b/fullstack-b-20.md](../fullstack-b/fullstack-b-20.md)
for the full implementation note + verification table.

## 2026-05-21 — poke (fullstack-b-21 committed; ci-8 dry-run #4 unblocked)

`-b-21` committed as `ae389f7` "chan-desktop: codesign bundled
chan sidecar for notarization". 3 files, 387 insertions / 3
deletions. Pre-commit `git diff --staged --stat` + post-commit
`git show --stat HEAD` clean.

### Option C ruled out empirically

Tested per the task body's recommendation first. Result:
tauri-build rejected `bundle.macOS.externalBin` with

```
unknown field `externalBin`, expected one of `frameworks`,
`files`, `bundle-version`, `bundle-name`, `minimum-system-version`,
`exception-domain`, `signing-identity`, `hardened-runtime`,
`provider-short-name`, `entitlements`, `info-plist`, `dmg`
```

Tauri 2's `MacOSConfig` schema does NOT include an `externalBin`
key. The task body's hypothesis (that per-platform externalBin
might behave differently from the top-level key WRT
triple-append) is moot — the field doesn't exist at the
per-platform level in this Tauri version. CLAUDE.md now records
this so future implementers don't retry it.

### Option A landed

Added a codesign step to `desktop/Makefile`'s `chan-bin`
recipe, gated on `APPLE_SIGNING_IDENTITY` being non-empty:

```make
@if [ -n "$$APPLE_SIGNING_IDENTITY" ]; then \
    codesign --force --options=runtime --timestamp \
        --sign "$$APPLE_SIGNING_IDENTITY" $(CHAN_BIN); \
fi
```

Runs after chan is staged to
`src-tauri/binaries/chan-<host-triple>` and before Tauri's
bundler picks it up. Tauri's bundle-signing pass preserves the
chan signature (only re-signs files it owns; chan is a
files-map payload from its perspective).

### Empirical verification

`make app-signed` from `desktop/` on aarch64 Mac with my
standing chan-desktop runtime permission:

```
$ codesign -dv --verbose=2 .../Chan.app/Contents/MacOS/chan
Identifier=chan-aarch64-apple-darwin
flags=0x10000(runtime)
Authority=Developer ID Application: Alexandre Fiori (W73XV5CK3N)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=21 May 2026 at 09:36:30
TeamIdentifier=W73XV5CK3N
Runtime Version=26.4.0

$ codesign --verify --strict --deep .../Chan.app; echo $?
0
```

All three notary-rejection criteria satisfied:
* Developer ID Application signature ✓
* Hardened runtime (`flags=0x10000(runtime)`) ✓
* Secure timestamp (not "none") ✓

The bundled chan's CodeDirectory identifier is
`chan-aarch64-apple-darwin` (inherited from the staging
filename); should not block notarization. If notary balks on
the identifier in dry-run #4, a follow-up can rename the staged
binary to `chan` before signing.

### `notarytool submit` not run locally

Per the task body's optional-quota clause; CI dry-run #4 is the
authoritative test. My standing permission covers a local
`xcrun notarytool submit` if you want belt-and-braces before
@@CI fires, but the codesign verification + the matching
identity should be sufficient.

### Hand-off

ci-8 dry-run #4 can fire against `ae389f7`. If green →
@@WebtestB second-Mac verify → @@Alex "cut it" → @@Systacean
cuts `chan-v0.11.2`. Standing by for verdicts.

See [../fullstack-b/fullstack-b-21.md](../fullstack-b/fullstack-b-21.md)
for the full implementation note + verification table.

## 2026-05-21 — poke (fullstack-b-22 ready for commit clearance)

`-b-22` (chan-desktop orphan sidecar reap + lock-takeover UX)
implementation landed locally; pre-push gate green; commit
held pending your clearance. Working tree is clean for the
three files I touched
(`desktop/src-tauri/src/{main,serve}.rs` +
`desktop/src/main.js`).

### Shape

Prevention (heavy invest, durable past phase-9):

* Process group on spawn (`process_group(0)` Unix +
  `CREATE_NEW_PROCESS_GROUP` Windows).
* `stop_child` signals the **group** via `killpg`, not just
  the leader PID — catches helper subprocesses too.
* `impl Drop for AppState` calls `serve::stop_all` for the
  panic-unwind path; bridges the gap when `RunEvent::Exit`
  doesn't fire.

Recovery (minimum-viable, per your phase-9 forward-look):

* New `ServeFailedPayload.drive_lock_conflict: bool` set by
  the reader thread when stderr matches chan-drive's
  `DriveLocked` Display string.
* SPA-side branch: `showServeFailed` reroutes into a
  `promptDriveLockTakeover` dialog (`ask()` plugin, Reclaim
  + Cancel buttons).
* New `reclaim_drive_lock(path)` IPC: `ps`-scans for orphan
  chan serve processes against the drive key, SIGTERM →
  SIGKILL after a 1s grace, retries `serve::start`. Returns
  `ReclaimResult { killed_pids, retry_succeeded, message }`.
* Failure path renders a `pkill -f "chan serve <key>"`
  copy-paste cleanup snippet rather than silently retrying.

### Pre-push gate

| Surface                                                                | State                                          |
|------------------------------------------------------------------------|------------------------------------------------|
| `cargo fmt --check`                                                    | Clean.                                         |
| `cargo clippy --workspace --all-targets -- -D warnings`                | Clean.                                         |
| `cargo test --workspace`                                               | All pass (chan-desktop 32 → 39, +7 new).       |
| `cargo build --workspace --no-default-features`                        | Clean.                                         |
| `web/` `npx svelte-check`                                              | 3987 / 0 errors / 0 warnings.                  |
| `web/` `npm run build`                                                 | Clean.                                         |
| `web/` `npx vitest run`                                                | 58 / 588 tests pass.                           |

### Runtime walkthrough — flagged for @@WebtestB

The task body's "verified: kill chan-desktop SIGTERM / SIGKILL /
panic; confirm no orphan via `ps aux | grep chan`" criterion is
explicitly the audit-trail walkthrough shape. Standing
chan-desktop runtime permission lets me run it myself, but the
canonical lane is @@WebtestB; my judgement is that fresh eyes on
the dialog text + click cycle is more valuable than my own pass.
Happy to run it myself on your say-so. Detailed walkthrough
script is in the task's "Runtime verification" section.

### No cross-lane touch

* No `tauri.conf.json` edit (recovery is pure IPC + SPA dialog;
  no new capability needed).
* No chan-drive change (flock auto-releases on kill).
* No @@Systacean cross-pollination materialised — the chan-drive
  lock-takeover protocol primitive the task body flagged as a
  possibility was not needed.
* Touches confined to: `desktop/src-tauri/src/main.rs`,
  `desktop/src-tauri/src/serve.rs`, `desktop/src/main.js`. None
  of those are touched by @@FullStackA / @@CI / @@Systacean's
  in-flight work in the current dirty tree.

### Suggested commit subject

```
chan-desktop: process-group sidecar reap + drive-lock-takeover UX (fullstack-b-22)
```

### Hand-off

Awaiting:
1. Your clearance to commit (then I'll explicit-add the three
   files only, `git diff --staged --stat` audit, commit, and
   `git show --stat HEAD` post-audit).
2. Routing on the runtime walkthrough — @@WebtestB lane or my
   follow-up. Either is fine.

See [../fullstack-b/fullstack-b-22.md](../fullstack-b/fullstack-b-22.md)
for the full implementation note + verification table.

## 2026-05-21 — poke (fullstack-b-22 committed; picking up -b-23)

`-b-22` committed as `3987e73` "chan-desktop: process-group
sidecar reap + drive-lock-takeover UX (fullstack-b-22)". 4 files,
753 insertions / 9 deletions. Pre-commit `git diff --staged
--stat` + post-commit `git show --stat HEAD` clean — no
stowaways. Push held per release discipline.

@@WebtestB lane retains the runtime walkthrough per your
clearance heading; my standing chan-desktop runtime perm
survives recycle if a follow-up empirical pass is needed.

Picking up `-b-23` (port chan.app source into web-marketing/)
per the pre-recycle handover queue. Will fire a permission event
to @@Alex on the chan.app source location if I can't locate it
in-tree.

See [../fullstack-b/fullstack-b-22.md](../fullstack-b/fullstack-b-22.md)
"2026-05-21 — committed as `3987e73`" for the post-commit audit
trail.

## 2026-05-21 — poke (fullstack-b-23 ready for commit clearance)

`-b-23` (port chan.app source into `web-marketing/`)
implementation landed locally; pre-push gate green; commit held
pending your clearance.

### Source located

Confirmed with @@Alex via in-session question (local-only on
their machine):
`/Users/fiorix/dev/github.com/chan-writer/chan-prod-setup/etc/chan-site`.
No need for a separate permission event after that.

The chan.app source turned out to be **pure static HTML** —
single `index.html` (all CSS + JS inline) plus PNGs and the
two install scripts. No framework, no npm, no build step. Means
the publishable artifact is the source tree itself; no `dist/`
shape needed.

### Shape

* Ported 7 source files + 2 screenshot assets from `chan-site/`
  into `web-marketing/` (`index.html`, `favicon.ico`,
  `chan-mark.png`, `install.sh`, `install.ps1`, `assets/editor-*.png`).
* Deliberately **not** ported: `site.nginx.conf` (legacy host
  config; decommissions with the `systacean-N` DNS task per
  round-2-plan's GitHub Pages decision).
* Mirrored `web/public/qr-donate.png` into
  `web-marketing/qr-donate.png` for the donation QR.
* Added a dedicated **§support section** above the footer
  containing the QR + a small caption pointing to
  `mailto:hello@chan.app`. Chose this over a footer-cramped or
  separate-page placement; rationale documented in the task
  tail. ~12 lines of new CSS in the existing inline style block;
  mobile-collapsing.
* Added `web-marketing/README.md` (build + preview + deployment
  docs) and `web-marketing/.gitignore` (forward-compat for
  `node_modules/`, `dist/`, etc.).

### Workspace boundary

* No `Cargo.toml` in `web-marketing/`; root workspace `members`
  is explicit (not glob), so cargo ignores the dir entirely.
  Verified: `cargo test --workspace` clean post-port.
* No `package.json` shared with `web/` either. Zero coupling.

### Pre-push gate

| Surface                                                 | State                                                |
|---------------------------------------------------------|------------------------------------------------------|
| `cargo fmt --check`                                     | Clean.                                               |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean.                                               |
| `cargo test --workspace`                                | All pass.                                            |
| `cargo build --workspace --no-default-features`         | Clean.                                               |
| `web/` `npx svelte-check`                               | 3987 / 0 / 0.                                        |
| `web/` `npx vitest run`                                 | 58 / 588 tests pass.                                 |
| `web/` `npm run build`                                  | Clean.                                               |
| Local preview                                           | `python3 -m http.server` + curl smoke on key URLs.   |

### Suggested commit subject

```
web-marketing: port chan.app static site source + donation QR section (fullstack-b-23)
```

Files (all NEW; explicit per-path `git add` plus the task file):

```
web-marketing/README.md
web-marketing/.gitignore
web-marketing/index.html
web-marketing/favicon.ico
web-marketing/chan-mark.png
web-marketing/qr-donate.png
web-marketing/install.sh
web-marketing/install.ps1
web-marketing/assets/editor-dark.png
web-marketing/assets/editor-recipes.png
docs/journals/phase-8/fullstack-b/fullstack-b-23.md
```

### Hand-off

Awaiting your clearance to commit. Pure additive — no overlap
with concurrent FullStackA / CI / Systacean / WebtestA / WebtestB
work in the dirty tree.

See [../fullstack-b/fullstack-b-23.md](../fullstack-b/fullstack-b-23.md)
for the full implementation note + verification table +
QR-placement rationale.

## 2026-05-21 — poke (fullstack-b-23 committed; pre-recycle wrap)

`-b-23` committed as `bc9e1f8` "web-marketing: port chan.app
static site source + donation QR section (fullstack-b-23)".
11 files, 944 insertions (pure additive, zero deletions).
Pre-commit `git diff --staged --stat` + post-commit
`git show --stat HEAD` clean. Push held per release discipline.

This session's full landed set:

| SHA       | Subject                                                                       |
|-----------|-------------------------------------------------------------------------------|
| `3987e73` | chan-desktop: process-group sidecar reap + drive-lock-takeover UX (fullstack-b-22) |
| `bc9e1f8` | web-marketing: port chan.app static site source + donation QR section (fullstack-b-23) |

Pre-recycle posture per your handover heading:

* `-b-22` runtime walkthrough remains routed to @@WebtestB.
* Standing chan-desktop runtime permission survives recycle
  (recorded in [`../../../agents/bootstrap.md`](../../../agents/bootstrap.md)
  §"Standing permissions").
* No in-flight work in my queue; ready for recycle.

See [../fullstack-b/fullstack-b-23.md](../fullstack-b/fullstack-b-23.md)
"2026-05-21 — committed as `bc9e1f8`" for the post-commit audit
trail.

## 2026-05-21 — session closed

@@Alex's tear-down signal received; honouring the "no
uncommitted code across sessions" gate. My lane verified clean
against HEAD before close:

* Working tree: no modifications under `docs/journals/phase-8/fullstack-b/`,
  `event-fullstack-b-architect.md` (this file, prior to this
  append), or `event-fullstack-b-alex.md`.
* HEAD has `3987e73` (-b-22) + `bc9e1f8` (-b-23) committed
  cleanly with explicit per-path adds + pre/post audits; no
  stowaways landed in either.

This append is the only outstanding journal write in my lane
at tear-down. Committing as a session-close docs commit per
the shared-worktree discipline, then standing by for
@@Alex's tear-down.

Recycled session bootstraps via `docs/agents/bootstrap.md`;
PRE-RECYCLE HANDOVER in `event-architect-fullstack-b.md`
covers the post-tear-down pickup state. Standing chan-desktop
runtime permission survives recycle per
[`../../../agents/bootstrap.md`](../../../agents/bootstrap.md)
§"Standing permissions".

## 2026-05-21 — poke (post-recycle bootstrap complete; queue-empty, standing by)

Fresh session up. Spotted the "Still not respawned" line in
[`../architect/journal.md`](../architect/journal.md) latest round —
closing that loop. Bootstrap surface sweep clean:

* HEAD: 14 commits ahead of `origin/main` (push held; `chan-v0.11.2`
  at `60901c1`, latest is the post-recycle dispatch + clearance
  round at `b886764`).
* My last work in HEAD: `-b-22` (`3987e73`) + `-b-23` (`bc9e1f8`).
  Both 8/8 HOLD via `webtest-a-3` (-b-23) + `webtest-b-3` (-b-22)
  per your clearance round 1.
* No task files past `-b-23.md`; no new inbound poke; no new
  `event-fullstack-b-alex.md` entries.
* New bug-list entry in my lane (chan-desktop orphan-detection
  heuristic too loose) noted as `-b-22` polish parked for
  wave-2/3 — not yet dispatched; not my call to pick up
  unbidden.

Standing chan-desktop runtime permission survives recycle.
Standing by for wave-3 dispatch.

## 2026-05-21 — poke (fullstack-b-24: scope question on actual lint locations)

`-b-24` received. Grep against the 11 names says 10 of them
live in `crates/chan-server/src/control_socket.rs` (Unix-only
file already gated `#[cfg(unix)]`; the declarations leak to
Windows), not in `desktop/src-tauri/src/`. Only `exit_signal`
is genuinely in chan-desktop.

Authorization in the task body covers `desktop/src-tauri/src/*.rs`
only; touching chan-server is shared lane scope and I want
explicit OK before editing it inside `-24`.

Scope question + three routing options + recommendation at
the tail of [`../fullstack-b/fullstack-b-24.md`](../fullstack-b/fullstack-b-24.md)
"scope question for @@Architect" section. Recommendation:
(A) expand `-24` authorization to also cover the chan-server
file, single commit, single smoke fire.

Holding on implementation. No code edits yet.

## 2026-05-21 — poke (-b-24 smoke iterations 1-5: clippy GREEN; 2 test failures remain — scope question)

Implementation landed; cascade resolved through 5 smoke
iterations.

Commits on main:
* `c0600e0` — original 11 lints
* `e8ff68a` — smoke #1 fixup (orphaned chan_drive/serde imports + parse_ps helper)
* `f796345` — smoke #2 fixup (ControlResponse in chan/main.rs)
* `68e1cbc` — smoke #3 fixup (Deserialize import split in chan/main.rs)
* `b01b310` — smoke #4 fixup (fs_graph.rs test helpers)

After smoke #5 ([run `26241431377`](https://github.com/fiorix/chan/actions/runs/26241431377)):

**Windows `cargo clippy --all-targets -- -D warnings` GREEN.**
The 11 original lints + the 4 cascade items all cleared.
`-24`'s stated lint-silencing scope is done.

### Two test-step failures remain (latent under the broken clippy gate)

| Job             | Failure                                                                                              | Lane                                |
|-----------------|------------------------------------------------------------------------------------------------------|-------------------------------------|
| Windows test    | `graph_scope_file_rejects_missing_target` in `chan/main.rs:2970` — assertion hard-codes Unix OS-error wording ("No such file" / "not found"); Windows says "The system cannot find the file specified" | Pre-existing portability gap; 1-line fix |
| Ubuntu test     | `removing_contact_frontmatter_demotes_node_back_to_file` in `chan-drive/tests/contacts_import.rs:296` — BGE model gap | systacean-18 #[ignore] sweep missed it |

### Scope question

Three options at the tail of [`../fullstack-b/fullstack-b-24.md`](../fullstack-b/fullstack-b-24.md)
"Scope question for @@Architect" section:

* **(A)** Fold the Windows test fix into `-24` (one-line
  assertion + `cannot find` substring). Single smoke fire
  validates everything together. **Recommended.**
* **(B)** Close `-24` here; cut a follow-up for the Windows
  test fix.
* **(C)** Cut both follow-ups separately and close `-24` now.

Ubuntu BGE failure is genuinely separate scope (systacean
lane) regardless of (A)/(B)/(C); flagging for routing.

Holding on the Windows test fix until you choose.

### Standing chan-desktop runtime perm unused

Not consumed this task — `-24` is declaration-only Rust;
no runtime walkthrough needed. Standing perm available
for future tasks.

## 2026-05-21 — poke (-b-24 smoke #6 verdict: structurally complete; 3 chan-drive lock failures need routing)

Smoke #6 ([run `26245378140`](https://github.com/fiorix/chan/actions/runs/26245378140))
landed.

### -24 scope fully done

| Check                          | State                                              |
|--------------------------------|----------------------------------------------------|
| Windows clippy                 | ✓ GREEN (was the gate-unblocker)                   |
| Windows `graph_scope_file_rejects_missing_target` | ✓ GREEN (option-A portability fix worked) |
| Ubuntu clippy                  | ✓ GREEN                                            |
| web / build / rustfmt          | ✓ GREEN                                            |

Commit `8e4ce5c` is the final `-24` commit. Full SHA
ledger at the tail of [`../fullstack-b/fullstack-b-24.md`](../fullstack-b/fullstack-b-24.md)
"Commit readiness for -24" section.

### Remaining Windows reds — out of -24's scope

Three chan-drive lock-primitive test failures on Windows:

| Test                                                                                     | Location                                  |
|------------------------------------------------------------------------------------------|-------------------------------------------|
| `drive::tests::second_open_blocks_on_writer_lock`                                        | `crates/chan-drive/src/drive.rs:4396`     |
| `library::tests::reset_drive_returns_locked_when_other_process_holds_lock`               | `crates/chan-drive/src/library.rs:989`    |
| `lock::tests::second_acquire_fails_while_held`                                           | `crates/chan-drive/src/lock.rs:72`        |

All 3 fail on `matches!(err, ChanError::DriveLocked)` —
chan-drive's file-locking primitive doesn't surface
`DriveLocked` on Windows the same way `flock` does on
Unix. Three remediation shapes laid out in the task
tail (Windows lock-primitive bridge / `#[cfg(unix)]` the
3 tests / cross-platform abstraction). Lane: @@Systacean
(chan-drive owns `lock.rs`). Not my call.

### Remaining Ubuntu red — already routed

`removing_contact_frontmatter_demotes_node_back_to_file`
BGE-model gap — already in @@Systacean's lane via `-18`
follow-up #4 + `systacean-19` per your clearance round 7.

### Routing ask

Need a call on whether the 3 chan-drive lock test
failures get a fresh `systacean-N` cut or fold into the
in-flight `-18` follow-up #4. Likely separate (lock
semantics vs BGE model presence are different problem
families) but either shape works from my lane's
perspective.

`-24` is done on my side. Standing by for clearance to
close + next dispatch.
