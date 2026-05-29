# @@FullStackB's phase-8 journal

Author: @@FullStackB
Date: 2026-05-19

Frontend + backend lane B. Same profile as @@FullStackA; operates
in parallel to clear the bug queue and (Round 2) feature queue.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 - Boot + queue scan

Fresh session boot. Read contact card, phase-8 process (inherits
phase-7 verbatim plus the @@CI lane delta and the bug-sweep round
shape), the phase request, the bug list, and my four task files:

| Task             | Scope                                                |
|------------------|------------------------------------------------------|
| fullstack-b-1    | Native window-config stack (chan-desktop Tauri)      |
| fullstack-b-2    | Cmd+T + scrollback + line-adjustment cluster         |
| fullstack-b-3    | Watcher dialog: out-of-root paths + create-dir UX    |
| fullstack-b-4    | Indexing-chart pan/zoom parity with Graph view       |

No inbound events in `alex/event-architect-fullstack-b.md` or
`alex/event-fullstack-b-alex.md` yet (files don't exist). Working
through tasks in numerical order.

## 2026-05-19 - fullstack-b-1 implemented

Landed the WindowConfig LRU stack inside chan-desktop's sidecar
config:

* `config::WindowConfig` plus `window_configs: Vec<WindowConfig>`
  in `Config` (cap = 20, newest first).
* Free-function helpers `push_window_config` / `pop_window_config`
  with dedupe-by-label semantics, plus
  `local_window_key` / `tunnel_window_key` to keep local and
  tunneled drives namespaced apart in the stack.
* `AppState::{push,pop}_window_config` wrap the lock + load +
  mutate + save pattern, best-effort (errors logged, not
  returned).
* `serve::spawn_local_drive_window` /
  `spawn_tunneled_drive_window` now pop a matching entry before
  generating a fresh label, reusing its `window_label` (so the
  `?w=<label>` lookup hits the same `session.json`) and stamping
  its saved URL hash into the new webview's fragment.
* `build_drive_window` installs a `WindowEvent::CloseRequested`
  handler that snapshots `WebviewWindow::url().fragment()` and
  pushes a fresh `WindowConfig` to the top of the stack.
* `pop_compatible_config` defends against stale entries: a popped
  label that no longer matches the drive's current hash prefix,
  or that's already live in this process, is dropped (the next
  close will produce a fresh entry anyway).

Why this shape: `session.json` is already keyed per-window in the
drive (`<drive>/.chan/sessions/<w>.json`), so reusing the same
`?w=` label across the close/open cycle is the smallest possible
delta that restores panes / tabs / selections (per `fullstack-15`
+ `fullstack-58`). The URL hash carries the overlay state
(`files=...`, `search=...`, `graph=...`) that chan deliberately
keeps out of `session.json` so shareable URLs stay shareable; we
mirror it onto the stack so close/reopen also round-trips those.

Tests: six new unit tests in `config::tests` covering insert,
dedupe, LRU eviction at MAX, and the local-vs-tunnel key
namespacing. `cargo test -p chan-desktop` is green (17 tests
total), `cargo clippy --workspace --all-targets -- -D warnings`
is clean, `cargo fmt --check` passes after one auto-rewrap.

Limits I'm aware of:

* Capturing the hash on close relies on `WKWebView.URL` (macOS)
  and `WebView2.Source` (Windows) reflecting hash changes pushed
  via `history.replaceState`. Both do today; if a future Tauri
  upgrade changes that, the captured hash falls back to empty
  (session.json restore still works via the reused `?w=` label).
* The "20 entries" cap is global, not per-drive. A user who opens
  twenty separate drive windows in quick succession could push
  older drive entries off the stack. Matches the bug ask
  ("keep up to 20 window configs"), and the more-recent drives
  are the ones the user is most likely to want restored anyway.
