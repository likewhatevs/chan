# task Lead -> LaneA (7): CLEANUP bullet lists - stop scaffolding, match hyphen/ordered

SUPERSEDES the snap-patch approach of task-6. @@Alex's direct call after testing.
This is an APPROACH change, not another patch. Read it before sinking more into
the task-6 caret-snap.

## @@Alex (verbatim intent)
"the bullet list now allows me to click the empty space to land at EOL ...
however, if I click anywhere IN THE TEXT of a NESTED bullet, the cursor lands at
the BEGINNING of the line before the bullet, not where I clicked ... it almost
feels like the default list works as expected (hyphenated, enumerated) but for
bullet we've added so much around it that it's broken ... feels like it needs
CLEANUP not more scaffolding, especially because these bugs are all very
SPECIFIC TO BULLET lists, which feels wrong."

## The diagnosis (yours to confirm)
Hyphen + ordered lists work because their markers are REAL POSITIONED TEXT (you
said it yourself: hyphen markers are real text, so they "get ordered-list cursor
behavior for free"). Bullet (star/plus) got the Google-Docs depth glyphs via a
zero-width source char + CSS ::before glyph, which DECOUPLES the visual glyph
from the source position -> click/cursor coordinates map into the prefix -> you
added clampListCaretPosition / listAwareArrowDown-Up snap to compensate -> each
new case (arrow, EOL-click, in-text-click) needs another band-aid. The
scaffolding IS the bug surface.

## Directive: CLEANUP, not more snap
Re-approach bullet lists so they get DEFAULT CodeMirror cursor/click behavior
the same way hyphen/ordered do - i.e. the marker is a REAL positioned character,
not a zero-width char + ::before glyph. Then click-where-you-click and arrow
motion work with NO snap logic. Concretely:
- REMOVE the bullet-specific caret-snap scaffolding (clampListCaretPosition /
  listAwareArrowDown/Up and friends) if it becomes unnecessary once markers are
  real - prefer deleting code over keeping it.
- Make the star/plus marker a real positioned glyph (e.g. a replace-decoration
  swapping `*`/`+` for the actual disc/circle/square CHARACTER, kept as a real
  width-bearing position) rather than zero-width-source + CSS ::before.
- Unify the three list types on ONE cursor/click path. The goal is LESS
  bullet-specific code, not more.

## Tension to FLAG (do not silently trade off)
If keeping the exact Google-Docs disc/circle/square depth visual proves
incompatible with correct positioning (i.e. the only way to show a different
glyph is the zero-width + ::before trick that breaks clicks), STOP and flag it to
me - that is a @@Alex call (correct cursor behavior vs the fancy glyph), and I
will survey him. Try for both first (a real-character glyph should give both).

## Regression bar (this is the trap)
After the cleanup, ALL of these must work at depth 1 AND depth >= 2, for BULLET,
HYPHEN, and ORDERED:
- click in the MIDDLE of the text -> cursor lands where clicked
- click empty space at EOL -> cursor at end of line
- arrow-down/up between items -> cursor tracks goal column (the ORIGINAL bug)
Add tests for the click-in-text-of-nested case. Browser-smoke all of it (runtime
pointer mapping; static gates miss it).

## Process
- Your editor work is committed at c9ea3c56; this cleanup reworks the bullet
  decoration and likely REMOVES code. It lands as its own commit (or I rework
  c9ea3c56 - your call, tell me). Back out the task-6 snap if you started it.
- Cut task-LaneA-Lead-4.md (what you removed + the unified approach + the
  depth-1/2 smoke matrix for all 3 list types), poke me.
