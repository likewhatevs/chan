# task Lead -> LaneA (6): BUG - click at EOL of a NESTED bullet lands cursor at line START

@@Alex found this clicking around the Wave-2 smoke server (your editor lane).
Real, reproducible, editor-domain. We do NOT ship it. Fix + re-smoke.

## Repro (from @@Alex, with screenshots)
In a bullet (unordered) list, click in the EMPTY SPACE at the END of a row:
- FIRST-level item (e.g. "two"): cursor lands at END of line. CORRECT.
- SECOND-level / nested item (e.g. "nested two-a"): cursor lands at the
  BEGINNING of the line (before the first text char). WRONG.
First indent level works; second (and presumably deeper) does not. He clicked
the END of the row in both cases; only the nested one mis-lands.

## My hypothesis (verify, don't trust)
This looks like your item-1 caret-snap over-firing on CLICKS, not just arrows.
Your fix snaps a caret that lands in the marker PREFIX forward to the first
TEXT column (clampListCaretPosition via listAwareArrowDown/Up). On a NESTED
item the larger left-indent likely makes a click-past-text resolve (posAtCoords)
into the prefix region; the snap then moves it to text-START - which is exactly
the beginning-of-line cursor @@Alex sees. A first-level click lands at true EOL
(not in the prefix), so no snap fires. So either:
  (a) the snap is being applied to pointer/click selections (it should only
      correct VERTICAL arrow motion), or
  (b) a click-at-EOL on a nested line genuinely lands in the prefix and your
      snap sends it to text-START when an EOL click should clamp to text-END.
Fix accordingly (e.g. gate the snap to arrow-key transactions only; or for a
click resolving into the prefix, clamp to the nearest text edge by click-X, so
an end-of-row click goes to text-END not text-START). Check ordered + hyphen
lists at depth >= 2 too (regression-test the click path).

## Scope / gate
Editor lane, your owned files (blocks.ts / list.ts / Wysiwyg.svelte etc.). Add a
test for the click-at-EOL-of-nested-item case. Gate: make web-check +
svelte-check + build, and BROWSER-SMOKE the click at depth 1 AND 2 (bullet,
hyphen, ordered) - this is a runtime pointer-mapping bug, static gates miss it.

## Note
Your editor work is already committed (c9ea3c56); this fix APPENDS as its own
commit - I'll commit it when you report green. On completion cut
task-LaneA-Lead-4.md (root cause + fingerprint + the depth-1/2 smoke result),
poke me. @@Alex may find more while poking; I'll queue them as they come.
