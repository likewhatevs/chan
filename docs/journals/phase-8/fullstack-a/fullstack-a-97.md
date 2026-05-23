# fullstack-a-97 — Terminal glyph rendering bug (RELEASE BLOCKER for v0.13.0)

Owner: @@FullStackA
Phase: 8, Round 3
Date cut: 2026-05-23
Priority: **P0 — release blocker for v0.13.0**

## Goal

Fix the terminal-renderer glyph-substitution bug that surfaced post-v0.12.0. Multiple terminal panes corrupt characters when rendering ANSI-styled text under animation (Claude Code task spinners, rolling color cycles, italic / bold runs).

## Background

Reported 2026-05-23 by @@Alex with three side-by-side screenshots: one terminal pane rendered correctly, two adjacent panes corrupted with per-character substitutions ("background terminal" → "backgrouLd teamSnal"; "@@Alex manual: ..." → "aw.cfhi(ec(I- kw)..."). @@Alex framing:

> "when the agents are switching the fonts to apply those rolling effects of colour and why not we get these messed up characters. this is a new bug. It was not happening in the previous release."

Audit-trail entry: `docs/journals/phase-8/phase-8-bugs.md` "Terminal glyph rendering corrupted ..." (first reported 2026-05-23 as P3 / single instance; promoted same day to RELEASE BLOCKER after recurrence + multi-pane confirmation).

## Suspect list (tight; bisect target)

Between `chan-v0.11.2` (clean) and `chan-v0.12.0` (broken):

1. **`-b-29` WebGL renderer** (primary suspect). Glyph-atlas / texture-coord bugs typically manifest as per-character substitution under high re-render frequency. Claude Code's task spinners + rolling color cycles + italic / bold runs hit the renderer's draw call path repeatedly per second.
2. **`-b-30` Source Code Pro spawn-time `fontFamily` reorder + on-demand download** (secondary). Partial font-face load could cause atlas-key misalignment when the renderer re-keys glyphs mid-stream.

Either path lives in `web/src/components/TerminalTab.svelte` and the WebGL renderer pipeline.

## Acceptance criteria

1. **Reproduction documented**: clear repro shape (chan-desktop dev build or browser repro against a `chan serve` throwaway drive; pipe ANSI-styled animated text to a terminal pane; observe corruption). Cite affected glyph patterns + timing.
2. **Root cause identified**: bisect against `-b-29` and `-b-30` shipped commits. State which path is responsible. If both contribute, name both.
3. **Fix landed**: glyph substitution does NOT occur on animated ANSI text in any terminal pane. Multiple parallel terminals (≥3, mix of active + idle) confirm clean rendering during animation cycles.
4. **Test pin**: a vitest or browser smoke that exercises animated ANSI text rendering. Idea: feed a known ANSI-styled animated stream + assert the rendered glyph output matches expected.
5. **Gate**: `npm run check` + `npm test -- --run` + `npm run build` green.

## How to start

1. **Confirm scope**: `git log --oneline chan-v0.11.2..chan-v0.12.0 -- web/src/components/TerminalTab.svelte web/src/components/TerminalRenderer*.{ts,svelte}` (or wherever the WebGL renderer lives). The `-b-29` and `-b-30` commits should surface in that diff.
2. **Reproduce locally**:
   - `cargo build -p chan` + `./target/debug/chan serve /tmp/chan-test-a-97/` against a fresh throwaway drive.
   - Open chan.app or a browser pointed at the served URL.
   - Spawn multiple terminal panes.
   - Run a tool that produces animated ANSI output (Claude Code session, `cargo build` with progress bar, `npm install`, etc.).
   - Observe glyph substitution on a subset of panes.
3. **Bisect**: if `git bisect` between `chan-v0.11.2` and `chan-v0.12.0` is tractable for the affected file paths, that's the fastest root-cause path. Otherwise, hypothesis-test the two suspects directly.
4. **Fix**: depends on root cause. Likely shapes:
   - WebGL: glyph-atlas key invalidation on style-attribute change; texture-coord rebuild on font swap; etc.
   - Font load: defer first-render until `document.fonts.ready` resolves; or invalidate the atlas when the FontFace loads.
5. **Verify**: same reproduction shape, post-fix, shows clean rendering across multiple panes during animation.

## Coordination

* **Priority**: release blocker for v0.13.0. This sits ABOVE `fullstack-a-96`'s sub-pass 4 in your queue. If you're mid-flight on `-96`, finish the current sub-pass + then pick `-97`. If you haven't started, pick `-97` first.
* **Safety guardrail (still in effect)**: do NOT touch @@Alex's running chan.app session. Use throwaway drives + dev builds per the standard test-server-workflow.
* **Don't reach into `desktop/`**: the bug repros in a browser against `chan serve`; no chan-desktop-side change should be needed unless the bisect surprises us. If it does, poke me; I bridge with @@Desktect via @@Alex.

## Authorization

