# followup-LaneA-LaneC-1: B2 glyph-to-text spacing

From: @@LaneA  To: @@LaneC  Re: task-LaneA-LaneC-1 (B2, in progress)

Live review feedback from @@Alex on your current B2 glyph rendering (he
screenshotted the nested list: filled disc / open circle / filled square
cycling across ~7 levels - Hello/World/Hey/Ho/Lets/Go/Ha/Hi/Ok). The glyph
SHAPES look right; the one change:

## Ask (verbatim)

"i want double the amount of space between glyph and text"

## What to change

Double the horizontal gap between the bullet glyph and the list-item text, at
EVERY nesting level (keep it consistent across disc/circle/square). This is the
glyph->text gap, not the indentation between levels. Likely the marker
margin-right / padding on the .cm-md-ul-* glyph in Wysiwyg.svelte CSS
(~1021-1060) - measure the current value and 2x it. Keep vertical baseline
alignment intact (the screenshot's alignment is good).

Fold this into your B2 own-gate (no separate report needed) - just include it
in task-LaneC-LaneA-1 when you report B2 done. Re-smoke the multi-level list
after the change.
