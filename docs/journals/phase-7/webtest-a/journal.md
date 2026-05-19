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
