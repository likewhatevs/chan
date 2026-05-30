# @@LaneC journal - Phase 14

Docs lane. C2 = the docs/journals second-brain reorg (committed to main as
`docs(journals): per-phase second-brain reports + raw archive`,
`741aa787`). C1 = the round-2 `/architect` pass over the frontend code
comments, documentation, and user-facing copy. Worktree
`../chan-p14-lane-c`, branch `phase-14-lane-c` (off the merged A+B base
`c37a11e2`). This is the C1 narrative.

## C1: what was reviewed and changed (2026-05-30)

Scope: the four frontend trees (`web/`, `gateway/crates/identity/web`,
`gateway/web-common`, `web-marketing/`). A read-only survey (four
subagents) found `web-common` and `web-marketing` essentially clean, so
the work concentrated in `web/` and the identity SPA.

What changed (all comments / docs / copy; no behavior change):

- ASCII typography sweep across all four trees: em dashes, en dashes, the
  typographic ellipsis, the middle dot, and the `&middot;`/`&mdash;`
  entities were normalized to `-` and `...` in both comments and
  user-facing strings (per @@Alex's "ASCII-only in UI" call). About 100
  files in `web/`, a couple in the identity SPA. The intentional product
  glyphs (the list-marker circles, screensaver characters) were left
  untouched; only the four target characters were swept.
- Factual corrections, each grounded against the current crate layout:
  - `web/src/design.md`: a botched rename codemod had turned the verb
    "Drives/Controls" into "Workspaces" ("Workspaces the entire CSS
    palette"); restored to "Controls".
  - Stale `chan-core` references (the pre-Phase-5 crate name) in `web/`
    corrected to the real post-split crates: graph types to
    `chan-workspace::graph`, `IndexStatus` to `chan-server::indexer`,
    the server to `chan-server`, the report summary to `chan-report`, the
    slug/blocks/progress to `chan-workspace`.
  - `EmptyPaneWelcome.svelte`: a comment listed the welcome tiles as
    "... / RP / Graph"; RP (the old Rich Prompt) is now "Team Work".
  - identity `Tokens.svelte`: empty-cell placeholders tightened to `-`.
  - identity `api.ts`: two roadmap-narration comments ("until X lands",
    "future health states land here") reframed to present-state.
  - `web-marketing` `preserve-release-metadata.mjs`: the comment narrating
    the old buggy `/dl` guard was removed (the present-state rule and the
    rationale stay; the history lives in `docs/journals` addendum-1 #1).
- `web/src/editor/design.md` was rewritten as a present-state snapshot
  (see the next section).

Voice note: `///` doc comments are an established house convention across
the editor SPA, so the identity SPA's `///` were kept (consistent), not
"normalized" to `//`.

## Editor history preserved (tiptap -> CM6)

`web/src/editor/design.md` was a migration-era planning doc: it described
the CodeMirror 6 editor as a rewrite of a "previous tiptap/ProseMirror
editor," with a future-tense "cutover (step 11)" and an `editor-cm6/`
layout that no longer exists (the files live under `editor/`). Per the
pristine-code mandate I rewrote it to describe the current editor without
that history. Per @@Alex, that history is useful for the story of chan's
making, so it is preserved here rather than lost:

The chan editor was rewritten from a tiptap/ProseMirror foundation to
CodeMirror 6 with a Live-Preview model (the same shape as Obsidian's). The
old model made the document the rendered tree and reconstructed the
markdown source by serialization, faking source in and out around the
caret via expand/collapse passes plus per-pattern editing flags. Every new
inline pattern (bold, italic, strike, code, link, wikilink, image, naked
URL) added another collapse/render race and another set of edge cases.

The recurring bug class that drove the rewrite: you could not edit `*a*`
(1-char italic) although `*aa*` worked, because a "caret strictly inside
the mark" check has no integer position satisfying `from < caret < to`
when the marked range is one character; pending-mark heuristics flickered;
markdown round-trip needed escape gymnastics (NBSP for blank paragraphs,
`\#` for heading prefixes, defensive image serializers); and the autosave
gate had to enumerate every active expansion flag. The class kept biting
because it was structural.

The CM6 rewrite flipped the model so the document text IS the markdown
source (`view.state.doc.toString()` is the file on disk, no transform
layer), decorating the source in place. `*a*` is then three real
characters; the markers reveal when the selection intersects them, and
round-trip is the identity function. The migration reused a set of
framework-agnostic helpers (the popover shell, the viewport positioner,
the date catalog, `scanMatches`, the link helpers, the block-anchor
parser) and was planned as an 11-step cutover. This evolution is a
candidate to weave into the specific phase report where the rewrite landed
once that phase is confirmed.

## Flags for @@Alex (raised on the bus, not silently changed)

- Identity charset mismatch: `Workspaces.svelte` tells users a workspace
  name allows `._-`, but the CLI flag `--tunnel-workspace-name` accepts
  only `[a-z0-9-]` (no dot or underscore). A UI-valid name like
  `my_workspace.v2` is then pasted into a command the CLI rejects. This is
  a product/validation inconsistency, not just copy; left for a decision
  (tighten the SPA validation, or adjust the example) rather than rewritten
  blindly.
- Tunnel domain drift: the marketing copy and the chan client code both
  say `drive.chan.app` (verified at `crates/chan/src/main.rs:257`), while
  the repo `CLAUDE.md` and gateway docs say `workspace.chan.app`. The code
  is the source of truth, so the copy is correct and the docs are stale.
  Out of C1's frontend scope to change; needs a canonical-domain call.

## Recording (per @@Alex)

The history of this change lives in the CHANGELOG (a phase-14 `[Unreleased]`
entry, with the stale v0.18.0 section promoted to its own header) and in
this journal, not in code comments. The code reads as a present-state
snapshot.

## Gate / status

The edits are comments, docs, and copy strings only (no logic). The
affected frontend gates are green on the worktree branch: `web/`
svelte-check 0/0, `web/` vitest 1563 tests pass, `web/` build clean;
identity SPA svelte-check 0/0 and build clean. Committed as `6e2d059c`
on `phase-14-lane-c` (not pushed).

## Resolutions from @@Alex (2026-05-30)

- Tunnel domain drift: RESOLVED. @@Alex chose `workspace.chan.app` as the
  canonical hostname (the online service is already deployed and tested
  on it; he is the sole user, so no back-compat). Renamed
  `drive.chan.app` -> `workspace.chan.app` across the code, the chan
  client tunnel default, the chan-tunnel-* crates (defaults, host-allow
  suffix-match, and tests), chan-server, desktop, `docs/manual`, and the
  marketing copy. The 75 `drive.chan.app` mentions under `docs/journals`
  are history and were left as written.
- Identity charset mismatch: OPEN CARRYOVER. @@Alex will sort it out as a
  follow-up after the phase closes. No code change made; the SPA still
  advertises `._-` while `--tunnel-workspace-name` accepts `[a-z0-9-]`.
