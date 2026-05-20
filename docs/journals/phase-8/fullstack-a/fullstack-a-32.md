# fullstack-a-32: Chord migration + context-aware spawn semantics + surface unification

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Pull the chord migration drafted in
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
"Chord migration + surface unification" forward into the
rich-prompt mini-wave, **expanded** with @@Alex's
2026-05-20 ask for context-aware spawn semantics.

Three coupled pieces, land as one commit:

1. **New chord set** — top-level chords replace the
   `Cmd+K <key>` spawn family.
2. **Context-aware spawn semantics** — each spawn chord
   picks up context from the focused surface.
3. **Surface unification** — empty-pane carousel slide 1
   + pane hamburger menu + empty-pane right-click menu
   all show the same first-class spawn entries.

## Background

Source: `../architect/round-2-plan.md` "Chord migration +
surface unification (added 2026-05-20)" section + @@Alex's
2026-05-20 refinement: "from a doc, cmd+shift+m does graph
from here using the doc; or cmd+t new terminal from current
cwd or doc's parent dir".

`fullstack-b-9` (Round-1) already landed Cmd+T native +
Cmd+Alt+T web + universal Hybrid NAV `t`. That part stays.

### Chord set (final)

| Action          | Native (Chan.app) | Web fallback     | Universal (Hybrid NAV) |
|-----------------|-------------------|------------------|------------------------|
| New terminal    | Cmd+T (-b-9)      | Cmd+Alt+T (-b-9) | Mod+. t (-b-9)         |
| File browser    | Cmd+O             | Cmd+Alt+O        | Mod+. o                |
| Rich prompt     | Cmd+P             | Cmd+Alt+P        | Mod+. p                |
| Graph           | Cmd+Shift+M       | (Hybrid NAV)     | Mod+. v                |

`Cmd+Shift+M` is the placeholder per @@Alex's
"pick anything else, cmd+shift+m for ex for now"; chan-
desktop overrides Tauri-side so Chrome's people-menu
doesn't capture it.

### Removals

Drop these (verify in shortcuts.ts + remove the keymap +
the help text):

* `Cmd+K 1` (was terminal — Cmd+T now)
* `Cmd+K 2` (was file browser — Cmd+O now)
* `Cmd+K 3` (was graph — Cmd+Shift+M now)
* `Cmd+K 4` (was new file — no top-level chord; reachable
  via FB context-menu / FB plus button)
* `Cmd+K p` (was rich prompt — Cmd+P now)

Keep: `Cmd+K t/T`, `Cmd+K f/F`, `Cmd+K h/H`, `Cmd+K <`/`>`,
`Cmd+K Backspace` (per existing -b-9 / phase-7 work).

### Context-aware spawn semantics

Each spawn chord derives its starting context from the
focused surface at the moment the chord fires. The
context-resolution rule:

* **Cmd+T (new terminal)**:
  * Focus on a terminal → cwd = focused terminal's cwd
    (the terminal session knows its own cwd).
  * Focus on a doc (editor surface) → cwd = doc's parent
    directory (the path one level above the doc's file
    path).
  * Focus elsewhere / unclear → cwd = drive root.

* **Cmd+O (file browser)**:
  * Focus on a doc → FB opens with the doc's parent
    directory expanded + the doc highlighted.
  * Focus on a terminal → FB opens with the terminal's
    cwd directory expanded.
  * Focus elsewhere → FB opens at drive root (today's
    behaviour).

* **Cmd+P (rich prompt)**:
  * Already context-bound to the active terminal (existing
    `showOrSpawnRichPromptInFocusedPane` does this). Wire
    Cmd+P to that same function — no new semantics needed.

* **Cmd+Shift+M (graph)**:
  * Focus on a doc → graph from here, source node = the
    doc. Pairs with [`fullstack-a-33`](fullstack-a-33.md)
    (graph "from here" as default mode).
  * Focus on a terminal → graph from here, source node =
    the terminal's cwd (graph the directory).
  * Focus elsewhere → drive-scoped graph (today's
    default).

The context-resolution helper should be a single named
function called by all four chord handlers — single source
of truth for "what's the focused surface and its
context?". Suggested shape:

```ts
function focusedSurfaceContext(): {
  kind: "terminal" | "doc" | "fb-tree-row" | "none";
  cwd?: string;      // terminal sessions
  filePath?: string; // docs / fb rows
  parentDir?: string; // derived for docs / fb rows
};
```

### Surface unification

Three menus must show the same first-class spawn entries
in the same order:

| Surface                            | File                          |
|------------------------------------|-------------------------------|
| Empty-pane carousel slide 1        | `EmptyPaneCarousel.svelte`    |
| Pane hamburger menu                | `Pane.svelte::paneMenu`       |
| Empty-pane right-click menu        | `Pane.svelte::emptyPaneMenu`  |

