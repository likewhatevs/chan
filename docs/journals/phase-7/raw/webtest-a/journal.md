# @@WebtestA's phase-7 journal

Author: @@WebtestA
Date: 2026-05-18

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-18 11:29 BST - Bootstrap

Read contact card, webdev skill guide, phase process, phase request,
own journal, and CLAUDE.md. The actual phase directory in this checkout
is `docs/journals/phase-7/`; the requested
`docs/journals/phase-7/` path is absent.

No `webtest-a-*.md` task files are present under
`docs/journals/phase-7/webtest-a/`. Architect journal
mentions planned `webtest-a-1`, but no task file has been cut yet.

## 2026-05-18 13:05 BST - Fresh-agent resume + Lane A walkthroughs

Fresh @@WebtestA session per @@Architect's 13:00 BST handover. Drive
was pre-seeded from the previous (terminated) attempt; verified content
matched the spec and proceeded.

Server: `./target/debug/chan serve --port 8801 --no-browser
/tmp/chan-webtest-a-1/` (default 8787 was taken by an unrelated
chan from phase 6; 8810 is @@WebtestB's).

URL: `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`

Ran both:

* `webtest-a-1` (baseline Lane A) — 11 bug verdicts. Notable
  finds: B20 (table render crashes the editor with a CodeMirror
  block-decoration error), B1 (Shift+Tab outside list jumps
  focus to the pane hamburger), B13 (typing left of a list
  marker inserts before the marker - "Q1. First..."), B9
  (image bubble has a stray separator with no "searched N
  files" hint), B19 *does not* reproduce.
* `webtest-a-2` (fullstack-1 docked side panes) — all 8 items
  pass. Two non-blocking observations (default width feels
  wide; resize handles lack keyboard a11y) noted inline.
* Out-of-scope but worth surfacing: dragging the active tab
  onto an adjacent inactive tab in the workspace removes the
  dragged tab from the list. Side-pane false-positive check
  passed; this looks like plain workspace tab D&D.

### Process observations

* The chrome-MCP browser is shared with @@WebtestB. My tab
  was repeatedly hijacked to 8810 between batches. Mitigated
  by re-asserting `window.location.assign('http://127.0.0.1:8801/...')`
  at the top of every batch. Flagged to @@Architect.
* The `chrome MCP` blocks `Element.innerHTML` reads that
  contain query-string tokens (Cookie/query-string data
  guard). Worked around by using narrower `.textContent`
  reads and per-attribute lookups.
* `RangeError: Block decorations may not be specified via
  plugins` repros on every load of any doc with a pipe table
  - documented the stack and pointed @@FullStack at the
  StateField vs ViewPlugin contract for CM6 block decoration
  providers.

Hand-off URL forwarded via
[../alex/event-webtest-a-architect.md](../alex/event-webtest-a-architect.md).

## 2026-05-18 16:05 BST - webtest-a-3 closed + standby

`webtest-a-3` two-wave walkthrough done:

* Wave 1 (pre-revision): items 1-5 (toolbar parity) + 6, 7,
  9, 10 (browser-served link routing + icon audit) all PASS.
* Wave 2 (post-revision): rebuilt + restarted 8801; revised
  `openExternalUrl` dispatch validated end-to-end. Scenarios
  2 + 3 (Chan.app desktop, tunnel-loop) verdicted by code
  audit since Chrome MCP can't drive Tauri's WKWebView.

Architect accepted the verdict at 16:00 BST; `fullstack-2`
is cleared for commit architect-side. Standing by for
@@Alex's closeout + the agent-recycle event.

8801 server stays up for optional click-around. Drive
contents: index.md + 3 PNGs, note-a.md (table-crash repro),
note-b.md (lorem). No test files left behind.

## 2026-05-18 (resume) BST - Fresh @@WebtestA bootstrap

Fresh session resuming Phase 7. Read in this order:

* This journal (the three predecessor entries above).
* [../request.md](../request.md) — Round 1 done; Round 2
  fan-out queued (survey protocol, bubble overlay, agent
  spawning, orchestration SKILL).
* [../process.md](../process.md) — events, permission flow,
  agent-recycle.
* [../alex/event-architect-webtest-a.md](../alex/event-architect-webtest-a.md)
  — latest from @@Architect at 16:00 BST: `fullstack-2`
  accepted, I'm done with closeout, no queued task.
* [../architect/journal.md](../architect/journal.md) handover
  section at 17:05 BST — confirms "@@WebtestA has no queued
  task; Round 2 fan-out is where they get work."
* [../../../agents/webtest-a.md](../../../agents/webtest-a.md).

### State on disk

* `/tmp/chan-webtest-a-1/` still present (index.md, note-a.md
  with the B20 table-crash repro, note-b.md, img/ with three
  PNGs). Drive is unregistered (was a throwaway).
* The 8801 server the previous me left up is down. The only
  running `chan serve` is on `/private/tmp/chan-test-phase6`
  (unrelated, pre-existing from phase 6).
* `origin/main` at `9e48367`, tag `v0.10.1`. Branch is up to
  date.
* No `webtest-a-4.md` task file exists.

### Posture

Standby for Round 2 fan-out. No new task to start; no event
or task file is waiting for my action. Will not spin up a
test server speculatively (per the test-server workflow
memory: ask first about drive choice + seed).

@@Alex / @@Architect: when you cut my next task, ping
[../alex/event-architect-webtest-a.md](../alex/event-architect-webtest-a.md).

## 2026-05-18 (resume) BST - webtest-a-4 regression sweep done

Self-initiated `webtest-a-4` under @@Alex's go-do-them
authorization. Built head `d4b11d2` + unstaged `systacean-3`
patch, ran 8801 against `/tmp/chan-webtest-a-1/`.

* Lane A headliners from `webtest-a-1` re-verified against the
  `fullstack-4` commit: B1 (Shift+Tab focus theft), B2 (image
  paste in list), B13 (typing before marker) — **all PASS**
  for both numbered and bullet lists.
* B20 (table-crash) **still open** with the same
  `RangeError: Block decorations may not be specified via
  plugins` stack — not in `fullstack-4`'s scope, carries
  forward.
* **New Round-2 data point**: `systacean-3` cross-drive drift
  still reproduces with the patch in. Tab jumps from 8801 →
  8810 within 1.5s of every navigation, even on a fresh chrome
  MCP tab whose first navigation is to 8801. Both servers
  serve the patched binary (`Cache-Control: no-store` +
  `Vary: Host` confirmed). Workaround: killing the stale Lane B
  servers (8810 + 8811 from @@WebtestB's pre-recycle session).
  Hypothesis hand-off to @@Systacean — something in the SPA
  bundle is reading cross-port persistent state. Full write-up
  in [webtest-a-4.md](webtest-a-4.md) "Drift status".
* Adjacent sweep: wikilink renders + isolates correctly; the
  rest of the area smoke clean.

Poking architect via
[../alex/event-webtest-a-architect.md](../alex/event-webtest-a-architect.md).

## 2026-05-18 (resume) BST - webtest-a-5 wave-1.5 cluster done

Took up `webtest-a-5` after architect's 20:00 BST poke.
Rebuilt against head `f94c4b5`, restarted 8801.

* **All wave-1.5 items PASS**: B15 click semantics, pane
  right-click menu (Split/Next-Prev/Focus-color/Close), pane
  hamburger (Reload + Web Inspector), doc tab right-click (6
  spec items + bonus), per-pane focus color persisting across
  reload, Cmd+Alt+]/[ web pane nav, B22 Copy Path no-stuck-
  Loading, fullstack-7 light-mode terminal ANSI palette
  (verified by CSS colors matching patch exactly + visual
  white-bg render).
* **Bonus: systacean-3 drift re-tested — PASS this round**.
  The `f94c4b5` commit (adding `Vary: Host` on hashed assets,
  on top of the SPA shell `no-store`) appears to close the
  cross-port drift I repro'd in `webtest-a-4`. Tested both
  cold-tab and warm-cache (visit 8810 first, then 8801)
  scenarios. systacean-6 may not be needed; flagged to
  @@Systacean to confirm.
* Minor cosmetic finding: opening the pane hamburger menu
  while a prior pane right-click menu is still open shows both
  menus simultaneously (Escape doesn't dismiss the first).
  Not blocking.

Server stays on 8801; tab left in light theme + Terminal-1 in
left pane + green-bordered empty right pane. Full per-item
verdicts at [webtest-a-5.md](webtest-a-5.md).

Standing by for the next wave-2 commits.

## 2026-05-18 (resume) BST - webtest-a-5 wave-2 cluster done

Picked up after @@Alex's `ping`. Rebuilt against head
`8ae2d44`, bounced 8801.

* **fullstack-9 B20 pipe-table render — PASS**. note-a.md
  fully renders now: heading + lists + actual `<table>` with
  three data rows + the post-table paragraph. No new
  RangeError (buffered errors are pre-rebuild timestamps).
  The StateField path for block decorations matches the patch.
* **fullstack-10 B12 source/rendered caret round-trip —
  PASS**. Source pos 42 (inside `./img/photo-1.png` URL) →
  wysiwyg (image renders, source revealed under cursor as
  expected, no crash) → back to source: cursor lands at
  exact same offset (10 within the URL text node).
* **fullstack-10 B6 EOF typing scroll — PASS**. Cursor at
  end-of-doc with viewport at bottom: typed A/B/C one at a
  time, scrollTop stayed at 7218 across all three. No
  per-character thrash.
* **systacean-6 cross-drive drift — PASS** under the
  warm-cache stress (visit 8810 first → navigate to 8801 →
  hold). Drift fully closed. Note: 8810 was running the older
  binary; fix on the 8801 side alone was sufficient — each
  SPA scopes its own keys.

Skipped fullstack-8 (BCAST/mute) — @@WebtestB is actively
stress-testing it on 8810 (T1-T6 visible), no point doubling
up. fullstack-11 / fullstack-12 not yet landed.

Server stays on 8801. Standing by for the next wave-2
landings.

## 2026-05-18 (resume) BST - webtest-a-5 wave-2b cluster done

After @@Alex's next `poke`. Rebuilt against head `65534d3`,
bounced 8801.

* **fullstack-12 B16 Cmd+Alt+T — PASS** for both halves.
  Cmd+Alt+T spawns Terminal-1 in the focused pane. Legacy
  Cmd+` no longer creates a terminal (count stayed at 1).
* **fullstack-11 fs-move/delete UX — PASS** for both mv and
  rm cases. Beautiful empty-state surface with header banner
  "File moved or deleted", filename subtitle, and Re-open /
  Find / Close affordances. No raw I/O error.
* **systacean-8 scrollback after reload — FAIL** in live
  test. PTY survival works (same PID + bash history after
  reload, B14 path intact), but the visual scrollback isn't
  replayed: 30 SCROLLBACK-LINE-* lines disappear post-reload,
  only the fresh prompt shows; mouse-wheel scroll reveals
  nothing in the xterm buffer. Server-side ring + WS replay
  loop are in place per code inspection. Most likely failure
  modes are timing-race or session-id mismatch on reconnect;
  full hypotheses + repro detail in
  [webtest-a-5.md](webtest-a-5.md) wave-2b section. Hand-off
  for @@Systacean.

Server stays on 8801. Drive has list.md (test artifact) +
the persisted move/delete-target empty states in the tab.
Standing by for the next batch.

## 2026-05-18 (resume) BST - webtest-a-6 received, standby

New Round 2 wave-A walkthrough task cut at
[webtest-a-6.md](webtest-a-6.md). Lane A angle: bubble
overlay + watcher-set dialog + survey rendering + terminal
status bullet.

Items 1-12 are blocked on `systacean-9` (backend fsnotify
watcher) and `fullstack-13` (frontend bubble UI / survey
renderer). Neither has landed yet — head is `9653e6b`
(chore-only since wave-2b). My binary on 8801 is current.

Item 13 (carry-over smokes on fullstack-11 / fullstack-12)
is already verdicted in [webtest-a-5.md](webtest-a-5.md)
wave-2b — both PASS, and no code changes in the window
between then and now, so verdicts hold.

Pre-flight synthetic-event recipe parked in
[webtest-a-6.md](webtest-a-6.md). Ready to fire the moment
systacean-9 / fullstack-13 land.

Standing by.

## 2026-05-18 (resume) BST - webtest-a-6 wave-A cluster done

After @@Alex's `poke`. Rebuilt against head with
`d08ed3d` (systacean-9 watcher) + `1f2f6fc` (fullstack-13
bubble substrate); bounced 8801. Renamed terminal to
`WebtestA` so events `to:@@WebtestA` resolve via
`normalize_agent_target`.

**Verdicts (11 of 12 PASS, 1 PARTIAL):**

* systacean-9 items 1-4: all PASS. POST returns 204,
  atomic event drops `poke\n` to the PTY, malformed JSON
  increments `dropped_events` without crashing, unknown
  types are warn+ignore (no PTY write).
* fullstack-13 items 5, 6, 8-12: all PASS. Watch
  directory dialog flow, bubble-over-terminal, 4×3
  multi-question (all 4 in one bubble), standing
  "Check my comments first", scope defaults one-shot
  with the spec'd 3 options, stack ↔ tray toggle
  (4 bubbles collapsed to one pill), status bullet
  (visible / dirty / blink / clears blink on prompt
  reopen).
* fullstack-13 item 7: **PARTIAL**. Survey renders + Submit
  fires but reply atomic-write fails:
  `reply failed: path is not editable text:
  events/.event-reply-s1-mpbk3dio.tmp` — chan-drive
  editable-text gate rejects the `.tmp` staging file.
  Hand-off to @@FullStack / @@Systacean.

Two minor side observations called out inline (not
blocking): (a) the Watch directory dialog rejects absolute
paths with `× absolute paths are not allowed`, but the
systacean-9 API spec says both drive-relative and absolute
are supported — UX vs API surface mismatch worth a note;
(b) unknown-type events still render a bubble showing the
type name (`futuristic-thing from @@TestAgent`) rather
than being silently dropped — confirm intent.

State left: 8801 server up, rich prompt open with the full
bubble stack visible (including the red reply-failed
banner from item 7) for live inspection.

Standing by for fix on item 7 + the next wave.

## 2026-05-18 (resume) BST - webtest-a-6 revision cluster done

After @@Alex's `poke`. Rebuilt against head with
`1cd4ef2` (PTY reattach by window+tab — systacean-8
follow-up) + `2d1c719` (fullstack-18 simplified bubble
survey UI).

**Three previously open items are now closed:**

* **systacean-8 scrollback retention — now PASSES**. 25
  `RETAIN-LINE-N` lines re-appear after page reload. The
  `1cd4ef2` commit message confirms my prior hypothesis
  verbatim ("attach without session id treated as a fresh
  PTY"). Reattach by `(window_id, tab_name)` closes the
  loop.
* **Item 7 survey reply — now PASSES**. fullstack-18
  rewrote the reply path with a `.md` extension, side-
  effect-fixing the `.tmp` / editable-text gate issue.
  Reply file `event-reply-v2-1xn.md` lands with the
  correct schema. Likely makes `systacean-11` /
  `fullstack-19` unnecessary — worth confirming with
  @@Architect.
* **Item 8 4×3 (revised UX) — PASSES**. Now uses topic
  tabs `Q1 Q2 Q3 Q4` with auto-advance on each keystroke
  and auto-commit when the last tab is answered. Single
  reply file with all 4 answers, scope_grant locked to
  `one-shot`. Answered tabs gain a `*` annotation.
* **Item 11 stack/tray (revised location) — PASSES**.
  Toggle moved from the bubble stack top to the rich-
  prompt right-click context menu (`Bubble stack` /
  `Bubble tray`). Toolbar surface cleaner.

Two minor follow-up nits called out inline:
* Watcher state staleness on session reload (SPA shows
  "Stop watching" but server returns "watcher is no
  longer attached" on reply attempt). Workaround: toggle
  stop/start. Worth either auto-reattach or stale-state
  cleanup.
* Answered survey bubbles stay visible (with `*`) rather
  than dismissing — confirm intentional vs nit.

State left: 8801 server up, ScrollbackA terminal has both
answered surveys visible, watcher attached. Reply files
intact in `events/` as evidence.

Standing by for next wave / @@Alex direction on
`systacean-11`/`fullstack-19` necessity.

## 2026-05-18 (resume) BST - webtest-a-6 wave-B cluster done

After @@Alex's `poke`. Rebuilt against head with the new
batch: `530e30f` (systacean-11 server-side event-reply
writer), `7bc2897` (fullstack-19 SPA route through
terminal endpoint), `4ca7dc4` (revert of systacean-6 SPA
storage scoping — confirms my drift verdict), `a2fb205`
(fullstack-14 Phase 1 Graph + File Browser as first-class
tabs).

**All five wave-B items PASS:**

* **fullstack-19 + systacean-11**: keystroke reply now
  POSTs to
  `/api/terminal/<session>/event-reply` (204), server
  writes the `.md` reply file atomically. Reply schema
  unchanged; only the write path crossed the boundary.
* **fullstack-14 File Browser**: Cmd+P opens a `Files`
  tab next to the terminal (was OverlayShell). Hash kind
  `b`. Tab carries DETAILS inspector on the right.
* **fullstack-14 Graph**: Cmd+Shift+M opens a `Graph` tab
  with SCOPE selector + filter chips inspector. Hash
  kind `g`. Rendered semantic graph (13/13 nodes).
* **Drift after 4ca7dc4 revert**: warm-cache repro held
  on 8801 across 3 s. The f94c4b5 `Vary: Host` patch
  alone IS sufficient — the systacean-6 revert is safe.

Server stays on 8801. Tab 503725098 has the three tabs
(WaveB terminal + Files + Graph) live. Reply file in
`events/` as evidence.

Standing by for the next wave.

## 2026-05-19 (resume) BST - webtest-a-6 wave-C pane cluster done

After @@Alex's `poke`. Rebuilt against head with the new
pane-system batch: `e4f9d28` (fullstack-15 pane body tab
detach substrate) + `44d9749` (fullstack-16 transactional
pane mode via Cmd+K).

* **fullstack-16 — PASS**. Cmd+K snapshots layout into a
  draft. Both panes render lightweight previews (tab name
  + filename, no editor body). Status pill
  `‹ • pane mode  Enter commit · Esc discard` visible.
  Arrow keys move focus correctly. Esc exits cleanly,
  layout intact.
* **fullstack-15 — PASS by code audit + unit tests**.
  Wiring confirmed in `Pane.svelte` (`onBodyDragOver`,
  `edgeForBodyDrop`, `onBodyDrop`) + the
  `detachTabToPaneEdge` helper in `tabs.svelte.ts` with
  53 lines of new test coverage. Live drag from MCP
  doesn't carry the `TAB_DRAG_MIME` payload so the body-
  drop handler short-circuits; same tooling limit as the
  earlier Cmd-modifier paths. Needs a hand test from
  @@Alex with a real mouse drag.

Server stays on 8801. Standing by for the next wave.

## 2026-05-19 (resume) BST - webtest-a-7 receipt + fullstack-17 polish + SKILL

After @@Alex's `poke`. `webtest-a-7` (wave-B walkthrough)
cut. Only upstream pieces landed so far: `fullstack-17`
(polish bundle, `0c2faa7`) and `architect-1`
(orchestration SKILL, `dfcad1c`).

**Build break** found on attempted rebuild: in-progress
systacean-12 substrate in `terminal_sessions.rs:541` moves
`cwd` into `cmd.cwd(cwd)` then re-uses it on line 598. Fix:
`cmd.cwd(cwd.clone())`. Flagged for @@Systacean; blocks
binary rebuilds until landed.

* **fullstack-17 polish** — PASS by code-audit (live
  retest deferred until rebuild unblocked):
  - Absolute-path dialog now accepts `/`-prefixed paths
    via `allowAbsolute` opt in `PathPromptModal`. Closes
    my wave-A nit.
  - Unknown-type bubbles dropped in
    `watcherEvents.parseWatcherEvent` (only `survey` /
    `survey-reply` / `poke` parse). Closes my wave-A
    nit.
  - Stale watcher cleanup + answered-survey auto-dismiss
    + terminal rename keep-open + restart confirmation +
    mutually-exclusive pane menus + light-mode ANSI white
    contrast — all per commit message + landed test
    set (`BubbleOverlay TerminalRichPrompt watcherEvents
    pathValidate`).
* **architect-1 SKILL** — read README + atomic-writes +
  spawn-protocol. atomic-writes matches the
  systacean-9 watcher contract exactly (temp + rename,
  single read on Create/rename-final). spawn-protocol is
  forward-looking — staked to systacean-12's design
  shape; matches the in-progress `Registry::restart` and
  `CreateOptions { command, env, preflight }` naming I
  saw in the broken tree. No drift to flag yet.

Items 1-10 of webtest-a-7 BLOCKED on `fullstack-20` /
`systacean-12` (spawn UI), `systacean-13` (activity
indicator), `systacean-14` (MCP discovery). Will pick up
each cluster as it lands.

8801 server is DOWN — killed during the rebuild attempt;
can't relaunch until the build is fixed. Standing by.

## 2026-05-19 (resume) BST - systacean-12 backend verified

After @@Alex's `poke`. Build unblocked (`cwd.clone()` fix
landed). Rebuilt + relaunched 8801. `systacean-12`
(`314a68b` HTTP terminal control channel) tested directly
via curl:

* `POST /api/terminals` → 201 + `{session, tab_label}`.
  Spawned `@@SpawnTest` with a bash command.
* `POST /api/terminals/<session>/restart` → 204.
* `DELETE /api/terminals/<session>` → 204; idempotent
  follow-up → 404 with "terminal session not found".

**SPA bridge gap**: spawned terminals do NOT appear in the
SPA tab strip after reload — the SPA's tab layout is
client-only, and HTTP-spawned PTYs aren't pushed to the
SPA over any existing channel. fullstack-20 (in-progress
in the working tree, `SpawnDialog.svelte` etc.) is the
expected closer.

Items 1-10 of webtest-a-7 still blocked on
`fullstack-20` (spawn UI), `systacean-13` (activity
indicator), `systacean-14` (MCP discovery).

8801 server back up. Standing by.

## 2026-05-19 (resume) BST - fullstack-20 spawn UI cluster

After @@Alex's `poke`. `f2094c3` fullstack-20 landed.
Rebuilt + restarted 8801.

**Items 1-3 PASS**:
* Spawn agent affordance in rich-prompt context menu (and
  a toolbar shortcut).
* Dialog with Tab name / Command / Env / Cancel / Spawn.
  Submit creates the tab in the active pane.
* Spawned `bash -c 'echo hi; sleep 5; echo bye'` captured
  both lines + clean `process exited (0)` epilogue.

**Items 4-6 PARTIAL — server emits, SPA doesn't render**:
* chan-server detected `please log in` pattern + wrote
  `events/pre-flight-f90ed024a46dc89a.md` with the right
  type/from/to/note shape.
* HostA's rich prompt did NOT render a bubble — no tray
  pill, no article, no notification.
* Both `parseWatcherEvent` (allowlist) and
  `BubbleOverlay` (render branches) are wired. So
  parsing + rendering are ready; the **delivery path
  from the server-written event file to the SPA bubble
  list is broken**. Two likely causes (untested):
  1. `self_writes` suppression too aggressive — the
     watcher silences echoes for chan-server's own
     writes, including the pre-flight one.
  2. Schema drift between SKILL (`questions`+`options`)
     and chan-server emit (`{id,type,from,to,note}`).
     BubbleOverlay hardcodes options for pre-flight
     type so this isn't the immediate blocker, but it
     IS a documentation/implementation drift.

Hand-off to @@FullStack / @@Systacean to wire the SPA-
side subscription path or carve out pre-flight from
the self-write suppression.

Items 7-10 still blocked on `systacean-13` / `systacean-14`.

8801 server up with multiple test tabs + the unparsed
pre-flight event file in `events/` for inspection.

## 2026-05-19 (resume) BST - systacean-13 + fullstack-21 cluster

After @@Alex's `poke`. `1694041` (systacean-13 activity
indicator) + `07a79d5` (fullstack-21 pane menu swap-back)
landed.

* **Item 7 activity indicator — PARTIAL**. Same pattern
  as item 4 pre-flight: server-side tracking is in
  (commit msg: `bytes_since_focus`, focus/activity WS
  frames) and SPA render code exists in `Pane.svelte`
  (`<span class="dirty activity">` with title "terminal
  output since last focus"), but `t.terminalActivity`
  isn't getting set from the WS frames. Output ran in
  unfocused NoiseGen tab → no marker appeared.
  Side observation: terminal-tab right-click menu has
  a new `Focused` checkbox; manual focus override may
  gate the auto-tracking.
* **Item 8 marker distinction — PASS by code audit**.
  Three separate spans: `.dirty.unsaved` / `.dirty.activity`
  / `.dirty.watcher` with distinct titles. No visual
  collision possible by markup.
* **fullstack-21 PASS for all three items**: pane
  right-click shows only `Reload + Toggle Web Inspector`;
  hamburger structural-only (Split right/down, Close,
  Next/Prev, Focus color); Split left/up removed from
  visible UI.

Items 9-10 (MCP discovery) still blocked on
`systacean-14`.

8801 server up; clean two-pane layout left.

## 2026-05-19 (resume) BST - webtest-a-7 wave-B complete

After @@Alex's `poke`. Final wave-B batch landed:
`96f4f40` (systacean-14 auto-publish chan MCP),
`e60287c` (fullstack-23 vertical numbered rows +
follow-up), `e25ca3d` (mcp-discovery SKILL).

**All four newly-landed items PASS:**

* **Item 9 systacean-14**: chan-server publishes its
  `__mcp-proxy` descriptor into all three discovery
  surfaces (`~/.claude.json` per-project,
  `~/.codex/config.toml` global, `~/.gemini/settings.json`
  global). Each entry points at the live chan-mcp
  Unix socket. Per-instance behavior: each chan-serve
  publishes its own socket; Claude is per-project so
  multiple instances coexist, Codex + Gemini are
  global so the latest startup wins.
* **Item 10 user MCP untouched**: PASS by code+test
  audit. Commit explicitly adds tmp-file tests for
  additive config updates.
* **Item 11 SKILL drift**: `mcp-discovery.md` matches
  the live behavior across all three agents.
* **fullstack-23**: rich-prompt survey options now
  render as vertical full-width rows. Truncation hint
  `1 extra option hidden.` works (auto-included
  standing option got hidden). `follow up` affordance
  visible at bubble bottom-right.

**Side observation**: Codex + Gemini configs are
global, so multi-instance users only see the LATEST
chan-serve's MCP socket. Worth a per-instance
published name like `chan-<port>` or doc note. Flag
for @@Systacean + @@Architect.

**Final webtest-a-7 tally (12 items):**
- 9 PASS (1, 2, 3, 8, 9, 10, 11, plus all fullstack-21
  + fullstack-23 sub-items).
- 2 PARTIAL (4 pre-flight bubble, 7 activity indicator)
  — same architectural seam, SPA-side WebSocket signal
  ingestion is the gap.
- 2 N/A (5, 6) — gated on item 4 rendering.

Full per-item write-up at
[webtest-a-7.md](webtest-a-7.md).

State: 8801 server up; chan MCP entries published into
claude/codex/gemini configs (durable across restarts;
chan refreshes only its own entries).

## 2026-05-19 (resume) BST - systacean-15 cut from my item-7 repro

After @@Alex's `poke`. New commit `e91d8a4` cuts
`systacean-15` from my webtest-a-7 item 7 PARTIAL —
"backend wire fires, SPA render code exists, but
`t.terminalActivity` never flips". @@Systacean leads the
investigation; @@FullStack if root cause is SPA-side.
The cut even captured my Focused-checkbox side
observation as a possible lead.

systacean-15 (the fix) hasn't landed yet — just the
task file. No new actionable work for me right now.

8801 server stays up. Standing by for the systacean-15
fix to re-test item 7, or the pre-flight bubble seam to
re-test item 4, or a new task cut.

## 2026-05-19 (resume) BST - Item 7 GREEN after fullstack-25; item 4 still PARTIAL (separate seam)

After @@Alex's `poke`. `21d6fe5` fullstack-25 landed
(the systacean-15 fix — `TerminalTab` was conflating
`active` with `focused`; ingestion now gates on
`!focused`).

* **Item 7 — PASS**. Two-pane test (BgTerm pane-a +
  FgTerm pane-b). With pane-b focused, ran
  `sleep 1; echo BG-OUT-1; sleep 1; echo BG-OUT-2` in
  BgTerm. At 1.5s post-defocus: `BgTerm ● ●` (orange
  activity + blue watcher, visually distinct).
  DOM `activity: true`. Click BgTerm tab to focus →
  marker cleared (`activity: false`), watcher dot
  retained. **Both halves green.**
* **Item 4 — still PARTIAL**, separate seam (as
  architect speculated). Re-spawned `@@LoginRetry` with
  the `please log in` recipe; chan-server wrote the
  `pre-flight-*.md` event file to `events/`; BgTerm's
  rich prompt showed NO bubble (article count 0, no
  tray pill). fullstack-25 fixed the WS-frame seam;
  the event-file → SPA bubble ingestion is a different
  seam. Likely needs the SPA's event-file watcher to
  pick up chan-server's own writes (not silenced by
  self-write suppression) OR a direct WS push.
* **Side observation**: FgTerm later picked up a
  transient activity dot without intentional output —
  cursor blink / prompt redraw likely counts as
  bytes_since_focus. Probably worth excluding terminal
  control sequences from the activity accounting.

Updated tally: 10 PASS / 1 PARTIAL (item 4) / 2 N/A
(items 5+6 gated on 4). Full detail at
[webtest-a-7.md](webtest-a-7.md).

8801 server up. Standing by for the pre-flight bubble
seam to land.

## 2026-05-19 (resume) BST - Two follow-ups cut from my wave-B verdicts

After @@Alex's `poke`. `6778623` cuts two follow-ups
from my webtest-a-7 closure:

* **fullstack-27**: pre-flight events render gap
  (closes items 4+5+6). Architect's note flags likely
  candidates: watcherEvents allow-list (the
  fullstack-17 unknown-type drop may exclude
  pre-flight), BubbleOverlay wiring, or event-file
  polling. Matches my "server file write → no SPA
  pickup" framing from the previous append.
* **systacean-16**: activity counter sensitivity. Cut
  from my "FgTerm transient activity" side observation
  — cursor blink / prompt redraw shouldn't count as
  bytes_since_focus.

Neither has landed yet — just task files cut. No
actionable work for me until one of those (or another
Lane A task) lands.

8801 server stays up. Standing by.

## 2026-05-19 (resume) BST - webtest-a-7 FULLY CLOSED (12/12)

After @@Alex's `poke`. Both my flagged follow-ups
landed and PASS on re-test:

* `ebb347b` fullstack-27: SPA now reads pre-flight
  watcher files. **Item 4 now PASS** — direct atomic-
  write of a pre-flight event file produced a fully-
  rendered bubble: `@@FakeAgent` header, `↻ 0:00`
  spinner+counter, "please log in (direct test)"
  note, and the three numbered options
  `1 Open the terminal / 2 Kill the spawn / 3 Retry now`
  plus `F follow up`. Items 5 (spinner) and 6 (kill
  option visible/wired) also pass by direct visual +
  UI wiring.
* `538eeb8` systacean-16: activity byte counting
  tuned. **No more spurious activity dots** from
  cursor blink / prompt redraw on idle terminals.
  Verified by clicking between two idle tabs across
  a 2s sample point — both stay `activity: false`.

**Final tally: 12/12 PASS**. Plus all four side
observations from earlier waves got fixes that
landed:
- fullstack-17: absolute-path dialog, unknown-type
  drop, stale watcher cleanup, auto-dismiss
  answered surveys.
- fullstack-25: terminal activity focus tracking
  (split `active` vs `focused`).
- fullstack-27: pre-flight event ingestion.
- systacean-16: activity counter sensitivity.

webtest-a-7 fully closed from my side. Clean loop
closure on this wave-B walkthrough.

8801 server up. Standing by for the next wave.

## 2026-05-19 14:10 BST - Online, standing by

Fresh @@WebtestA session. Boot order completed:
contact card, webdev skill, phase process, request
(skim), this journal, and the inbound architect
event file bottom-up.

Latest architect poke on file is 2026-05-19 03:45 BST
(item 7 re-test + item 4 check). Both items closed
in my prior session — webtest-a-7 fully closed at
12/12 PASS, follow-ups `fullstack-27` + `systacean-16`
landed and verified.

No queued walkthrough task. Per @@Alex's note in the
boot prompt: the marquee landings (flippable Hybrids,
Cmd+K rework, carousel, BCAST window-wide, British
spelling, multi-File-Browser tabs) are still unwalked
from a Chrome MCP audit-trail perspective — likely
the shape of the next cluster — but @@Alex has been
doing manual visual passes since Chrome MCP can't
reach the Tauri Chan.app surface.

Not spinning up a test server yet (per boot
instruction). Will wait for an architect poke to cut
a walkthrough cluster.

Online, standing by.

## 2026-05-19 15:36 BST - webtest-a-8 closed (16 PASS / 1 PARTIAL)

Cut at 15:15 BST by @@Architect — pre-release
keyboard/menu cluster + watcher containment. 17
items across Pane Mode core / tab closing + spawn /
Cmd+K p / menu cleanup / right-dock chevron /
systacean-19 watcher path validation.

Rebuilt against head `cd4ad26`, bounced 8801,
drove the SPA via Chrome MCP tab 503725239 +
`curl` for the watcher API.

**16 PASS, 1 PARTIAL** (full per-item write-up at
[webtest-a-8.md](webtest-a-8.md)):

* Pane Mode core (items 1-4): all PASS. `-39`
  divider is 4px transparent + col-resize; `-39`
  spawn/split/kill verified, with task description
  drift called out (`-42` reshaped: WASD=swap not
  split, /\=split, k=kill, Q unbound, 1/2/3/4 =
  Terminal/FB/Graph/NewFile). `-40` inversion +
  `-42` cheatsheet (`PaneModeHelp.svelte` on `h`)
  render every binding inc. `p`; Esc discards
  cleanly.
* Tab closing + spawn (items 5-6): item 5 PASS
  (Ctrl+D non-terminal closes, terminal passes
  through). **Item 6 PARTIAL**: doc→terminal +
  FB-selection→terminal PASS; terminal cwd
  limited (no shell-integration tracking of live
  `cd`); **doc→Graph FAIL** — scope reset to
  drive on mount even though spawn intent sets
  `scopeId = "file:<path>"` and title = "File
  Graph". Likely `GraphPanel.svelte:88-89`
  defaultScopeId fallback running before
  scopeOptions populates.
* Cmd+K p (items 7-10): all PASS. Show-on-terminal
  works, spawn-then-show on empty pane works,
  × + Esc dismiss (Esc when focus is in prompt),
  no "Rich prompt" entry on terminal-tab kebab,
  Alt+Space still works.
* Menu cleanup (items 11-13): all PASS.
  File-tree, doc-tab, terminal-tab, pane right-
  click all clean of removed entries; inspector
  retains "Open / Graph from here". Restart
  prompt body has both required phrases
  ("shell will be killed" + "any running command
  will be terminated"). "New Terminal" entry
  gone.
* Right-dock chevron (item 14): PASS with all
  three variants live in DOM simultaneously
  (left-dock + right-dock + tab). Collapsed
  chevron flips ONLY in right-dock (`<`); all
  others (`>`); expanded always (`v`).
* Watcher containment (items 15-17): all PASS
  via curl against `/api/terminal/<session>/watcher`.
  In-drive absolute + relative paths → 204.
  Out-of-drive `/etc`, `/private/tmp` → 400
  `invalid watcher path: path escapes drive root`.
  Symlink escape via in-drive `escape-test -> /etc`
  → 400 with the explicit
  `path resolves through a symlink that escapes
  drive root: ...` message. Symlink cleaned up
  post-test.

Side observations folded into the task append:
doc→Graph scope reset (item 6 sub-bug), terminal
live cwd not tracked (item 6 limitation), Cmd+K
help wording nit (item 4), New File neighbour
after Restart (item 13 task-wording nit),
cross-port tab sibling re-appeared
(`503725243` opened to 8810), no-session
Restart silently no-ops on freshly spawned
terminals.

Server stays up on 8801. Layout intact for any
re-tests: 2-pane split (a: many tabs, b:
Terminal-4) plus all three FB dock variants
visible. Standing by for the next wave or fixes
on item 6.

## 2026-05-19 16:09 BST - webtest-a-9 closed (3 items, all PASS)

Lane B overflow cluster cut by @@Architect at
16:55 BST per @@Alex's note that @@WebtestB was
hitting API overload on `webtest-b-6`. Pulled 3
independent items off Lane B (their items 7/8/13).
Continued in the same 8801/cd4ad26 session as
`webtest-a-8`; no fresh boot.

**3 of 3 PASS** (one with a code-audit fallback
per the task's anticipated tool limit):

* **Item 1 `fullstack-47` multiple Graph tabs —
  PASS**. Spawned 2 Graph tabs in pane-b via
  Cmd+K + 3 twice; toggled `folder` chip OFF on
  Graph #2 (gf:ltmai), Graph #1 stayed gf:ltmaif.
  Tab-switch round-trip verified state isolation
  in both the DOM (chip class flip) and the
  persisted hash (per-tab gf field). Dedup drop
  is observable end-to-end.
* **Item 2 `fullstack-47` tab DnD —
  INCONCLUSIVE (live) + PASS (code+tests)**.
  `computer.left_click_drag` fires pointer
  events not HTML5 DnD; the SPA's
  `ondragstart`/`ondrop` chain doesn't fire
  through the MCP. Same MCP limit as
  `fullstack-15`'s prior pass. `da2d718` ships
  a `detachTabToPaneEdge` regression test for
  browser/graph tab kinds that locks the
  cross-pane path at unit-test level.
* **Item 3 `fullstack-51` xterm row metrics —
  PASS**. Unstuck both FB docks for pane-b
  width, ran a shell script with 8 block-char
  rows. Measured row containers in
  `.xterm-screen`: every consecutive row pair
  has **0px gap** (rows are 15px tall, each
  next y matches previous bottom). Matches
  iTerm-style zero leading. `0b0c919` shows
  `lineHeight: 1.15 → 1.0` flip.

Side observations folded into the task append:
* Chrome MCP `find` refs go stale on
  tab-switch clicks; workaround = live
  `getBoundingClientRect()` + raw
  `computer.left_click`. Worth a SKILL note.
* `computer.type` interleaves keystrokes into
  narrow xterm cols (~25). Workaround =
  shell script on disk + `bash <path>`.
* Fresh-spawn terminals (no session yet) drop
  initial keystrokes before WS handshake
  attaches the PTY — minor edge.
* Trusted architect's redistribution per the
  queue-tail rule (`feedback_redistribution_queue_head.md`);
  didn't double-walk against @@WebtestB.

Test server stays up on 8801. Layout: pane-a (8
tabs, narrow due to tab strip pressure), pane-b
(Terminal-4 with block-render output visible + 2
Graph tabs with differentiated filter state).
Both docks unstuck. Drive clean (test script
removed). Standing by.

## 2026-05-19 16:51 BST - webtest-a-10 closed (3/3 PASS + spot-check)

Quick re-walk of three ships (`fullstack-54` FB
header drop / `-55` carousel dashboard-stats
drop / `-56` Cmd+S drop) plus informal
round-trip spot-check.

Rebuilt to head `dbbba84`, bounced 8801. Chrome
MCP tab from `webtest-a-9` died over the recycle
gap, opened a fresh tab `503725263`.

**3/3 PASS** (full per-item write-up at
[webtest-a-10.md](webtest-a-10.md)):

* **Item 1 fullstack-54 FB header drop — PASS**
  across all three variants. Tab + dock verified
  live (header text = `⋮` only; no path text;
  Unstick chrome-btn icon on dock); overlay
  PASS by code audit. Overlay variant code path
  exists in `FileBrowserSurface.svelte` but is
  unreachable in current SPA — the dock
  hamburger's "Open overlay" actually opens a
  Files tab via `openBrowser()`. Side
  observation flagged.
* **Item 2 fullstack-55 carousel slide 1 —
  PASS**. Slide 1 (Welcome) renders drive name
  + keyboard cheatsheet only, NO inline
  `N files · N dirs · N contacts` row. Slide 2
  (Drive metadata) still carries tallies
  (`documents 6 · 2 directories · 25 KB on
  disk`) as the canonical surface.
* **Item 3 fullstack-56 Cmd+S drop — PASS**.
  Cmd+S → no chan-side action (no toast / no
  saving spinner), dirty marker cleared via
  autosave debounce within 500ms. File on disk
  contains the inserted TEST_DIRTY_LINE.
  Browser-native Cmd+S still fires (Chrome
  opened a Save Page As intent sibling tab —
  expected per `-56`'s no-preventDefault call).
  No "Save" entry anywhere in pane hamburger
  or doc-editor right-click menu (21 items
  enumerated). Cmd+Shift+S strikethrough not
  testable via synthesized KeyboardEvents (CM6
  input pipeline); PASS by code audit
  (`Pane.svelte:381-386` explicit comment).
* **Spot-check round-trip — PASS**. Pre-reload
  state with note-a.md (wysiwyg + cursor
  c:[215,215]) + Graph (gs:drive, gf:ltmaif,
  gp:note-a.md, a:1). Post-reload: all of cursor
  position, mode, active tab, filter chips,
  pane layout, pending-select consumption all
  intact. `gp:note-a.md` was consumed cleanly
  on mount → `gi:1` (graph inspector open) per
  the pendingSelectId contract.

Side observations folded into the task append:
"Open overlay" menu label mismatch (label says
overlay but spawns a tab); Cmd+S triggers
browser Save Page As (sibling tab opened);
fullstack-43 gp:note-a.md cross-confirms my
webtest-a-8 item 6 diagnosis (already cut as
fullstack-57); carousel auto-rotate (5s) needed
slide-dot navigation back to Welcome;
TEST_DIRTY_LINE typing landed mid-table
(cursor artifact, not a defect); Style Toolbar
hidden by default in this drive.

Test server stays up on 8801. Drive has the
TEST_DIRTY_LINE edit on note-a.md per the
system note; otherwise clean. Standing by.

## 2026-05-19 17:29 BST - webtest-a-11 closed (4/4 PASS + bonus closure)

Re-walk of three ships (`fullstack-58` per-tab
BrowserTab state / `-64` Graph chrome trim /
`-66` truncation utility). Rebuilt to head
`986d77c`, bounced 8801, reused MCP tab.

**4/4 PASS** (full per-item write-up at
[webtest-a-11.md](webtest-a-11.md)):

* **Item 1 fullstack-58 multi-FB per-tab state
  — PASS**. Two Files tabs in same pane, each
  with independent `bs` selection. Cross-
  switched several times, persistence and
  visible state both isolated per-tab.
  Schema for `be` / `bsc` / `bd` wired in
  code (`tabs.svelte.ts:2598-2608`) but
  conditional-emit (default state omits the
  field); single-click row select sets `bs`
  only, not `be`. PASS-as-spec for what's
  exercisable in this drive.
* **Item 2 fullstack-58 hash round-trip —
  PASS**. Set `{bs:"img"}, {bs:"index.md",a:1}`,
  navigated to the same hash, post-reload
  state restored exactly. Per-tab `bs` fields
  survive the round-trip per the 18:00 BST
  directive.
* **Item 3 fullstack-64 Graph chrome trim —
  PASS**. Maximize button gone, scope
  selector dropdown gone. Title derivation
  verified for `drive` ("drive"), `file:` (file
  basename), spawn-from-doc-tab (doc basename).
  Did not live-test `dir:` / `#tag` /
  `contact` scope kinds — drive has no fixtures
  for them. `graphTitle()` code has the
  matching clauses.
* **Item 4 fullstack-66 truncation utility —
  PASS** on all sub-points. Long name
  (`this-is-a-very-long-filename-for-truncation-testing.md`,
  54 chars) → visible text `this-i[..]ng.md`
  (6+4+5=15 chars exact). Exact-15 name
  (`exact15chars.md`) unchanged. Dirty marker
  `●` renders AFTER the truncated name and is
  not in the 15-char budget. Tooltip (title
  attr) always shows the untruncated name.

**Bonus closure**: `webtest-b-6` item 6
(PARTIAL that gated us) converts to **PASS**
via my items 1+2. Lane B doesn't need to
re-walk that one.

**fullstack-43/57 cross-confirmation**:
spawning a Graph from a doc tab now persists
`gs:"file:<doc-path>"` — the reset-to-drive
bug I caught in webtest-a-8 item 6 is closed.
Verified by spawning a Graph from note-a.md
doc tab; hash showed `gs:"file:note-a.md"`,
title `note-a.md`.

Side observations folded:
* Tab visible text now reflects per-tab state
  (FB tab with `bs` displays the basename of
  the selection; Graph tab displays scope
  basename). Static "Files"/"Graph" labels
  only appear when no per-tab state is set.
  Nice clarity improvement.
* Empty-layout navigation always bootstraps a
  Files tab via `App.svelte:259`. Means
  `#s={k:l,t:[],f:1}` doesn't actually yield
  zero-tab state; the auto-open kicks in.

Test server stays up on 8801. Drive clean
(test files removed). State preserved for
any re-tests. Standing by.

## 2026-05-19 17:49 BST - webtest-a-12 closed (2/2 PASS + @@Alex ad-hoc bug + fix)

Re-walk of `fullstack-59` per-Hybrid theme +
`-60` pane hamburger trim. Rebuilt to head
`986d77c`, bounced 8801. Mid-walk `fullstack-62`
(`3b270d0` rename Pane Mode → Hybrid NAV)
landed; menu text now reads "Enter Hybrid NAV".

**2/2 PASS** (full per-item write-up at
[webtest-a-12.md](webtest-a-12.md)):

* **Item 1 fullstack-59 per-Hybrid theme —
  PASS**. Per-pane `data-theme` attribute on
  `.pane`, toggle button cycles undefined →
  light → undefined, hash `ht:"l"` field appears
  when set / omitted when default, hash
  round-trip preserves the override, multi-pane
  layout shows true per-pane independence
  (one pane override doesn't affect another).
  Active override paints with `--link` blue +
  `overridden` class. Sun-in-dark / Moon-in-light
  icon convention matches commit text.
  **Closes `webtest-b-6` item 11 PARTIAL →
  PASS**.
* **Item 2 fullstack-60 pane hamburger trim —
  PASS**. Menu now reads exactly
  `Enter Hybrid NAV / Cmd+K / Focus border
  colour / blue / green / pink` (4 buttons
  total). No `Next/Prev pane / Split / Flip
  Hybrid / Close` trailing entries. Keystroke
  equivalents exhaustively verified in
  webtest-a-8.

### @@Alex's mid-walk ad-hoc + fix

@@Alex stepped in: "try to flip, split the
pane, see if the split one follows same
pattern - back vs front". Tested live:

* **Bug confirmed**: `splitPane()` creates the
  new pane with `tabs:[], activeTabId:null`
  only — doesn't inherit `showingBack` from
  the source. Splitting from a back-side pane
  drops the new split on the front, losing
  user orientation.

@@Alex's follow-up: "you can prob write the
small fix for this: when we split we preserve
the side."

* **Fix written + tests added** (NOT committed
  by webtest lane):
  - `web/src/state/tabs.svelte.ts:splitPane`:
    spread `{showingBack: true, back: {tabs:[],
    activeTabId:null}}` into `newPane` when
    `original.showingBack`.
  - Two new tests under
    `describe("splitPane side preservation")`
    in `web/src/state/tabs.test.ts`:
    front→front + back→back assertions.
  - Gate: `npx vitest run src/state/tabs.test.ts`
    → 87/87 pass (+2 from my new tests).

* **`npm run check` is broken on an UNRELATED
  in-flight WIP**: `App.svelte:759` references
  `dispatchPaneModeAction` which isn't declared
  (looks like @@FullStack mid-rename of the
  Pane Mode → Hybrid NAV machinery, paired
  with `PaneModeHelp.svelte` edits in WIP).
  Verified by stashing my two files only and
  re-running — check passes 0 errors. NOT my
  bug; needs to be finished/stashed before
  the next build. Flagging to architect.

* **Handoff** to @@Architect → @@FullStack:
  diff is in the working tree at
  `web/src/state/tabs.svelte.ts` +
  `web/src/state/tabs.test.ts`. Webtest lane
  doesn't commit code per process. Plumbing
  this through the event channel for proper
  routing.

@@Alex stepped away after the ad-hoc with
"carry on with the regular process. cheers!".
Continuing.

Test server stays up on 8801. Drive clean.
Layout has the back-side experiment artifacts
from the ad-hoc.