* No teardown of stale `session.json` files in the drive. The
  stack drops the label after 20 entries, but the per-window
  session file lives on in chan-drive. Not a correctness issue
  (the SPA loads only the label it's given), but it's some
  growing dust over time. Out of scope for this fix.

Followed CLAUDE.md: no em dashes in comments / docs, factual
prose, pinned toolchain unchanged. Atomic write contract carried
forward via the existing `ConfigStore::save` (temp + rename).

## 2026-05-19 - fullstack-b-1 committed

@@Architect approved + cleared. Committed as `203c6e8`. Push
waits for Round-1 close per the standing rule. Two new tasks
landed in the queue while I was on -1: `fullstack-b-5`
(per-Hybrid theme propagation) and `fullstack-b-6` (scope FB
watcher to selection — phase-7 backlog item 9 pulled in early
because the pain is current). Working order is -2, -3, -4, -6,
-5 per @@Architect's note.

## 2026-05-19 - fullstack-b-2 implemented

Terminal cluster: three coordinated fixes shipped together.

* Cmd+T (native) + Cmd+Alt+T (web Mac) come back as direct
  chords for "new terminal in active pane". Pane Mode (Cmd+K 1)
  stays as the keystroke-light alternative; the two coexist.
  Native chord lives in `KEY_BRIDGE_JS`; web chord in
  `App.svelte`'s `onWindowKey`. Win/Linux web stays gated to
  Pane Mode because Ctrl+Alt+T there is already
  `app.tab.reopenClosed`.
* Scrollback truncation root cause: Pane.svelte unmounted every
  TerminalTab when Hybrid NAV was active (`{#if !paneMode.active}`
  around the each-block). Each entry into pane mode disposed the
  xterm.js EditorView and dropped the 20k-line buffer. Fix:
  drop the wrapper, gate the `active` + `focused` props on
  `!paneMode.active` so the existing
  `visibility: hidden; pointer-events: none` CSS rule hides the
  surface without unmounting. Buffer is preserved across pane
  mode AND across intra-pane tab switching (already mounted by
  the same CSS-driven path).
* Line-adjustment vs iTerm: diagnosed from
  `docs/journals/phase-8/attachments/image-{3,4}.png`. Root cause
  was `lineHeight: 1.0` packing ascenders against the next row's
  descenders so multi-row ASCII glyphs (Claude Code splash cube,
  figlet output) rendered visibly squished. Bumped to 1.2 to
  match iTerm's visual density. Conservative single-token change;
  walkthrough verification owed to @@WebtestB.

Bonus housekeeping picked up by the resync:
`crates/chan/src/main.rs::SERVE_LONG_ABOUT` was multiple phases
out of date (still referenced "Pane Mode" / "Save Cmd+S" / no
"Flip Hybrid"). Regenerated via
`node web/scripts/shortcuts-table.mjs --serve-long-about` so
`chan serve --help` matches reality.

Test deltas:

* New `web/src/components/paneTerminalMount.test.ts`: pins the
  structural shape (no `{#if !paneMode.active}` adjacent to the
  terminal each-block; `active` / `focused` props gated by
  `!paneMode.active`).
* `web/src/state/shortcuts.test.ts`: flipped the
  no-standalone-Terminal-chord guards into positive assertions
  for the new "New terminal" label. Bare-`Terminal` row absence
  guards retained.
* `desktop/src-tauri/src/serve.rs::tests`: updated to expect
  Cmd+T present (`key_bridge_keeps_independent_chords`) and not
  absent from `key_bridge_drops_chords_covered_by_pane_mode`.

Pre-push gate: clean across `cargo fmt` / `cargo clippy -D
warnings` / `cargo test` / `cargo build --no-default-features` /
`npm run check` / `npm run build` / `npx vitest run` (450/450).
Two pre-existing terminal-tab tests flake under load at the 15s
default timeout but pass standalone; not a regression here.

## 2026-05-19 - fullstack-b-3 implemented

Watcher dialog cluster: removed the drive-root gate for absolute
paths, added silent create-on-attach, and reshaped the path
prompt modal so an attach intent no longer pretends to be a
create / rename.

Substantive backend change in
`crates/chan-server/src/routes/terminal.rs::resolve_watcher_dir`:
absolute paths skip the `path_canon.starts_with(root_canon)`
check entirely; relative paths still flow through
`resolve_safe_strict` (in-drive sandbox preserved). The function
now `create_dir_all`s its target up front, so a missing watcher
dir is created silently before the metadata check fires. Three
new tests + one renamed test pin the new behaviours.

Frontend: extended `PathPromptMode` with `attach` and routed the
modal's `status` derivation + `pathSegments` to short-circuit on
that mode so existing dirs don't trip the overwrite warning and
absolute paths don't manufacture a fake ancestor preamble. The
watcher dialog in `TerminalRichPrompt.svelte` now passes
`mode: "attach"` instead of the previous `mode: "move"`. New
`PathPromptModal.test.ts` pins the four attach-mode branches.

Notes for the audit trail:

* This is a deliberate widening of the trust surface: chan-server
  will now `create_dir_all` and watch arbitrary filesystem paths
  the user types. That's appropriate for the rich-prompt watcher
  (infra traffic, not user content), but it does mean a stray
  paste of `/etc/cron.d` would create the dir if it doesn't
  exist (it does, on most systems). Worth flagging if @@Alex
  thinks tighter guards are needed; the current behaviour
  matches the bug ask verbatim.
* No SPA-side warning when the user picks a path outside the
  drive. The status row renders the path verbatim — they see
  what they typed. Easy to add an explicit hint later.

Pre-push gate green across all crates and web. One pre-existing
flake in `routes::graph::tests::link_to_non_markdown_disk_file_resolves_to_real_file`
on parallel runs (passes standalone); not from this task.

## 2026-05-19 - fullstack-b-4 implemented

Indexing chart in `EmptyPaneCarousel.svelte` got a self-contained
pan/zoom layer modelled on `GraphCanvas` but scoped to SVG-space
(no Canvas / d3-force needed). Transform-driven wrapper `<g>`
holds both the edges and nodes groups; pointer capture on the
SVG drives drag-pan; wheel with `exp(-delta * 0.0015)` smoothing
drives cursor-anchored zoom; a Locate icon button pinned to the
chart's bottom-right corner resets to identity. Leaving slide 3
also resets so the next return-visit lands on a fitted view.
Gestures sit on the SVG element directly, not the slide
container, so the Round-2 Infographics-tabs refactor (backlog
item 4) inherits the behaviour without re-wiring.

Coordination footprint: @@FullStackA's `fullstack-a-7` Hybrid NAV
chord swap (Mod+K → Mod+.) had left
`shortcuts.test.ts::"advertises Hybrid NAV (Cmd+K)"` failing.
Picked it up as a trivial one-line update since it was blocking
my own pre-push gate; also re-synced `SERVE_LONG_ABOUT` to match.
Called out in the task file so the audit trail attributes both to
the right initiating change.

Eight new pinned-source tests in
`EmptyPaneCarousel.indexingChart.test.ts` plus the updated
shortcuts test; pre-push gate green (464/464 SPA tests). One
pre-existing chan-server graph test still flakes on parallel
runs but passes standalone (not from this task).

## 2026-05-19 - fullstack-b-6 implemented

FB watcher scope narrowed entirely on the SPA. Replaced the
unconditional `refreshTree()` in `onWatchEvent` with a
scope-aware path: each FB instance contributes a scope from its
selection (drive root / dir / parent-of-file); the watcher tree
refresh only fires when an event path lands in at least one
active scope, and only re-fetches the affected parent dir
(`refreshTreeForPath`). No chan-server / chan-drive changes —
the task header had floated a "subscribe-by-prefix in
chan-drive" direction, but the SPA-side filter turned out to be
sufficient for the visible bug (tree flicker on out-of-scope
activity).

Known limitation, documented: `tree.entries` is still shared
across FB instances, so when two FBs are open on different
scopes, an event matching EITHER scope refreshes the shared
tree state — both FBs see a re-render. The "ship strict first"
note in the task explicitly allowed this trade-off; a true
per-FB tree would be a much larger refactor.

Ten new unit tests in `web/src/state/watcherScope.test.ts`
cover the scope-match semantics, the active-scopes collection
across overlay + per-pane paths, and the no-op behaviour of
`refreshTreeForPath` when the parent dir isn't loaded. Pre-push
gate green (474/474 SPA tests).

## 2026-05-19 - fullstack-b-5 implemented

Per-Hybrid theme propagation fixed entirely in the editor theme
CSS. Root cause: each shipped editor theme
(`github.css`, `google_docs.css`, `word.css`) gated its dark
variant on `:root[data-editor-theme="<name>"][data-theme="dark"]`.
The `data-theme="dark"` half only matches the root, so a
Hybrid pane that overrode `data-theme="light"` via the phase-7
`fullstack-59` cascade saw the editor-token dark rule fire
globally — visible as "dark text on light pane background"
(the "back of the Hybrid in light mode while dark globally"
repro Alex flagged in chat).

Fix: extended each theme's dark selector to a comma-joined pair
that also matches `:root[..] .pane[data-theme="dark"]`
(higher specificity than the global rule, so the per-pane
variant wins inside any opted-in pane). Added a sibling
`.pane[data-theme="light"]` block to each theme that re-asserts
the base light defaults — needed because the global dark rule
fires on the root and cascades into the pane's CSS variables;
the per-pane light override has to actively reassert each
editor token to break the cascade.

Truth table verified across all six combinations of
`(global, pane) ∈ {dark, light, none}`. No JS / Svelte changes;
the existing `data-theme` attribute on `Pane.svelte` (from
`fullstack-59`) is the cascade carrier. xterm.js + GraphCanvas
wiring from `fullstack-78` is untouched.

Aside: an attempted pinned-source test for the editor themes
got pulled because `?raw` CSS imports return empty under our
JSDOM vitest setup, and the `node:fs` fallback hits a missing
`@types/node`. The CSS change is small + reviewable in the diff
so manual review + @@WebtestB's walkthrough is the right layer
here.

Pre-push gate green across all crates and web. Queue empty
pending @@Architect review for -2/-3/-4/-5/-6.

## 2026-05-20 - Round-1 wrap-up + teardown confirmation

Round-1 detour queue closed. Two final tasks landed this
session after the recycle gap:

* `fullstack-b-11` — Settings: terminal scrollback (MB) +
  default TERM with spawn-time semantics. Committed
  `ab5a0ce`.
* `fullstack-b-12` — Terminal: bundle Source Code Pro
  Regular + iTerm cursor/size parity. Committed
  `96d0db0`.

Both push-parked until end of Round 2 per the structural
change to the release plan.

Teardown footprint (per the @@Architect Round-1 teardown
checklist): no-op.

* No `make run` / `Chan.app` launches from my lane this
  session — `fullstack-b-7` runtime click stayed parked
  on the permission event to @@Alex.
* No ad-hoc `chan serve` from visual checks on -11 / -12;
  validation was source-side only (cargo unit tests +
  vitest + npm build + grep on the bundled CSS).
* No Chrome MCP tabs opened.
* Running `chan serve` processes I observed at close
  (`/tmp/chan-test-phase8-wa`, `/tmp/chan-test-phase8-wb`,
  `/Users/fiorix/Documents/ChanRoadmap`) belong to the
  webtest lanes + @@Alex's own session, not mine.
* Build artifacts in `target/debug/` are the normal
  workspace cargo cache; leaving in place.

Ready for `agent-recycle`. Round-2 deliverables wait for
post-recycle fan-out under the updated -13 / -14 / -15 /
-16 numbering noted in the architect inbound.

## 2026-05-20 — Round-2 rich-prompt mini-wave boot (fullstack-b-13)

Fresh session after the recycle. Bootstrapped per
`docs/agents/bootstrap.md`. Read contact card + skill guides,
phase-8 process, request, my journal, the new task file, and
all inbound/outbound events.

Task in queue: `fullstack-b-13` — shell/agent submit-mode
toggle + survey-reply echo consumer. Part of the rich-prompt
mini-wave dispatched 2026-05-20 by @@Architect; paired with
@@FullStackA's `fullstack-a-28` (BubbleOverlay regression).

### Grounding turned up one finding worth flagging

The task body assumed the survey-reply `poke<Enter>` emission
is SPA-side ("the SPA emits a literal 'poke' string + Enter
into the PTY"). Grep across `web/src/` + `crates/` says it's
**server-side**: `crates/chan-server/src/terminal_sessions.rs:502`
in `dispatch_agent_event` writes `b"poke\n"` unconditionally
when the fsnotify ingest path parses a new reply file. The
SPA only writes the reply file via `api.writeTerminalEventReply`;
the chan-server fsnotify ingest path is what emits the PTY
bytes.

Implications: the per-prompt toggle has to reach the server
somehow. Three options sketched (per-session config field
+ HTTP route, SPA-intercepts-via-WS-frame, full SPA-side
emission relocation) with footprint + trade-offs and my
recommendation (Option 1, smallest delta) at the tail of
[`fullstack-b-13.md`](fullstack-b-13.md). Poke fired to
@@Architect for the architecture call.

### Permission event fired to @@Alex (chord-encoding probe)

The agent-submit chord byte sequence is the front-loaded
design call per the task body. I'd prefer not to guess; the
toggle's whole purpose is "send bytes the agent treats as
submit," so the wrong constant nullifies the fix.

Asked @@Alex to settle it one of two ways:
* Option 1: type once in their own Claude Code session +
  TTY-fd byte injection (fiddly).
* Option 2: authorise me to spin a throwaway chan test
  server and poke candidates via the browser devtools
  console.

Either form of approval works.

### Coordination touchpoints found via `git status`

* `tabs.svelte.ts` + `BubbleOverlay.svelte` carry unstaged
  @@FullStackA `-a-28` work (the `dbi?: string[]` SerTab
  field + `dismissedIds` state wiring + the bubble overlay
  rendering pass). My eventual `rpsm?: "s" | "a"` add to
  SerTab is ADDITIVE and in a separate region of the same
  file, but per the user's explicit coordination directive
  on the paired -a-28/-b-13 work I am NOT racing them on
  `tabs.svelte.ts`. Will land my SerTab + state-type adds
  after @@FullStackA commits, or with explicit @@Architect
  go-ahead.
* `event_watcher.rs` + `process.md` carry unstaged @@Systacean
  `-10` work (watcher event-filename regex filter at the
  fsnotify ingest path). Adjacent to but not overlapping
  with my server-side touchpoint (`terminal_sessions.rs::dispatch_agent_event`).
  Will stay clear of `event_watcher.rs` entirely.

### Status: paused

Three external dependencies all blocking substantive work:

1. @@Alex — chord-encoding probe result (permission event).
2. @@Architect — Option 1 / 2 / 3 architecture choice (poke).
3. @@FullStackA — `-a-28` commit lands on `tabs.svelte.ts`,
   freeing me to add `rpsm?` SerTab field + state-type
   field without racing.

Holding. Auto Mode default would be "scaffold what's safe,"
but the user's "coordinate well before you edit the same
file" directive on the paired -a-28/-b-13 wave is the
governing instruction. Will resume on @@Architect's reply
or @@Alex's chord answer (whichever lands first).

## 2026-05-20 — Chord probe done (chord pinned)

@@Alex approved Option 2 (throwaway chan server + WS-frame
injection) via @@Architect transcription. Ran the probe live.

**Setup**: throwaway drive at `/tmp/chan-test-phase8-rpsm`,
debug binary `./target/debug/chan serve`, Chrome MCP open at
the launch URL. Installed a `WebSocket.prototype.send`
interceptor to capture the live WS handle into
`window.__capturedWs__` and a `window.__chordProbe__(bytes)`
helper that fires `{type:"input", data:bytes}` frames.
Spawned a terminal via Cmd+Alt+T (web-Mac chord).

### Claude Code v2.1.145

| Bytes              | Effect                                       |
|--------------------|----------------------------------------------|
| `probe1`           | Lands as draft text.                         |
| `\x1b[27;9;13~`    | **Submits.** "probe1 acknowledged."          |
| `\n` (LF)          | Multi-line newline in draft (status flips to "ctrl+g to edit in Vim"). |

**Chord**: `\x1b[27;9;13~` — xterm modifyOtherKeys "Cmd+Enter."

### Codex v0.130.0

| Bytes              | Effect                                       |
|--------------------|----------------------------------------------|
| `\n` (LF)          | Silent. Even at the trust-prompt.            |
| `\r` (CR)          | Trust-prompt confirms; main-prompt submits.  |
| `probeC1`          | Lands as draft text.                         |
| `\x1b[27;9;13~`    | No effect. probeC1 stays in draft.           |
| `\r` (post-chord)  | Submits the draft; codex runs tools.         |

**Chord**: `\r` — divergent from Claude Code.

### Decision

Single-chord ship with Claude Code's `\x1b[27;9;13~` per
@@Alex's "if codex fails it's fine, just want the signal"
directive. Codex's `\r` chord documented as future work for
a per-agent encoding map (Round-3 polish or later mini-wave).
Gemini probe skipped per @@Alex's bandwidth allowance.

Constant to ship:
```ts
// fullstack-b-13: Claude Code v2.1.145 accepts this byte
// sequence as the "submit" chord (xterm modifyOtherKeys CSI
// for Cmd+Enter). codex v0.130.0 accepts `\r` instead; the
// divergence is documented in fullstack-b-13.md. Single-
// chord ship per @@Alex 2026-05-20.
const AGENT_SUBMIT_CHORD = "\x1b[27;9;13~";
```

### Teardown footprint

* `kill <pid>` on `./target/debug/chan serve`; exit 144
  (SIGTERM expected).
* `rm -rf /tmp/chan-test-phase8-rpsm` — throwaway drive
  removed.
* `./target/debug/chan remove /private/tmp/chan-test-phase8-rpsm`
  — registry entry unregistered.
* Chrome MCP tab closed via `tabs_close_mcp`.

No persistent side effects. Test-server-workflow audit-trail
shape per `feedback-test-server-workflow`.

### Status

Chord constant in hand. Still parked on two external
dependencies:
* @@Architect's architecture call (Options 1 / 2 / 3 for the
  server-side echo path).
* @@FullStackA's `tabs.svelte.ts` settling (now -a-28 +
  -a-29 + -a-30 all in flight on that file).

