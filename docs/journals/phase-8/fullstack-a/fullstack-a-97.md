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
