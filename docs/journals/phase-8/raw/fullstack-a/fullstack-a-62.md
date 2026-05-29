# fullstack-a-62 — Docked file browser: fade long filenames at edge (don't wrap to 2 lines)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Make docked file browser rows fade long filenames at
the right edge (like tab names) instead of wrapping to
2 lines. Resize the FB column → mask extent changes
automatically.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "Docked file
browser wraps long filenames to 2 lines instead of
fading at the edge (like tab names)" — full bug body
with audit-confirmed file pointers + fix-shape CSS.

## Fix shape

`web/src/components/FileTree.svelte:1039-1048` `.name`
gets the fade-mask pattern from `Pane.svelte:1607-1608`:

```css
.name {
  /* keep existing flex:1 + button reset */
  display: block;
  white-space: nowrap;
  overflow: hidden;
  mask-image: linear-gradient(to right, black calc(100% - 1.25rem), transparent);
  -webkit-mask-image: linear-gradient(to right, black calc(100% - 1.25rem), transparent);
}
.tree.right-dock .name {
  /* mirror for right-aligned text in right-dock */
  mask-image: linear-gradient(to left, black calc(100% - 1.25rem), transparent);
  -webkit-mask-image: linear-gradient(to left, black calc(100% - 1.25rem), transparent);
}
```

Resize behavior is automatic — mask is keyed off the
row's own width.

## Acceptance

1. **Long filename fades at edge**: filenames like
   `chan-desktop-onboarding-redesign.md`,
   `phase-9-desktop-native-vision.md` render on ONE
   line with fade-out on the right edge. No 2-line
   wrap.
2. **Resize widens visible text**: pull the FB column
   wider → more of the filename visible; less fade.
3. **Resize narrows visible text**: pull the FB column
   narrower → more of the filename faded; never wraps.
4. **Right-dock mirror**: when FB is on the right
   dock, fade direction mirrors (fades to the LEFT
   edge since text right-aligns).
5. **Overlay variant**: left-dock and overlay variants
   keep the default left-to-right fade.

### Tests

Vitest pin for the CSS class shape (regex match on the
`mask-image` property in compiled CSS — same as the
existing `Pane.svelte` style pin if one exists). If
DOM render testing is impractical for this style-only
change, structural source pin is acceptable.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA lane. SPA-only CSS.
* Atomic-audit-commit discipline.
* ~10 LOC change; trivial scope.

## Authorization

**Yes** for `web/src/components/FileTree.svelte` +
test pin + task tail + outbound.

## Numbering

This is `-a-62`.

## Out of scope

* Tab-strip fade pattern changes (already shipped per
  `Pane.svelte:1594-1608`; don't touch).
* FB column resize semantics beyond the visual fade.
* FB row vertical density / line-height tuning.

## 2026-05-22 — ready for review

Two-file change. SPA CSS only; no Rust touched.

`web/src/components/FileTree.svelte`:

* `.name` rule: added `display: block` +
  `white-space: nowrap` + `overflow: hidden` +
  `mask-image: linear-gradient(to right, black
  calc(100% - 1.25rem), transparent)` +
  `-webkit-mask-image` prefix. Mirrors the
  `Pane.svelte` tab-name fade pattern verbatim,
  including the 1.25rem mask width for visual
  parity.
* `.tree.right-dock .name` rule: mirrored
  mask-image direction (`to left`) so the fade
  flips when the FB is on the right dock + text
  right-aligns.

`web/src/components/fileTreeNameFade.test.ts`
(new): 4 raw-source pins (single-line shape,
default right-edge fade, right-dock left-edge
mirror, existing flex/button-reset preservation).

### Acceptance

1. Long filenames render on ONE line with fade
   on the right edge ✓ (CSS contract).
2. Resize widens visible text — mask is keyed
   off the row's own width ✓ (no JS needed).
3. Resize narrows visible text — same ✓.
4. Right-dock mirrors fade direction ✓ (CSS
   contract — `.tree.right-dock .name` selector).
5. Overlay variant keeps default left-to-right
   fade ✓ (the overlay doesn't have
   `.right-dock` class so the default rule
   applies).

### Gate

* vitest **722 / 722** (+4 net from `-a-58`'s
  718).
* svelte-check 0 errors / 0 warnings across
  3998 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Suggested commit subject

```
File tree: fade long filenames at edge instead of wrapping (fullstack-a-62)
```

Single commit. CSS rule + test pin tightly
coupled.

### Files for `git add` (per-path discipline)

* `web/src/components/FileTree.svelte`
* `web/src/components/fileTreeNameFade.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-62.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the `feedback-atomic-audit-commit` memory
rule. Working tree currently has @@Systacean's
`-22` work in progress (chan-server/src/routes/
graph.rs) — explicit per-path `git add` only,
no broad-add.

Push held — multi-agent tree commit discipline.
Standing by for clearance.
