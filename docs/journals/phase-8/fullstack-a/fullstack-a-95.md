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
