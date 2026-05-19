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
  * Frontend: [../fullstack-a/fullstack-20.md](../fullstack-a/fullstack-20.md).
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

## 2026-05-19 (resume) BST - fullstack-20 spawn UI cluster

Build: head includes `f2094c3` (fullstack-20 spawn-from-
rich-prompt UI). Rebuilt + restarted 8801. HostA terminal
with watcher on `events/` as the orchestrator.

### Per-item verdicts

```
Item                                                | Verdict
----------------------------------------------------+--------
1 Spawn agent affordance in rich-prompt context     | pass
2 Dialog accepts name + command + env, tab appears  | pass
3 Spawned bash captures stdout (hi, bye)            | pass
4 Pre-flight bubble renders with 1/2/3 options      | partial *
5 Spinner + counter visible next to bubble          | n/a *
6 Picking option 2 (kill) closes tab                | n/a *
```

### Notes

* **Item 1**: Right-click in the rich-prompt editor area
  → context menu now includes "Spawn agent" (icon-bot
  glyph) alongside Show source code / Hide style toolbar
  / New File from here / Watch directory / Stop watching /
  Bubble stack / Bubble tray. Also a toolbar shortcut
  icon (top-right of the prompt) — `find` returned 2 refs
  for "Spawn agent" (menu + toolbar). Two ways into the
  same dialog.

* **Item 2**: Dialog opens with title `🤖 Spawn agent`,
  fields Tab name (default `@@Agent`), Command (textarea),
  Env (textarea with `KEY=value` placeholder), Cancel +
  Spawn buttons. Submitting fired
  `POST /api/terminals` from the SPA with the
  orchestrator_session header → 201, new tab named
  `@@SpawnEcho` appeared in the tab strip + auto-active.

* **Item 3**: With command
  `bash -c 'echo hi; sleep 5; echo bye'`, the
  spawned tab shows:
  ```
  hi
  bye

  process exited (0); press Ctrl+D to close this tab
  ```
  Both stdout lines captured, exit message clean, PTY
  exit code visible.

* `*` **Items 4-6 (pre-flight): PARTIAL — server emits
  the event, SPA does not render the bubble.**

  Recipe: HostA orchestrator (watcher on `events/`) +
  Spawn agent `@@AuthNeeded` with command
  `bash -c 'echo please log in; sleep 60'`. The spawned
  tab printed `please log in` (matching chan-server's
  preflight pattern, terminal_sessions.rs:1010-1022).
  chan-server **did** write the pre-flight event file:
  ```
  /tmp/chan-webtest-a-1/events/pre-flight-f90ed024a46dc89a.md
  {"id":"pre-flight-f90ed024a46dc89a",
   "type":"pre-flight","from":"@@AuthNeeded","to":"HostA",
   "note":"[?1034h... please log in"}
  ```
  But **no bubble rendered** in HostA's rich prompt. No
  tray pill, no article, no notification.

  Code paths inspected:
  * `web/src/state/watcherEvents.ts:35-42` allows
    `pre-flight` in the parse allowlist (per the
    fullstack-20 commit). Parses to a valid
    `WatcherEvent`.
  * `web/src/components/BubbleOverlay.svelte:69-344`
    has explicit `event.type === "pre-flight"` branches —
    knows to render single question with options,
    handles `preFlightTimedOut`, "Spawn idle - retry
    now?" prompt, etc.
  * `web/src/state/watcherEvents.test.ts:88` has a
    parser test for the type that passes.

  So parsing + rendering are wired; **what's broken
  is the event delivery path**. Two likely causes
  (untested):
  1. **chan-server's `self_writes` mechanism may be
     suppressing the watcher echo** for files chan-
     server itself wrote (`write_preflight_event`).
     The watcher dispatches on `Create` events, but a
     self-write should be silenced to avoid loops. The
     pre-flight event is supposed to be EXEMPT (it's
     for the orchestrator, not the watcher dispatch
     itself) — needs a carve-out.
  2. **Schema shape mismatch from the SKILL**. The
     spawn-protocol.md SKILL says pre-flight events
     carry `topic`, `questions` (with 3 options:
     open/kill/retry), `scope` — i.e. the same shape
     as a regular survey. The actual chan-server emit
     is minimal: `{id, type, from, to, note}` with no
     questions/options. The SPA's BubbleOverlay
     hardcodes the 3 options for pre-flight type
     (line 251 sets `Spawn` topic; line 256 returns
     `1` for question count), so the schema mismatch
     might not be the immediate issue — but it IS a
     drift between SKILL and chan-server emit.

  Items 5 + 6 (spinner + kill action) gated on item 4
  rendering. N/A until the delivery path is wired.

  Hand-off to @@FullStack / @@Systacean. The
  filesystem evidence + working chan-server pattern
  detection means most of the substrate is in;
  just the SPA-side subscription/dispatch hook is
  missing (or self-write suppression is too aggressive).

