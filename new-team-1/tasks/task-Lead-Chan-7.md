# task-Lead-Chan-7 — SVG editor-embed renders "image not found" (Alex smoke finding)

From: @@Lead. To: @@Chan. Found by @@Alex during the file-drop hand
smoke (which PASSED — this is adjacent, not the drop arc). Round
close holds for this fix.

## Repro (Alex, desktop, throwaway workspace /private/tmp/notes)

Drag an SVG from Finder onto the editor: the file copies into the
drafts folder correctly, the markdown embed is written
(`./sdme-logo.svg#w=250` style), but the widget renders the pink
"image not found: ./sdme-logo.svg#w=250" box. The same flow with a
PNG works end-to-end.

## My recon (the obvious layers are all correct — bug is deeper)

- chan-workspace fs_ops.rs:345 classifies `svg` → FileClass::Image
  (test-pinned at :1674).
- web image widget allowlist: extensions/image.ts:12 IMAGE_EXTS
  includes "svg".
- chan-server content_type_for (static_assets.rs:174): svg →
  image/svg+xml — and SVG is the one format browsers never
  content-sniff in <img>, so anything else in that header fails to
  render. Verify which response header the <img> fetch ACTUALLY gets.
- files.rs:526+ serves Image-class reads as raw bytes; the pink box
  is the widget's broken-image placeholder (image.ts:334 area) — so
  either the fetch 404s/errors or <img> rejects what it gets.

## How to attack

1. FIRST determine pre-existing vs round-regression: your web scrub
   (51664864) touched the image machinery files (comments-only by
   intent). Check the same embed against the v0.31.1 binary (chan
   upgrade --version 0.31.1 on a scratch copy, or git checkout
   v0.31.1 build). This decides urgency framing and where the fix
   lands in the story.
2. Reproduce in Chrome (plain `chan serve --standalone` on a
   throwaway, drop or hand-write an svg embed) and watch the NETWORK
   panel for the <img> fetch: status + Content-Type + response body.
   That pins the failing layer in one observation. (Per the round
   discipline: scoped teardown after.)
3. Fix where the truth is; vitest-pin the svg case alongside the
   png case in whatever layer broke; if the fix is server-side,
   `-p chan-server` gates with RUSTFLAGS=-D warnings.

@@Alex's repro had `#w=250` in the src — include a fragment-bearing
svg in the pinned cases either way (the error string shows the
fragment surviving into the failure path).

Completion: task file + poke. The round-close gate re-runs after
this lands.
