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
