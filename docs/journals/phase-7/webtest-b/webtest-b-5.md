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