Holding for whichever lands first.

## 2026-05-20 — `-b-13` server-side + `-b-14` both landed (commit-ready)

@@Architect chose Option 1 for -b-13 architecture (per-session
config field + thin HTTP route). @@Alex queued -b-14 (chan-
desktop title format) alongside. Both shipped in this session.

### -b-13 server-side slice

* `SubmitMode { Shell, Agent }` enum, default Shell. `submit_chord()`
  returns `b"\n"` for Shell, `b"\x1b[27;9;13~"` for Agent.
* `Session.agent_mode: AtomicBool` (default false), with
  `submit_mode()` / `set_submit_mode()` accessors.
* `Registry::set_submit_mode(session_id, mode) -> bool` for the
  SPA-driven flip. Mirrors `set_watcher`'s shape.
* `dispatch_agent_event` branches on the session's mode. Shell
  mode is byte-for-byte today's behaviour (`b"poke\n"`); Agent
  emits `b"poke\x1b[27;9;13~"`.
* New route `PUT /api/terminal/:session/submit-mode`, mirroring
  `set_terminal_watcher` (tunnel-public gate, JSON body,
  204/400/404).
* Four new tests pin: chord constants + default, registry
  setter, end-to-end PTY delivery in Agent mode (proves the
  `\n` shape is gone, the chord landed), route 204/400/404.