Items + ordering:

1. Terminal (icon + label + chord hint "Cmd+T")
2. File Browser (icon + label + chord hint "Cmd+O")
3. Rich Prompt (icon + label + chord hint "Cmd+P")
4. Graph (icon + label + chord hint "Cmd+Shift+M")
5. Separator
6. Existing items (highlight colour picker, etc.) stay
   below the separator.

Clicking each first-class item triggers the same
chord handler as the chord — so the context-aware spawn
semantics apply uniformly whether the user fires a chord
or clicks a menu item.

## Acceptance criteria

* `Cmd+O`, `Cmd+P`, `Cmd+Shift+M` all spawn the right
  surface, with context-aware semantics per the table
  above.
* Web fallbacks (`Cmd+Alt+O`, `Cmd+Alt+P`) reachable on
  both Mac + non-Mac browsers.
* Hybrid NAV `o` / `p` / `v` work universally (web +
  native).
* Old `Cmd+K 1/2/3/4/p` chords no longer fire.
* `PaneModeHelp` + `SERVE_LONG_ABOUT` reflect the new
  chord set (audit + resync; the chord-table-drift sweep
  from `fullstack-a-19` is the template).
* The three surfaces (carousel slide 1 + pane hamburger +
  empty-pane right-click) show identical first-class
  spawn entries in identical order; clicking any entry
  triggers the same chord handler.
* Context-aware spawn: spawning a terminal while focused
  on `crates/chan-drive/src/lib.rs` (in an editor tab)
  produces a terminal at `crates/chan-drive/src/`.
  Spawning a graph while focused on the same file
  produces a graph rooted at that file (depends on -a-33
  for the default rendering mode).
* `vitest` green; pin the context-resolution helper +
  the chord-handler routing.

## How to start

1. Read `web/src/state/shortcuts.ts` for the current
   chord registry. The four spawn chords each have a
   descriptor; extend with the new universal +
   web-fallback bindings.
2. Read `EmptyPaneCarousel.svelte` slide 1 (today the
   shortcuts table) + `Pane.svelte::paneMenu` +
   `Pane.svelte::emptyPaneMenu` to find the existing
   menu shapes; converge them.
3. Implement the context-resolution helper first; it
   becomes the load-bearing piece every chord handler
   calls.
4. Wire `Cmd+O` / `Cmd+P` / `Cmd+Shift+M` to call the
   helper + dispatch the spawn.
5. Drop the old `Cmd+K 1/2/3/4/p` keymap entries.
6. Resync the help cheatsheets.

## Coordination

* Couples with [`fullstack-a-33`](fullstack-a-33.md) —
  the graph "from here" default that this task's
  `Cmd+Shift+M` handler depends on. Land -33 first OR
  in the same commit if scope overlaps; the chord
  handler in -32 expects the default rendering mode in
  -33 to land first or land defensively.
* Coexists with [`fullstack-a-28`](fullstack-a-28.md) /
  [`-29`](fullstack-a-29.md) / [`-30`](fullstack-a-30.md) /
  [`-31`](fullstack-a-31.md) — different surfaces; no
  file conflict expected on the chord layer + the
  carousel / hamburger / right-click menus.
* @@WebtestA verifies on lane-A; @@WebtestB welcomed for
  native-chan-desktop chord verification (`Cmd+Shift+M`
  capture under Tauri).
* Push held for the patch-release commit-grouping cut.

## 2026-05-20 — impl note + ready for review (fresh @@FullStackA session)

Landed in one commit-ready bundle. Files touched:

