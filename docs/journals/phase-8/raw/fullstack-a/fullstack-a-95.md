# fullstack-a-95 — Remove stale "per-tab scope" mechanic + welcome hint; graph scope is now picker-only post-FS-backbone transition

Owner: @@FullStackA
Cut: 2026-05-23 by @@Architect
Status: dispatched

## Goal

@@Alex flagged: "we no longer have the concept of
scope since we transitioned to using the filesystem
as the backbone of the graph; let's make sure this
is well reflected in the code and docs."

Sweep + remove the stale per-pane-tab-contributes-
scope mechanic + the welcome hint that references
it. Keep the picker-driven scope (drive / dir / file
/ tag / git_repo) selected via the graph overlay
— that's the active concept.

## Reference

@@Alex 2026-05-23 (screenshot of welcome pane):
"Each pane's visible tab is part of the scope for
Graph" — stale.

### Audit findings (architect-side grep)

* `web/src/components/EmptyPaneWelcome.svelte:152`
  — the welcome hint itself.
* `web/src/components/infographicsTabAndCarousel.test.ts:145`
  — test pin that asserts the welcome hint exists.
* `web/src/state/store.svelte.ts:607-635` —
  "One FB overlay + N per-pane browser tabs
  contribute their scopes" — the actual mechanism
  contributing tab-derived scopes.
* `web/src/state/store.svelte.ts:1631` —
  `graphOverlay.scopeId = tab.scopeId;` ties
  tab-derived scope into the overlay.
* `web/src/state/store.svelte.ts:1671` — comment
  "inspector toggle is window-scoped now (was
  per-tab when the…)".
* `web/src/state/store.svelte.ts:1747` — comment
  about per-tab graph selection.
* `web/src/state/store.svelte.ts:1763` —
  `resolveGraphSpawnContext(tab.scopeId)`.
* `web/src/state/tabs.svelte.ts:464` — comment
  about pane-derived scope.

## Scope

### 1. Audit-first

Before deleting code, verify what the per-pane-tab
scope contribution path STILL drives in the UI.
Candidate dead surfaces:
* `fbScopeForSelection` — file-browser tab passing
  selection to graph as scope.
* `tab.scopeId` field on tabs.
* `resolveGraphSpawnContext` consuming tab scope.

If any of these still feed a user-visible flow,
identify it + decide: keep + reword vs remove.

### 2. Remove stale mechanism

For paths that are dead per the audit:
* Drop the per-pane-tab scope contribution loop.
* Drop unused `scopeId` field on tabs if no longer
  read.
* Drop test pins for dead behavior.

### 3. Update welcome hint

Either:
* **Option A** — remove the welcome-hint
  `<p class="welcome-hint">` entirely; nothing
  important is communicated by it now.
* **Option B** — reword to reflect the picker-
  driven scope (e.g. "Pick what shows in Graph
  from the scope dropdown" or similar — brief +
  factual).

Implementer's call after audit; if option B,
text should be ≤2 lines + match the actual
overlay UX.

### 4. Doc sweep

* `web/src/state/store.svelte.ts` comments
  referencing per-tab scope — update or remove.
* `web/src/state/tabs.svelte.ts:464` comment —
  update.
* Architect docs: `design.md`, addendum-a, etc.
  if any mention per-tab scope as a live concept
  (architect can sweep separately if needed; flag
  back).

### 5. Tests

* Update `infographicsTabAndCarousel.test.ts:145`
  to match new welcome content.
* Drop pins for removed mechanism.
* New pin if the picker-driven scope wording
  lands (optional).

## Acceptance

1. **Welcome pane** no longer shows the stale
   "scope for Graph" hint (removed OR reworded).
2. **Per-pane-tab scope contribution mechanism**
   audited; dead code removed.
3. **graphOverlay.scopeId** picker-driven path
   preserved (still user-pickable from drive /
   dir / file / tag / git_repo).
4. **No test regression** post-sweep.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Autonomous-commit per standing batch auth.
* If you find architectural ambiguity (e.g.
  scope is still partially-needed for some narrow
  case), flag back rather than over-prune.

## Authorization

Yes for `web/src/components/EmptyPaneWelcome.svelte`
+ `web/src/state/store.svelte.ts` + `web/src/state/tabs.svelte.ts`
+ related tests + task tail + outbound.

Architect (me) will sweep `design.md` + addendums
in parallel if needed.

## Numbering

This is `-a-95`.

## Out of scope

* Re-architecting the graph overlay's scope
  picker — that's the active concept; keep as-is.
* Filesystem-graph behavior — already shipped via
  `-a-66` umbrella + Drafts saga.

## 2026-05-23 — audit-first findings + sweep applied

Per the architect's audit-first directive, I
walked the candidate dead surfaces before
pruning.

### Audit findings

1. **`web/src/components/EmptyPaneWelcome.svelte:152`
   welcome hint** — **STALE**, per @@Alex's flag.
   Drop.

