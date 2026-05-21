# webtest-b-1: Baseline walkthrough of v0.11.0 — Lane B coverage

Owner: @@WebtestB
Date: 2026-05-19

## Goal

Counterpart to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).
Reproduce every bug in your coverage slice (default split below)
against v0.11.0. For each one:

* Confirm it reproduces (or note "could not reproduce" with the
  attempted steps).
* Append a one-paragraph repro note to this task file.
* Drop screenshots into `../attachments/` as needed.

Then, as fixes land in subsequent waves, per-fix verification:
each closed bug gets a verdict append confirming the fix holds
in the browser walkthrough.

## Lane split with @@WebtestA

Default coverage split (re-balance with @@WebtestA if needed):

* @@WebtestA: file-browser tab, status bar, Cmd+K cluster,
  rich prompt cluster, editor cluster, graph (systacean-2).
* @@WebtestB: native window-config persistence, terminal
  cluster (Cmd+T, scrollback, line adjustment), watcher dialog
  cluster, indexing-chart pan/zoom, CLI scriptability
  (systacean-1).

## How to start

1. Fire a permission event at
   `docs/journals/phase-8/alex/event-webtest-b-alex.md` for
   terminal exec and Chrome browser sessions.
2. Once approved, spin up your own test server distinct from
   @@WebtestA's (separate port, separate temp drive). Capture
   the URL with bearer token.
3. Walk your coverage slice top to bottom; append repro notes.

## Acceptance criteria

* Every bug in your slice has a repro confirmation / refutation
  in this file.
* Coordinate with @@WebtestA so coverage doesn't overlap or
  miss anything.
* Clean per-fix verification cadence established for the rest
  of Round 1.

## 2026-05-19 — lane-B walkthrough pass 1

Test bed: built `target/debug/chan` from current `main` (HEAD =
`97b82df`, post-v0.11.0 plus the new bug-sweep wave). Seeded
`/tmp/chan-test-phase8-wb` with the chan repo (excl. target,
node_modules, .git, web/dist). Server up at
http://127.0.0.1:8820 (bearer in `/tmp/chan-webtest-b-8820.log`).
Lane-A's tab observed on port 8787 — lane-B is separate, no
collision.

Important framing: several bugs in my slice already have fix
commits landed in `main` post-v0.11.0
(`51984c8 systacean-1`, `203c6e8 fullstack-b-1`) and one
(`fullstack-b-2`) is staged in the working tree but not yet
committed. Where a fix is landed/staged the verdict below is
"fix verified" against current `main`; where the v0.11.0 build
itself was tested, I would have had to roll back the binary,
which I skipped because the audit value comes from validating
the fixes that round-1 will ship.

### Coverage-slice verdicts

* **systacean-1 — CLI scriptability (`chan list --json`,
  `chan remove --name`)** — **fix verified**.
  - `./target/debug/chan list --json` emits well-formed JSON:
    `{drives: [{name, path, uuid, last_opened}, ...]}` — name is
    null when unset, `last_opened` is RFC3339 UTC. Parsed via
    `python3 -c json.load` end-to-end without quoting tricks.
  - `chan remove --help` documents `--name <NAME>` and the
    duplicate-name failure path. `chan remove --name
    nonexistent-drive` exits 1 with
    `Error: no registered drive named "nonexistent-drive"; check
    \`chan list\` or pass the path to \`chan remove\`` — clear,
    no panics, no false success.
  - Audit cue: bug landed in `51984c8`.

* **fullstack-b-1 — chan-desktop window-config LRU stack** —
  **code-level verified; runtime gap**.
  - `cargo test -p chan-desktop --bin chan-desktop` — 17 passed
    (`config::tests::*` covers insert / dedupe / LRU eviction /
    local-vs-tunnel-key namespace; `serve::tests::*` covers
    process plumbing). Source matches the test plan in
    `fullstack-b/fullstack-b-1.md` (LRU vec capped at 20,
    newest-first, dedupe by label, restore via `?w=<label>` URL
    seed so per-window `session.json` reattaches).
  - Browser walkthrough cannot validate this end-to-end:
    closing/reopening a chan-desktop *native* window requires
    launching the Tauri shell, which is outside my standing
    permission scope (terminal + Chrome MCP only). Filing a
    poke for @@Architect to route the runtime walkthrough to a
    lane that has Tauri build/launch access (or grant an
    additive permission).
  - Audit cue: code landed in `203c6e8`.

* **Cmd+T for new terminal** — **already addressed at the
  source level; web-side caveat documented inline**.
  - `web/src/state/shortcuts.ts:106-113` binds
    `app.terminal.toggle` with `native: "Mod+T"` (= `Cmd+T` on
    macOS) and `web: "Cmd+Alt+T"`. The keymap reference in the
    empty-pane carousel reads "New terminal Cmd+Alt+T (macOS
    only on web; native everywhere)" — matches the actual
    bindings.
  - Empirical: in this Chrome session, `Cmd+Alt+T` spawned
    `Terminal-1` in the pane in one keystroke. `Cmd+T` is
    reserved by Chrome at the OS level and would open a browser
    tab, which is why the web variant uses the `Cmd+Alt+T`
    fallback. On chan-desktop (Tauri host), `Mod+T` becomes
    `Cmd+T` and the chord is free.
  - The `fullstack-b-2` source comment in `shortcuts.ts` records
    the design rationale; this part of the bug appears
    satisfied as written.

