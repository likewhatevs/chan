# task-Chan-Lead-4 — COMPLETE: SVG embed "image not found" (task-Lead-Chan-7)

From: @@Chan. To: @@Lead. Fix: b7d2b205, one file.

## Verdict on step 1: PRE-EXISTING at v0.31.1, not a round regression

Every image-path file changed since baseline 37a0f001 is comment-only
(files.rs, static_assets.rs, blocks.ts, image_drag_indicator.ts diffs
verified line-by-line; widgets/image.ts, extensions/image.ts,
image_drop.ts untouched). No rebuild of the old tag needed — the
diff inspection is conclusive.

## Root cause (one network observation, as you predicted)

GET /api/files/logo.svg returned 200 + content-type APPLICATION/JSON
(311 bytes — the editor's FileResponse envelope), while dot.png
returned raw image/png. read_file_sync delegated raw-vs-text to the
content-aware editable gate (read_text_with_stat): SVG is valid
UTF-8 XML, PASSES the sniff, ships as editor JSON; <img> rejects it;
the widget's onerror renders the pink box. Binary formats fail the
sniff into the raw branch — that's the png/svg asymmetry. Your three
recon layers were indeed all correct; the bug sat in the branch
ORDER between them.

## Fix

read_file_sync classifies the path FIRST: FileClass::Image | Pdf →
raw bytes + content_type_for (svg → image/svg+xml). That matches
FileClass::Image's own documented contract ("read-only via read /
write_bytes") and the route's existing doc comment. Deliberately at
the ROUTE layer: the chan-workspace gate (and therefore MCP
read_file, which agents may use to read .svg sources as text) is
unchanged. Pdf included for explicitness — behavior identical (pdf
is binary; it already failed the sniff into raw).

## Verification

- New unit test: svg with XML text content → ReadFileResult::Binary
  (sits beside the existing binary + odd-suffix-text pins). The
  fragment note is in the test doc: `#w=250` never reaches the
  server — the widget already strips it from the fetch URL
  client-side (observed: the png fetch URL carried no fragment
  BEFORE the fix; the error string showed the fragment because the
  placeholder echoes the raw SRC, not the fetch URL).
- Chrome smoke (throwaway --standalone workspace, torn down +
  unregistered): fresh binary rebuilt + provenance-checked
  (content-type flipped to image/svg+xml on the same URL), then the
  editor rendered ./logo.svg#w=250 at 100x100 with zero
  .cm-md-image-broken boxes; dot.png control intact. Reproduced in
  plain Chrome pre-fix, so it was never WKWebView-specific.
- Gates: cargo fmt clean; RUSTFLAGS=-D warnings clippy -p
  chan-server --all-targets clean; cargo test -p chan-server 419
  passed (was 418; +1 the new pin).

Ready for the round-close gate re-run. @@Alex may want to re-drag
his sdme-logo.svg on a rebuilt desktop binary as the final word.
