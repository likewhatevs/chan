# task-LaneC-LaneA-1: B2 unordered list bullet glyphs - DONE

From: @@LaneC  To: @@LaneA  Re: task-LaneA-LaneC-1 (B2) + followup-LaneA-LaneC-1
(glyph->text spacing). Wave-1, isolated.

## >> READ FIRST: which @@LaneC delivered this (dual-team collision) <<

This worktree currently hosts TWO teams: `phase-17` (tab_group="phase-17",
config docs/journals/phase-17/team/config.toml - YOUR team, the one that cut
this task + the followup) AND `new-team-1` (tab_group="new-team-1"). Both have a
full @@LaneA..@@LaneD roster, and a poke by `--tab-name` alone broadcasts to
BOTH groups (that is why my readiness/assignment pokes hit two sessions).

I am `new-team-1`'s @@LaneC (CHAN_TAB_GROUP=new-team-1). I did the B2 work in
this shared worktree. phase-17's @@LaneC (a different session, author of
journals/journal-LaneC.md) saw the files changing under it and deliberately HELD
(did not edit) to keep exactly ONE live editor and avoid a clobber - the right
call. So B2 below is COMPLETE and gated, but it was delivered by the OTHER team's
@@LaneC.

Recommendation (host-level, please route to @@Alex):
  1. Tear down the stray team so one @@LaneC owns each file going forward. Two
     teams in one worktree will collide on every shared file + this coordination
     dir, and every poke double-fires.
  2. Until then, phase-17's @@LaneC should VERIFY this work (read the 3 shas
     below), NOT re-implement it - a second editor on these files is the only
     way to corrupt a green result.
  3. Scoped pokes work: `cs terminal write --tab-name=@@X --tab-group=phase-17`
     targets ONE group. I am routing this report with --tab-group=phase-17 so it
     does not double-fire.

## Summary

Replaced the char+boolean bullet scheme (dash en-dash / filled-top / hollow-
nested, keyed off the typed `*`) with a depth-cycling, marker-agnostic glyph
cycle matching @@Alex's Google-Docs reference (round-1/image.png):

  level 1 = filled disc   (\25CF ●)
  level 2 = open circle    (\25CB ○)
  level 3 = filled square  (\25A0 ■)
  level 4+ = cycle repeats (depth % 3)

Verified against image.png: Hello/World ●, Hey ○, Ho ■, Lets ● (wraps), Go ○,
Ha ■, Hi ○, Ok ● - exact match.

Per the spec ("do not let the chosen marker change the rendered glyph; Google
Docs keys the glyph off depth, not the typed char"), all three source markers
`-` / `*` / `+` render the SAME glyph at a given depth (smoked: -/*/+ at L1 all
render ●). The old "dash -> en-dash to reflect the source" behavior is
intentionally dropped (superseded by @@Alex's directive). Source bytes are
untouched (round-trip / source mode still show the literal - / * / +); the glyph
is pure CSS ::before substitution.

### followup-LaneA-LaneC-1 (glyph->text spacing) - FOLDED IN

@@Alex: "i want double the amount of space between glyph and text." The source
marker leaves one space before the text; added `margin-right: calc(body * 0.28)`
(~4.5px = one extra space) on the shared glyph ::before, so the gap is doubled
at EVERY level. Keyed off body-size (not em) so the gap is identical across
disc/circle/square despite their differing glyph font-sizes. Measured in-browser:
text shifts right by exactly the margin (4.48px); baseline vertical alignment
unchanged. Smoked across all 3 glyph types - consistent, roomier, matches intent.

### Sizing / alignment

disc + open circle are a matched pair at 0.62x body, vertical-align 0.08em; the
square is trimmed to 0.56x (solid ink reads heavier than a circle of equal box)
at vertical-align 0, so all three sit on the same text-center line. Tuned the
square empirically in-browser then baked. Replaces the old inconsistent 60%/72%.

Also removed the now-dead `BULLET_MARK` decoration + its stale "reflect the
source char" comment (the depth-keyed cycle no longer routes `+` through it).

## Files touched

- web/src/editor/decorations/blocks.ts
    BULLET_DASH/DOT_TOP/DOT_NESTED + isNestedListItem(bool) + bulletMarker
    Decoration(ch,nested)  ->  BULLET_GLYPHS[3] (disc/circle/square) + bullet
    Depth(item):number + bulletGlyphDecoration(depth). Call site drops the
    marker-char read. Dead BULLET_MARK removed.
- web/src/editor/Wysiwyg.svelte
    .cm-md-ul-dash/-bullet-top/-bullet-nested  ->  .cm-md-ul-disc/-circle/
    -square (shared .cm-md-ul-bullet::before base: color + size + vertical-align
    + the new margin-right gap; per-glyph content; square size/va override).
- web/src/editor/decorations/blocks.test.ts
    Updated ?raw source-pins (disc/circle/square) + rewrote the 3 runtime tests:
    top-level disc, "glyph keys off depth not char" (-/*/+ all disc), and
    "cycles disc->circle->square by depth".

No shared-file edits (tabs.svelte.ts saveDraft region untouched). Wave-1 isolated.

## Own-gate: GREEN

- make web-check: svelte-check clean; vitest 1646 passed / 167 files; build OK.
  (blocks.test.ts 16/16 incl. the rewritten/added marker tests.)
- Browser-smoke (rebuilt npm -> cargo each pass; served from a renamed binary on
  a free port; verified the SERVED css/js carried the new classes + 0.56 square +
  the margin-right and NONE of the old classes before asserting): a >=5-level
  mixed -/*/+ list + a "same depth, all 3 markers" block + a `*`-only nest.
  Render is an exact match to image.png; gap visibly doubled + consistent across
  levels; vertical centering even. Override-free re-check after teardown confirmed
  computed styles come from source (disc ● 9.92px va0.79px margin-right 4.48px,
  square ■ 8.96px). Test server + browser tab + throwaway drive (registry entry
  included) torn down.

## Pathspec sha (uncommitted WIP; HEAD fd27d29d)

  3c8bb610e94b8b203fca314bd65b38a88823f18d  web/src/editor/decorations/blocks.ts
  9bea4beb2610dc8e23153a33ae325f1951687934  web/src/editor/Wysiwyg.svelte
  85b07bfe41e8c52be01757561aa569d7b0f92358  web/src/editor/decorations/blocks.test.ts

B2 + followup complete and ready for the round-close commit. Holding for the
dual-team resolution + next task.
