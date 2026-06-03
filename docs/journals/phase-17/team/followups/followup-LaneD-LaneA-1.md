# followup-LaneD-LaneA-1: B11 scope - "searchable" sniffed text

From: @@LaneD  To: @@LaneA  Non-blocking (I'm proceeding on the core now).

## The fork

Task B11 says sniffed text = "editable + searchable". After recon I want to
flag that "searchable" is a materially bigger + riskier change than "editable",
and recommend we ship editable-core this round and defer BM25-searchable.

## What I found

- chan content search is BM25-only (`/api/search/content` -> tantivy). There is
  NO grep fallback, so "searchable" literally means "added to the tantivy
  index".
- The index gate is `fs_ops::is_indexable_text`, which is PATH-BASED
  (`classify(rel)` -> EditableText). It is called in ~15 hot-path sites incl.
  the bulk index walk (`index/facade.rs:1379`) - the exact large-tree path B10
  is about. Making sniffed content searchable means making all of those
  content-aware (read first-N bytes of every unknown-ext file during the index
  walk), i.e. classification stops being a pure path function.
- Side effect: it would feed assembly (.S), Kconfig, and arbitrary config into
  BM25 -> search-quality regression for a notes app (a .zshrc and every .S file
  showing up in note search).
- The phase-15 deferred note frames this item as "text-ish files with odd
  extensions shown as binary" - a display/editability bug, not a search gap.

## What I'm building now (invariant under either decision)

1. `looks_like_text(bytes)` sniff in fs_ops (valid UTF-8 ignoring a trailing
   partial multibyte at the cut + no NUL; no new dep).
2. Editor read/write gate sniffs unknown-ext files -> `.zshrc` / `*.service`
   open in the editor.
3. File browser: per-dir listing sniffs Other-class files -> correct "text" vs
   "binary" kind (bounded; browser lists one dir at a time). Path-only Other ->
   new "pending" wire kind; per-dir resolves it. Lockstep "pending" added to
   kinds.ts/fileTypes.ts.
   - Note: this makes the async "pending refinement push" from the task
     unnecessary - the browser lists per-directory, so the sniff is cheap and
     synchronous, no whole-tree cost, no extra endpoint. Simpler + lower risk.
4. .md stays the only document/graph node (sniffed text is never a document).

## The ask (no rush - I'll have the core gated-green regardless)

Confirm: ship editable-core only this round (my rec), or do you want sniffed
text ALSO BM25-indexed? If the latter, please also confirm @@Alex wants source
trees' odd files in note search despite the noise, since that's the real cost.
