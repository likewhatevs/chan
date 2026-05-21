# fullstack-a-43 — Hybrid back-side architecture refactor (Task A)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Introduce the per-surface configuration concept for Hybrid
pane back-sides. The back of a Hybrid stops being "another
collection of content tabs" and becomes a configuration
surface scoped to the type of the currently-active front
tab.

Foundation only. Populating the actual settings UI is Tasks
B/C/D/F (separate cuts). Task E drops the front/back
independent theme. Task G (About + donation QR) is already
cut as `fullstack-a-42`, sequenced after A+B+C+F.

## Background

Locked design:
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited — flip becomes per-surface
configuration". Source spec from @@Alex:
[`../alex/hybrid-revisited.md`](../alex/hybrid-revisited.md).

Inspiration: Propellerheads Reason's "flip the rack" UX.
Front = content; back = the wiring behind that surface.

Per-surface scope after the refactor:

| Front-tab type     | Back-side content                                    |
|--------------------|------------------------------------------------------|
| Hybrid Terminal    | All terminal settings (Task B will populate)         |
| Hybrid Editor      | All editor settings (Task C will populate)           |
| Hybrid Graph       | Node-type legend grid (Task D will populate)         |
| Hybrid File Browser| Search / Indexing settings (Task F will populate)    |

## Acceptance criteria

* Four new components mounted:
  `HybridTerminalConfig.svelte`,
  `HybridEditorConfig.svelte`,
  `HybridGraphConfig.svelte`,
  `HybridFileBrowserConfig.svelte`. Each renders the
  family-name title band (e.g. "Hybrid Terminal") + an
  empty body placeholder (Tasks B/C/D/F populate).
* `Pane.svelte`'s flip path reads active-front-tab type +
  mounts the matching back-side component. Switching the
  front tab while flipped swaps the back's content.
* Back side no longer renders a tab strip. The previous
  per-Hybrid back-side tab-collection state gets removed
  cleanly. Per-Hybrid theme (from `-b-5`) stays for now;
  Task E simplifies to single-value.
* Flip animation (`-a-22`) preserved. Only the destination
  changes; the HOW-IT-LOOKS-FLIPPING is unchanged.
* Tests cover: front-tab-type → back-side-component mount
  mapping + swap-on-front-tab-switch behaviour + flip
  animation timing not regressed.

## How to start

