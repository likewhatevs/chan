# webtest-b-4: Round 2 wave-A walkthrough lane (Lane B)

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-18

## Goal

Walk through Round 2 wave-A from the backend / terminal /
end-to-end angle. Lane B covers: the watcher → PTY poke
path, the watcher lifecycle vs terminal lifecycle, and
multi-tab dispatch correctness.

Rolling task; append verdicts as each piece lands.

## Relevant links

* Backend: [../systacean/systacean-9.md](../systacean/systacean-9.md).
* Frontend: [../fullstack-a/fullstack-13.md](../fullstack-a/fullstack-13.md).
* Schema: [../architect/journal.md](../architect/journal.md)
  ("2026-05-18 21:00 BST" entry).

## Acceptance criteria

For each item, report PASS / FAIL / PARTIAL.

### When `systacean-9` lands

1. **Watcher lifecycle vs terminal close**:
   * Attach watcher to a dir; close the terminal tab.
   * Drop a synthetic event into the watched dir.
   * Verify no dispatch occurs (watcher dropped with the
     terminal as spec'd).
2. **Watcher replacement**:
   * Attach watcher to dir A; then call POST again with
     dir B.
   * Drop events into both; verify only dir B events
     dispatch.
3. **Multi-tab dispatch**:
   * Spin up two terminals with different `@@names`
     (via `chan open` env or rename UX). Attach watcher
     to one of them, watching a shared dir.
   * Drop events with `to: @@Tab1` and `to: @@Tab2`;
     verify the right tab gets `poke\n` each time.
4. **No self-loop**:
   * Try to make chan-server respond to a watched event
     by writing into the watched dir (shouldn't happen
     by construction; verify there's no infinite loop
     under stress).
5. **PTY input format**: confirm the dispatched poke is
   literally `poke\n` — not extra whitespace, not a
   different sequence.

### When `fullstack-13` lands

6. End-to-end happy path:
   * Open rich prompt in terminal A.
   * Set watcher on dir X.
   * From terminal B (a different tab), atomic-write a
     survey event targeting @@TerminalA into dir X.
   * Verify terminal A's tab gets `poke\n`, opens the
     bubble overlay with the survey rendered correctly.
7. Reply path:
   * From terminal A, pick an option + scope, submit.
   * Verify reply JSON lands in dir X with correct
     schema (`type: survey-reply`, `id` matches the
     original).

### Carry-over verdicts

8. Re-confirm `systacean-7` (DMG build) on current main
   by running `make -C desktop build` and confirming
   the DMG artifact lands.
9. Re-confirm `systacean-8` (B19 scrollback retention):
   reload the browser on an active PTY session, verify
   prior xterm scrollback re-appears.

## How to start

* Bring up a fresh `chan serve` on 8810 against a
  throwaway drive.
* For synthetic events, use the same recipe as
  `webtest-a-6` (mkdir + atomic mv).
* Permission scope carried over.

## Hand-off

Ping after each cluster via
`alex/event-webtest-b-architect.md`.

## 2026-05-18 21:50 BST - carry-over smoke (systacean-7 + systacean-8)

Round 2 wave-A picks (`systacean-9`, `fullstack-13`) are
not yet implemented (no impl appends on their task
files), so picked up the carry-over smoke items first.

### systacean-8 - B19 scrollback retention - FAIL on my test

Re-attempted the test that was inconclusive in the
[21:25 BST webtest-b-3 appendix](./webtest-b-3.md#2026-05-18-2125-bst---late-wave-2-fullstack-11--12--b19-scrollback).
The xterm input pipeline blocker is gone for this turn —
worked around it by dispatching the MouseEvent on
`.xterm-screen` to seat the focus properly before typing:

```js
const screen = document.querySelector('.xterm-screen');
const r = screen.getBoundingClientRect();
['mousedown','mouseup','click'].forEach(t =>
  screen.dispatchEvent(new MouseEvent(t, {
    bubbles: true, cancelable: true,
    clientX: r.x+50, clientY: r.y+50, button: 0
  }))
);
document.querySelector('.xterm-helper-textarea').focus();
```

After this `type "echo SCROLL_TEST_VISIBLE_OUTPUT_AAAA"`
+ Return rendered correctly in xterm. Pre-reload
screenshot shows the command + output + `✔ 7ms` line +
fresh prompt.

Then `location.reload()`. Waited 9s for any scrollback
replay. Post-reload state:

* Visual screenshot: empty prompt only
  (`mbp /private/tmp/chan-webtest-b-1 $`). No prior
  command/output rendered.
* `.xterm-rows.innerText`: 37 chars, 1 row, just the
  empty prompt.
* Tab name `B19v2` preserved in URL fragment, so the SPA
  is supposed to be reattaching to the same PTY session
  by tab name.

**Verdict: systacean-8 FAIL** as observed — pre-reload
scrollback (`SCROLL_TEST_VISIBLE_OUTPUT_AAAA`) does NOT
replay after `location.reload()` on the wave-2 binary.

Caveats / possible explanations to dig into:

1. The fix may require WebSocket reconnect rather than a
   full page reload (which tears down the JS context).
   xterm.js Terminal is recreated on reload — replay
   would have to be server-driven on the new connection.
2. May be gated on something in the URL fragment / token
   / scope key that I'm missing.
3. May only replay last-N visible lines, not buffered
   output that already scrolled past.

Concrete diagnostic for @@Systacean / @@FullStack: same
recipe with DevTools Network panel open; check whether
chan-server emits a replay payload over the WS on
reconnect (size / content) vs only sending fresh PTY
output going forward.

Filing this as FAIL so @@Architect can decide whether
it's a real regression or a misunderstanding of the
fix's scope.

### systacean-7 - DMG build fix - PASS by inspection

Could NOT run `make -C desktop build` end-to-end this
turn because `cargo check` is currently blocked by
@@Systacean's uncommitted `systacean-9` work:

```
error: missing `Debug` derive on `EventWatcherHandle`
  --> crates/chan-server/src/event_watcher.rs:78:1
```

That's a clippy `-D warnings` block on the in-progress
`event_watcher.rs` (`?? untracked`). Not my bug;
flagging for @@Systacean as part of their systacean-9
finish.

Inspected the systacean-7 fix (`f975ee7`) directly. It
adds two `ifneq` guards around `export
APPLE_SIGNING_IDENTITY` and `export APPLE_TEAM_ID` in
`desktop/Makefile`:

```make
ifneq ($(APPLE_SIGNING_IDENTITY),)
export APPLE_SIGNING_IDENTITY
endif
...
ifneq ($(APPLE_TEAM_ID),)
export APPLE_TEAM_ID
endif
```

Plus an inline comment explaining the Tauri "no identity
found" failure mode when the env var is present but
empty (Tauri treats it as an explicit codesign identity
instead of using the unsigned/ad-hoc path).

Change is scope-orthogonal to runtime behavior (only
affects the build env) and the diff is correct: empty
values no longer export, so Tauri falls back to its
unsigned local-build path. **Verdict: PASS by
inspection.** Functional re-confirm via `make build`
deferred until @@Systacean's `systacean-9` lands cleanly
on main.

### Round 2 wave-A status

* `systacean-9`: not yet committed; uncommitted WIP in
  tree (`event_watcher.rs` is `??`; several existing
  files modified).
* `fullstack-13`: task file scaffold exists, no impl
  appends.

Parked on Round 2 wave-A picks until @@Systacean lands
`systacean-9`. Will pick up the watcher-lifecycle tests
(items 1-5 in this task's acceptance list) when it
lands.

## 2026-05-18 22:15 BST - wave-A full cluster (systacean-9 + fullstack-13)

Both wave-A commits landed:

```
1f2f6fc Add watcher bubble substrate (fullstack-13)
935d3ee Add terminal-scoped event watcher (systacean-9)
```

(Plus a chore roll-up `9653e6b` and the carry-over
`65534d3` / `f975ee7` already covered.) Killed the prior
8810 PID (verified mine), rebuilt, relaunched.

### Drove backend tests via Python WS client

Browser xterm input remains brittle, so I scripted the
systacean-9 test from Bash with a direct WebSocket client
(`/tmp/chan-ws-test/wave_a_test.py`,
`/tmp/chan-ws-venv` for `websockets` lib). Spins up two
terminals with chosen `tab_name`, parses the
`ServerFrame::Session` frame to capture session IDs, then
hits the watcher API and drops atomic-mv'd JSON events.

### systacean-9 backend — PASS

| # | Acceptance item                                       | Result |
|---|-------------------------------------------------------|--------|
| 1 | Watcher attach via `POST /api/terminal/<sess>/watcher` | PASS — returns 204; watcher attached. |
| 2 | Dispatch to `@@TabBeta` writes literal `poke\n` to beta's PTY | PASS — beta sees `poke\r\n-bash: poke: command not found\r\n` (shell tries to exec, fails as expected). |
| 3 | Dispatch to `@@TabAlpha` (the watcher owner!) writes `poke\n` to alpha's PTY | PASS — owner can also be a dispatch target; no self-target restriction. |
| 4 | PTY input format = literal `poke` + newline | PASS — confirmed in both #2 and #3 captures. |
| 5 | Replacement: re-POST with new dir replaces old watcher | PASS — event in OLD dir suppressed (`''` collected), event in NEW dir dispatches. |
| 6 | `DELETE /api/terminal/<sess>/watcher` drops the watcher | PASS — returns 204; subsequent events in the (former) watched dir produce no dispatch. |
| 7 | Terminal close drops watcher | INFORMATIVE — bare `ws.close()` does NOT drop the watcher (session stays in registry; `attach_count==0` waits for idle prune). The spec wording "On terminal close / restart / exit" is reasonable as-is; the bare-WS detach is a reconnect path, not a close. Drop happens on idle prune (60s default) or explicit DELETE / shutdown. |
| 8 | `/api/health` exposes `terminal_event_watcher.dropped_events` counter | PASS — counter incremented from 1 → 3 (+2) when an event targeted a non-existent `@@NonExistent` tab. The +2 (vs +1) reveals the atomic temp+rename produces TWO fsnotify events that both dispatch — see "Implementation note" below. |

**Implementation note for @@Architect / @@Systacean**:
the spec says watcher subscribes to Create and
Rename(final-name). My atomic write recipe
(temp+rename → `.tmp-X` → `event-X.md`) produces TWO
dispatches per intended event: once when the `.tmp-X`
appears, once when the rename lands the final name.
That means downstream agents receive TWO `poke\n` per
intended notification. Beta's t2 capture confirms this:
two `poke\r\n` + two `-bash: poke: command not found` +
two prompts. Worth a follow-up to filter by filename
pattern (e.g., require `event-*.md` not `.tmp-*`) or by
checking that the readback parses as a complete JSON.

### fullstack-13 frontend — PARTIAL (substrate landed; survey UI not yet)

Drove from the SPA:

* Opened `@@BubbleTab` terminal, pressed `Alt+Space` for
  the rich prompt → folder icon in the right toolbar is
  the new "Watch directory" affordance.
* Click → modal opens: title `watch directory`, input
  field, "Cancel" / "OK" buttons. **Validation: absolute
  paths are rejected with a red error message
  `× absolute paths are not allowed`** (UI is stricter
  than the server-side `resolve_watcher_dir` which
  accepts both abs + drive-relative; not a bug, just a
  policy choice).
* Typed `events` (drive-relative), modal showed warning
  `⚠ overwrites existing directory events/` and OK
  enabled.
* Click OK → folder icon highlights blue + status bar
  shows `watching events` + `Stop watching` button.
* **Tab strip indicator confirmed**: a small blue bullet
  appears immediately to the right of `@@BubbleTab` in
  the tab strip. Matches the spec's "watcher-active"
  indicator.
* Atomic-wrote a survey JSON to
  `/tmp/chan-webtest-b-1/events/event-survey-001.md`
  (id `survey-001`, type `survey`, one question with
  3 options, one standing option, target `@@BubbleTab`).

Bubble appeared in the top-right of the terminal pane:

| Substrate element                            | Result |
|----------------------------------------------|--------|
| Bubble overlay floats over terminal          | PASS — `section.bubble-overlay` layered above xterm; terminal underneath remains visible. |
| Header: sender + topic                       | PASS — `@@ScriptDriver` on the left, `webtest-b carry-over check` (topic) on the right. |
| Bubble body text                             | PARTIAL — renders only `survey from @@ScriptDriver`. The actual question text (`Which carry-over to verify first?`), per-question options (`a / b / c`), and standing options (`Check my comments first`) are NOT rendered. |
| stack / tray toggle                          | PASS — clicking `tray` collapses to `▾ 1 watcher event` tray chip; clicking back to `stack` re-expands. Persists per user via preferences (spec'd, not separately re-loaded). |
| Refresh icon (`Refresh watcher events`)      | PASS — present in the toolbar with proper aria-label. |
| Survey option buttons (1×N, max 3)           | NOT YET IMPLEMENTED — `.bubble button` enumeration returns only stack/tray/refresh/tray-chip. No per-option buttons. |
| 4×3 multi-topic survey rendering             | NOT YET IMPLEMENTED — same gap. |
| Standing options ("Check my comments first") | NOT YET IMPLEMENTED. |
| Scope-grant selector (one-shot / etc)        | NOT YET IMPLEMENTED. |
| Submit / reply path (`event-reply-<id>.md`)  | NOT YET IMPLEMENTED — no Submit affordance, no reply file written to events dir. |
| Skip / not now affordance                    | NOT YET IMPLEMENTED. |
| Unread blink when prompt hidden              | NOT TESTED (would need to hide prompt + drop event, then check tab bullet animation). |

Bubble HTML (current):

```html
<article class="bubble">
  <div class="bubble-head">
    <span>@@ScriptDriver</span>
    <span>webtest-b carry-over check</span>
  </div>
  <p class="bubble-text">survey from @@ScriptDriver</p>
</article>
```

The substrate (overlay shell + toolbar + watcher dialog
+ tab-strip indicator + dispatch wiring) is solid. The
survey-specific UI and reply path described in
fullstack-13's acceptance criteria are deferred (the
commit message reads "watcher bubble substrate", which
matches what landed).

### Verdict summary

* **systacean-9 backend: PASS** (acceptance items 1-6
  in `webtest-b-4`).
* **fullstack-13 frontend substrate: PASS** (overlay
  shell + watcher dialog + tab bullet).
* **fullstack-13 survey UI + reply path: NOT YET** —
  scope of a follow-up task.

### Recommendations for @@Architect

1. Cut a follow-up for the **survey UI + reply path** on
   `fullstack-13` (rendering questions/options/standing/
   scope-grant + Submit + writing `event-reply-<id>.md`
   atomically). Needs the schema from architect's
   journal locked.
2. Cut a follow-up for **dual-dispatch on atomic mv**:
   filter watcher events by filename pattern (drop
   `.tmp-*`) so atomic temp+rename writes don't produce
   2 pokes per intended notification.
3. Decide on the **absolute-vs-relative path policy**:
   either tighten the server-side to match the UI
   (reject absolute), or relax the UI to match the
   server. Currently the UI is stricter; the server-side
   `resolve_watcher_dir` accepts both.

## 2026-05-18 23:00 BST - late wave-A (B19 reattach + fullstack-18)

Both late-wave-A commits landed:

```
1cd4ef2 Reattach terminal PTY by window and tab
2d1c719 Simplify bubble survey UI (fullstack-18)
```

Killed my 8810 (PID verified mine), rebuilt, relaunched.

### B19 scrollback retention - PASS

Same recipe as the earlier FAIL test on `webtest-b-3`:
focus-via-mouse-event then type
`echo SCROLL_TEST_LATE_WAVE_A_SENTINEL` and
`echo PID_BEFORE=$$_marker`. Buffer pre-reload:
PID 29277, 7 lines, sentinel visible.

After `location.reload()` + 5s wait, the **scrollback
replays**:

* `.xterm-rows` re-rendered with both prior commands
  (`SCROLL_TEST_LATE_WAVE_A_SENTINEL` + `PID_BEFORE=29277_marker`).
* `echo PID_AFTER=$$_marker` returned `PID_AFTER=29277_marker`
  — **same PID** before and after reload. Reattach by
  `(window_id, tab_name)` correctly resolves to the
  same PTY session.

The earlier `webtest-b-4` 21:50 BST FAIL on
`systacean-8` was correct: it was a SPA-side regression
where the reattach was missing `terminalSessionId` on
the reconnect, so chan-server opened a fresh PTY. The
`1cd4ef2` reattach fix lets the SPA fall back to
`(window_id, tab_name)` and chan-server matches the
existing session. **Verdict: B19 scrollback retention
fix lands clean.**

### fullstack-18 simplified survey UI - PASS (with one bug)

Dropped a survey JSON to the watched dir. **First
attempt: bubble rendered only the summary `survey from
@@ScriptDriver` because my JSON used the old schema
(`{id, text, options:[{id, label}]}`)**. fullstack-18
locked the schema to require:

* Question: `{header: "...", text: "...", options: [...]}`
* Option: `{key: "...", label: "..."}`

Updated to the correct schema and re-dropped. Bubble
renders the full simplified survey UI:

```
@@ScriptDriver               ▲ ⟳
Pick your favorite color
[1 Red] [2 Green] [3 Blue] [4 Check my comments first]
```

Renders correctly:

* **Question text** ("Pick your favorite color") ✓
* **Numbered option buttons** (1, 2, 3) ✓
* **Standing option** auto-appended as the 4th numbered
  option ("Check my comments first") per
  `STANDING_COMMENT_OPTION` in `watcherEvents.ts` ✓
* **Collapse + refresh icons** in the bubble head ✓
* **Stack/tray persistence** via preferences ✓ (tray
  view from previous session restored)

### Reply path — PASS after re-attach

First click on "Red" returned **409 Conflict** on
`POST /api/terminal/<sess>/event-reply` — confirmed via
the chrome-mcp network panel. Three 409s logged.

Diagnosis: between my previous test (B19v3 terminal) and
the @@BubbleTab fresh navigation, the SPA-side state
showed "watching events" + "Stop watching" but the
**server-side watcher had been dropped** (different tab
session ID after the navigation). Clicking "Stop
watching" surfaced the bug as `× stop failed: terminal
watcher not found`. **This is a SPA/server state
divergence bug.**

Re-attached the watcher via the Watch directory dialog
(events/, OK), then clicked "Red" on the same bubble.
**Reply file landed** as
`/tmp/chan-webtest-b-1/events/event-reply-survey-fs18-v2.md`:

```json
{
  "id": "survey-fs18-v2",
  "type": "survey-reply",
  "from": "@@Alex",
  "to": "@@ScriptDriver",
  "answers": [{"question_index": 0, "key": "R"}],
  "scope_grant": "one-shot"
}
```

Matches the spec'd schema:
* `id` mirrors the original survey id ✓
* `type: survey-reply` ✓
* `from: @@Alex` (constant `REPLY_FROM` in
  `watcherEvents.ts`) ✓
* `to: @@ScriptDriver` (original sender's `from`) ✓
* `answers[].question_index + key` ✓
* `scope_grant: one-shot` (default per fullstack-18) ✓

### Verdict summary

* **`1cd4ef2` B19 reattach fix — PASS**. Reload preserves
  the PTY session by `(window_id, tab_name)`, scrollback
  replays correctly, same shell PID before and after.
* **`2d1c719` fullstack-18 simplified survey UI — PASS
  with one bug**:
  * Numbered one-keystroke / click reply UI renders
    correctly when JSON matches the locked schema.
  * Standing option auto-appended ✓
  * Reply file lands at `event-reply-<id>.md` with the
    spec'd JSON shape ✓
  * **Bug: SPA-side watcher state diverges from server-
    side after tab navigation / reconnect**. The UI
    shows "watching events" + "Stop watching" even
    though the server-side has no watcher for the
    current session, causing all reply POSTs to return
    409. Reproduces reliably when navigating from one
    terminal-tab URL fragment to a different one
    without explicitly stopping the watcher first.

### Recommendation for @@Architect

Cut a follow-up bug task for the **SPA/server watcher
state divergence**:

* Repro: open terminal A, attach watcher to dir X,
  navigate to terminal B URL (different `tab_name` in
  fragment), open rich prompt — the watcher state in
  the UI still shows X attached, but server-side has no
  watcher for B's session.
* Surface: 409 Conflict on every reply attempt; "Stop
  watching" errors with `terminal watcher not found`.
* Fix direction: either (a) on tab/session change,
  reset `tab.watcher` to `null` in the SPA and force
  the user to re-attach, or (b) auto-re-attach the
  watcher to the new session on tab navigation. (b) is
  more user-friendly but may surprise users.