Pre-push gate green at workspace level (fmt + clippy
`--workspace -D warnings` + test --workspace + no-default-
features build). chan-server suite 198 → 202.

SPA side intentionally NOT in this slice — `tabs.svelte.ts`
still carries unstaged @@FullStackA -a-28/-29/-30 work per
`git status`. The new server-side API surface is reachable;
SPA-side commit follows once @@FullStackA settles.

### -b-14 slice

* `drive_title(key)` simplified to `key.to_string()` (no
  basename derivation, no `chan drive: ` prefix).
* Tunneled-drive title dropped prefix to `"{tenant} ·
  {drive}"`.
* New test `drive_title_is_the_path_verbatim`. LRU restore
  path from -b-1 verified: title is derived live from `key`,
  not stored in `WindowConfig`, so restored windows
  automatically pick up the new shape.

chan-desktop suite 19 → 20.

### Status

Both slices commit-ready. Holding for @@Architect commit
clearance. Push held for the patch-release commit-grouping
cut.

## 2026-05-21 — fullstack-b-15 committed; starting fullstack-b-16

Round-2 Wave-1 fan-out cut `fullstack-b-15` (bundle chan +
`bundled_chan_path()` helper) and `fullstack-b-16` (PATH-first
resolver). Hard sequential per the task brief.