| File | Reason |
|------|--------|
| `web/src/state/shortcuts.ts` | Three new chord descriptors (`app.files.toggle` / `app.terminal.richPrompt` chord update / `app.graph.toggle`); registry comment refreshed to record `fullstack-a-32`'s new spawn family. |
| `web/src/state/store.svelte.ts` | New `openGraphWithContext(ctx)` mirrors `paneModeOpenGraph` for the live layout. Live-layout sibling of the draft-only Hybrid NAV path. |
| `web/src/App.svelte` | Context-aware spawn helpers (`spawnTerminal/Browser/RichPrompt/GraphFromContext`); top-level chord handlers for `Cmd+Alt+O/P` + `Cmd+Shift+M`; existing `Cmd+Alt+T` rewired through helper; `Alt+Space` legacy alias kept; `runCommand` chan:command bridge routes through helpers; Hybrid NAV numeric `1/2/3/4` cases dropped; `o/O` and `v/V` mnemonic cases added; `t/T` and `p/P` retained. |
| `web/src/components/Pane.svelte` | New `spawnActions` list = single source of truth for the four first-class items. Pane hamburger menu prepends the four entries + separator above Enter Hybrid NAV. Empty-pane right-click menu uses the same list + extras (Search) below the separator. |
| `web/src/components/EmptyPaneCarousel.svelte` | Slide 1 (Welcome) gains a 4-up `spawn-row` of clickable spawn buttons above the ASCII shortcut table; dispatches via `chan:command` so it goes through the context-aware helpers. |
| `web/src/components/PaneModeHelp.svelte` | Spawn group cheatsheet: drop numeric caps (1/2/3/4), surface letter mnemonics (t/o/p/v) only. |
| `crates/chan/src/main.rs` | `SERVE_LONG_ABOUT` regenerated from `renderTable("web", "mac")`; Hybrid NAV section updated to `t / o / p / v` mnemonics. |
| `desktop/src-tauri/src/serve.rs` | `KEY_BRIDGE_JS` gains native bindings for `Cmd+O` (`app.files.toggle`), `Cmd+P` (`app.terminal.richPrompt`), `Cmd+Shift+M` (`app.graph.toggle`); legacy negative-assertion + positive-assertion tests updated. |
| `web/src/components/paneModeKeymap.test.ts` | Existing 1/2/3/4 case pins replaced with t/o/p/v + numeric-absence assertions; new top-level chord handler pins for `Cmd+Alt+O/P` + `Cmd+Shift+M` + `chan:command` bridge. |
| `web/src/components/paneModeHelpClickable.test.ts` | Cheatsheet pins updated: numeric caps absent; letter mnemonics present. |
| `web/src/components/Pane.test.ts` | Pane right-click + hamburger menu pins updated for the new ordered set. |

### Chord set (after -a-32)

| Action          | Native (Chan.app) | Web fallback     | Universal (Hybrid NAV) |
|-----------------|-------------------|------------------|------------------------|
| New terminal    | Cmd+T             | Cmd+Alt+T (Mac)  | Mod+. t                |
| File browser    | Cmd+O             | Cmd+Alt+O (Mac)  | Mod+. o                |
| Rich prompt     | Cmd+P             | Cmd+Alt+P (Mac)  | Mod+. p                |
| Graph           | Cmd+Shift+M       | Cmd+Shift+M      | Mod+. v                |

Note on Cmd+Shift+M: browsers don't reserve it, so the same
chord works in web + native; no Cmd+Alt+M fallback needed.

Hybrid NAV `1/2/3/4` removed entirely. `4` (new file) was a
Hybrid-only chord with no top-level equivalent; per the task
spec, new-file is now reachable only via the FB context menu /
plus button. `Alt+Space` stays bound as a secondary rich-prompt
alias (legacy muscle memory; not advertised in the registry to
avoid duplicate rows in the chord table).

### Context-aware spawn semantics

Single helper `resolveSpawnContext()` (already shipped in
`fullstack-43`) returns `{ dir, file? }` based on the focused
tab kind:

* terminal → `{ dir: cwd }`
* file → `{ dir: parentDir(path), file: path }`
* browser → from `browserSelection`
* graph → from `scopeId`

Each new chord handler resolves the context fresh at keypress
time and threads it through the matching spawn API:

* `Cmd+T` → `openTerminalInActivePane({ cwd: ctx.dir })`
* `Cmd+O` → `revealAndSelect(ctx.file || ctx.dir)` + `openBrowser()`
* `Cmd+P` → `showOrSpawnRichPromptInFocusedPane()` (already
  context-bound — talks to the focused pane's terminal)
* `Cmd+Shift+M` → `openGraphWithContext(ctx)` →
  `openGraphInActivePane({ mode: "semantic", scopeId,
  depth: 1, pendingSelectId })`. `fullstack-a-33`'s default
  "from here" rendering means the new graph spawns already
  scoped + the breadcrumb above the inspector body renders
  the ancestor chain.

### Surface unification

Four first-class spawn entries appear in three surfaces with
identical ordering:

| Surface                             | Where |
|-------------------------------------|-------|
| Empty-pane carousel slide 1         | `EmptyPaneCarousel.svelte` `spawnEntries` |
| Pane hamburger menu                 | `Pane.svelte` `spawnActions` prepended above Enter Hybrid NAV |
| Empty-pane right-click menu         | `Pane.svelte` `emptyPaneActions` (= `spawnActions`) + extras + Settings |

Each surface dispatches the same `chan:command` events so the
context-aware helpers route uniformly. Click + chord behaviour
is identical down to the resolved spawn destination.

### `openGraphWithContext` — why a new function

`openGraph()` (no args) is the legacy "open at drive scope"
entrypoint with a single in-app caller. Adding an optional
`ctx?: SpawnContext` parameter to it would have:

1. Changed the signature for the caller (chan:command bridge)
   to require an argument-resolution branch.
2. Mixed the "no context, drive scope" path with the
   "context-aware, file/dir scope" path — two distinct
   intentions in one function.

A sibling `openGraphWithContext(ctx)` keeps each entrypoint's
intent clear. `openGraph()` stays for any future "open at
drive scope unconditionally" caller; `openGraphWithContext()`
is the one the chord layer / chan:command bridge now use.

### Native-side hot path

`KEY_BRIDGE_JS` intercepts the OS-reserved chords (Cmd+O,
Cmd+P) BEFORE the browser eats them. Tauri 2 webviews don't
reserve those chords at the OS level, so a plain JS keymap
captures them just fine — the bridge fires the matching
`chan:command` event back at the same window, which lands in
`runCommand` on the SPA side and routes through the same
helpers the web chords use.

Cmd+Shift+M wasn't browser-reserved historically (the pre-
`fullstack-42` binding worked on web directly), but the
KEY_BRIDGE_JS branch covers it anyway for parity. Chrome /
Safari / Firefox don't claim Cmd+Shift+M as of 2026-05-20.