Yes for SPA-side edits (`web/src/`) + WebGL renderer paths + Rust-side terminal tests + new throwaway drives + dev `cargo build` for repro. Time-boxed expectation: 1-2 sessions for repro + bisect + fix; if the bisect surprises (e.g., bug isn't from `-b-29` or `-b-30`), scope-poke back to me.

## Out of scope

* Performance refactor of the WebGL renderer beyond what's needed to fix this bug. The renderer landed in `-b-29`; it's part of the v0.12.0 surface. Wholesale rework is post-v0.13.0 territory.
* Font choice changes (revert SCP, etc.). Fix the renderer, don't paper over with a font swap.
* Chan-desktop bundling / signing — chan-desktop team's lane.

## 2026-05-23 — candidate fix ready: WebGL atlas refresh on animated SGR

Root-cause path narrowed by tag-range audit:

* `9029924` (`fullstack-b-29`) introduced
  `@xterm/addon-webgl` in `TerminalTab.svelte`.
* `613fc0c` / `e363ee6` (`fullstack-b-30`) changed
  the terminal font chain, but did not explain
  per-character substitution under repeated ANSI color /
  style churn.
* xterm.js exposes `Terminal.clearTextureAtlas()` with
  docs naming corrupted WebGL texture atlases as the
  intended workaround class. That matches @@Alex's
  observed symptom: correct terminal buffer text but
  wrong glyphs drawn in some WebGL-rendered panes during
  animated ANSI style updates.

### Fix

`web/src/components/TerminalTab.svelte` now:

* Tracks whether the WebGL renderer is active.
* Detects CSI SGR sequences (`ESC [ ... m`) in binary
  websocket output, including sequences split across
  chunks.
* Coalesces a `term.clearTextureAtlas()` + full-row
  `term.refresh(...)` onto the next animation frame
  whenever styled animated output hits the WebGL path.
* Clears the active/queued state on WebGL context loss
  and terminal teardown.

This keeps WebGL enabled for the box/block glyph fix from
`-b-29` and targets the renderer's atlas corruption path
instead of changing fonts or disabling the renderer.

### Test pin

`TerminalTab.renderer.test.ts` now pins:

* WebGL context-loss state reset.
* SGR detection helper.
* `maybeRefreshWebglAtlas(...)`.
* `clearTextureAtlas()` + full refresh.
* Both `ArrayBuffer` and `Blob` websocket byte paths
  calling the atlas refresh check after `term.write(...)`.

### Verification

* `npm test -- --run src/components/TerminalTab.renderer.test.ts`
  — 6/6 pass.
* `npm run check` — 0 errors / 0 warnings.
* First `npm test -- --run` hit three 15s UI-test
  timeouts under parallel load; reran the three affected
  files directly (`TerminalTab.test.ts`, `Pane.test.ts`,
  `EmptyPaneCarousel.test.ts`) — 26/26 pass.
* Second `npm test -- --run` — 1335/1335 pass, 11
  skipped.
* `npm run build` — clean; existing chunk-size /
  ineffective-dynamic-import warnings only.

### Verification gap

I did not visually reproduce / recheck the WebGL glyph
corruption from this shell. Empirical follow-up needed:
browser or chan-desktop dev build, three terminal panes,
animated SGR output, verify no glyph substitutions during
color/style churn.

Commit readiness:
`web: terminal: refresh WebGL atlas on styled output (fullstack-a-97)`.

## 2026-05-23 — @@Architect: approved + commit clearance

Fix-shape review:

* Root-cause path correct: xterm.js docs explicitly name corrupted WebGL texture atlases as the target failure mode for `clearTextureAtlas()`. Symptom @@Alex reported (correct buffer text, wrong glyphs drawn) is exactly this class.
* The SGR-detection + clearTextureAtlas-coalesced-to-rAF shape is the right fix: keeps WebGL enabled (preserves `-b-29`'s box/block-glyph win), targets the corruption path, doesn't paper over with a font swap.
* Sequences split across websocket chunks are handled — important detail for the streaming animated-output case.
* Test pin is structural (pins the wiring + the detection helper + the rAF coalescing path). Appropriate for the SPA test layer; the actual visual fix needs an empirical walk.

### Verification-gap routing

The verification gap @@FullStackA flagged is real but expected — this is a visual rendering bug; vitest can't fully smoke it. Routing the empirical walk to @@WebtestA as a follow-up (heads-up in their channel; not blocking your commit). v0.13.0 cut waits on @@WebtestA's HOLD verdict.

### Suggested commit subject

```
web: terminal: refresh WebGL atlas on styled animated output (fullstack-a-97 — P0 v0.13.0 release blocker fix)
```

### Commit instructions

Per the standing pre-authorization for your lane:

* Per-path `git add` only.
* Pre-commit `git diff --staged --stat` + post-commit `git show --stat HEAD` per the atomic-audit pattern.
* The `crates/chan-server/src/routes/files.rs` `spawn_blocking` work for `-96` sub-pass 4 already shipped (`793a28a`); keep that scope separate.

### Lane state post-`-97` commit

* `-96` sub-pass 4 ✓ (shipped `793a28a`).
* `-97` ready to ship.
* `-96` sub-passes 1/2/3 (dead-code / a11y / perf) still queued for your lane — pick up post-`-97` if session bandwidth, otherwise defer to next session.

Thank you for the tight diagnostic + the SGR-split-across-chunks edge handling.

## 2026-05-23 - teardown-complete

No FullStackA-owned server, build, dev-server, or throwaway drive
state remains for this task. Phase-8 stand-down acknowledged.
