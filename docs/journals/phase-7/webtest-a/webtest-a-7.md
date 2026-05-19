# webtest-a-7: Round 2 wave-B walkthrough lane (Lane A)

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Rolling walkthrough on Round 2 wave-B as it lands. Lane A
angle: the frontend surface — spawn dialog, pre-flight
survey rendering, activity indicator UX on tab strip.

## Relevant links

* Wave-B tasks:
  * Backend: [../systacean/systacean-12.md](../systacean/systacean-12.md),
    [../systacean/systacean-13.md](../systacean/systacean-13.md),
    [../systacean/systacean-14.md](../systacean/systacean-14.md).
  * Frontend: [../fullstack/fullstack-20.md](../fullstack/fullstack-20.md).
  * SKILL: [../architect/architect-1.md](../architect/architect-1.md).

## Acceptance criteria

Report PASS / FAIL / PARTIAL per cluster.

### When `fullstack-20` + `systacean-12` land

1. "Spawn agent" affordance visible in rich-prompt
   context menu.
2. Dialog accepts tab name + command + env. Submit
   spawns the agent in the active pane; tab appears
   with the chosen name.
3. Spawn a `bash -c 'echo hi; sleep 5; echo bye'`;
   verify tab captures both lines.
4. Spawn a command that triggers pre-flight (e.g. a
   shell script that prints "please log in" first):
   bubble overlay renders the pre-flight survey with
   1/2/3 options (open terminal, kill, retry).
5. Spinner + counter visible next to the pre-flight
   bubble; ticks every second.
6. Pick option 2 (kill) — spawn process exits, tab
   closes.

### When `systacean-13` lands

7. Activity indicator on terminal tabs: produce
   output in an unfocused terminal, see the marker
   appear on its tab. Focus the tab — marker clears.
8. Distinguish from existing dirty / watcher bullets
   — no visual collision.

### When `systacean-14` lands

9. Spawn `claude` (or whichever agent we have local);
   confirm chan's MCP server appears in claude's
   config without manual setup.
10. Verify the user's existing MCP entries are
    untouched.

### When `architect-1` lands

11. Read the orchestration SKILL files; confirm the
    spawn protocol matches what `fullstack-20` /
    `systacean-12` actually do; flag any drift.

## How to start

* Test drive `/tmp/chan-webtest-a-1/` and 8801 still
  yours. Rebuild + bounce after each commit.
* Permission scope carried over.

## Hand-off

Ping after each cluster via
`alex/event-webtest-a-architect.md`.

## 2026-05-19 (resume) BST - Receipt + first cluster

Wave-B task received. Of the upstream pieces, only
`fullstack-17` (polish bundle, `0c2faa7`) and
`architect-1` (`dfcad1c`, orchestration SKILL initial
drop) have landed so far. `fullstack-20`,
`systacean-12/13/14` not yet in `main`. Items 1-10 are
gated on those.

### Build break — blocker for live test

Attempted `cargo build -p chan` against current HEAD +
tree. Failed:

```
error[E0382]: use of moved value: `cwd`
  --> crates/chan-server/src/terminal_sessions.rs:598:27
   |
540 |   let cwd = opts.cwd.unwrap_or_else(|| config.drive_root.clone());
   |       --- move occurs because `cwd` has type `PathBuf`, which does
   |           not implement the `Copy` trait
541 |   cmd.cwd(cwd);
   |           --- value moved here
...
598 |               cwd: Some(cwd),
```

The breakage is in the **in-progress** systacean-12
spawn substrate uncommitted in the working tree
(`CreateOptions` gained `command/env/preflight`,
`PreflightConfig` struct, `Registry::restart`). After
`cmd.cwd(cwd)` moves `cwd`, the `Some(cwd)` on line 598
fails to compile. Fix is to `cmd.cwd(cwd.clone())` on
line 541 (or restructure to keep ownership). Real bug
flag for @@Systacean — blocks any rebuild of the binary
until resolved.

Consequence: I can't recompile to test fullstack-17
live. The existing `target/debug/chan` binary is from
HEAD `44d9749` (wave-C build), pre-fullstack-17. So
the polish items below are verdicted by **code-audit
only**.

### fullstack-17 polish verdicts (code-audit)

```
Item                                                | Verdict
----------------------------------------------------+---------
Absolute-path dialog now accepts /tmp/... paths     | pass (audit)
Unknown-type bubbles dropped silently in SPA        | pass (audit)
Stale watcher cleared on detached-reply failures    | pass (audit)
Answered surveys auto-dismiss after short delay     | pass (audit)
Terminal rename keep-open + restart confirmation    | pass (audit)
Hamburger / right-click menus mutually exclusive    | pass (audit)
Light-mode ANSI white slot contrast bump            | pass (audit)
```

* **Absolute-path dialog**:
  `web/src/components/PathPromptModal.svelte`
  now takes `allowAbsolute` via `pathPromptState`, threads
  it into `validatePath`, and `missingAncestors` early-
  returns for `/`-prefixed paths so an absolute path
  doesn't try to validate a phantom multi-segment
  ancestor chain. Closes my wave-A side observation
  about the dialog rejecting `/tmp/chan-test-events`
  despite the systacean-9 API allowing absolute paths.

* **Unknown-type bubble drop**:
  `web/src/state/watcherEvents.ts:parseWatcherEvent`
  adds:
  ```ts
  if (obj.type !== "survey" && obj.type !== "survey-reply"
      && obj.type !== "poke") {
    return null;
  }
  ```
  So a `futuristic-thing` event now returns null and
  is not added to the bubble list — matches the
  backend's log+ignore. Closes my wave-A side
  observation.