2. **`web/src/state/store.svelte.ts:607-635`
   "One FB overlay + N per-pane browser tabs
   contribute their scopes"** — **ALIVE, not
   stale, not graph-related**. The mechanism
   is `activeFbScopes()` →
   `fbScopeForSelection()` →
   `pathInAnyScope()`. Consumed by the watcher-
   event hook at `store.svelte.ts:350` to gate
   tree-refresh on event paths (so a watcher
   event for `tasks/foo.md` only re-fetches if
   at least one open FB has selected a path
   under `tasks/`). This is FB watcher-scope
   filtering — has nothing to do with the
   Graph's scope concept. KEEP.

3. **`store.svelte.ts:1631` `graphOverlay.scopeId
   = tab.scopeId`** — **ALIVE, picker-driven**.
   `mirrorGraphTabToOverlay` copies the graph
   tab's user-picked scope (set via the
   GraphPanel scope picker) into the overlay.
   This IS the active picker-driven scope. KEEP.

4. **`store.svelte.ts:1671` "(was per-tab when
   the browser was a tab kind)"** — **historical
   comment, accurate**. Documents the FB
   overlay's inspector-toggle scope history;
   doesn't reference Graph scope. KEEP.

5. **`store.svelte.ts:1747` "per-tab graph
   selection lives inside GraphPanel.svelte"** —
   **accurate**. Documents how
   `resolveSpawnContext` reads a graph tab's
   picker-set `scopeId` to derive the spawn
   CWD when spawning from a graph tab. The
   "per-tab graph selection" here = per-tab
   picker state, which IS the active concept.
   KEEP.

6. **`tabs.svelte.ts:464` graphTabLabel
   comment** — **accurate**. Documents how
   the graph tab title falls back to the
   picker-set scope's derived title when no
   node is selected. KEEP.

### Architect/user wording reconciliation

@@Alex's flag: "we no longer have the concept
of scope since we transitioned to using the
filesystem as the backbone of the graph".

Reading carefully: the welcome hint says
"Each pane's visible tab is part of the
scope for Graph" — implying that
**OPENING a tab automatically contributes
that tab to the Graph's scope**, an
implicit/aggregate behavior. That mechanism
is GONE — there's no code that aggregates
across open tabs into a single Graph view.

The remaining `tab.scopeId` is the user's
EXPLICIT picker choice on a single Graph
tab, not an aggregate behavior. That's the
"active concept" the architect said to keep.

### Sweep applied

Only the welcome hint needed pruning:

* `web/src/components/EmptyPaneWelcome.svelte`
  * Dropped the `<p class="welcome-hint">`
    block with the stale text + `<br>` line
    split.
  * Dropped the `.welcome-hint` CSS rule.
  * Left a retirement comment marker
    (`fullstack-a-95: per-tab-contributes-
    Graph-scope welcome hint dropped`) so
    grep-history finds the bridge.
* `web/src/components/infographicsTabAndCarousel.test.ts`
  * Pin "renders the … hint" flipped to
    require absence of `.welcome-hint` /
    "Each pane's visible tab is part of the
    scope" / the rendered `<p>` shape.
  * Source comment with retired-text echo
    explicitly allowed so the journal
    breadcrumb survives.

### Decisions

* **Option A (drop entirely)**, not Option B
  (reword). The welcome surface is a clean
  spawn grid; an unrelated tip about Graph
  picker-scope would still feel out of place.
  The picker IS reachable from the Graph
  overlay's chrome where the user opens the
  graph, so the discoverability is intact.
* **`activeFbScopes` is FB-not-Graph** — the
  comment block at store.svelte.ts:607 is
  accurate; the "scope" word there refers to
  FB's selection-driven watcher-event filter.
  No rename needed; not in scope of this
  task's "per-tab Graph scope" sweep.

### Files touched

* `web/src/components/EmptyPaneWelcome.svelte`
  * `<p class="welcome-hint">…</p>` dropped.
  * `.welcome-hint` CSS rule dropped.
  * Retirement comment added.
* `web/src/components/infographicsTabAndCarousel.test.ts`
  * Pin flipped from REQUIRE to FORBID for
    the welcome hint + class.

### Architect-side scope flagged back

The audit walk turned up nothing else in code
that needs pruning. The architect can sweep
docs (design.md / addendum-a / etc.) on the
side if any of those reference per-tab Graph
scope as a live concept; SPA side is now
clean.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1321 / 1321** (one pin flipped;
  no net change).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy --all-targets
  -- -D warnings` → clean (no Rust delta).

### Suggested commit subject

```
Welcome pane: drop stale "scope for Graph" hint (fullstack-a-95)
```

### Files (per-path)

* `web/src/components/EmptyPaneWelcome.svelte`
* `web/src/components/infographicsTabAndCarousel.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-95.md`

Autonomous-commit mode. No clearance held.