### -15: implementation note

Bundling-the-binary was already wired via Tauri's `externalBin`
mechanism + `desktop/Makefile`'s `chan-bin` recipe. The actual
missing pieces from the task spec were:

1. A public helper exposed to launcher code (the existing
   `chan_bin()` was private in `main.rs`).
2. An *exact-match* version probe per locked Round-2 decision 3
   (the existing probe used `MIN_CHAN_VERSION = "0.8.1"` as a
   floor; the locked contract is exact equality).
3. A unit test pinning the resolution contract.
4. Documentation of the bundle layout in `desktop/CLAUDE.md`.

Changes shipped:
* `desktop/src-tauri/src/serve.rs` — new `pub fn bundled_chan_path()
  -> Result<PathBuf, String>` (pure path math; existence check moved
  to `compute_bin_status`) + `pub fn probe_chan_version(&Path)` (exact
  match against `env!("CARGO_PKG_VERSION")`). New unit test
  `bundled_chan_path_is_sibling_of_chan_desktop_executable` (chan-
  desktop 20 → 21 tests).
* `desktop/src-tauri/src/main.rs` — dropped relocated helpers +
  the `MIN_CHAN_VERSION` constant. `compute_bin_status()` and the
  three IPC handlers (`add_drive`, `remove_drive`, `set_drive_on`)
  route via `serve::bundled_chan_path()`.
