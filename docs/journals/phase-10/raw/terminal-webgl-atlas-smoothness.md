# Terminal glyph-rendering smoothness: remove the per-frame WebGL atlas-clear workaround

Phase 10. Owner: @@Frontend.

This document carries the full implementation plan (verbatim copy of
`~/.claude/plans/jolly-cuddling-wand.md`) so the phase journal holds the
rationale and the step ordering, and reserves an "Implementation summary"
section at the end, filled in when the round closes.

---

## Plan

### For the next agent (read first)

**Goal of this round.** Make the embedded terminal render rich/animated
TUI output smoothly (iTerm2-class) by removing the per-frame WebGL
texture-atlas clear that currently force-repaints every terminal pane
~60x/sec. Root cause and evidence are in Context below.

**Scope — two files only.**
- `web/src/components/TerminalTab.svelte`
- `web/src/components/TerminalTab.renderer.test.ts`

**Coordination — you are not alone in this tree.** @@Desktect is running
the chan-desktop track in parallel (work lives under `desktop/`; see
`docs/journals/phase-10/desktop-in-process-registry.md`). This change is
web-only and should not collide. BUT: if you find you need to touch any
file outside the two above — anything under `desktop/`, or if @@Desktect
turns out to be mid-edit on `TerminalTab.svelte` or terminal rendering —
STOP and reach out to @@Alex for coordination before editing. @@Alex
merges this alongside @@Desktect's desktop changes and runs the final
manual verification; do not assume your branch lands in isolation.

**Deliverable — journal + summary + commit.**
1. As the first step, copy this plan verbatim into
   `docs/journals/phase-10/terminal-webgl-atlas-smoothness.md`, matching
   the phase-10 convention (title H1, a `Phase 10. Owner: @@Frontend.`
   line, a `## Plan` section holding this plan, and a reserved
   `## Implementation summary` section at the end). Use
   `desktop-in-process-registry.md` as the format reference.
2. Implement the changes.
3. At the end of the round, fill in the `## Implementation summary`
   section (what changed, what the verification showed, whether the
   cross-pane case held up, any fallback applied) and commit the journal
   together with the code changes as one deliverable.

### Context

The embedded terminal (`web/src/components/TerminalTab.svelte`) uses
xterm.js with `@xterm/addon-webgl` for gap-free box-drawing / block
glyphs. @@Alex reports glyph rendering glitches during rich/animated
TUI redraw, across multiple panes, after resize/split-drag, and after
theme switch / flip — and wants the terminal to feel as smooth as
iTerm2 for TUI-heavy workloads in the webview.

The flip was the initial suspect; it is ruled out. The flip is a pure
CSS `rotateY` animation on `.pane` (`Pane.svelte:1534-1543`), not WebGL.
WebGL lives in exactly one place: the xterm renderer in TerminalTab.

#### Root cause (grounded in installed library source)

Installed stack is modern: `@xterm/xterm` **6.0.0**, `@xterm/addon-webgl`
**0.19.0** (`node_modules/@xterm/*/package.json`). That version already
handles natively the things an earlier read flagged as "missing guards":

- **DPR changes**: xterm core has `onDprChange` / `_setDpr` / `updateDpr`
  with `matchMedia` listeners; the WebGL addon subscribes and rebuilds.
- **Theme/color changes**: the addon has an `onChangeColors` handler that
  refreshes its atlas; setting `term.options.theme` routes through it.
- **Options changes**: the addon has `handleOptionsChanged`.

The actual smoothness problem is the **`fullstack-a-97` workaround
itself**. The addon keeps a process-global texture-atlas cache
(`acquireTextureAtlas`): every terminal whose font config matches an
existing entry is pushed onto that entry's `ownedBy[]` and **shares one
`TextureAtlas`** (verified in the bundled source — the cache returns
`f.atlas` when `Mi(f.config, u)` matches). All Hybrid Terminal panes
have identical config (font chain, size 14, lineHeight 1.2), so they
share **one** atlas.

On top of that, `TerminalTab.svelte` scans PTY output for SGR color
sequences (`bytesContainSgrSequence`, lines 460-472) and, on any chunk
that contains one, calls `clearTextureAtlas()` + full-screen
`refreshTerminalRows()` coalesced to once per `requestAnimationFrame`
(`maybeRefreshWebglAtlas`, lines 474-483; invoked at lines 639 & 648).
For an animated TUI emitting color every frame this clears the **shared**
atlas and force-repaints **every terminal pane** ~60×/sec. That is a
textbook cause of the un-smooth, glitchy rendering across panes.

The workaround was written against an older addon to stop cross-pane
glyph substitution; @@Alex still observes cross-pane glitches **with it
active**, which means it is both ineffective at its goal and a jank
source. Decision (per @@Alex): remove it directly, then verify.