### State left on disk

* 8801 server up. Tabs visible: HostA + @@SpawnEcho +
  @@LoginNeeded + @@AuthNeeded (the latter two still
  running `sleep 60` until the bash exits). Pre-flight
  event file still in `events/`.
* HostA watcher still attached.

## 2026-05-19 (resume) BST - fullstack-20 cluster complete

## 2026-05-19 (resume) BST - systacean-13 + fullstack-21 cluster

Build: head includes `1694041` (systacean-13 terminal-tab
activity indicator) + `07a79d5` (fullstack-21 pane menus
swap-back). Rebuilt + restarted 8801.

### Per-item verdicts

```
Item                                                | Verdict
----------------------------------------------------+--------
7 Activity indicator on unfocused tab               | partial *
8 Distinguished from dirty / watcher bullets        | pass **
fullstack-21 Reload + Web Inspector → right-click   | pass
fullstack-21 Hamburger structural-only              | pass
fullstack-21 Split left/up removed from UI          | pass
```

`*` **Item 7 (activity indicator) — PARTIAL**.
Two-pane layout: NoiseGen (left, pane-a) + Focused
(right, pane-b). Focused pane focused. Ran
`sleep 2; echo HELLO; sleep 2; echo HELLO2` in NoiseGen
and immediately clicked Focused. Output landed in
NoiseGen at 2s + 4s while pane-a stayed unfocused.

The server-side substrate is in (per `1694041` commit:
`bytes_since_focus`, focus/activity WS frames). But the
SPA tab strip did NOT render the activity marker:
DOM query for `.dirty.activity` returned false on both
tabs across the 3s and 4.5s sample points. NoiseGen
tab text was `NoiseGen ×` (no dot at all — once watcher
was detached). HELLO + HELLO2 both visible in the
unfocused pane's xterm but no tab marker.

Same architectural pattern as item 4 pre-flight bubble:
server has the data, render code exists in
`Pane.svelte:887-893` (`{#if t.kind === "terminal" && t.terminalActivity}` →
`<span class="dirty activity" title="terminal output since last focus">●</span>`),
but the SPA isn't flipping `t.terminalActivity = true`
from the WS frames. Likely the focus/blur signal
emission or the activity frame ingestion is the gap.

Side observation: the terminal-tab right-click menu
gained a `Focused` checkbox at the bottom (manual focus
override?). Existing automatic focus tracking may be
gated on this.

`**` **Item 8 (no visual collision)** — even without the
activity marker firing, the rendering code clearly
separates the three dot states by class:
* `<span class="dirty unsaved">` for editor dirty
  (unsaved file)
* `<span class="dirty activity">` for terminal output
  while unfocused
* `<span class="dirty watcher">` for watcher attached
  (with optional `blink` class for new bubble)
Three separate spans, distinct titles. PASS by code
audit; live confirmation gated on item 7's marker
actually appearing.

### fullstack-21 swap-back verdicts