### Tests

Three test files updated, one new shape pinned:

* `paneModeKeymap.test.ts`: numeric `1/2/3/4` cases
  asserted ABSENT; letter mnemonic cases (`t/o/p/v`) asserted
  PRESENT with the matching `paneModeStageSpawn` /
  `commitPaneMode` shape. New section pins the top-level
  chord handlers (`Cmd+Alt+T/O/P` + `Cmd+Shift+M`) and the
  chan:command bridge wiring.
* `paneModeHelpClickable.test.ts`: cheatsheet caps now
  pinned as `t/o/p/v`; `1/2/3/4` asserted absent.
* `Pane.test.ts`: hamburger menu labels expected to include
  the four spawn entries at the top; empty-pane right-click
  labels include the same four + Search + Settings.
* `desktop/src-tauri/src/serve.rs::tests`: negative
  assertions slimmed (the four spawn commands no longer
  asserted absent — they're now in KEY_BRIDGE_JS);
  positive assertions added for the three new commands
  (`app.files.toggle`, `app.terminal.richPrompt`,
  `app.graph.toggle`).

### Composition

* Hard-pair follow-on of [`fullstack-a-33`](fullstack-a-33.md):
  the breadcrumb + default from-here mode let `Cmd+Shift+M`
  spawn a graph already scoped + ancestor-navigable. -33
  was committed first; -32 layers cleanly on top.
* Coexists with [`fullstack-a-28`](fullstack-a-28.md) /
  [`-29`](fullstack-a-29.md) / [`-30`](fullstack-a-30.md) /
  [`-31`](fullstack-a-31.md): no shared file edits (chord
  layer vs bubble overlay vs rich-prompt internals vs
  terminal broadcast). The cheatsheet sync was a single-
  file change in PaneModeHelp; no overlap.
* `fullstack-b-9`'s `Cmd+T` native + `Mod+. t` universal
  preserved exactly — extended pattern to `o`, `p`, `v`.
* `fullstack-a-7`'s `Cmd+.` Hybrid NAV entry chord unchanged.
* `fullstack-a-22`'s pane flip animation chord (`Cmd+. Tab`)
  unchanged.
* `fullstack-a-27`'s Hybrid hamburger entries (Theme /
  Flip) sit below the new spawn block in the pane
  hamburger menu, gated on `pane.back !== undefined` as
  before.

### Gate

* vitest: **530 / 530** (+5 net from `-33`'s 525, all
  from new positive pins on the chord layer).
* svelte-check: 0 errors / 0 warnings across 3976 files.
* npm build: clean.
* `cargo fmt --check`: clean.
* `cargo clippy -p chan -- -D warnings`: clean.
* `cargo test -p chan`: 58 / 58 passed.
* `cargo test --no-default-features key_bridge` (desktop):
  2 / 2 passed.

### Suggested commit subject

```
Chord migration + context-aware spawn + surface unification (fullstack-a-32)
```

Single commit covering the SPA chord layer, the native
KEY_BRIDGE_JS, the SERVE_LONG_ABOUT cheatsheet, the
PaneModeHelp cheatsheet, the three menu surfaces (pane
hamburger, empty-pane right-click, carousel slide 1), and
all test updates. The pieces are tightly coupled (shortcut
descriptors, chord handlers, cheatsheets, surface unification
all share the new chord set); splitting would leave the
intermediate state with stale cheatsheets or untested chord
paths.

Push held for the patch-release commit-grouping cut.