### Changes

#### 1. `web/src/components/TerminalTab.svelte` — remove the SGR atlas-clear workaround

Delete only the per-frame workaround; keep all event-driven refreshes.

Remove:
- State/consts: `WEBGL_ATLAS_SCAN_TAIL_BYTES` (180), `webglAtlasRefreshQueued`
  (182), `webglAtlasScanTail` (183).
- `bytesContainSgrSequence` (460-472) and `maybeRefreshWebglAtlas`
  (474-483) in full.
- The two call sites in the WebSocket `onmessage` handler (lines 639 &
  648) — the `writePtyOutput(bytes)` calls stay; only the trailing
  `maybeRefreshWebglAtlas(bytes);` lines go.
- The teardown resets for the removed state (lines 925-926).
- The `fullstack-a-97` rationale comment block in the test (see below);
  in the component there is no separate a-97 comment to remove beyond the
  function bodies.

Keep unchanged (these are event-driven, not per-frame — acceptable):
- `webglRendererActive` (181) — still gates `clearTextureAtlas` and the
  `onContextLoss` fallback.
- `clearTextureAtlas` (392-397), `refreshTerminalRows` (399-405),
  `refreshTerminalRenderer` (407-419).
- `refreshTerminalRenderer()` calls on mount (590), focus (277), blur
  (302), and host-resume (425/432). These fire on discrete events, not
  per data chunk.
- The WebglAddon load + try/catch + `onContextLoss` block (556-589) and
  its gap-free-glyph rationale comment — unrelated to the workaround.

#### 2. `web/src/components/TerminalTab.renderer.test.ts` — update the pins

This file is source-text regex pins. Remove the block that locks in the
deleted workaround; keep everything else.

- Remove the `"refreshes the WebGL texture atlas after animated SGR
  output"` test (lines 43-64) and its `fullstack-a-97` comment.
- In the binary-output test, drop the assertions that require
  `writePtyOutput(bytes); ... maybeRefreshWebglAtlas(bytes);` ordering
  (the `maybeRefreshWebglAtlas` parts of the regexes at ~60 & ~63);
  keep the assertions that `ArrayBuffer`/`Blob` convert to `Uint8Array`
  and call `writePtyOutput` without String coercion.
- Keep the WebglAddon construction / `onContextLoss` / try-catch-fallback
  pins (14-41) and the focus/blur/host-resume `refreshTerminalRenderer`
  pins (80-99) — those behaviors are unchanged.
- Add one negative pin asserting the per-data-chunk atlas clear is gone,
  so it cannot silently return: e.g. assert the file does **not** match
  `/maybeRefreshWebglAtlas/` and does **not** match
  `/bytesContainSgrSequence/`.

#### 3. Theme-change insurance (conditional — only if verification needs it)

First rely on the addon's native `onChangeColors` (triggered by
`applyTerminalTheme` setting `term.options.theme`, lines 387-390). If
the manual theme-switch test still shows stale-colored glyphs, add a
single guarded `clearTextureAtlas()` at the end of `applyTerminalTheme()`
(one-shot per theme change, not per frame — cheap). Do not add this
pre-emptively; prove it's needed first.

### Risk and fallback

Primary risk: because panes share one atlas, removing the SGR clear could
let cross-pane glyph substitution reappear if the shared atlas overflows
during heavy TUI use. @@Alex already sees cross-pane glitches *with* the
workaround, so removal is unlikely to be worse — but it must be checked.

Note on "repainting other panes": legitimate cross-pane repaints already
happen on their own. When multiple panes resize (split-divider drag,
window resize), each affected terminal repaints independently via its own
`ResizeObserver` -> `queueFit()` -> `fit.fit()` -> `term.resize(cols,
rows)` path (`TerminalTab.svelte:595-597, 844-887`). That repaint is
driven by a real layout change and does not touch the shared texture
atlas (glyphs are unchanged, only the grid dimensions). So no pane needs
another pane's *output* to trigger its repaint — the workaround's
cross-pane force-repaint is redundant with what resize does naturally,
which is further reason removal is safe.

If cross-pane substitution reproduces after removal, the fallback is a
**heavily debounced** atlas clear (250-500 ms trailing, coalesced) on
SGR-bearing output instead of the per-rAF clear — ~2-4×/sec bounds
contamination while keeping animation smooth. The deeper proper fix
(per-pane atlas isolation) is an upstream/addon concern; track as a
follow-up only if the debounce proves insufficient. Do not reintroduce
the per-frame clear.

### Verification