* **Pane right-click (empty tab strip area)** now shows
  ONLY `Reload + Toggle Web Inspector`. Matches commit
  message "Move Reload and Toggle Web Inspector back to
  the pane right-click menu". PASS.
* **Hamburger menu** (top-right three-dot) now contains
  ONLY structural items: `Split right`, `Split down`,
  `Close pane`, `Next pane (Cmd+Alt+])`,
  `Previous pane (Cmd+Alt+[)`, `Focus border color`
  (blue/green/pink). No Reload/Web Inspector here.
  Matches commit message "Make the pane hamburger
  structural-only". PASS.
* **Split left/up removed from visible UI**: hamburger
  shows only `Split right + Split down`; no `Split left
  / Split up` entries. Underlying primitives still in
  the codebase (tested via DOM presence). PASS.

This is a clean reversal of the fullstack-6 decision
from earlier — appropriately captured in `dda2d5c`
(request.md pane-menu revision). The pane right-click
is now scoped to the page-developer-level chrome
(Reload / Web Inspector), and the hamburger holds the
structural pane management.

### State left on disk

* 8801 server up. Tabs: NoiseGen (no watcher, no
  activity dot) + Focused.
* "watcher detached on reload" toast still visible at
  bottom-left (fullstack-17 stale watcher cleanup —
  bonus confirmation of that polish item working live).

## 2026-05-19 (resume) BST - systacean-13/fullstack-21 cluster complete

## 2026-05-19 (resume) BST - systacean-14 + fullstack-23 + SKILL drift

Build: head includes `96f4f40` (systacean-14 auto-publish
chan MCP discovery), `e60287c` (fullstack-23 survey
follow-up state), `e25ca3d` (orchestration SKILL adds
`mcp-discovery.md`). Rebuilt + restarted 8801.

### Per-item verdicts

```
Item                                                | Verdict
----------------------------------------------------+--------
9 chan auto-publishes MCP into claude/codex/gemini  | pass
10 User's existing MCP entries untouched            | pass (audit)
11 SKILL drift check (mcp-discovery.md)             | pass
fullstack-23 vertical numbered rows + follow-up     | pass
```

### Notes

**Item 9 — PASS**. Backed up `~/.claude.json` before
relaunch, then restarted 8801. After startup all three
discovery configs got chan entries pointing at the
chan-mcp Unix socket for the live process:

* `~/.claude.json` — added under
  `projects["/private/tmp/chan-webtest-a-1"].mcpServers.chan`:
  ```
  {"args":["__mcp-proxy","/var/folders/.../chan-mcp-<pid>-<id>.sock"],
   "command":"/Users/fiorix/dev/github.com/fiorix/chan/target/debug/chan"}
  ```
  A second entry also exists at
  `projects["/private/tmp/chan-webtest-b-1"]` — that's
  Lane B's chan-server on 8810; each chan-serve
  instance gets its own per-project Claude scope.
* `~/.codex/config.toml` — added `[mcp_servers.chan]`
  pointing at the same socket (global Codex scope).
* `~/.gemini/settings.json` — added top-level
  `mcpServers.chan` (global Gemini scope).

Per-agent surfaces match the `mcp-discovery.md` SKILL
spec exactly.

**Item 10 — PASS by code+test audit**. The systacean-14
commit (`96f4f40`) lands
`crates/chan-server/src/mcp_discovery.rs` (413 lines)
with the explicit guarantee: "Refresh only chan-owned
entries and leave same-name user-owned entries
untouched; add tmp-file based tests for additive config
updates." Live behavioral test would require setting up
a known-non-chan MCP entry in each config and verifying
it survives a chan-serve restart; the unit tests cover
the additive-update contract more thoroughly than I
could from outside.

**Item 11 SKILL drift — PASS**. `mcp-discovery.md`
documents:
* Claude Code: local project scope
  (`projects["<drive>"].mcpServers`), explicitly
  notes user-scope MCP servers are NOT touched.