* `desktop/CLAUDE.md` — new "Bundled chan sidecar" section with
  bundle layout per build profile + resolution-helper API +
  universal2 follow-up flagged for `ci-7`.

Pre-push gate green workspace-wide: fmt + clippy `-D warnings` +
test (chan-desktop 21; chan-server 202; full workspace clean) +
no-default-features build + svelte-check (3978 files / 0 errors) +
npm build + vitest 544/544 (after one transient flake on first run
that cleared on rerun).

### -15 commit + stowaway recovery

Committed as `6f4f697`. Pre-commit `git diff --staged --stat` and
post-commit `git show --stat HEAD` both clean (5 files, mine
only).

The commit window hit a multi-agent index race. While my
`git add -p` for CLAUDE.md was selecting only my "Bundled chan
sidecar" hunk (leaving @@CI's notarization section unstaged),
@@CI's session committed `c279733` with the message "ci: tag-
triggered signed + notarized chan-desktop release" — but the
actual content was MY 65-line CLAUDE.md hunk, absorbed as a
stowaway. @@Systacean then committed systacean-13 (`01f10d3`,
notarytool keychain-profile work) on top of the stowaway. Both
later reset --soft'd; @@CI re-committed cleanly as `666c027`
(actual ci-7 work: `release-desktop.yml` + `ci-7.md`); I
re-staged + re-committed my -15 as `6f4f697`.