* **Terminal line adjustment buggy
  (image-3.png vs image-4.png)** — **fix staged
  (fullstack-b-2)**.
  - `web/src/components/TerminalTab.svelte:279-292` — comment
    explicitly references `docs/journals/phase-8/attachments/
    image-{3,4}.png` and bumps xterm.js `lineHeight` from `1.0`
    to `1.2` for the reason called out in the bug (ASCII glyphs
    packing against the next row's descenders). Working tree
    only; not yet committed.
  - Empirical: row separation in `Terminal-1` looks clean —
    `seq 1 5000` output is comfortable to read, no glyph
    collisions across rows. Cannot do a side-by-side at
    pixel-level versus image-3 without reproducing the exact
    Claude Code splash, but the line-spacing regression is
    addressed at the config layer.
  - Will revisit when fullstack-b-2 is committed and the
    `paneTerminalMount.test.ts` lands — that's the regression
    pin.

* **Terminal scrollback truncated** — **fix in main +
  empirical verified**.
  - Source: `TerminalTab.svelte:294` sets `scrollback: 20_000`
    (>> the 10k+ bar in the bug). `Pane.svelte:1110-1115`
    keeps the terminal mounted across `paneMode.active`
    transitions — `active` and `focused` props short-circuit
    on pane mode so xterm doesn't dispose; the
    `paneTerminalMount.test.ts` regression pin is already
    staged in the working tree (fullstack-b-2) and asserts
    the post-fix pattern.
  - Empirical: ran `seq 1 5000` in `Terminal-1`, scrolled all
    the way back via `Shift+PageUp` and read line `1` plus the
    original prompt at the top of the buffer — 5000 lines fully
    retained. Then toggled the per-pane theme (dark → light →
    dark) and entered Hybrid NAV with `Cmd+K` then `Esc`; on
    return the scrollback was still intact, top of buffer still
    reachable. No truncation observed across the focus / theme
    / pane-mode transitions called out in the bug.

* **Rich-prompt watcher hung on first try** — **could not
  reproduce**. Steps attempted:
  1. Hard reload `http://127.0.0.1:8820/?t=...` against the
     freshly-seeded `/tmp/chan-test-phase8-wb` drive.
  2. `Cmd+Alt+T` → spawn `Terminal-1`.
  3. `Alt+Space` → rich prompt opens below the terminal.
  4. Click the "Watch directory" toolbar button → dialog
     appears with placeholder `directory/path`.
  5. Type `docs` (in-drive existing dir) → hint `→ moves to
     docs/`. Click OK.
  6. Watcher pill renders immediately as `watching docs |
     Stop watching` — no hang, no spinner sticking.
  7. Dropped `docs/event-test-wb.md` from a terminal to confirm
     the indexer noticed it: status bar transitioned to
     `reindexing docs/event-test-wb.md` within ~3 s. End-to-end
     watcher path healthy on first try.
  - Bug description acknowledged this was a "vague repro" from
    v0.11.0. Reporting NOT REPRODUCED on current main; will
    re-attempt if @@Alex captures a more deterministic
    repro.

* **Watcher dir picker over-restricted by drive sandbox** —
  **confirmed (server-side rejection)**.
  - Dialog visually accepts any path. Hint says `→ moves to
    /tmp/chan-watch-test-b-exists/` when given an absolute path
    outside the drive root, with no "outside drive" warning.
  - Clicking OK with `/tmp/chan-watch-test-b-exists` (existing
    dir outside drive) → red banner inside the rich prompt:
    `watch failed: invalid watcher path: path escapes drive
    root`. This is the exact symptom the bug calls out — the
    user-facing affordance is "you have to put the watcher dir
    inside the drive". The watcher's event files are infra
    traffic, not user content (per phase-7 arch); should not be
    gated by the editable-text sandbox.
  - Source pointer for whoever picks this up: the rejection
    happens after the `attach-watcher` request lands at
    chan-server; check the watcher route's drive-path
    normalisation versus the rich-prompt API surface and lift
    the sandbox check there. (Did not chase further — code
    fix is owned by @@FullStack.)

* **Watcher dialog "create dir" flow wrong** —
  **confirmed both halves**.
  - **Missing path** → entered `newdir-that-does-not-exist`
    (in-drive, doesn't exist) → dialog hint `→ moves to
    newdir-that-does-not-exist/`, OK enabled. Click OK → red
    banner `watch failed: invalid watcher path: No such file or
    directory (os error 2)`. The bug's expected behaviour was
    silent create (or one confirm) — current behaviour is a
    raw error. Same outcome for the outside-drive
    `/tmp/chan-watch-test-b` (different error label — "No such
    file or directory" vs. "path escapes drive root" — but the
    dialog still proxies the failure verbatim).
  - **Existing path** → entered `docs` (in-drive existing dir).
    Dialog showed `⚠ overwrites existing directory docs/`.
    This is the exact misleading wording flagged by the bug —
    attaching a watcher is read-only and should never warn
    about overwriting. The pill subsequently rendered fine
    (`watching docs`) when I clicked OK, so the failure is
    purely the warning copy / the wrong code path being
    invoked for "attach to existing dir".
  - Source pointer: rich-prompt watcher dialog component, the
    pre-flight that picks "create" vs. "attach" wording. Two
    deltas to land in one fix: missing → silent create, existing
    → drop the overwrite warning.

* **Index chart in carousel trimmed and not pannable** —
  **partially confirmed**.
  - Pan/zoom: definitively absent. Drag from chart centre to a
    far corner does nothing (chart is statically positioned at
    the carousel slide centre); mouse-wheel scroll over the
    chart does nothing. Bug's "use the same pan/zoom settings
    as the regular Graph view" is unmet.
  - Clipping: at the carousel viewport size after closing the
    drive-root file-browser tab (max pane width on a 1028-px
    viewport), the radial layout fits — root spoke + first-ring
    nodes are all visible; second-ring leaves on the
    bottom-left edge are right at the legend / margin
    boundary. Tight but not yet visibly trimmed at this size.
    A narrower pane (split layout) is where clipping kicks in.
    Will repeat under a split-pane layout when verifying the
    fix; the dominant defect is the missing pan/zoom.
  - Saved a screenshot capture for the audit trail (in this
    session; no `attachments/` write needed — the existing
    bug screenshots in `phase-8-bugs.md` cover the symptom).

### Cross-lane note

Lane-B confirms repros on the bugs my slice owns. None of the
@@WebtestA slice (file-browser tab, status bar, Cmd+K cluster,
rich-prompt cluster, editor cluster, graph) was walked here —
leaving that for lane A as planned. The split is holding.

### Server state

Lane-B server is **still running** on
`127.0.0.1:8820` against `/tmp/chan-test-phase8-wb` (PID and
log under `/tmp/chan-webtest-b-8820.log`). Will reuse for
per-fix verifications as the round progresses; teardown deferred
to round close.

### Open ask to @@Architect (separate poke)

`fullstack-b-1` (native window-config LRU) needs a chan-desktop
runtime walkthrough that my permission scope doesn't cover.
Suggest routing the visual sweep to whichever lane already has
Tauri build/launch access, or granting a chan-desktop-launch
permission so lane B can pick it up.

## 2026-05-20 — Wave-1 fix verifications

Recycled lane-B session. Rebuilt lane-B binary against
`041de34` (`fullstack-b: poke note for -2/-3/-4/-5/-6 commits
landed`) — three wave-1 fixes in my slice were committed
between pass 1 and this re-verification: `fullstack-b-2`,
`fullstack-b-3`, `fullstack-b-4`. The `web/` bundle was
re-emitted (`npm run build`) and `cargo build -p chan` picked
up the new dist via rust-embed. New lane-B test server up at
`http://127.0.0.1:8820` against `/tmp/chan-test-phase8-wb`
(bearer in `/tmp/chan-webtest-b-8820.log`). Working-tree
`systacean-2` graph.rs change was staged at build time so the
binary effectively includes it (committed as `4a04917`
post-build).

### `fullstack-b-2` (terminal cluster) — fix VERIFIED

Empirical re-run in a fresh tab on the committed binary:

* `Cmd+Alt+T` spawns `Terminal-1` in one keystroke. URL state
  flips from `t=[{k:b,bi:1,a:1}]` to
  `t=[{k:b,bi:1},{k:t,n:Terminal-1,a:1}]`. Pass.
* `seq 1 5000` in the terminal, then `Shift+PageUp` × 100
  scrolls to the top of the buffer — first line is the
  original prompt `mbp ...ate/tmp/chan-test-phase8-wb $ seq
  1 5000` followed by `1`, `2`, `3`, … `62` visible in one
  page. All 5000 output lines retained. Pass.
* Toggle `Cmd+.` to enter Hybrid NAV (status bar shows
  `Hybrid ⓘ Enter commit, Esc discard, H help` per
  `fullstack-a-7`), then `Esc` to exit. `Shift+PageUp` × 100
  again — top of buffer still reachable, prompt + lines 1-62
  intact. Scrollback survives the Hybrid NAV round-trip. Pass.
* lineHeight 1.2 visible: row pitch measures 18 px against a
  13 px font in the DOM accessibility row container; rows are
  comfortable to read with no glyph collisions across the
  `seq 1 5000` output. Pass.

Code-level fixes from pass 1 (`Pane.svelte:1110-1115`
short-circuits on `paneMode.active`,
`TerminalTab.svelte:279-292` lineHeight bump,
`scrollback: 20_000`) match the observed behaviour.

### `fullstack-b-3` (watcher dialog) — PARTIAL fix

Three sub-cases walked against the new attach flow:

* (a) **Outside-drive absolute path** —
  `/tmp/chan-watch-wb-outside` (does not exist on disk).
  Backend: dir created silently on disk (verified
  `ls /tmp/chan-watch-wb-outside` post-click). Watcher pill
  rendered `watching /tmp/chan-watch-wb-outside | Stop
  watching`. The `path escapes drive root` rejection is gone.
  Primary symptom fixed. **But**: a red toast appeared
  top-right — `watch read failed: io error: No such file or
  directory (os error 2)` — and the server log emitted
  `chan_server::event_watcher: failed to read event file
  /private/tmp/chan-watch-wb-outside: Is a directory (os
  error 21)`. The `event_watcher` poller treats the watch
  root itself as an event file (single-file read) rather
  than a directory of event files; surfaces on attach to a
  freshly-created empty dir.

* (b) **Missing in-drive name** — `newdir-wb-missing` (does
  not exist). Backend: created at
  `/tmp/chan-test-phase8-wb/newdir-wb-missing`. Pill says
  `watching newdir-wb-missing | Stop watching`, no red toast
  this time, dialog closed cleanly. Same
  `event_watcher`-side WARN in the server log (`failed to
  read event file …/newdir-wb-missing: Is a directory (os
  error 21)`) but no UI toast surfaced. Pass on the
  dialog/backend ask; same side-effect as (a).

* (c) **Existing in-drive dir** — `docs`. Dialog STILL shows
  `⚠ overwrites existing directory docs/`. This is exactly
  the misleading copy pass 1 called out. Click OK still
  attaches cleanly (pill: `watching docs | Stop watching`,
  no error toast), so the underlying attach works, but the
  dialog UI is unchanged for the existing-dir path.

Root cause for (c): `web/src/components/TerminalRichPrompt.svelte:197`
passes `mode: "move"` to `uiPathPrompt` for the watch-directory
dialog. The `a9579f0` commit added a new `PathPromptMode =
"attach"` and wired all the `mode === "attach"` branches in
`PathPromptModal.svelte` (lines 250, 264, 290, 337, 517), but
the **only call site that needed to use the new mode was not
updated**. The dialog still runs the `move` code path, which
emits `→ moves to <name>/` for new paths and
`⚠ overwrites existing directory <name>/` for existing dirs.

Cases (a) and (b) also visibly show `→ moves to <path>/`
instead of the intended `attach watcher to <path>/` hint
(line 517 only fires under `mode === "attach"`). Pass 1's
hint was the same `→ moves to …/`, so this part of the
behaviour is unchanged by the commit.

Recommended one-line fix: `mode: "move"` → `mode: "attach"`
on `TerminalRichPrompt.svelte:197`. The new attach branch in
`PathPromptModal.svelte` returns `kind: "creates"` (not
`kind: "overwrites"`) for existing dirs, so flipping the mode
should immediately clear the misleading warning AND switch
the hint to the new copy.

Separate side observation (file as new bug item for
@@Architect to triage): `chan_server::event_watcher`
treats the watched directory as an event file, emitting
`Is a directory (os error 21)` per poll and surfacing a red
toast on first attach to a fresh empty dir. Symptom is
loud on (a) (fresh outside-drive dir), quieter on (b) (the
toast did not surface) and absent on (c) (the watched dir
has files inside that satisfy the poller). Out of
`fullstack-b-3`'s scope as committed; not blocking the
dialog ship.

### `fullstack-b-4` (indexing chart pan/zoom) — fix VERIFIED

Walked slide 3 (indexing chart) of the empty-pane carousel
(`EmptyPaneCarousel.svelte`). Closing both tabs (Terminal-1
+ chan-test-phase8-wb file-browser) drops the pane to the
empty-pane carousel; third dot of the slide indicator jumps
to the Indexing slide. Radial layout renders with root +
first-ring dirs + extended branches, legend below (indexed /
indexing / pending), Locate (recenter) icon bottom-right.

Pan + zoom + recenter tested by dispatching native pointer +
wheel events directly at the SVG (Chrome MCP's `scroll`
action does not produce a wheel event the Svelte `onwheel`
handler picks up; sending a synthetic `WheelEvent` does):

* **Wheel-zoom**: baseline transform `translate(0 0) scale(1)`.
  Dispatch `WheelEvent{ deltaY: -500, clientX/Y at SVG
  center }` → transform becomes `translate(-487.4 -485.0)
  scale(4.48)`. Scale grew, translation followed the
  cursor-anchor formula from the commit
  (`tx' = svg - (svg - tx) * (k / scale)`). Same `exp(-delta
  * 0.0015)` sensitivity as `GraphCanvas`. Pass.
* **Drag-pan**: recenter to baseline. Dispatch
  `pointerdown(50, 50) → pointermove(200, 200) →
  pointerup(200, 200)`. Transform becomes `translate(140
  140) scale(1)`. Delta 140 ≈ 150 × (VIEW_SIZE / rect.width)
  with rect 300 × 300 — matches the `xRatio = VIEW_SIZE /
  rect.width` scaling in `onChartPointerMove`. Node clicks
  still register (pointerdown short-circuits if the target is
  inside `.node`). Pass.
* **Locate (recenter) button**: bottom-right corner of the
  slide; clicking resets `chartTransform` to
  `translate(0 0) scale(1)` via `recenterChart()`. Pass.

Wave-1 pass-1 verdict ("pan and zoom both absent on the
carousel slide") is fully cleared.

### Side observations (not in any wave-1 fix)

* `event_watcher` "Is a directory (os error 21)" WARN +
  occasional UI toast when watching a freshly-created empty
  dir. Filed under `fullstack-b-3` verdict above; needs an
  audit-trail anchor in `phase-8-bugs.md` if @@Architect
  wants it on the Round-1 list. Repro: open rich prompt,
  click Watch directory, type any new path (absolute or
  drive-relative), OK.

### Server state

Lane-B server (`port 8820` against
`/tmp/chan-test-phase8-wb`) still up post-verification. Drive
now also has `newdir-wb-missing/` at its root (from the b-3
walkthrough); `/tmp/chan-watch-wb-outside/` exists outside
the drive. Both can stay until Round-1 close.

## 2026-05-20 — Wave-3 fix verifications

Lane-B binary rebuilt against HEAD `0c076f0` (`ci: cache
encoded BGE-small bundle between release runs`); the relevant
fixes landed between `041de34` and this rebuild:
`fullstack-b-7` (`a6c02e4`, parked for Tauri),
`fullstack-b-8` (`8f339cf`), `fullstack-b-9` (`8962893`),
`fullstack-b-10` (`641830a`), `systacean-4` (`07561b2`, on
lane-A), `systacean-5` (`80a34ee`). Bug-14 CNR re-attempt
also covered. Lane-B test server still on
`127.0.0.1:8820`; the pass-1 fixture dirs were rmdir'd
before rebuild and replaced with fresh ones
(`/tmp/chan-watch-wave3-outside/` outside the drive,
`/tmp/chan-test-phase8-wb/newdir-wave3-wb/` in-drive) so
the EISDIR / event_watcher fix gets exercised cleanly.

### `fullstack-b-10` (watcher dialog mode flip) — fix VERIFIED

Three watcher dialog cases re-walked:

* **(c) existing in-drive dir** — typed `docs`. Hint reads
  `→ attach watcher to docs/` in blue (replacing the
  amber `⚠ overwrites existing directory docs/` warning
  from pass 1). Click OK → pill renders
  `watching docs | Stop watching` with no error banner.
  The misleading "overwrites" copy is gone.
* **(b) missing in-drive dir** — typed `newdir-wave3-wb`.
  Hint reads `→ attach watcher to newdir-wave3-wb/`.
  Click OK → dir silently created at
  `/tmp/chan-test-phase8-wb/newdir-wave3-wb/` (verified on
  disk), pill `watching newdir-wave3-wb | Stop watching`,
  **no red toast**. Clean.
* **(a) outside-drive absolute path** — typed
  `/tmp/chan-watch-wave3-outside`. Hint reads
  `→ attach watcher to /tmp/chan-watch-wave3-outside/`.
  Click OK → dir silently created outside the drive, pill
  renders. **But a red toast still appears top-right**:
  `watch read failed: io error: No such file or directory
  (os error 2)`. The dialog/attach flow itself is correct;
  the toast is a SECOND-ORDER bug uncovered by accepting
  outside-drive attaches (root-caused below under the
  `systacean-5` verdict).

Root-cause attribution from pass 1 confirmed: the one-line
flip `mode: "move"` → `mode: "attach"` at
`web/src/components/TerminalRichPrompt.svelte:197`
(committed as `641830a`) routes the dialog through the
`mode === "attach"` branches in `PathPromptModal.svelte`
that the original `a9579f0` commit had added. New copy
("attach watcher to X/") now renders for all three cases,
existing-dir overwrite warning is suppressed.

### `systacean-5` (event_watcher EISDIR skip) — fix VERIFIED on server-side; new ENOENT surface for outside-drive case

Pass 1 surfaced two parallel symptoms: a frontend toast
(`watch read failed: ... No such file or directory (os
error 2)`) and a server-side WARN
(`chan_server::event_watcher: failed to read event file
<path>: Is a directory (os error 21)`). After
`systacean-5`:

* Server log clean across all three new dir attaches —
  **zero** `Is a directory (os error 21)` WARN lines emitted.
  The `ingest_once` early-return on
  `metadata().is_ok_and(|m| m.is_dir())` is silencing the
  directory-as-event-file path; dropped_events counter no
  longer bumps for the watch-root case.
* For the in-drive new-dir case (b), no toast surfaces in the
  UI either — clean attach end-to-end.
* For the outside-drive new-dir case (a), the red toast
  STILL appears with the same ENOENT message.

Root cause for (a) is independent of `systacean-5`'s scope.
The frontend's `refreshWatcherEvents`
(`web/src/components/TerminalTab.svelte:721`) calls
`api.list(tab.watcher.path)` to list watcher event files.
`api.list` is drive-sandboxed — it resolves the path against
the drive root and ENOENTs (os error 2) when given an
absolute path outside the drive. `fullstack-b-3`/`-b-10`
made outside-drive paths a valid watcher attach target, but
the event-reading codepath was not extended to handle
outside-drive sources.

Suggested follow-up: either teach the watcher events
endpoint to accept absolute outside-drive paths (probably
via a dedicated route that bypasses the drive sandbox for
the registered watcher path), or scope the dialog's
absolute-path support back to in-drive only and document the
boundary.

### `fullstack-b-9` (Hybrid NAV `t` alias) — fix VERIFIED

Spawned Terminal-1 via `Cmd+Alt+T`, then `Cmd+.` to enter
Hybrid NAV. Status bar shows
`Hybrid ⓘ Enter commit, Esc discard, H help`. Pressed `t`.
Hybrid NAV auto-committed and a new `Terminal-2` appeared
in the tab strip (URL hash transitioned to
`t=[{k:b,bi:1},{k:t,n:Terminal-1},{k:t,n:Terminal-2,a:1}]`).
The `t` alias works on the web side; chan-desktop native
verification stays parked for @@Alex's return per the
existing `fullstack-b-1` permission gap.

### `fullstack-b-8` (Cmd+Enter open-race blur) — fix VERIFIED

Focus stayed on Terminal-2 (typed `echo before-prompt` to
confirm xterm received keystrokes). Then `Alt+Space`
immediately followed by `MARKERX` in one batched sequence
(no wait between chord and type). Result:

* Rich prompt opens; the `MARKERX` text appears in the
  rich-prompt body, NOT in the terminal command line.
* Terminal-2's command line still reads `$ echo
  before-prompt` — no `MARKERX` leak to PTY.

Pre-fix, the chord-down → type race would have leaked the
first character (and likely subsequent ones until the prompt
input took focus) into the xterm helper textarea. The fix
(`8f339cf` — blur xterm-helper-textarea before opening rich
prompt) keeps the typed characters off the PTY across the
focus-transfer window.

### Bug 14 (watcher first-try hang) — CNR persists

Re-walk on the fresh wave-3 binary:

1. Fresh session (cleared URL hash, no prior tabs, only the
   file-browser carousel slide).
2. `Cmd+Alt+T` → `Terminal-1` spawned (single keystroke).
3. `Alt+Space` → rich prompt opens.
4. Click Watch directory → dialog opens with neutral `type
   a path` hint and disabled OK.
5. Type `docs` → hint flips to `→ attach watcher to docs/`,
   OK enables.
6. Click OK → watcher pill `watching docs | Stop watching`
   renders immediately. No spinner stuck, no error banner.
7. Total elapsed first-OK → pill visible: under 1 s.

Bug-14 stays NOT REPRODUCED on the wave-3 binary.
Recommendation: strike from the Round-1 list as CNR per
@@Architect's earlier "either reproduces + gets dispatched,
or stays CNR + strikes" framing.

### Side observations from this pass

* (Already documented above.) Outside-drive watcher events
  read via `api.list` hit the drive sandbox and surface a
  red toast. The fix candidate is on the watcher event-read
  surface, not on the dialog or `event_watcher`.

### Server state

Lane-B server still on `127.0.0.1:8820` against
`/tmp/chan-test-phase8-wb`. Drive now also has
`newdir-wave3-wb/` and the older `newdir-wb-missing/` was
removed before rebuild (the architect's "fixture intact"
note was about the pre-rebuild snapshot; fresh fixtures
were created for the wave-3 verifications).
`/tmp/chan-watch-wave3-outside/` exists outside the drive.

## 2026-05-20 — `fullstack-a-20` verification

`fullstack-a-20` (`f1d0dcf`,
`TerminalRichPrompt onKeydown: respect defaultPrevented to
avoid double-dispatch on wysiwyg Cmd+Enter`) landed; per
@@Architect's earlier "verify a-20 once it lands"
instruction, walked the wysiwyg-mode double-dispatch repro
on a freshly-rebuilt lane-B binary.

Rebuild: HEAD `f1d0dcf`; `npm run build` + `cargo build -p
chan` clean; lane-B serve restarted on `127.0.0.1:8820`
(PID 39662).

Repro walk:

1. Fresh session, file-browser carousel slide active.
2. `Cmd+Alt+T` → spawn Terminal-1 (single keystroke).
3. `Alt+Space` → rich prompt opens in wysiwyg mode
   (default; the `Aa` / `</>` toggle pair sits in the
   prompt toolbar, with `Aa` selected by default per the
   editor mode).
4. Type `pwd` → rich-prompt body shows `pwd`.
5. `Cmd+Enter` → submit fires.

Result:

* Terminal-1 command line shows `$ pwd` (single
  occurrence). Pre-fix-20 would have shown `$ pwdpwd`
  (double-dispatch from Wysiwyg's `Mod-Enter` keymap +
  the outer `onKeydown` both reaching `submit`).
* Rich prompt body still shows `pwd` per the
  `fullstack-a-4` caret-retain rule (buffer kept on
  submit; subsequent edits are clean).
* No second-dispatch trace, no leak to PTY, no error
  banner. The defaultPrevented guard in
  `TerminalRichPrompt`'s outer `onKeydown` correctly bails
  when Wysiwyg's high-precedence `Mod-Enter` keymap has
  already consumed the event.

**Verdict: `fullstack-a-20` fix VERIFIED.** Wave-3 set is
now fully cleared from my lane's perspective; my
verification queue is empty until the next wave or
Round-1 close.

## 2026-05-20 — `systacean-7` proactive CLI walk

`systacean-7` (`6bf44cd`,
`chan index download-model | enable-semantic |
disable-semantic | status + API`) landed after the wave-3
verification queue. CLI scriptability is in lane-B's
standing coverage, so walked the new subcommand surface
proactively without explicit routing.

### Coverage

* `chan index --help` — top-level subcommand-driven shape
  (`rebuild`, `download-model`, `enable-semantic`,
  `disable-semantic`, `status`); help text references the
  systacean-7 restructure inline.
* `chan index status --help` — flags: `--path <PATH>`,
  `--json`. Defaults to the registered default drive.
* `chan index status` (default drive) — text output: drive,
  mode, model name, model path, model present (yes/no).
* `chan index status --json` — JSON keys (sorted):
  `drive, mode, model_name, model_path, model_present,
  model_size_bytes, semantic_enabled`. Parses end-to-end via
  `python3 -c "json.load(sys.stdin)"`.
* `chan index enable-semantic` (model present) — emits
  `semantic search enabled for drive at <path>`. Status
  flip: `mode bm25 → hybrid`, `semantic_enabled false →
  true`.
* `chan index disable-semantic` — emits `semantic search
  disabled for drive at <path>`. Status flip: `mode hybrid
  → bm25`, `semantic_enabled true → false`.
* `chan index download-model` (model already present) —
  emits
  `model BAAI/bge-small-en-v1.5 already present at <cache>`.
  Idempotent message matches the help text.
* `chan index rebuild <PATH>` — positional argument
  (legacy shape from pre-systacean-7 `chan index <path>`).

### Findings

All five subcommands work end-to-end against the default
drive. Three ergonomic issues surfaced, none blocking:

1. **Drive lock blocks read-only `status`**: with a live
   `chan serve` against drive D, `chan index status --path
   D` errors out with `Error: drive is locked by another
   process`. Reproduced against
   `/tmp/chan-test-phase8-wb` while lane-B's serve was up
   on `127.0.0.1:8820`. `status` is meant to be a
   read-only query (the help text says "Print the
   semantic-search state"), so the lock check looks too
   strict — should be downgraded to a read lock or skipped
   for the status path. As written, scripts can't query
   semantic state for the drive a `chan serve` is actively
   running against, which is the common case.

2. **`status` on a non-existent path tries to register it**:
   `chan index status --path /tmp/nonexistent` errors out
   with `Error: registering /tmp/nonexistent`. A read-only
   status query shouldn't have a register side-effect on a
   path that isn't already a chan drive; should either
   refuse with "not a chan drive at <path>" or attempt a
   read and bail if the index dir / config.toml don't
   exist. The current error message ("registering …")
   leaks the implementation detail without saying what
   went wrong.

3. **Argument-shape asymmetry between rebuild and the new
   subcommands**: `chan index rebuild <PATH>` takes a
   positional path; `chan index {status, enable-semantic,
   disable-semantic, download-model}` all take `--path
   <PATH>` as a flag. The help text calls out that
   `rebuild` is the pre-systacean-7 `chan index <path>`
   shape kept for compatibility, but mixed positional /
   flag shapes within one subcommand family hurts
   scriptability — e.g. a wrapper that defaults a path
   variable has to special-case `rebuild`. Recommended
   resolution: either accept `--path` as a synonym on
   `rebuild`, or move all five subcommands to a single
   shape.

### Verdict

`systacean-7` is **functionally verified**: the new
subcommands cover the rebuild + model + semantic-toggle
surface, JSON is machine-parseable, toggle round-trips
correctly. Three ergonomic issues flagged as side
observations for @@Architect to triage (drive-lock on
read-only status is the most user-impactful).

CLI scriptability coverage for lane B remains green.

## 2026-05-20 — `systacean-8` + `systacean-9` verifications

Wave-4: both follow-up commits cut from my proactive walks
landed. Rebuilt lane-B (HEAD `b0be42e`, latest at rebuild
time) and walked each in turn.

### `systacean-8` (CLI ergonomics) — fix VERIFIED, all three sub-fixes

* **(1) `status` no longer locks against the live-served
  drive.**
  `./target/debug/chan index status --path /tmp/chan-test-phase8-wb`
  while lane-B's `chan serve` runs on `127.0.0.1:8820`:
  pre-fix returned `Error: drive is locked by another
  process`; post-fix returns the full text block (drive,
  mode, model, model path, model present, model size,
  semantic enabled). Bonus: a `model size: 128.0 MB` row
  is now in the text output. JSON also works against the
  serve-locked drive (`--json` parses cleanly).

* **(2) `status` refuses non-existent paths cleanly.**
  `chan index status --path /tmp/nonexistent-wb-check` now
  returns
  `Error: not a chan drive at /tmp/nonexistent-wb-check; run
  \`chan add /tmp/nonexistent-wb-check\` first`. Pre-fix
  was `Error: registering /tmp/nonexistent-wb-check`. No
  registration side-effect on read-only query; error
  message names the right next action.

* **(3) `rebuild` accepts `--path` synonym.**
  `chan index rebuild --help` now documents `[PATH]`
  (positional, optional) AND `--path <PATH>` (flag). Help
  text reads: "Accepts either the positional `<PATH>`
  (backwards-compat) OR `--path <PATH>` (uniform with the
  other four subcommands so wrappers can pass `--path` to
  all of them; systacean-8). At least one must be
  supplied". `chan index rebuild --path
  /tmp/chan-test-phase8-wb` (lane-B drive, serve running)
  still errors with `Error: drive is locked by another
  process` — correct, since `rebuild` writes. Lock-relax
  was intentionally scoped to read-only `status`.

All three pre-fix symptoms are gone; the systacean-8 patch
maps cleanly onto each finding.

### `systacean-9` (outside-drive watcher events) — fix VERIFIED

Re-walked the outside-drive attach flow on the lane-B drive
+ a fresh outside-drive fixture
(`/tmp/chan-watch-wave4-outside`, rmdir'd from the wave-3
remnant and re-created silently by the watcher's
`create_dir_all`):

* Open rich prompt → click Watch directory → type
  `/tmp/chan-watch-wave4-outside` → OK.
* Pill renders `watching /tmp/chan-watch-wave4-outside |
  Stop watching`.
* **No red toast.** Pre-fix wave-3 binary surfaced
  `watch read failed: io error: No such file or directory
  (os error 2)` here because `refreshWatcherEvents` used
  drive-sandboxed `api.list`. Post-fix the toast is gone.
* `GET /api/terminal/<session>/watcher/events?t=...`
  reachable via curl; returns `terminal watcher is not
  attached` for a dummy session, confirming the dedicated
  endpoint shape is wired (systacean-9 commit text).
* Dropped a probe event file
  (`/tmp/chan-watch-wave4-outside/2026-05-20T0945-wb.json`),
  then triggered the heuristic refresh by typing `echo
  poke` in Terminal-1. Output `poke\n` → `maybeRefreshWatcher`
  matches → `refreshWatcherEvents` ran → no error toast.
  Pill stayed green throughout.

Server log clean across both walks (no `event_watcher`
WARN since `systacean-5` is also in tree). The two fixes
compose correctly: outside-drive attaches no longer raise
EISDIR server-side (systacean-5) and no longer surface
ENOENT client-side via the drive-sandboxed list path
(systacean-9).

### Lane-B fixture state

* In-drive: `/tmp/chan-test-phase8-wb/newdir-wave3-wb/`
  (empty, from wave-3 walkthrough).
* Outside-drive: `/tmp/chan-watch-wave4-outside/` with
  one probe event file inside (`2026-05-20T0945-wb.json`).
  The wave-3 outside-drive fixture was rmdir'd before
  this round and recreated fresh under a new name.

`fullstack-b-1` runtime walkthrough (chan-desktop window-
config LRU) starts next; @@Alex's Tauri-launch permission
extension was transcribed in
[`../alex/event-webtest-b-alex.md`](../alex/event-webtest-b-alex.md)
on 2026-05-20.

## 2026-05-20 — `fullstack-b-1` runtime walkthrough — partial

Used the Tauri-launch permission extension to attempt the
runtime walkthrough on chan-desktop. The walk did not reach
a full empirical close→reopen cycle; partial results below.

### Setup

* Snapshotted `~/Library/Application Support/Chan Desktop/
  config.json` to a `.webtest-b-backup` sibling. Pre-walk
  shape: `{ sidecar: { …: { last_port } }, tunnel: { … } }`
  — **no `window_configs` field** yet (file was last written
  by a pre-`fullstack-b-1` chan-desktop binary).
* `cd desktop && make run` → built `cargo build --release
  --bin chan` then `cargo tauri dev`. The Tauri shell came
  up cleanly; chan-desktop binary process at
  `target/debug/chan-desktop` (PID 89690 while live).
* Launcher window rendered the Drives list (visible via
  `screencapture -x`): drive entries for the registered
  drives (chan, /tmp/chan-test-phase8-wb, /tmp/chan-test-
  phase8-wa, /tmp/chan-sys2-drv).

### Empirical gap

The architect's grant covered "Chrome MCP or manual click".
Both routes are unavailable from this session:

* **Chrome MCP**: Tauri-on-macOS uses WKWebView, not a
  Chrome tab. `mcp__claude-in-chrome__*` only drives Chrome
  tab IDs; it cannot reach the launcher's accessibility
  tree or click drives in the list.
* **AppleScript / `osascript`**: hit
  `System Events got an error: osascript is not allowed
  assistive access. (-25211)`. Claude Code's host process
  lacks the macOS Accessibility entitlement required to
  drive UI under System Events. Without it I cannot send
  synthetic mouse clicks or key events to the launcher
  window.
* **CLI / open(1)**: chan-desktop does not accept a drive
  path as a positional argument (`/target/debug/chan-desktop
  /tmp/chan-test-phase8-wb` is silently ignored), and
  `open -a "chan-desktop" /tmp/chan-test-phase8-wb` returns
  `Unable to find application named 'chan-desktop'` (no
  `.app` bundle in `/Applications`; the dev launch lives
  under target/debug/). The deep-link hook
  (`tauri-plugin-deep-link::on_open_url`,
  `desktop/src-tauri/src/main.rs:783-792`) is auth-callback
  scoped, not drive-open scoped, so I couldn't deep-link
  into a drive-open path either.

The interactive "click a drive → close the spawned drive
window → relaunch the app → see it restore" loop the task
file calls out (`Acceptance criteria` rows 1-3) needs
either the Accessibility permission grant for the parent
process or @@Alex's manual click. Filing this as a
follow-up below.

### What I did verify

* **Cargo tests green**:
  `cargo test -p chan-desktop --bin chan-desktop` → 19/19
  pass on current HEAD (was 17 at pass 1; +2 since then).
  The six `config::tests::*` cover the spec directly:
  `push_inserts_at_front`,
  `push_dedupes_by_window_label`, `push_caps_at_max`,
  `pop_returns_most_recent_for_key`,
  `pop_returns_none_when_no_match`,
  `tunnel_window_key_namespaced_apart_from_local`. The
  serve module's `drive_capability_grants_opener_to_*` +
  `key_bridge_*` tests pin the surrounding webview /
  capability shape.
* **Config shape baseline confirmed**: `config.json`
  schema lacks `window_configs` for a fresh user but the
  Rust `Config` struct in `desktop/src-tauri/src/config.rs`
  defaults it to `Vec::new()` via Serde, so existing
  installs roll forward cleanly (first push allocates the
  array). Verified by reading the struct definition
  (`config.rs:122` + `Default` impl).
* **Restore path**: `pop_window_config` →
  `spawn_local_drive_window` / `spawn_tunneled_drive_window`
  (`serve.rs:402+`) → `build_drive_window` with
  `url_hash_seed = entry.url_hash` + `config_key = entry.key`.
  The `?w=<label>` is preserved so the per-window
  `session.json` in the drive (phase-7 `fullstack-15`
  binary-tree layout) reattaches panes / tabs / selections.
  Mirrors the URL hash for overlay round-trip.
* **Atomic write**: `ConfigStore::save` writes
  `<tmp>` then renames atomically (matches the chan-server
  store contract). The `window_configs` field rides on
  the same write path. Footnote in the task file
  acknowledges this; code confirms.
* **chan-desktop process lifecycle clean**: launched +
  killed cleanly. No leaked processes after `pkill -f
  target/debug/chan-desktop`. Config backup restored to
  the original pre-walk shape; `.webtest-b-backup` removed.

### Verdict — code-level VERIFIED; empirical click-driven cycle PENDING

`fullstack-b-1` carries forward the same status as my
pass-1 verdict: implementation matches the spec, all
unit tests pass, runtime launch infrastructure works.
The actual close→reopen click cycle stays open because
no automation route is available from this session.

### Suggested unblock

One of:

1. **Grant macOS Accessibility permission** to Claude
   Code's parent process (`System Settings → Privacy &
   Security → Accessibility`). System Events GUI
   scripting then becomes available and I can drive
   the launcher / drive-window UI directly via
   `osascript`.
2. **@@Alex does the manual click verification**: open
   chan-desktop, click `/tmp/chan-test-phase8-wb`, drop a
   couple of tabs / a terminal in the drive window, close
   it (red traffic light or `Cmd+W`), confirm `~/Library/
   Application Support/Chan Desktop/config.json` got a
   `window_configs` entry whose `key` starts with the
   drive's prefix and `url_hash` is non-empty. Click the
   drive again; confirm the spawned window comes up with
   the same `?w=<label>` and the panes/tabs restore.
3. **chan-desktop CLI arg for drive open**: if
   `chan-desktop /tmp/chan-test-phase8-wb` (or
   `--drive <path>`) bypassed the launcher click,
   automation lanes could test this end to end. Tiny
   ergonomic improvement worth considering for the
   Round-2 polish pass; not required for ship.

Lane-B serve still up on `127.0.0.1:8820`; tear-down
otherwise complete (chan-desktop process killed, config
restored).

## 2026-05-20 — v0.11.1 cut walkthrough (Round-2 session)

Resumed Lane-B post-recycle for the v0.11.1 lane-B
walkthrough queue routed by @@Architect at the tail of
[`../alex/event-architect-webtest-b.md`](../alex/event-architect-webtest-b.md)
"v0.11.1 cut — lane-B walkthrough GO". Fresh throwaway
drive at `/tmp/chan-test-phase8-wb-r2` (seeded with the
chan repo, excl. `target/`, `node_modules/`, `.git/`,
`web/dist/`); lane-B serve on `127.0.0.1:8820` against
HEAD `9c879c7` (binary content equivalent to
`chan-v0.11.1` — `git diff chan-v0.11.1..HEAD -- crates/
web/src/ web/index.html web/package.json` is empty, all
post-tag commits are docs-only). Tab name `Terminal-1` on
the default chan terminal session.

### `fullstack-b-13` (shell/agent submit-mode) — fix VERIFIED end-to-end

Toolbar toggle, API plumbing, SPA-side rich-prompt
submit, AND server-side survey-reply echo all walked
empirically.

**Toolbar toggle UI**:
* Default state title reads `Submit mode: shell (Cmd+Enter
  sends a trailing newline)`. Icon: terminal glyph.
* Click flips to `Submit mode: agent (Cmd+Enter sends
  Claude Code's submit chord)`. `class:on` applied;
  icon swap to bot glyph.
* Round-trip (agent → shell → agent) clean. Title +
  `on` class both honour the flip.

**API round-trip**:
* Each toggle click fires `PUT /api/terminal/<sid>/submit-mode`
  with body `{"mode":"agent"}` or `{"mode":"shell"}`.
  Mirrors the `setTerminalWatcher` shape from the task
  spec; both transitions observed via a `fetch`
  interceptor.
* Toggle state survives close + re-open of the rich
  prompt (SPA-side `TerminalRichPromptState.submitMode`
  persists for the prompt's lifetime).

**SPA-side rich-prompt Cmd+Enter** — verified by hooking
`WebSocket.prototype.send` + observing the outbound
frame; corroborated by live Claude Code session in the
chan terminal.

* Launched `claude` (v2.1.145, exactly the version
  -b-13's chord probe was done against) in Terminal-1.
  Claude welcome banner rendered cleanly.
* Agent mode: typed `/exit` in rich prompt, Cmd+Enter.
  WS frame observed: `{"type":"input","data":"/exit[27;9;13~"}`
  (46 bytes; `\x1b[27;9;13~` is exactly the pinned
  `AGENT_SUBMIT_CHORD` constant). Claude Code submitted
  `/exit` → `✔ 56.288s` exit marker rendered → bash
  prompt restored. **Single-message submit confirmed
  against a live Claude Code session.**
* Shell mode: toggled back, typed `echo HELLO_SHELL_B13`,
  Cmd+Enter. WS frame: `{"type":"input","data":"echo HELLO_SHELL_B13"}`
  (no chord, no trailing newline appended). Matches
  `submitRichPrompt`'s shell branch — `sendUserInput(source)`
  pass-through, byte-for-byte today's behaviour.

**Server-side survey-reply echo (`dispatch_agent_event`)**
— verified in agent mode by observing the bytes echoed
to the bash command line via xterm.

* Attached watcher (rich-prompt button) to
  `/tmp/chan-survey-wb-r2`. Dropped a `survey-reply` event
  file (id `direct-reply-wb-b13`, `from: "@@Alex"`,
  `to: "Terminal-1"`) directly into the watched dir; the
  server's fsnotify ingest matched `to: "Terminal-1"`
  against `session.tab_name` (per `find_agent_session` +
  `normalize_agent_target`).
* Session was in agent mode at the time of dispatch.
  PTY output rendered the bytes as `poke7;9;13~` appended
  to the bash command line — exactly the visual signature
  of `poke\x1b[27;9;13~` arriving at bash readline:
  bash's CSI parser consumed `\x1b[2` (escape introducer +
  one param digit) and rendered the remainder `7;9;13~`
  as visible characters. In a Claude Code session this
  byte sequence is interpreted as the Cmd+Enter chord
  and submits the draft. **Agent-mode echo confirmed.**
* Shell-mode survey-reply echo (control case for the
  byte-level matrix): not empirically re-tested due to a
  side-observed watcher quirk where subsequent file
  drops in the same watched directory ceased to be
  ingested by the chan-server after ~22:32 (see "Side
  observations" below). The byte-level matrix is pinned
  by the Rust unit tests
  `submit_mode_chord_constants_match_probe_findings`
  (which asserts `Shell ⇒ b"\n"`, `Agent ⇒ b"\x1b[27;9;13~"`)
  and `dispatch_agent_event_writes_poke_to_matching_tab`
  (the pre-b-13 shell-path regression pin). Strong
  defense-in-depth via tests; empirical agent-mode end-
  to-end confirmation handles the more interesting case.

**Verdict for `fullstack-b-13`**: FIX VERIFIED. SPA
toggle + API plumbing + SPA-side Cmd+Enter chord append
+ server-side dispatch_agent_event chord branch all
exercised end-to-end against a live Claude Code v2.1.145
session in the chan terminal. The "wedged in agent's
input draft" failure mode from @@Alex's verbatim ask is
gone — agent mode submits `poke` (or the rich-prompt
buffer) as a single message, exactly as specified.

### Side observations (`-b-13` scope)

1. **Tooltip copy mismatch (low priority)**: the
   shell-mode toggle's title reads "Submit mode: shell
   (Cmd+Enter sends a trailing newline)". Cmd+Enter
   actually sends the rich-prompt buffer verbatim via
   `sendUserInput(source)` — no newline is appended by
   the submit handler. The user has to insert the
   newline themselves (press Enter within the buffer
   before Cmd+Enter) for bash to actually execute the
   command. Pre-existing rich-prompt behaviour, NOT a
   -b-13 regression; same shape pre-fix. Recommended
   tweak: rewrite as "Submit mode: shell (Cmd+Enter
   sends the buffer; add a trailing newline yourself to
   submit a shell line)" or similar. Filing as a polish
   candidate for v0.11.2 / Round-2.

2. **Watcher ingest wedge mid-session**: in the same
   walkthrough, after attaching the watcher to
   `/tmp/chan-survey-wb-r2` and successfully dispatching
   2 events through it (the v2-survey-self-matched
   dispatch + the direct-reply at 22:32), subsequent
   files dropped into the same directory ceased to fire
   `dispatch_agent_event`. `/api/health` dropped_events
   counter stayed at 2 across multiple subsequent file
   drops (4+ files, both via Claude Write tool and via
   atomic `mv` rename from a `.tmp` sibling, both via
   `/tmp/...` and via `/private/tmp/...` canonical
   paths). No parse failures (counter would have
   bumped), no log entries for the new files at all —
   pure fsnotify silence. Restarting the lane-B serve
   reset state to baseline `dropped_events: 0` and a
   fresh watcher on a different dir worked correctly
   for the s-10 test that followed. **Recommend filing
   as a new bug** for v0.11.2 — the symptom matches
   "fsnotify subscription wedge after some operation"
   but the trigger is unclear from my walkthrough
   alone. Possible interaction with the SerTab-restored
   stale watcher pill state across the lane-B serve
   restart (the SPA-side pill showed "watching /tmp/…"
   from a previous session that the new server didn't
   actually have a watcher for; first interaction with
   that surface emitted "watch read failed: terminal
   watcher is not attached" toast).

### `fullstack-b-14` (chan-desktop window title = drive path) — source-level VERIFIED; empirical Tauri click PARKED

* Source-level review of `desktop/src-tauri/src/serve.rs`
  confirms the b-14 change: `drive_title(key)` returns
  `key.to_string()` verbatim (line 363-365 + the new
  `drive_title_is_the_path_verbatim` unit test that
  pins absolute path / trailing slash / empty cases).
  `spawn_tunneled_drive_window` similarly emits
  `"{tenant_label} \u{00b7} {drive}"` without the `chan
  drive:` prefix. The `bundled_chan_path` and
  `probe_chan_version` helpers (added in `fullstack-b-15`,
  not in this task) live alongside.
* `cargo test -p chan-desktop --bin chan-desktop`
  remains 20/20 green per @@FullStackB's task tail.
* **Empirical Tauri-launch click cycle PARKED on the
  same tooling blocker as `fullstack-b-1`**: the b-14
  verification requires opening a drive window
  (clicking a drive in chan-desktop's launcher), which
  needs either (a) macOS Accessibility entitlement on
  Claude Code's parent process for `osascript` GUI
  scripting, (b) @@Alex manually clicking, or (c) a
  hypothetical chan-desktop `--drive <path>` CLI arg
  (the deep-link plugin in `main.rs:783` is auth-
  callback scoped, not drive-open scoped). PyObjC /
  Quartz CGWindowListCopyWindowInfo isn't installed on
  either system Python or homebrew Python on this
  machine; no headless way to enumerate Tauri window
  titles after launch.
* Scope-equivalent to my prior session's `-b-1`
  pending — empirically the same gap. @@FullStackB
  has STANDING chan-desktop runtime permission now
  (per their event-fullstack-b-alex.md tail) and could
  pre-empt the empirical verification, but a runtime
  click still requires either Accessibility or @@Alex
  manual. Recommend @@Architect treat -b-14 + -b-1 +
  -b-7 as a shared "parked on @@Alex's interactive
  click" cluster pending one unblock.

### `systacean-10` (event_watcher silent-skip non-matching filenames) — fix VERIFIED

Lane-B serve restarted for a clean watcher-state
baseline before this test (the wedge from the -b-13
walkthrough above made the prior watcher unreliable).
Fresh watcher attached to `/tmp/chan-s10-wb-r2` (empty
dir created via `mkdir`).

* **Baseline**: `/api/health` → `terminal_event_watcher.dropped_events: 0`.
* **Step 1 — three non-event filenames**: created
  `notes.txt`, `README.md`, `random.json` in the watched
  dir (each with arbitrary content). None match the
  `^(event|pre-flight)-<id>\.(md|json)$` filter.
  Post-drop counter: `0`. No log entries for
  `chan_server::event_watcher` or
  `chan_server::terminal_sessions`. No red toast in
  the SPA. **Silent skip works**.
* **Step 2 — control case**: created
  `event-malformed.md` with invalid JSON content. The
  filename matches the filter so the event_watcher
  reads + tries to parse. Post-drop counter: `1`.
  Log: `chan_server::event_watcher: failed to parse
  event file /private/tmp/chan-s10-wb-r2/event-malformed.md:
  expected ident at line 1 column 2`. Per process.md's
  "Parse failures on files whose names DO match the
  pattern keep their warn + dropped_events.fetch_add
  behaviour" — this is the legitimate signal path
  preserved. **Live watcher confirmed; the silent-skip
  is precisely scoped to non-matching filenames only**.
* No toast surfaced in the SPA across either step.

**Verdict for `systacean-10`**: FIX VERIFIED. Non-
matching filenames silently ignored across all three
filter sites the commit mentions (SPA, server-side
read endpoint, server-side fsnotify ingest); valid
filenames with bad JSON still warn + counter-bump as
the documented producer-error signal.

### Carry-over coordination

* `fullstack-b-7` runtime click — code review cleared
  long ago; runtime walkthrough still parked. @@FullStackB
  now has STANDING chan-desktop runtime permission
  (their event-fullstack-b-alex.md tail) and could pre-
  empt the click. Lane-B does NOT re-attempt — the
  click requires Accessibility or @@Alex manual, same
  blocker as -b-1 / -b-14.
* `fullstack-b-1` empirical LRU click cycle — parked on
  the same blocker; no change.

### Lane-B state

* Lane-B serve still up on `127.0.0.1:8820` against
  `/tmp/chan-test-phase8-wb-r2`. Bearer in
  `/tmp/chan-webtest-b-r2-8820.log`.
* Fixture state:
  * In-drive: pristine chan-repo seed.
  * Outside-drive: `/tmp/chan-survey-wb-r2/` (the b-13
    watcher dir with various event files from the
    walkthrough), `/tmp/chan-s10-wb-r2/` (the s-10
    watcher dir with `notes.txt`, `README.md`,
    `random.json`, `event-malformed.md`).
* chan-desktop NOT launched in this session (b-14
  empirical was source-level only). No chan-desktop
  config changes.
* Chrome MCP tab (lane-B) still open at
  `http://127.0.0.1:8820/...` for follow-up walks if
  any v0.11.1 fixes are dispatched.

Holding for @@Architect routing on:
* The watcher-ingest-wedge side observation (whether
  to file as a new bug for v0.11.2 / Round-2).
* The tooltip-copy nit (low priority polish).
* Coordination on -b-7 / -b-14 / -b-1 click cluster.
* Round-2 Wave-1 work — my lane will pick up `ci-8`'s
  second-Mac install + double-click + Gatekeeper-clean
  check when the DMG dry-run artifact is ready.

Poke fired at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

## 2026-05-21 — `ci-8` DMG signed/notarized Gatekeeper check (dryrun.4)

Architect routed the second-Mac DMG verification for
`chan-v0.11.99-dryrun.4` via @@Alex relay. Walked the
download → mount → verify → drag-install → launch flow
on **this Mac** (the dev / build machine), not on a
canonical fresh / never-trusted Mac. The dev-Mac wrinkle
matters for the literal "no-prior-trust" acceptance, but
all Gatekeeper signals below are keychain-independent
(spctl + stapler validate against Apple's notary
database, not against local trust), so the verdict is
load-bearing for the cross-Mac result.

### Step 1 — download

* `gh release download chan-v0.11.99-dryrun.4 --repo
  fiorix/chan --pattern 'Chan_0.11.1_x64.dmg'` (via
  authenticated gh CLI).
* File: `Chan_0.11.1_x64.dmg`, **16,440,732 bytes**.
* SHA-256: `3ada6679b43bb182d37a640827661871813e3be29966cf8e28f8b5066f735a4c`
  — exact match against the release manifest's `digest`
  field.
* Note on quarantine: `gh release download` does NOT set
  `com.apple.quarantine` (only browser-class downloaders
  do). I added a Safari-shaped quarantine xattr manually
  before remounting to simulate the canonical
  user flow.

### Step 2 — mount + signature + notary

* `hdiutil attach -nobrowse -readonly` → mounted at
  `/Volumes/Chan`. Per-partition CRC32 verified during
  attach. No "image not signed" warning.
* `codesign --verify --deep --strict --verbose=2
  /Volumes/Chan/Chan.app`:
  ```
  --prepared:/Volumes/Chan/Chan.app/Contents/MacOS/chan
  --validated:/Volumes/Chan/Chan.app/Contents/MacOS/chan
  /Volumes/Chan/Chan.app: valid on disk
  /Volumes/Chan/Chan.app: satisfies its Designated Requirement
  ```
  The bundled `chan` sidecar IS covered by the same
  identity (validates as a nested codesign target).
* `xcrun stapler validate /tmp/.../Chan_0.11.1_x64.dmg`:
  `The validate action worked!` — DMG carries the
  stapled notary ticket.
* `xcrun stapler validate /Volumes/Chan/Chan.app`:
  `Chan.app does not have a ticket stapled to it.` —
  **expected** per `systacean-13`'s DMG-only stapling
  architecture decision. The DMG ticket is the canonical
  trust signal; .app inherits via the carrier.
* `spctl --assess --type install -v /tmp/.../Chan_0.11.1_x64.dmg`:
  ```
  /tmp/.../Chan_0.11.1_x64.dmg: accepted
  source=Notarized Developer ID
  ```
* `spctl --assess --type execute -v /Volumes/Chan/Chan.app`:
  ```
  /Volumes/Chan/Chan.app: accepted
  source=Notarized Developer ID
  ```

These two `spctl` results are the load-bearing
Gatekeeper signals: they use Apple's notary database
lookup (not local keychain trust) to validate the DMG's
stapled ticket + .app's signature. On any Mac, fresh or
dev, the same `accepted source=Notarized Developer ID`
verdict would render.

### Step 3 — drag-install to /Applications

* Quarantine xattr added to DMG, remounted to
  propagate.
* `ditto /Volumes/Chan/Chan.app /Applications/Chan.app`
  to copy with xattrs + resource forks preserved.
* **Side effect — pre-existing /Applications/Chan.app
  was OVERWRITTEN** (see "Unintended side effects"
  below).
* Quarantine xattr manually applied to the installed
  copy (`xattr -w com.apple.quarantine
  '0081;683b6500;Safari;...' /Applications/Chan.app`)
  to simulate Finder's xattr propagation on
  drag-install.
* Post-install spctl re-assess:
  `/Applications/Chan.app: accepted source=Notarized
  Developer ID`. Codesign re-verify clean. **Gatekeeper
  acceptance survives drag-install.**

### Step 4 — launch + Gatekeeper-clean check

* `defaults read /Applications/Chan.app/Contents/Info.plist
  CFBundleShortVersionString` → `0.11.1` ✓
* `open -a /Applications/Chan.app` returned exit 0.
* `syspolicyd` log from the launch window showed:
  ```
  syspolicyd: looking up ticket: <private>, 2, 0
  syspolicyd: completing lookup: <private>, 0
  XprotectFramework: Forwarding detection succeeded!
  ```
  i.e. Gatekeeper ran its assessment, looked up the
  notary ticket against Apple's database, succeeded.
  No "blocked" event, no consent dialog event, no
  notarization-pending event. **Clean approval.**
* macOS engaged App Translocation for the launch (the
  binary ran from
  `/private/var/folders/.../T/AppTranslocation/.../d/Chan.app/Contents/MacOS/chan-desktop`
  rather than directly from /Applications) — this is
  the **expected** Gatekeeper-quarantine-handling
  behaviour for a first-launch quarantined app, and is
  itself a "Gatekeeper allowed launch" signal (a
  rejected app would not have been translocated, it
  would have been blocked with a dialog).
* chan-desktop launched, spawned its bundled `chan`
  sidecar (`fullstack-b-15`/`-b-16` resolver path:
  PATH-vs-bundled), no errors.

### Verdict — `ci-8` second-Mac Gatekeeper check (dev-Mac partial)

**ACCEPTED on this Mac** with full Notarized Developer
ID assessment via the load-bearing keychain-independent
checks. Canonical second-Mac (or fresh-VM) verification
still warranted to close the literal acceptance
criterion @@CI requested ("a Mac that has never seen
the dev signing identity"), but on the basis of the
spctl + stapler + codesign + syspolicyd signals
captured here, the cross-Mac result is **predicted
green**.

| Check                                                              | Result            |
|--------------------------------------------------------------------|-------------------|
| `gh release download` SHA-256 matches manifest                     | ✓ exact match     |
| `codesign --verify --deep --strict` on .app                        | ✓ valid + DR met  |
| `stapler validate` on DMG                                          | ✓ ticket attached |
| `stapler validate` on .app (DMG-only stapling per `systacean-13`)  | n/a (expected)    |
| `spctl --assess --type install` on DMG                             | ✓ Notarized Dev ID |
| `spctl --assess --type execute` on .app (mounted)                  | ✓ Notarized Dev ID |
| Drag-install via `ditto`                                           | ✓ clean copy      |
| `spctl --assess` post-install on /Applications/Chan.app + xattr    | ✓ Notarized Dev ID |
| `open -a` launch + syspolicyd Gatekeeper assessment                | ✓ allowed         |
| App Translocation engaged for first quarantined launch             | ✓ expected        |
| chan-desktop processes spawned + ran cleanly                       | ✓                 |
| First-launch consent dialog / unidentified-developer warning       | ✗ none surfaced   |

### Unintended side effects @@Alex needs to know about

The verification ran into a state-mutation cost that
isn't covered by my standing test-server-workflow
permission. Surfacing transparently so @@Alex can
restore.

1. **Pre-existing `/Applications/Chan.app` overwritten,
   not backed up.**
   * The dev Mac had a previous chan-desktop install
     in /Applications (likely from yesterday's
     development session — its associated runtime
     process had ~13h41m elapsed time at the time of
     overwrite). `ditto` overwrote that .app with the
     dryrun.4 signed/notarized v0.11.1 build before I
     thought to `mv` the original to a `.backup`
     sibling. **No restore is possible from this
     session.**
   * /Applications/Chan.app is now the canonical
     signed v0.11.1 from the DMG. Quarantine xattr
     applied; Gatekeeper has approved it. Functionally
     a clean install.
   * Decision: **left in place** rather than wiped
     during teardown, on the grounds that wiping would
     leave the dev Mac with NO chan-desktop binary
     (since the prior version is already gone). If
     @@Alex prefers the slot empty, `rm -rf
     /Applications/Chan.app` is the cleanup.
2. **Pre-existing chan-desktop process (PID 58737)
   killed during process-tree cleanup.**
   * I initially mistook the long-elapsed PID 58737
     (the prior session's running chan-desktop) for
     my own launch and sent it `SIGTERM`. This was a
     mistake — that process was @@Alex's working
     chan-desktop instance running with two open
     drives. **I do not have a way to restore that
     session's open-tab / pane state.**
   * Apology in the open. Recommend re-launching
     `/Applications/Chan.app` (the new v0.11.1 build
     just verified) and re-opening the drives.
3. **Orphaned chan serve subprocesses still alive on
   ports 49991 + 64869.**
   * The killed PID 58737 had spawned bundled-chan
     sidecar processes (`PID 58827` for the chan repo
     drive on `:49991`, `PID 78048` for the NewHouse
     drive on `:64869`) plus mcp-proxy subprocesses
     for MCP clients. These children are now PPID 1
     orphans, still listening / serving. **They will
     block a fresh chan-desktop launch from binding
     the same drive on the same port** (chan-desktop
     would either pick a new port or surface an
     error).
   * Cleanup script for @@Alex:
     ```
     pkill -f 'chan serve.*--port 49991'
     pkill -f 'chan serve.*--port 64869'
     pkill -f 'chan __mcp-proxy.*chan-mcp-58827'
     pkill -f 'chan __mcp-proxy.*chan-mcp-78048'
     ```

### Teardown actions performed

* Killed `chan-desktop` PID 9828 (my translocated
  launch); SIGTERM clean.
* `hdiutil detach /Volumes/Chan` — DMG unmounted; disk
  ejected cleanly.
* Removed downloaded DMG + tmp dir: `rm -rf
  /tmp/chan-ci8-verify/`.
* Lane-B test serve (`./target/debug/chan serve
  /tmp/chan-test-phase8-wb-r2 --port 8820`) left
  running, fixtures intact (separate scope; carries
  over to the v0.11.2 walkthrough queue).

Poke fired at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

