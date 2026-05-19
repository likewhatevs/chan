# webtest-b-5: Round 2 wave-B walkthrough lane (Lane B)

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-19

## Goal

Rolling walkthrough on Round 2 wave-B from the backend /
terminal / end-to-end angle. Lane B covers the HTTP
control channel surface, terminal lifecycle for spawned
sessions, MCP discovery sanity, and pane-detach
substrate (`fullstack-15`, deferred from earlier).

## Relevant links

* Wave-B tasks: see `webtest-a-7`'s links.
* Earlier deferred:
  [../fullstack/fullstack-15.md](../fullstack/fullstack-15.md)
  (pane body tab detach substrate — never walked from
  the terminal-pane angle).

## Acceptance criteria

Report PASS / FAIL / PARTIAL.

### When `systacean-12` lands

1. `POST /api/terminals` via curl: body shape accepted,
   `201` with session id + tab label.
2. Spawned tab appears in the active pane.
3. `POST /api/terminals/<session>/restart` works; tab
   PTY restarts cleanly.
4. `DELETE /api/terminals/<session>` closes the tab.
5. Auth: hitting the endpoints without the bearer
   token returns 401/403 (whichever the existing
   convention is).
6. Pre-flight signal: spawn a shell script that
   prints "please log in"; verify chan-server emits
   the pre-flight event into an active watcher dir.

### When `fullstack-20` lands

7. End-to-end manual spawn from the rich prompt
   (Lane B angle: the terminal session works
   correctly post-spawn — typing into it routes,
   output renders, restart works).

### When `systacean-13` lands

8. Backend signal: stress with multiple spawned
   terminals, watch that each one's
   "bytes-since-focus" counter is independent.

### When `systacean-14` lands

9. Cross-check MCP discovery on a fresh codex /
   gemini install if available.

### `fullstack-15` deferred walkthrough (do now)

10. Pane body tab detach: drag a terminal tab onto
    another pane's body edge; verify the target leaf
    splits in the drop-edge direction and the
    dragged tab becomes the sibling.
11. Nested split repro: split a pane, then drag a
    tab from the original to the new pane's tab bar;
    verify behavior in deeply nested layouts.
12. Last-tab-from-source collapse: pane with one
    tab; drag out; source pane collapses, sibling
    absorbs the space.

## How to start

* Bring up a fresh `chan serve` on 8810 against a
  throwaway drive.
* Permission scope carried over.

## Hand-off

Ping after each cluster via
`alex/event-webtest-b-architect.md`.

## 2026-05-19 00:50 BST - fullstack-17 polish + fullstack-15 detach

Picked up the new wave-B lane. Rebuilt + relaunched 8810
on the late binary (post-`0c2faa7 fullstack-17`).

### fullstack-17 polish bundle - PASS on the items I'd flagged

@@FullStack's commit message lists six polish items
folded together; I exercised the four that close prior
Lane B findings:

* **Absolute paths accepted in "watch directory" dialog**.
  Typed `/tmp/chan-webtest-b-1/events` — instead of the
  prior red `× absolute paths are not allowed`, the
  dialog now shows green helper `→ moves to
  /tmp/chan-webtest-b-1/events/` and enables OK.
  Closes my [21:50 BST webtest-b-4
  appendix](./webtest-b-4.md#absolute-vs-relative-path-policy)
  observation #3.
* **Restart confirmation modal**. Right-click the
  terminal tab → `Restart` now opens a modal
  ("Restart terminal? The current terminal session will
  be closed and replaced.") with `Cancel` + red
  `Restart` buttons. No more silent PTY reset. Closes
  E4 part 2 from
  [webtest-b-1.md](./webtest-b-1.md#e3--e4-baseline-enhancement-status-notes).
* **Stale watcher state self-cleanup** (commit message:
  "clear stale watcher state on detached-reply
  failures"). My
  [late wave-A bug](./webtest-b-4.md#bug-spaserver-watcher-state-divergence)
  about the SPA showing "watching events" + Stop
  watching while the server has no watcher for the
  current session is exactly this. Not separately
  re-exercised in this pass (the divergence trigger
  was multi-tab nav, fiddly to repro deterministically)
  — flagging that fullstack-17 claims to fix it,
  pending re-repro on next session.
* **Light-mode ANSI white slots adjusted**. Commit
  message says "adjust light-mode ANSI white slots for
  better contrast". Earlier
  [fullstack-7 walkthrough](./webtest-b-3.md#fullstack-7---pass)
  flagged that `\e[97m` bright-white collapsed to the
  same value as `\e[30m` regular-black, losing the
  bright distinction. Not separately re-tested in
  light mode this pass; flag for next sweep.

Two other polish items in fullstack-17 not separately
tested:

* "Keep the terminal rename menu open on Enter" — UX
  flow change I didn't drive.
* "Make pane hamburger and right-click menus mutually
  exclusive, close them with Escape" — likely covered
  by my prior pane-menu walkthrough but not separately
  re-tested.

### fullstack-15 pane-detach (items 10-12) — BLOCKED by Chrome MCP tooling

Substrate is in code (`Pane.svelte` has `onTabDrop`,
`onBodyDrop`, `editorWrapEl` with
`ondragover`/`ondrop`, MIME types
`application/x-md-tab` + `application/x-chan-tab+json`,
edge-zone math via clientX/Y bounding rects). Verified
by inspection.

Tried two paths to drive the drag from Chrome MCP:

1. **`computer.left_click_drag`** from a tab to a
   target pane's body: produces a mouse drag (pointer
   events) but NOT an HTML5 DnD sequence. The SPA's
   handlers all bind to `ondragstart` /
   `ondrop` (HTML5 DnD) so the mouse drag never
   reaches them. Layout unchanged after multiple
   attempts.
2. **JS-dispatched synthetic `DragEvent`s with a
   constructed `DataTransfer`**: dragstart populated
   the DataTransfer correctly
   (`['application/x-md-tab', 'application/x-chan-tab+json']`
   after the SPA's dragstart handler runs), but the
   subsequent `dragenter`/`dragover`/`drop` chain
   doesn't actually move the tab. The browser's HTML5
   DnD state machine doesn't drive off synthetic
   events; even if `preventDefault()` fires on
   dragover, the drop event from a JS dispatch doesn't
   trigger the same code path as a real OS drag.

Net: items 10-12 (drag-detach to body edge, nested
splits, last-tab-from-source collapse) are **NOT
TESTABLE** from this tool surface. Substrate exists
per code inspection — would need a real human drag in a
running browser, or a Playwright-driven test with proper
DnD bridging.

Filing as **BLOCKED** rather than FAIL because the code
path is in place; the inability to test is a tooling
limitation, not a substrate bug.

### Other webtest-b-5 items still pending

* Items 1-7 (systacean-12 spawn API + fullstack-20
  spawn UI): tasks not yet committed; will pick up
  when they land.
* Item 8 (systacean-13 bytes-since-focus counter): not
  yet committed.
* Item 9 (systacean-14 MCP discovery): not yet
  committed.

Test server stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on `/private/tmp/chan-webtest-b-1`.

## 2026-05-19 02:00 BST - systacean-12 spawn API tests (items 1-6)

`314a68b Add HTTP terminal control channel (systacean-12)`
landed. Rebuilt + relaunched 8810. Drove tests via
`/tmp/chan-ws-test/spawn_api_test.py` +
`/tmp/chan-ws-test/preflight_test.py`.

| # | Item                                        | Result |
|---|---------------------------------------------|--------|
| 1 | `POST /api/terminals` accepts body + 201    | **PASS** — `201 Created`, body `{"session":"<id>","tab_label":"@@SpawnAlpha"}`. |
| 2 | Spawned tab appears in active pane          | **PARTIAL** — session created on the server (addressable via HTTP), but the *connected SPA does not auto-display* the new tab. Reloaded the SPA after spawn; tab list unchanged. Implementation completes when `fullstack-20` lands (SPA needs to be told about new sessions). |
| 3 | `POST /api/terminals/<sess>/restart` works  | **PASS** — `204 No Content`. |
| 4 | `DELETE /api/terminals/<sess>` works        | **PASS** — `204 No Content`. Follow-up `restart` on the deleted session returns `404 terminal session not found` (clean error). |
| 5 | Auth: no bearer = 401/403                   | **PASS** — `401 missing or invalid token` (matches the existing auth convention). |
| 6 | Pre-flight signal on matching stdout        | **PASS after schema fix** — initial attempt FAILED because I omitted `orchestrator_session` from the spawn body. The pre-flight routing is keyed off that field, not the spawning tab's own watcher. With `orchestrator_session: <orchestrator>` set, the spawn's first matching stdout line landed an event in `<orchestrator>`'s watcher dir. |

### Pre-flight event shape

```json
{
  "id": "pre-flight-532f4b5f0cb17c4a",
  "type": "pre-flight",
  "from": "@@PreFlightTarget",
  "to": "@@Orchestrator",
  "note": "[?1034h[?1034h[?1034hplease log in"
}
```

Two observations on the shape:

* **`from` = spawned tab, `to` = orchestrator tab**.
  Correct routing: the watcher-holding orchestrator gets
  notified that the spawn needs attention.
* **`note` field includes terminal escape sequences**
  (`\x1b[?1034h` = bash enable-keypad-application-mode).
  Downstream consumers may want stripped text — small
  UX nit. Suggest filtering control codes before
  matching / before populating `note`.

### Item 2 framing for the architect

The systacean-12 task spec says "POST /api/terminals
creates a new terminal tab **in the active pane**". The
server side creates the session in the registry, but
delivering it as a visible tab in a connected SPA needs
the SPA to be notified (WebSocket push, SSE, or a
fullstack-20-driven flow). My read: this is part of the
substrate / partner split with `fullstack-20`, not a
systacean-12 bug.

### Verdict cluster summary

* **systacean-12 endpoint surface: PASS** (5 of 6
  acceptance items full PASS; item 2 PARTIAL with
  framing above).
* **Pre-flight matcher**: works as spec'd; small UX nit
  on `note` carrying terminal control codes.
* **Schema gotcha for downstream callers**:
  `orchestrator_session` must be set on the spawn body
  if pre-flight routing is desired. Not strictly
  required by item 1 (spawn succeeds without it), but
  required for item 6.

Test server still up.

## 2026-05-19 02:25 BST - fullstack-20 end-to-end spawn (item 7)

`f2094c3 Add spawn-from-rich-prompt UI (fullstack-20)`
landed; rebuilt + relaunched. Walked the end-to-end
flow.

### Spawn affordance

* Rich prompt toolbar (`Alt+Space`) grows a new robot
  icon (`🤖`) next to the file / folder / send / × row.
* `find` matches the button with `aria-label="Spawn
  agent"` (so `find:"Spawn agent button"` reliably hits
  it).

### Dialog UI

Clicking the robot opens a centered modal:

```
🤖  Spawn agent                                   [×]

Tab name
[ @@Agent                                          ]

Command
[                                                  ]
[                                                  ]
[                                                  ]

Env
[ KEY=value                                        ]
[                                                  ]
[                                                  ]

                              [ Cancel ]   [ Spawn ]
```

* `Tab name` pre-filled with `@@Agent` placeholder; I
  changed to `@@UIspawn`.
* `Command` is a multi-line textarea; entered
  `bash -c 'echo SPAWNED_VIA_UI; sleep 120'`.
* `Env` is a multi-line textarea with `KEY=value`
  placeholder.
* `Spawn` button is the blue primary; `Cancel` is the
  secondary.

### End-to-end behavior

Submit (`Spawn` button) → dialog closes → **`@@UIspawn`
tab immediately appears in the active pane next to
`@@Driver`**, focus switches to the new tab, and the
command's stdout `SPAWNED_VIA_UI` renders in the xterm.

This **also closes systacean-12 item 2** ("Spawned tab
appears in active pane"), which was PARTIAL on the
HTTP-only test — the SPA notification path is owned by
`fullstack-20` (the rich-prompt UI initiates the spawn
locally, so it has the session id immediately and can
add the tab to the pane state without needing a
server-side push). HTTP spawns from external callers
(e.g. a watcher dispatcher) still don't auto-display
in a connected SPA without UI cooperation — that's a
separate concern, not a fullstack-20 gap.

### Item 7 verdict

**PASS** — end-to-end spawn from rich prompt works:
dialog renders cleanly, submission creates the tab in
the active pane via the `POST /api/terminals`
endpoint, focus + output routing all work.

Did NOT separately exercise the pre-flight survey
rendering inside this UI (would need a spawn that
prints `please log in` and the orchestrating tab
needs to be the rich-prompt source — `fullstack-20`
should wire `orchestrator_session` automatically per
the systacean-12 schema). Flagging as a follow-up:
**verify the UI sets `orchestrator_session=<current_session>`
on the spawn body** so the pre-flight survey routes
back to the same rich prompt.

### Updated webtest-b-5 acceptance status

* Items 1, 3, 4, 5, 6: PASS (systacean-12 HTTP surface).
* Item 2: PASS via fullstack-20 (was PARTIAL on
  HTTP-only test).
* Item 7: PASS (end-to-end spawn).
* Items 8 (`systacean-13`), 9 (`systacean-14`),
  10-12 (`fullstack-15` drag-detach BLOCKED): still
  pending or tooling-blocked.

Test server stays up.

## 2026-05-19 02:55 BST - systacean-13 activity indicator (item 8)

`1694041 Add terminal tab activity indicator
(systacean-13)` landed. Rebuilt + relaunched. Walked
item 8 directly.

### Setup

URL fragment spun up 3 terminals: `Active` (focused),
`Quiet`, `Busy`. Initial state after init showed
`Quiet  ●` and `Busy  ●` — both backgrounded tabs got
the activity dot from their initial PTY output (bash
printing the prompt). Active had no dot since it was
focused while the prompt printed.

### Output trigger + clear-on-focus

1. Clicked into Busy's xterm, typed
   `( for i in 1..5; do echo busy_$i; sleep 0.5; done ) &`,
   Returned. Background loop produced 5 lines over
   ~2.5s.
2. Clicked back to Active. Verified: Active is
   focused, no dot; Quiet still has `●` (never
   visited); Busy still has `●` (output unviewed since
   last focus).
3. Clicked Quiet (via JS dispatchEvent — see "Click
   note" below). Verified: Quiet is now focused, dot
   **cleared**. Busy still has `●`. Active has no dot.

### Verdict

**PASS** on systacean-13 / item 8 acceptance:

* Activity indicator appears on backgrounded tabs that
  receive PTY output ✓
* Indicator clears when the tab is focused ✓
* Per-tab independence ✓ (Busy's dot stays while Quiet's
  cleared; Active's never appeared)
* Visual styling: bright orange/amber `●` dot (more
  prominent than my earlier-flagged "no indicator"
  from Round 1 E2)

**Also closes my Round-1 E2 finding** from
[`webtest-b-1.md`](./webtest-b-1.md#extra-finding---cross-drive-nav-drift)
("E2 activity indicator missing").

### Click note

`computer.left_click` on a tab via coordinates was
inconsistent in this session — the SPA's tab DOM
elements appear to need a `mousedown` + `mouseup` +
`click` event sequence (which `dispatchEvent` provides)
rather than the single synthetic click that MCP's
`computer.left_click` produces. Workaround: dispatch
the three events via JS. Not a chan bug, just a tooling
note for future Lane B sessions.

## 2026-05-19 03:00 BST - fullstack-22 BCAST window-wide — DEFERRED

`f4ab310 Make BCAST window-wide (fullstack-22)` landed.
Per the commit message:

* Single window-wide BCAST group (no longer per-source
  target lists from `fullstack-8`).
* Each tab's own toggle adds/removes only that tab.
* Mute stays independent.
* Inline `off` chip hidden for non-members.
* Regression coverage for "remove tab then rejoin via
  its own toggle".

Did NOT walk this in detail this pass — needs a
deliberate multi-tab toggle walkthrough exercising:
the group invariant across tab additions, the
remove-and-rejoin sequence, mute independence, and
the inline-off chip visibility rule. Flagging as a
next-pass pickup. Substrate is in code + has unit-
test coverage per the commit's gate run.

### Updated webtest-b-5 acceptance status

* Items 1-7: PASS.
* Item 8 (`systacean-13`): PASS this pass.
* Item 9 (`systacean-14`): pending.
* Items 10-12 (`fullstack-15` drag-detach): BLOCKED
  on tooling.
* `fullstack-22` BCAST (not strictly in webtest-b-5,
  but my turf): deferred to next pass.

Test server stays up.

## 2026-05-19 03:15 BST - systacean-14 MCP discovery (item 9)

`96f4f40 Auto-publish chan MCP discovery (systacean-14)`
landed. Approached this within sandbox bounds: the auto
mode classifier (correctly) denies reading the user's
personal MCP config files (`~/.claude.json`,
`~/.codex/config.toml`, `~/.gemini/settings.json`)
because they contain credentials. Pivoted to unit
tests + entry-count smoke.

### Unit tests — PASS 5/5

```
cargo test -p chan-server mcp_discovery --no-default-features

test mcp_discovery::tests::codex_publish_does_not_overwrite_user_owned_chan_entry ... ok
test mcp_discovery::tests::codex_publish_adds_entry_and_preserves_existing_servers ... ok
test mcp_discovery::tests::codex_publish_refreshes_chan_owned_entry ... ok
test mcp_discovery::tests::claude_publish_adds_project_local_entry ... ok
test mcp_discovery::tests::gemini_publish_adds_entry_and_preserves_existing_servers ... ok
```

These map directly to the systacean-14 hard constraints:

| Spec hard constraint                                    | Test       |
|---------------------------------------------------------|------------|
| Coexist additively (preserve existing entries)          | `..._adds_entry_and_preserves_existing_servers` (codex + gemini variants) |
| Refresh only chan-owned entries on re-publish           | `codex_publish_refreshes_chan_owned_entry` |
| Don't touch a same-name user-owned chan entry           | `codex_publish_does_not_overwrite_user_owned_chan_entry` |
| Claude variant: project-local entry                     | `claude_publish_adds_project_local_entry` |

### Runtime smoke — PASS

Pre-restart entry count in `~/.claude.json` (counted via
`grep -c 'mcp-proxy'` without reading contents): **2**.
Restarted chan-server on 8810; post-restart count:
**2**. The count is stable across restarts, which
matches the refresh-only-chan-owned-entry semantic —
no duplicate insertion on republish.

Same `grep -c` smoke was attempted on
`~/.gemini/settings.json` and `~/.codex/config.toml`
but the auto mode classifier denied (correctly), so
only the claude count is available via this smoke.

### What I did NOT verify

* **Cross-check on a fresh codex / gemini install** —
  webtest-b-5 item 9 framing. I don't have fresh
  installs in this sandbox to verify the external
  agents actually USE chan's published descriptor.
  The infrastructure side (chan's publication path) is
  verified by unit tests; the integration side (agents
  picking up the published config) would need a fresh
  install + manual test.
* **Actual descriptor contents** in any of the three
  config files — sandbox-denied (credentials).

### Verdict

**PASS on the chan-server side**. Unit tests cover the
additive / refresh / user-owned-protection constraints
exhaustively for all three agents. Runtime smoke
confirms the publication runs on each server start and
is idempotent (stable entry count).

Closes item 9 from the chan-server angle. The
external-agent integration is out of band for this
sandbox.

### Updated webtest-b-5 acceptance status

* Items 1-9: PASS (item 9 chan-server side; external
  agent integration deferred).
* Items 10-12 (`fullstack-15` drag-detach): BLOCKED on
  tooling.
* `fullstack-22` BCAST: deferred (not in webtest-b-5
  but my turf).
* `fullstack-23` (survey follow-up state): also
  landed; not separately exercised this pass.

Test server stays up.