Net result: commit content matches commit message on every
post-recovery SHA. @@Systacean's systacean-13 was wiped by their
own reset and needs re-staging + re-committing on top of
`6f4f697` (their files are still in the working tree unstaged).
Flagged in [`../alex/event-fullstack-b-architect.md`](../alex/event-fullstack-b-architect.md)
so @@Architect can route a heads-up if needed.

The `git add -p` discipline I used (selective per-hunk staging
to coexist with @@CI's CLAUDE.md hunk) was correct. The race
wasn't from my staging being wide; it was from another agent's
commit landing while my partial staging was in flight. The
`feedback_shared_worktree_commits` recovery protocol cleanly
absorbed it.

### Starting -16

`fullstack-b-16` (PATH-first resolver) builds on `-15`'s
public surface. Algorithm shape per the locked decision:
PATH chan via `Command::new("chan").arg("--version")` → exact-
match against `env!("CARGO_PKG_VERSION")` → match → PATH path;
else → `bundled_chan_path()`. Implementation lives next to the
`-15` helpers in `serve.rs`.

## 2026-05-20 — three commits landed (mini-wave queue empty)

@@Architect cleared both -b-13 server-side and -b-14;
@@FullStackA's -a-28/-29/-30 committed (`1a83050`,
`20ece30`, `3d708a2`) so `tabs.svelte.ts` reached HEAD.
SPA-side -b-13 was then unblocked and landed.