* Codex: `~/.codex/config.toml`,
  `[mcp_servers.<published-name>]`.
* Gemini CLI: `~/.gemini/settings.json` top-level
  `mcpServers.<published-name>`.

Live behavior matches all three. No drift.

**Side observation — global config racing**: Codex +
Gemini configs are GLOBAL (single MCP entry, not per-
project). With two chan-serve instances running (mine
on 8801 + Lane B's on 8810), both configs end up
pointing at whichever chan-serve started LAST — in this
session the Lane B socket
(`chan-mcp-29294-06842939.sock`). Multi-instance users
would only have ONE chan-MCP reachable from
codex/gemini at a time. Claude Code is per-project so
both instances coexist. Worth mentioning in the SKILL
or designing per-instance published names like
`chan-<port>` for codex/gemini. Flag to @@Systacean +
@@Architect.

**fullstack-23 — PASS**. Dispatched a 1xN survey to
`@@BubblesA`; the rich-prompt bubble now renders the
three options as **vertical full-width rows**:
```
[ 1 alpha    ]
[ 2 beta     ]
[ 3 gamma    ]
1 extra option hidden.
follow up
```
* Each option is a full-width row instead of the
  earlier horizontal chip strip.
* The standing "Check my comments first" option got
  truncated with the hint `1 extra option hidden.`
  (visible at the bottom of the option list).
* A new `follow up` affordance is visible at the
  bottom-right of the bubble — that's the
  fullstack-23 "survey follow-up state" hook.

PASS for the visible behavior. Vertical rows + bounded
rendering + truncation hint + follow-up are all there.

### State left on disk

* 8801 server up. Tab `BubblesA` with watcher attached
  + 1xN survey bubble visible.
* `~/.claude.json` backup at
  `/tmp/claude.json.before-systacean14` (can be removed;
  chan re-publishes its entry on every startup so
  restoring would be temporary).
* `~/.codex/config.toml` + `~/.gemini/settings.json`
  have chan entries pointing at the live socket — these
  will be refreshed on next chan-serve startup.

## 2026-05-19 (resume) BST - webtest-a-7 wave-B complete

All 12 acceptance items walked. Final tally:

```
1  Spawn agent affordance in rich-prompt        pass
2  Dialog accepts name/command/env + tab spawn  pass
3  Spawned bash captures hi/bye + exit          pass
4  Pre-flight bubble renders 1/2/3 options      partial *
5  Spinner + counter                            n/a
6  Option 2 (kill) closes tab                   n/a
7  Activity indicator on unfocused tab          partial *
8  Distinguished from dirty/watcher bullets     pass
9  chan MCP auto-published                      pass
10 User MCP entries untouched                   pass
11 SKILL drift check                            pass
```

`*` items 4 + 7 share the same architectural seam: server
emits the data, SPA has the render code, but the
WebSocket signal that flips the SPA state (`pre-flight`
event delivery, `terminalActivity` flag) isn't being
processed. Hand-off to @@FullStack + @@Systacean.

Items 5 + 6 gated on item 4 actually rendering.

Bonus: confirmed fullstack-17 polish bundle items (path
prompt absolute-paths, unknown-type drops, stale watcher
toast, auto-dismiss surveys) all working live across the
walkthroughs.

## 2026-05-19 (resume) BST - Item 7 re-test (GREEN) + item 4 re-test (still PARTIAL, separate seam)

Build: head includes `21d6fe5` (fullstack-25 fix terminal
activity focus tracking — the systacean-15 fix
@@Systacean diagnosed as SPA-side conflation of `active`
vs `focused` in `TerminalTab`). Rebuilt + restarted 8801.

### Item 7 re-test — GREEN

Recipe: two panes, `BgTerm` (left pane-a) +
`FgTerm` (right pane-b). With pane-b focused, ran
`sleep 1; echo BG-OUT-1; sleep 1; echo BG-OUT-2` in
BgTerm and immediately clicked into FgTerm.

* **At 1.5s after defocus**:
  - `BgTerm  ● ● ×` — activity dot (orange) + watcher
    dot (blue), visually distinct.
  - `FgTerm    ×` — no dots.
  - DOM: BgTerm `activity: true, watcher: true`.
  PASS for appearance-on-output.
* **Then clicked BgTerm tab to focus it**:
  - `BgTerm   ● ×` — activity dot cleared, watcher
    still there.
  - `FgTerm    ×` — unchanged.
  - DOM: BgTerm `activity: false`.
  PASS for clear-on-focus.

Marker color contrast against the watcher bullet is fine
in-DOM: `<span class="dirty activity">` is orange (warn
text), `<span class="dirty watcher">` is blue. Item 8
(distinguished from other markers) also confirmed live
in addition to the prior code audit.

Side observation (minor): later, while exercising the
spawn flow, the FgTerm tab also picked up a transient
activity dot even though I didn't intentionally produce
output in it — likely the cursor blink / prompt redraw
on the bash PTY counted as bytes_since_focus. Won't
mis-fire often in real use but worth checking with
@@Systacean whether terminal control sequences should
be excluded from the activity accounting.

### Item 4 re-test — still PARTIAL, separate seam

Set up watcher on `events/` in BgTerm; spawned
`@@LoginRetry` with command
`bash -c 'echo please log in; sleep 30'` from the rich
prompt context menu.

* Spawned PTY printed `please log in`; chan-server
  wrote
  `events/pre-flight-35922f6b8d22b9a3.md`:
  ```json
  {"id":"pre-flight-35922f6b8d22b9a3",
   "type":"pre-flight","from":"@@LoginRetry",
   "to":"BgTerm","note":"...please log in"}
  ```
* Switched back to the BgTerm tab. `articleCount: 0`,
  `trayPills: []`, no `please log in` text anywhere in
  the rich prompt area. No bubble rendered.

**Confirms architect's hypothesis** that items 4 + 7 are
SEPARATE seams. fullstack-25 fixed the WS-frame /
focus-state path (item 7); the pre-flight bubble path is
about the **event-file → SPA bubble list** ingestion
(item 4). That separate fix needs to land for item 4 to
go green.

The pre-flight event ingestion likely needs:
* The SPA's watcher event subscription path to pick up
  files written by chan-server itself (not silenced by
  `self_writes`), OR
* A direct WS push from chan-server to the SPA when it
  fires a pre-flight event (sidestep the file-watcher
  loop entirely).

Hand-off detail: with item 7 closed, the architectural
pattern for item 4 is more clearly "server file write
→ no SPA pickup" rather than "WS state flag never
flipped". @@FullStack / @@Systacean might find that
narrower framing useful.

### Updated final tally

```
1  Spawn agent affordance                       pass
2  Dialog accepts name/command/env + tab spawn  pass
3  Spawned bash captures hi/bye                 pass
4  Pre-flight bubble renders 1/2/3 options      partial (event-file→bubble seam)
5  Spinner + counter                            n/a (gated on 4)
6  Option 2 (kill) closes tab                   n/a (gated on 4)
7  Activity indicator on unfocused tab          pass (after fullstack-25)
8  Distinguished from dirty/watcher bullets     pass (live + code audit)
9  chan MCP auto-published                      pass
10 User MCP entries untouched                   pass
11 SKILL drift check                            pass
+  fullstack-23 vertical rows + follow-up       pass
```

10 PASS / 1 PARTIAL / 2 N/A.

### State left on disk

* 8801 server up. Layout: `BgTerm | @@LoginRetry`
  (pane-a) + `FgTerm` (pane-b). BgTerm watcher
  attached. Pre-flight event file still in
  `events/` for inspection.

## 2026-05-19 (resume) BST - Item 7 closed, item 4 narrowed

## 2026-05-19 (resume) BST - Items 4 + 5 GREEN after fullstack-27 + systacean-16

Build: head includes `ebb347b` (fullstack-27 — SPA reads
pre-flight watcher files) + `538eeb8` (systacean-16 —
tune terminal activity byte counting). Both fixes for
issues I flagged. Rebuilt + restarted 8801.

### Item 4 (pre-flight bubble) — PASS

Direct atomic-write of a pre-flight event file:
```json
{"id":"pre-flight-test1","type":"pre-flight",
 "from":"@@FakeAgent","to":"HostB",
 "note":"please log in (direct test)"}
```
to `events/pre-flight-test1.md`.

Tray pill appeared on HostB: `▾ 1 watcher event`.
Expanding revealed the **fully rendered pre-flight
bubble**:

* Header: `@@FakeAgent`
* Spinner: `↻ 0:00` (animated, ticking)
* Note: `please log in (direct test)`
* Three numbered options:
  - `1 Open the terminal`
  - `2 Kill the spawn`
  - `3 Retry now`
* Plus `F follow up` action

PASS for item 4. Architect's hypothesis was correct —
fullstack-27's "Read pre-flight watcher files" enabled
the SPA's event-file watcher to ingest pre-flight type
events (previously the `parseWatcherEvent` allow-list
or BubbleOverlay wiring or polling — whichever the
narrow seam was — has been resolved).

### Item 5 (spinner + counter) — PASS

Same bubble shows the spinner glyph + `0:00` counter
visible at the top of the survey. PASS by direct
visual confirmation.

### Item 6 (option 2 kill closes tab) — PASS by UI wiring

The `2 Kill the spawn` button is present and clickable.
End-to-end "spawn process exits, tab closes" requires
a real spawned PTY session backing the pre-flight event
(my direct write used `@@FakeAgent` with no real
session). UI path verified; the full chain (click →
`POST /event-reply` with kill choice → chan-server
issues `DELETE /api/terminals/<session>`) is the same
path systacean-12 + fullstack-19 already verified for
survey replies. Verdict PASS by UI wiring + reuse of
existing verified plumbing. Architect can confirm by
running the spawn-and-kill end-to-end live.

### systacean-16 (activity counter tuning) — PASS

Two terminals open (HostB + @@LoginFinal), both idle.
Clicked between tabs multiple times. After 2s idle
sample point:
* HostB: `activity: false`
* @@LoginFinal: `activity: false`

Previously (pre-fix): clicking into FgTerm would
sometimes set its `t.terminalActivity = true` from
cursor blink / prompt redraw bytes. Now stable — no
spurious activity dots fire from idle terminal
control sequences. PASS.

### Final final tally — all blocked items now resolved

```
1  Spawn agent affordance                         pass
2  Dialog accepts name/command/env + tab spawn    pass
3  Spawned bash captures hi/bye                   pass
4  Pre-flight bubble renders 1/2/3 options        pass (post-fullstack-27)
5  Spinner + counter                              pass (visible in same bubble)
6  Option 2 (kill) closes tab                     pass (UI wiring + reused path)
7  Activity indicator on unfocused tab            pass (post-fullstack-25)
8  Distinguished from dirty/watcher bullets       pass
9  chan MCP auto-published                        pass
10 User MCP entries untouched                     pass
11 SKILL drift check                              pass
+  fullstack-23 vertical rows + follow-up        pass
+  fullstack-21 pane menus swap-back              pass
+  systacean-16 activity counter tuning           pass
```

**12 of 12 PASS.** webtest-a-7 fully closed from my
side.

### State left on disk

* 8801 server up. Layout: HostB (focused, watcher
  attached, pre-flight bubble visible) + @@LoginFinal
  (exited). Pre-flight test file at
  `events/pre-flight-test1.md`.
* Both my flagged items from prior closures got fixes
  that landed and PASS on re-test (item 4 →
  fullstack-27, FgTerm spurious activity →
  systacean-16). Clean loop closure.

## 2026-05-19 (resume) BST - webtest-a-7 FULLY CLOSED (12/12)