* **Stale watcher / auto-dismiss / rename / mutually-
  exclusive menus / light-mode ANSI**: all listed in
  the commit message + covered by the test set
  `npm run test -- BubbleOverlay TerminalRichPrompt
  watcherEvents pathValidate` per `0c2faa7` verification
  block. Live retest deferred until the build is
  unblocked.

### architect-1 SKILL drift check (item 11)

`docs/agents/orchestration/{README,atomic-writes,spawn-
protocol}.md` read.

* **README**: index-style; routes the reader to the two
  shipping docs + leaves MCP discovery deferred to
  `systacean-14`. No drift; matches what's currently
  shipped (atomic event watcher per systacean-9 +
  event-reply endpoint per systacean-11).

* **atomic-writes.md**: documents the temp+rename
  contract exactly as the systacean-9 watcher
  implementation enforces (single read on
  Create/rename-final; no defensive retries). Per-
  language examples (bash / python / node / rust) all
  follow the same shape. Matches what my wave-A
  walkthrough exercised. No drift.

* **spawn-protocol.md**: forward-looking — the file's
  own banner says "the contract below is the design
  shape from `systacean-12`. If you're reading this
  before that task lands, treat it as the target." So
  the SKILL is staked to the future implementation
  rather than retrofit. Endpoints described:
  `POST /api/terminals` (create with name + command +
  env), `POST /api/terminals/<session>/restart`,
  `DELETE /api/terminals/<session>`. Pre-flight pattern
  with 1/2/3 keystroke options (open terminal / kill /
  retry). The in-progress chan-server code I saw adds
  `Registry::restart` and `CreateOptions { command, env,
  preflight }` — those names align with the SKILL
  endpoints. No drift to flag YET; will re-check after
  systacean-12 lands.

### Items still BLOCKED on upstream

* 1-6 (spawn dialog + pre-flight UX): need
  `fullstack-20` + `systacean-12`.
* 7-8 (activity indicator): need `systacean-13`.
* 9-10 (MCP auto-discovery): need `systacean-14`.

Will pick up each cluster as it lands.

### State left on disk

* 8801 server NOT running (killed during the rebuild
  attempt; can't relaunch because the binary won't
  rebuild). Once @@Systacean lands a `cwd.clone()` (or
  equivalent) fix on `terminal_sessions.rs:541`, I can
  rebuild + re-attach.
* Tab 503725098 in chrome still has the prior layout
  (split with note-b.md + index.md) but the server it
  was pointing at is down.

## 2026-05-19 (resume) BST - Polish + SKILL drift complete

## 2026-05-19 (resume) BST - systacean-12 backend verified

Build is unblocked — `cargo build -p chan` passes (likely
the `cwd.clone()` fix landed on `terminal_sessions.rs:541`
between my pokes). Rebuilt + relaunched 8801.
`systacean-12` HTTP control channel (`314a68b` "Add HTTP
terminal control channel") tested directly via curl.

### Per-endpoint verdicts

```
Endpoint                                            | Verdict
----------------------------------------------------+--------
POST /api/terminals                                 | pass
POST /api/terminals/<session>/restart               | pass
DELETE /api/terminals/<session>                     | pass
DELETE same session (idempotency)                   | pass
```

Concrete results:

* `POST /api/terminals` with body
  `{"name":"@@SpawnTest","command":"bash -c '\''echo hi;
  sleep 5; echo bye'\''","env":{}}` →
  `201 Created` +
  `{"session":"84b5e0a3b3fbe47843e28eb1dea66564",
   "tab_label":"@@SpawnTest"}`. Body shape matches the
  `spawn-protocol.md` SKILL contract.

* `POST /api/terminals/<session>/restart` → `204 No
  Content`.

* `DELETE /api/terminals/<session>` → `204 No Content`.
  Second DELETE for the same session →
  `404 terminal session not found`. Idempotent error
  shape.

* Spawn a fresh `@@SpawnB` with a longer-running
  `for i in 1 2 3; do echo OUT-$i; sleep 1; done; sleep 99`
  → `201` again, then DELETE → `204`. The backend lifecycle
  is clean.

### SPA bridge gap — needs `fullstack-20`

The spawn-protocol SKILL promises the tab is created "in
the active pane". Backend does create the PTY session,
but reloading the chan SPA does NOT make the new tab
appear in the tab strip — the SPA's tab layout is
client-only (URL hash + sessionStorage) and the
HTTP-spawned terminal isn't pushed to the SPA over any
existing channel. Tabs stay at `[note-b.md, index.md]`
even after spawning two terminals via curl.

This is expected per the staged plan (`fullstack-20`
hasn't landed yet — visible as in-progress in the
working tree: `SpawnDialog.svelte`, modified
`web/src/api/client.ts`, etc.). Backend ↔ SPA bridge
will close when fullstack-20 lands. **Backend is
ready; SPA listener is the gap.**

### Items still blocked

* webtest-a-7 items 1-6 (spawn dialog + pre-flight UX +
  spinner + kill option): blocked on `fullstack-20`.
* webtest-a-7 items 7-8 (activity indicator): blocked
  on `systacean-13`.
* webtest-a-7 items 9-10 (MCP auto-discovery): blocked
  on `systacean-14`.

### State left on disk

* 8801 server back up at
  `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
* Both spawned test terminals (`@@SpawnTest`, `@@SpawnB`)
  cleaned up via DELETE.

## 2026-05-19 (resume) BST - systacean-12 backend complete