Three clean commits on my lane:

| SHA       | Subject                                                  |
|-----------|----------------------------------------------------------|
| `e24b931` | chan-server: per-session shell/agent submit-mode toggle + dispatch_agent_event chord branch (fullstack-b-13 server-side) |
| `8dbaaed` | chan-desktop: window title = drive path verbatim (fullstack-b-14) |
| `dce2373` | Rich prompt: shell/agent submit-mode toolbar toggle + SerTab roundtrip + agent-chord submit path (fullstack-b-13 SPA-side) |

Pre-commit `git diff --staged --stat` ran on each, post-commit
`git show --stat HEAD` verified — no stowaways from
@@FullStackA / @@Systacean concurrent work.

### Mini-wave verification snapshot

| Surface      | State                                                 |
|--------------|-------------------------------------------------------|
| chan-server  | 198 → 202 tests (+4 covering chord constants, registry setter, PTY dispatch + chord echo + legacy poke\n absence, route 204/400/404 branches). `workspace test` green. `clippy --workspace -D warnings` clean. `cargo build --no-default-features` clean. `fmt --check` clean. |
| chan-desktop | 19 → 20 tests (+1 covering the title format). |
| SPA          | svelte-check 0/0; vitest 514 → 522 (+8 covering SerTab rpsm round-trip and the encodeForAgentSubmit helper); npm build clean (pre-existing chunk-size warnings only). |

### Queue status

Mini-wave queue empty. Standby until next wave. Note —
codex's `\r` chord divergence parked for Round-3 Track 5
per @@Alex 2026-05-20. @@WebtestB's lane-B walkthrough
against a live Claude Code session is the user-visible
verification gate for -b-13 (per the task body).

Push held for the patch-release commit-grouping cut.