1. Read locked design in
   [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
   §"Hybrid back-side revisited" + the source spec at
   [`../alex/hybrid-revisited.md`](../alex/hybrid-revisited.md).
2. Audit current back-side rendering in `Pane.svelte` to
   inventory what gets removed (tab strip, back-side tab
   collection) vs preserved (flip animation, per-Hybrid
   theme).
3. Sketch the new component shape (title band + family
   name + empty body) for the 4 surface types.
4. Wire `Pane.svelte`'s flip path to read active-front-tab
   type + mount the matching back-side component.
5. Migrate (or remove) the back-side tab collection state.

## Coordination

* SPA + state only. No other agent's lane touched.
* Pre-push gate must be green: fmt + clippy + cargo test
  + svelte-check + npm build + vitest.
* When ready for commit, append a "Commit readiness"
  section + fire a poke to @@Architect; do NOT commit
  unilaterally (multi-agent tree commit discipline).
* When this lands in HEAD, Tasks B/C/D/E/F + the
  relocated G (About) cut as follow-ups. Do NOT try to
  land them in this commit.

## Numbering note

Highest committed `-a-N` is `-a-41`; `-a-42` exists as
About + QR (cut 2026-05-21). Per the round-2-plan Hybrid
wave convention, this task takes `-a-43`; subsequent B-F
land at `-a-44..-a-47` (real numbers at fan-out). About
(`-a-42`) keeps its number; it gates on A+B+C+F per its
own task body's prereq chain.

## 2026-05-21 — implementation note

Five-file change. SPA + state only; no Rust touched.

### State model (tabs.svelte.ts)

* `HybridSide` type slimmed from `{ tabs, activeTabId,
  theme? }` to `{ theme? }`. The tab-collection slot is
  gone — the back is no longer content.
* Module comment above the type updated to spell out the
  new invariant: `pane.tabs` ALWAYS describes the front;
  `showingBack` toggles whether the pane renders the front
  content tabs or the back-side configuration view via
  `Pane.svelte`.
* `flipHybrid()` no longer swaps `tabs` / `activeTabId`.
  It only swaps `theme` (preserved for `-b-5`'s per-side
  override behaviour until Task E collapses it) and
  toggles `showingBack`. The `requestPaneFlip(node.id)`
  bus bump stays — `-a-22`'s flip animation is preserved.
* `cloneNode` + `splitPane`: drop `tabs` / `activeTabId`
  from the cloned `back` payload (now just an optional
  theme override).
* `serializeNode`: emits `hb` (back theme) only when the
  back actually has a theme. The `bt` (back tabs) slot is
  removed entirely from emission. `sb` + `ht` unchanged.
* `deserializeNode`: accepts legacy `bt` from older
  session blobs but discards the tab contents — only the
  shape ("the user had a back materialised") survives,
  rendered as `back = {}`. New sessions never carry `bt`.
* `SerLeaf`: `bt` retained as a legacy-tolerated optional
  field (parsed, then dropped). Its doc comment flags
  this for future readers.

### Rendering (Pane.svelte)

* Imports `HybridTerminalConfig` / `HybridEditorConfig` /
  `HybridGraphConfig` / `HybridFileBrowserConfig` from
  sibling components.
* The tab strip (`.tabs` block) is wrapped in
  `{#if !pane.showingBack}` — hidden entirely on the
  back side. The user flips back via the `Cmd+. Tab`
  chord (or `Cmd+. f` / `Cmd+. b` / etc. for specific
  front tab kinds via `-a-32`'s mnemonic set).
* `.editor-wrap` dispatch gains a new top-level branch
  `{#if pane.showingBack && !paneMode.active}` that
  renders the matching `HybridXConfig` based on
  `active?.kind`:
  - `"terminal"` → `<HybridTerminalConfig />`
  - `"file"`     → `<HybridEditorConfig />`
  - `"graph"`    → `<HybridGraphConfig />`
  - `"browser"`  → `<HybridFileBrowserConfig />`
  - none/empty   → `.back-empty` placeholder ("Open a
    tab on the front to configure its surface here.")
  Pane mode takes priority over showingBack so
  `Cmd+. Tab` navigation continues to preview the FRONT
  content + tab structure even when the user is on the
  back.
* Terminal each-block (kept mounted across pane mode for
  scrollback per `-b-2`) gains `!pane.showingBack` on
  both `active` and `focused` props so xterm doesn't get
  pulled into focus while the config view is up.
* `backHasAttention` derived + `.back-attention`
  indicator chrome dropped. The back is no longer
  content; there's nothing on a config surface to flag
  as "unread / active". The CSS keyframes + reduced-
  motion fallback for `.back-attention` removed
  alongside.
* New `.back-side`, `.back-empty`, `.back-title`,
  `.back-hint` CSS for the wrapper + empty-state fallback.
  Each `HybridXConfig` carries its own header/body
  styling so the wrapper stays minimal.

### Four stub components

* `web/src/components/HybridTerminalConfig.svelte`
* `web/src/components/HybridEditorConfig.svelte`
* `web/src/components/HybridGraphConfig.svelte`
* `web/src/components/HybridFileBrowserConfig.svelte`

Each: `<section class="hybrid-config">` with a header
band (family name in `<h2 class="config-title">`) and an
empty `<div class="config-body">` placeholder. Comment
header in each file names the populating task
(B / C / D / F) so the next implementer reads it
without bouncing back to the plan. Scoped CSS handles
header border + body scroll.

### Tests

`web/src/state/tabs.test.ts`:
* "closing the last tab on the back side keeps
  showingBack=true (`-a-11`)" — rewritten as "closing
  a front tab while flipped preserves showingBack=true
  (`-a-11 + -a-43`)". The pre-`-a-43` shape relied on
  `pane.tabs` being the back's tabs while flipped; under
  the new model it's always the front's. The
  regression-pin's INTENT (close-doesn't-flip) stays
  identical.
* "splitting from the back side puts the new pane on
  its back too" — rewritten: drops the `back.tabs`
  / `back.activeTabId` assertions, just pins
  `newPane.back === {}` (the lazy-init shape).
* "first flip lazy-initializes back with inverted theme
  and swaps slots" — rewritten as
  "first flip lazy-initializes back with inverted theme;
  front tabs stay put (`-a-43`)". Asserts
  `live.tabs.map(t.id) === ["front"]` to pin the new
  invariant.
* "flipping back restores the original front + preserves
  user themes" — rewritten as
  "flipping back round-trips showingBack + theme; front
  tabs never swap (`-a-43`)". Three sequential flips,
  front tabs identical throughout, theme swap survives.
* "serialize / restore round-trips back tabs + themes +
  showingBack" — rewritten as
  "serialize / restore round-trips per-side themes +
  showingBack (`-a-43`)". Drops the `bt`/`back.tabs`
  round-trip assertions; new assertion that `"bt":`
  never appears in the serialized JSON.

`web/src/components/Pane.test.ts`:
* Two obsolete pins for `.back-attention` indicator
  (front+back terminal scenarios) dropped with a
  replacement comment explaining why.
* New `describe("Pane back-side configuration view
  (fullstack-a-43)")` block with four pins:
  - Active terminal front tab → `HybridTerminalConfig`
    mounts (by aria-label) + tab strip hidden.
  - Active file front tab → `HybridEditorConfig` mounts
    (by aria-label).
  - No active front tab → `.back-empty` placeholder +
    `[aria-label="hybrid back side"]` wrapper visible.
  - Front-tab content suppressed when `showingBack=true`
    (`.tabs` null + `.back-side` present).

`web/src/components/paneTerminalMount.test.ts`:
* Two pins updated to include the new `!pane.showingBack`
  gate on the terminal each-block's `active` + `focused`
  props (alongside the existing `!paneMode.active`
  gate).

### Gate

* vitest **588 / 588** (no net change; 4 pins
  rewritten, 4 new pins added, 2 pins dropped,
  paneTerminalMount pin regex tightened).
* svelte-check 0 errors / 0 warnings.
* npm build clean.
* `cargo fmt --check` clean.
* `cargo clippy -p chan --all-targets -- -D warnings`
  clean.
* `cargo test -p chan` not re-run (no Rust touched).

### Subtle deviations to flag

* **Theme swap preserved.** Task A's body says "Per-
  Hybrid theme (`-b-5`) stays for now; Task E
  simplifies to single-value." The round-2-plan
  §"Hybrid back-side revisited" §"Implementation
  breakdown" Task A bullet says "drop front/back
  independent theme + tabs collections". I read these
  as: drop the tab collections in Task A, leave the
  theme split intact (per the task file). `flipHybrid`
  still swaps `pane.theme` ↔ `back.theme` so existing
  per-side theme overrides round-trip exactly as
  before. Task E collapses to a single per-Hybrid
  value with the hamburger toggle from `-a-27` flipping
  both sides at once.
* **Back-existence round-trip.** Under the old model
  the back was materialised on every flip with a tab
  collection that emitted `bt` — that doubled as the
  "back exists" wire marker. Under the new model, a
  flipped pane with no theme overrides has nothing to
  emit on `n.back` (no `bt`, no `hb`). After restore,
  `pane.back === undefined` but `showingBack === true`.
  The next flip lazy-inits back fresh, with an
  inverse-theme seed against the current `pane.theme`.
  This produces a SUBTLE first-flip-after-restore
  difference vs an uninterrupted session for the
  narrow edge case "user flipped, set visible-side
  theme dark, did NOT set back-side theme, restored
  via URL hash, then flipped". Existing pre-`-a-43`
  serializer had the equivalent loss for "no theme,
  no tabs"; the new loss is structurally identical.
  Documented in the round-tripped test; not gating.
* **Empty pane on the back.** When a pane has no active
  front tab and the user flips, the back renders a
  generic Hybrid placeholder asking the user to open a
  front tab first. The flip chord still works (user
  can flip back via `Cmd+. Tab`). The placeholder is
  Task A's surface; future polish could route
  Hybrid-NAV chords directly off the back-empty case.

### Suggested commit subject

```
Hybrid back-side architecture refactor: per-surface config view (fullstack-a-43)
```

Single commit. State model + Pane render rewire + 4
component stubs + test updates are tightly coupled
around the same conceptual change; splitting would
leave intermediate states that don't compile (the
type change cascades into Pane.svelte) or render
broken (config component imports without the dispatch
branch).

Push held — multi-agent tree commit discipline +
@@Architect routing the commit per the task's
Coordination section.
