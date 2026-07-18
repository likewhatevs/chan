# Release v0.70.3 - the selection-highlight regression patch

Patch round run 2026-07-18 off the v0.70.2 tag. Two fixes, both surfaced from the owner's daily use: a v0.70.2 regression that hid the editor's text-selection highlight, and a long-standing sticky status pill on a refused launcher Open. Diagnosis, verification, gate, and release were integrator-owned in the owner's session; the code is two small, file-disjoint changes. Cut straight to GA without an rc by the owner's call.

## What shipped

- **The editor's text-selection highlight is visible again.** v0.70.2's `ee1174f3` (page-width scrollbar at the viewport edge) moved the page `--bg` off `.cm-editor` and onto `.cm-content`, and dropped `.cm-content` from the transparent-background rule. CodeMirror draws the selection in `.cm-selectionLayer`, a sibling of `.cm-content` inside `.cm-scroller` painted behind it at `z-index: -1` (confirmed against the vendored `@codemirror/view`: the layer z-index is `(above ? 150 : -1) - pos`, and the selection background is scoped `.cm-focused > .cm-scroller > .cm-selectionLayer`). An opaque `.cm-content` background paints over that layer, so with `chan-page-capped` on, and the page cap is the 80% default (`pageWidth.svelte.ts` `DEFAULT_RATIO`), selected text had no visible highlight at all. The fix keeps `.cm-content` transparent (restored to the transparent-background rule) and paints the page `--bg` on a `.cm-content::before` at `z-index: -2`, strictly behind the selection layer, positioned to the content (`position: relative`) so the page stays pixel-aligned with the text. The full-width scroller, the centered page, and the off-page shade that `ee1174f3` introduced are all unchanged.
- **A refused launcher Open no longer sticks on the workspace forever.** `executeOpen` (`state/commands/global.ts`) caught a refusal (a path outside the workspace root, a binary target, or no connected window) with a bare `ui.status = ...`. Per the store's own contract a bare write leaves `statusKind` null, and `AppStatusBar` renders the dismiss control only for `statusKind === "persistent"`, so the pill had no dismiss affordance and never auto-cleared. The catch now sets `ui.statusKind = "persistent"` alongside the message, matching the house pattern (`TerminalTab`, `FileEditorTab`, `FileInfoBody`, the store upload-failed path) and the author's own stated intent that the refusal persist until seen. A `?raw` guard test pins the shape.

## Team / process

Integrator-driven, solo, no subagents: the scope was two small changes on disjoint files (`editor/Wysiwyg.svelte`; `state/commands/global.ts` plus a new test). Both bugs were root-caused against live code, and the selection bug additionally against the vendored CodeMirror source rather than from memory, which is what pinned the exact stacking cause instead of guessing at the CSS. The owner reported both from real use and set the release scope.

## Validation

svelte-check clean (0 errors, 0 warnings) and the full `workspace-app` vitest green (292 files, 2828 tests, the new open-refusal guard included). The selection fix was proven in a real page-capped WYSIWYG editor through the committed `scripts/e2e/browser-smoke` harness (headless Chrome over a throwaway server): a select-all pixel diff changed 50.9% of the editor with the fix versus 0.69% with the pre-fix CSS injected at runtime, and a point sample at a selection rect read CodeMirror's `#d7d4f0` selection lavender with the fix versus the white page without it. The page cap was confirmed on by default in that run (`chan-page-capped`, 1280px). Full `make pre-push` green on the version-bumped GA commit, across both workspaces including the gateway build and the web check with the full vitest and production build.

## Retrospective

### Highlights

- Reading the vendored `@codemirror/view` to confirm the selection layer's `z-index` and DOM position turned a plausible CSS guess into a proven root cause, and made the fix a two-line-of-intent layering change rather than a revert of the v0.70.2 scrollbar work.
- The pixel-diff A/B (fix versus old CSS injected into the same live page) is a self-contained regression proof: it both demonstrates the fix and reproduces the exact reported symptom, without a second build.

### Lowlights

- The launcher-Open pill fix rests on the store contract, the render condition, the four-site house pattern, and a source-shape guard test rather than a live headless drive; the faithful repro needs the command launcher's `Open <path>` arg path, which the `chan:command` bridge does not expose, so a browser drive would have meant scripting the launcher UI for a one-line change.

### Honest feedback

Both regressions are the kind a change-focused review misses: the v0.70.2 scrollbar commit was correct about the scrollbar and never considered that painting the content opaque would occlude the selection layer behind it, and the Open refusal predates this round with a status write that looked right but skipped the `statusKind` half of the pill contract. A short "what sits behind this element, and does this write set both halves of the status shape" check would have caught each at authoring time.

## Follow-ups

- Ubuntu Launchpad (PPA) needs no manual version edit: `packaging/distros/debian/build-source.sh` derives the version from `HEAD:Cargo.toml` and fills the `@VERSION@`/`@DATE@` changelog templates at build time. Only the Fedora COPR specs carry a hand-bumped `%global upstream_version` plus a dated `%changelog` entry. Noted here so the distinction is not re-litigated next patch.