Use the test-server workflow (throwaway drive under `/tmp/chan-test-*`,
seeded with a couple of notes). Build: `cargo build -p chan` after
`npm run build` in `web/`; launch `./target/debug/chan serve <path>`.
Fresh-binary discipline: `pkill -f "chan serve"` + rebuild before each
empirical pass so a stale bundle can't produce a false positive.

Automated gate (pre-push): from `web/`, `npm run build`,
`npx svelte-check`, and the vitest suite (must pass with the rewritten
`TerminalTab.renderer.test.ts`); from repo root `cargo test`,
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.

Manual, in two side-by-side terminal panes (the cross-pane case):
1. **Rich TUI**: run an animated, color-heavy TUI in pane A (e.g. a
   running `claude`/`htop`/`top` or `vim` with syntax colors). Confirm
   pane A redraws smoothly with no glyph flicker, and pane B's glyphs are
   never substituted/corrupted while A churns.
2. **Resize / split drag**: drag the split divider during active TUI
   output; confirm no glyph corruption during or after the drag.
3. **Theme switch**: toggle light/dark (and the per-surface terminal
   theme override) with a TUI on screen; confirm colors update cleanly
   with no stale glyphs. If stale glyphs appear, apply change #3.
4. **Flip**: flip a Hybrid Terminal to its config back-side and back;
   confirm the terminal repaints cleanly on return.
5. **Monitor move** (if a second display is available): drag the window
   between a Retina and non-Retina display; confirm glyphs re-sharpen
   (validates that core `onDprChange` handling suffices without our help).

Capture a short before/after comparison of the rich-TUI case so the
smoothness change is documented per the project's factual-analysis rule.

---

## Implementation summary

### What changed

`web/src/components/TerminalTab.svelte`: removed the `fullstack-a-97`
per-frame WebGL atlas-clear workaround in full — the
`WEBGL_ATLAS_SCAN_TAIL_BYTES` constant, the `webglAtlasRefreshQueued` /
`webglAtlasScanTail` state, the `bytesContainSgrSequence` and
`maybeRefreshWebglAtlas` functions, the two `maybeRefreshWebglAtlas(bytes)`
call sites in the WebSocket `onmessage` handler, and the matching teardown
resets. The `writePtyOutput(bytes)` writes and all event-driven refreshes
(mount, focus, blur, host-resume, font-ready) are untouched;
`webglRendererActive`, `clearTextureAtlas`, `refreshTerminalRows`, and
`refreshTerminalRenderer` stay as the event-driven primitives.

`web/src/components/TerminalTab.renderer.test.ts`: dropped the
`"refreshes the WebGL texture atlas after animated SGR output"` and
`"checks ArrayBuffer and Blob terminal output for SGR atlas refresh"`
pins (they locked in the removed workaround). Kept the WebglAddon
construction / `onContextLoss` / try-catch-fallback pins, the
no-string-coercion binary-output pin, and the focus/blur/host-resume
pins. Added two new tests: one keeping `clearTextureAtlas` /
`refreshTerminalRows` wired as event-driven helpers, and a negative pin
asserting the source no longer contains `maybeRefreshWebglAtlas` or
`bytesContainSgrSequence` so the per-chunk clear cannot silently return.

Change #3 (theme-change `clearTextureAtlas` insurance) was NOT applied:
deferred to manual verification per the plan, since xterm's native
`onChangeColors` should cover it. No fallback (the debounced clear) was
needed at the automated stage.

### Automated gate results

- `web/`: `npx vitest run` 1479 passed / 11 skipped; `npx svelte-check`
  0 errors / 0 warnings; `npm run build` OK (pre-existing chunk-size
  warnings only).
- root: `cargo clippy --all-targets -- -D warnings` clean (exit 0);
  `cargo build -p chan` OK with the freshly re-embedded bundle.
- `cargo fmt --check`: one diff, in `desktop/src-tauri/src/main.rs:251`.
  This is @@Desktect's uncommitted chan-desktop work, present in the
  shared working tree; it is NOT part of this deliverable and was left
  untouched. Flagged to @@Alex. Full `cargo test` was deferred to
  @@Alex's integration pass because the tree carries @@Desktect's
  uncommitted Rust changes; this change adds no Rust logic.

### Cross-pane status

Not yet verified at runtime. The cross-pane case (and the resize, theme,
flip, and monitor-move cases) is @@Alex's final manual verification, run
in two side-by-side terminal panes per the Verification section. If
cross-pane glyph substitution reproduces, apply the debounced-clear
fallback described under "Risk and fallback" rather than reinstating the
per-frame clear.

### Commit

Committed with the two web files as a single deliverable, scoped to this
agent's files only (path-scoped `git add`), leaving @@Desktect's
uncommitted desktop changes in the shared tree untouched.
