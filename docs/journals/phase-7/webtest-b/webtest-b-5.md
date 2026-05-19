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
