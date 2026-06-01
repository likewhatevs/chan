# Phase-15 round-3 - @@LaneB journal (Editor + Search frontend)

## Wave 1 - DONE (gated-green + committed + browser-smoked)

Commit `b273e0b5` "feat(editor): [[ completion writes relative-markdown
links on disk". Five files, all under web/src/editor/ (my lane):
Wysiwyg.svelte, bubbles/wiki.ts, bubbles/wikiLinkTargets.test.ts,
widgets/wikilink.ts, widgets/wikilinkParse.test.ts.

### What landed

1. Relative-markdown links on disk. File-mode `[[` completion now emits
   `[stem](./path#anchor)` via links.ts `wikiLinkToMarkdown` instead of
   `[[path]]`. Per-file style: a file that already contains a complete
   `[[...]]` wiki link keeps the wiki form (snapshot taken when the
   bubble opens, regex `WIKI_LINK_RE`); every other file emits relative
   markdown. Raw URL-slot completions (`[label](|)`) relativize too.
   `fromPath` (currentPath) threaded from Wysiwyg.svelte into the bubble.

2. The `[[`-stuck-on-Indexing bubble. When the file-mode empty state
   renders "Indexing...", a one-shot interval (`startIndexWatch`,
   200ms) watches the shared index status and re-fetches the moment the
   in-flight reindex finishes, instead of pinning "Indexing... 0
   documents" until the next keystroke. Torn down on dismiss.

3. Resolver percent-decode (a bug I found during the browser smoke, not
   in the original task). The on-disk relative-markdown URL is percent-
   encoded (a spaced filename becomes `Brazilian%20Rice.md`). The
   backend graph scanner (pulldown-cmark) decodes the destination, so
   the on-disk edge is valid, but the editor's `parseInternalLink` did
   NOT decode, so the pill resolved `Brazilian%20Rice` (no such file)
   and rendered as a broken/strikethrough link. Added `decodePercent`
   in widgets/wikilink.ts so the editor mirrors the backend. Without
   this fix the feature ships visibly-broken links for any filename
   with a space or other URL-special char.

### Empirical verification

Built an isolated worktree (HEAD + only my web diff), debug `chan`
serving its own bundle from disk, on a NESTED throwaway drive
(Recipes/Pasta.md + Recipes/Brazilian Rice.md + root Welcome.md +
WikiNote.md). Verified ON DISK + via the backlinks API:

- Markdown-mode note (Recipes/Pasta.md): `[Welcome](../Welcome.md)`
  (../ up to root) and `[Brazilian Rice](./Brazilian%20Rice.md)` (./
  sibling, %20). Both render as resolved pills and both produce graph
  edges (`/api/backlinks` confirms Pasta -> Brazilian Rice).
- Wiki-mode note (WikiNote.md, pre-seeded with `[[Welcome]]`): a new
  link stayed `[[Recipes/Brazilian Rice.md]]` (wiki form kept).
- Heading hit from file mode -> `[Welcome](../Welcome.md#welcome)`
  (anchor preserved).
- The percent-decode fix: re-smoked after rebuild; the previously-
  broken (strikethrough) Brazilian Rice pill now renders resolved.

EMPIRICALLY-UNVERIFIED (told to @@Architect): the stuck-Indexing fix's
stuck STATE could not be reproduced on this throwaway. The bug only
manifests when a reindex (re-read + re-embed) lingers long enough to
catch; small drives reindex in milliseconds and a debug build without
the bundled embed model never opens that window. The fix is verified by
construction + the full green gate + the static source test; the
recovery path (re-fetch on completion) is simple and well-contained,
but a true stuck-state repro needs a large churning drive.

### Gate (integrated shared tree, with @@LaneC + @@LaneD web work present)

svelte-check 0 errors / 0 warnings; vitest 159 files / 1587 tests pass
(incl. the new wikilinkParse round-trip test + updated wikiLinkTargets
test); `npm run build` green. No Rust touched by this lane, so the
cargo gate is unaffected by my commit.

### Scope notes / Wave-2 carryover

- Explicit `#` heading mode and `^` block mode still commit the wiki
  form (`[[target#anchor]]` / `[[target^id]]`). Their typed target may
  be a title/stem rather than a resolvable path, so relativizing it
  into markdown is unsafe (the markdown resolver is path-based, not
  title-based). Wave-2 (heading/block round-trip) owns resolving the
  typed target to a real path FIRST, then converting. File-mode heading
  hits (real path + anchor) already convert in Wave 1.
- The image resolver (extensions/image.ts `resolveImageSrc` ->
  normalizeHref) likely needs the same percent-decode for images whose
  filenames contain spaces. The image bubble currently inserts LITERAL
  paths (no encoding), and pulldown does NOT parse a literal space in a
  destination (empirically: a `[lit](./Brazilian Rice.md)` probe
  produced NO graph edge). So images with spaces are probably already
  broken end-to-end and want the same encode-on-write + decode-on-read
  treatment. Out of Wave-1 scope; flagged for a follow-up.

## Incident: shared-index commit race (recovered, no work lost)

First commit attempt swept in @@LaneD's chan-shell/main.rs/submitMode +
@@LaneA's control_socket.rs because a concurrent agent staged files in
the shared `.git/index` in the window between my pre-check (`git diff
--staged` empty) and my `git add` of 5 explicit paths. Even the chained
add+audit+commit did not protect against it. Recovered with
`git reset HEAD~1` (mixed; preserves everyone's working-tree changes)
then re-committed with the race-proof `git commit -F msg -- <paths>`
pathspec form, which commits ONLY the named paths regardless of index
state. The bad commit (adb68241) never had anything built on top and is
gone; @@LaneA's preflight (d1b7c427) and @@LaneC's team (8eb99391)
commits below it are untouched. Lesson for the round: in this shared
worktree, NEVER `git add` + `git commit`; use `git commit -- <paths>`.

## Wave 2 - DONE (gated-green + committed; 2 items browser-pending)

Commit `9349dba2` "feat(editor): heading/block links + image spaces as
relative markdown, click-to-caret". 11 files, all under web/src/editor/
(my lane): Wysiwyg.svelte, links.ts, widgets/wikilink.ts,
extensions/image.ts (+imageSrcEncode.test.ts), bubbles/{image,image_drop,
wiki}.ts, bubbles/wikiLinkTargets.test.ts, click_caret.ts (+
clickCaret.test.ts).

### What landed, per item

1. **Heading `#` / block `^` round-trip (Theme 3).** Both explicit modes
   now route their commit through the existing `fileLinkInsert`, so they
   emit relative markdown `[stem](./path.md#anchor)` (or keep wiki form
   in a wiki-mode file) exactly like a file-mode hit, instead of a
   verbatim `[[target#anchor]]` / `[[target^id]]`.
   - Heading: the typed target is a REAL path at commit time
     (`/api/headings` is an exact `rel_path` match, so a heading hit can
     only exist when the target names an indexed file), so relativizing
     it is safe. Anchor is the heading slug.
   - Block: the anchor is emitted as a `#^id` fragment. The backend
     `split_anchor` (workspace.rs:4107) keeps `^id` for a `.md` target,
     so the link resolves. The OLD `[[target^id]]` wiki form never
     resolved (split_anchor only cuts on `#`, leaving `target^id` as an
     unresolvable path), so this is a strict fix, not just a reformat.
   - EMPIRICALLY VERIFIED via the live API (my server :7820): writing the
     exact byte strings my commit logic produces and reading the graph
     back, `/api/backlinks/Recipes/Pasta.md` returned
     `[{src:BlockTest.md, anchor:"^testblk"}, {src:Welcome.md,
     anchor:"ingredients"}]`. Heading + block anchors both resolve.

2. **Image filenames with spaces (Wave-1 carryover).** The image bubble
   (`bubbles/image.ts commitPath`) and the drop/paste handler
   (`bubbles/image_drop.ts`) now percent-encode the path on write
   (`./images/My%20Photo.png`), and `resolveImageSrc` percent-DECODES on
   read before `normalizeHref` so the path is encoded exactly once for
   `/api/files` (no `%2520` double-encode). Mirrors the Wave-1 `[[`
   wiki-link fix. Shared `encodeRelPath` / `decodePercent` helpers now
   live in `links.ts`; the wikilink resolver's private `decodePercent`
   copy is deduped onto the shared one.
   - EMPIRICALLY VERIFIED: a spaced image written `./images/My%20Photo
     .png` produced a graph edge to `images/My Photo.png` AND
     `/api/files/images/My%20Photo.png` returned HTTP 200 (bytes fetch).
     A legacy literal-space src still resolves (decodePercent is a no-op
     without `%`), so no display regression.

3. **Click-to-place-caret (Theme 3).** New `click_caret.ts`: a
   `mousedown` domEventHandler that drops the caret on the nearest row
   position for a blank-area click (right of a short line, a row's
   trailing space, below the last line). CONSERVATIVE by construction: it
   only fires when `posAtCoords(coords)` (precise) returns null, so a
   normal click on text - and the image/pill widget mousedown handlers
   that stop propagation first - are untouched. Wired after
   `listCaretGuard` (both ignore precise clicks). Root cause confirmed in
   the editor CSS: `.cm-editor` is page-width-capped + centered
   (`--chan-page-max-width`, Wysiwyg.svelte:704) and `.cm-content` has a
   60px bottom padding (:726), so blank-area clicks miss every glyph box
   and CM6's precise hit-test returns null, leaving the caret unmoved.
   - **EMPIRICALLY-UNVERIFIED IN BROWSER** (told to @@Architect): the
     plan explicitly wants a running-server browser smoke for this
     (Svelte/CodeMirror runtime), but my `navigate` to the test server
     was DENIED (the browser is shared across lanes). The handler is
     gated-green + source-tested + root-cause-grounded + conservative
     (can't regress precise clicks), committed under the
     pre-release-merge-unverified norm. Needs a live smoke (or revert if
     it does not resolve the dead click). Routed to @@Architect.

### Theme 4 search FE (item from lane doc) - DISPLAY-ONLY, no change

@@LaneA's PROBE landed (search-api-contract.md + their fix c854d3f8):
mentions/paths/.md now match server-side, response shape UNCHANGED.
Verified SearchPanel already passes the raw trimmed query straight to
`api.searchContent` (SearchPanel.svelte:178 + 201) with no client-side
punctuation stripping or mention/path special-casing. Nothing to change.
The optional "keyword match" affordance is polish, skipped.

### Carryover / unverified for @@Architect

- **Click-to-caret browser smoke** (item 3 above): pending browser access.
- **`[[` stuck-Indexing bubble** (Wave-1 fix, untouched this wave): still
  browser-unverified. Its precondition (a churning drive opens a
  catchable reindex window) was already PROVEN by @@LaneA's Wave-1 smoke
  (300-file drive, edit burst, 584 reindexing samples). The FE
  bubble-resolves-on-idle behavior remains the only unobserved part;
  needs the same large-drive browser smoke I am blocked on.
- **Graph edges PK finding (A-domain, flag only):** the edges table PK is
  `(src, dst, kind)` with anchor as a plain column (graph.rs:373), so a
  single file that links the SAME target with BOTH a heading anchor and a
  block anchor collides on INSERT OR IGNORE and keeps only the first
  anchor. Not a Theme-3 bug (single links resolve fine), but it caps
  per-file multi-anchor links to one. Surfaced to @@LaneA.

### Gate (web)

svelte-check 0 errors / 0 warnings; vitest 161 files / 1609 tests pass
(incl. new clickCaret + imageSrcEncode + heading/block source-pattern
tests + the deduped decodePercent still green in wikilinkParse); `npm run
build` clean. No Rust touched, so the cargo gate is unaffected by this
lane. Built + smoked against a renamed binary copy (/tmp/chan-laneb-smoke,
debug = serves web/dist from disk) on an isolated drive (:7820); the
shared-tree cargo build is red mid-wave on @@LaneC/@@LaneD's in-flight
survey wiring (chan-shell SurveySpec.followup), so the binary build was
done from the committed HEAD via the debug disk-serve path, not the
flickering worktree.

## Wave 3 - DONE (Theme-6 cleanup committed; graphData already-satisfied)

Commit `a930a96f` "docs(journals): essence-only phase reports, drop raw
provenance" (514 files: 500 raw deletions + 13 README mods + 1 new
phase-14 README; +379 / -113566). Committed on top of @@LaneA's graph
fix `beb0dc49` via the race-proof pathspec form (`git commit -F msg --
<phase dirs>`), so it did NOT sweep A's graph.rs or any peer work;
post-commit `git show --name-only` audited: every committed path is under
docs/journals, phase-8 + phase-15 untouched.

### Theme-6 docs/journals cleanup (DELETE-RAW + SUMMARIZE, @@Host-confirmed)

Spec: `round-3-lane-b-theme6-spec.md`. Ran after my Wave-1 relative-link
rule (b273e0b5), as required.

- phases 1-7, 9-13: each README gained a `Tags:` outcome-hashtag line; the
  trailing `## Raw material` link list was collapsed to a one-line
  git-history note; every inline `[..](raw/..)` body reference was
  de-linked to a plain backtick path (a provenance pointer into git
  history, not a dead clickable link); then `raw/` was deleted. The bulk
  README pass ran as four parallel edit-only subagents over disjoint
  phase groups; I did the de-link + deletion + commit centrally.
- phase 14: had flat raw + coordination/ but no README. A subagent
  synthesized an essence README in the phases-1-13 shape (gateway
  monorepo migration + frontend pristine cleanup + paced graph hot paths
  + pre-flight relocation, with a git-dated Duration); then the flat raw
  was deleted.
- top-level docs/journals/README.md: recorded the layout change
  (README-only; raw in git history), flipped phase 14 to closed, added
  phase 15 in-progress.

DATA-LOSS BUG CAUGHT (confabulation-discipline win): my first inline
de-link used a too-greedy perl regex `\[[^\]]*\]\(raw/..\)`. In phase-9 a
literal "`[[` search mismatch" sits before a `[..](raw/rich-prompt-
revamp.md)` link with no `]` between, so the regex swallowed the whole
span and deleted a paragraph of real content. Caught by diffing the
de-linked result against HEAD (a Python detector flagged any raw-link
whose greedy visible-text contained a `[`); ONLY phase-9 was affected
(2 spots). Restored phase-9 from HEAD and redid it with explicit Edits.
Verified post-fix: all 13 READMEs have 0 remaining `](raw/`, 0
double-backtick artifacts, phase-9's deleted content (search-mismatch +
backend-enhancements + Spawn-agents bullet) is back, ASCII clean.

SAFEGUARDS APPLIED (audit-trail rule):
- phase-8 `raw/` DEFERRED (not deleted): `docs/agents/desktect.md` +
  `docs/agents/bootstrap.md` cite phase-8 content. Those links are
  ALREADY broken (they point at the pre-`raw/` layout, e.g.
  `phase-8/architect/..` when the file now sits at
  `phase-8/raw/architect/..`), so my deletion would not break a working
  link, but it would remove even the content a future fix points at.
  docs/agents/ is out of my lane -> ESCALATED to @@Architect (assign the
  5-citation repoint, then phase-8 raw goes).
- phase-15 (active round bus) + pub-site-release (untracked, non-phase):
  out of scope, untouched.
- images: already transcribed+removed for phases 1-13; none in phase-14;
  phase-15's 4 are part of the active bus (deferred to round close).

Gate: docs-only change (only docs/journals/*.md + deletions). Nothing
under docs/journals is compiled or embedded (rust-embed bakes web/dist;
the binary embeds web/, never docs/), and no .rs/.ts references a journal
path (grep-confirmed; the only refs are docs/agents/*.md). So the
cargo/web gate is unaffected. Local commit only, not pushed.

### Graph hygiene frontend (graphData.svelte.ts) - ALREADY SATISFIED, no change

@@LaneA's ghost-node fix landed (beb0dc49): unresolved link targets now
produce neither a node nor a dangling edge; only indexed-then-vanished
files still carry `missing: true` (a distinct stale-index signal). I read
graphData.svelte.ts end to end against this: it has NO ghost/`missing`
logic at all. It is a passive streaming cache (nodes-by-id, edges-by-key)
plus two lookup helpers, both already carrying `if (!target) continue` /
`if (src && ..)` guards. Those guards are streaming-ORDER safety (an edge
batch can arrive before its target node's batch), NOT ghost handling, so
they stay. With the backend no longer emitting ghost nodes/edges,
graphData renders the cleaner wire correctly with zero code change. The
"+ less clutter" half is delivered by the Theme-6 deletion above (the
chan-source graph no longer carries ~500 raw/ doc nodes).

Checked the edges-PK angle too: `GraphViewEdge` carries no `anchor` field
(the visual graph intentionally collapses multi-anchor-same-target link
edges to one edge), so `edgeKey` is correct and A's edges-PK DB fix does
not change this wire. No graphData change there either.

This is an "already satisfied" outcome, not a skipped task: fabricating a
graphData diff would add risk for no behavior change. Verified by (1)
source reading, (2) A's backend tests
(`unresolved_link_target_produces_no_ghost_node_or_edge`,
`link_to_directory_does_not_synthesize_ghost_file_node`), (3) the
objective file reduction from Theme-6.

EMPIRICALLY-UNVERIFIED (told to @@Architect): the live Cytoscape RENDER of
the chan-source graph (the exact @@Host scenario) is GraphPanel's canvas,
not my lane, and the shared browser was denied to B in prior waves ->
folds into the Wave-3 joint smoke / @@Host desktop verify.

### Escalations to @@Architect (cross-lane)

1. phase-8 raw deletion deferred: assign the docs/agents 5-citation
   repoint (out of my lane), then phase-8 raw can go.
2. STALE TYPE COMMENT from A's beb0dc49: `web/src/api/types.ts:381-383`
   still says `missing` is "True for ghost nodes synthesized as the
   target of a broken link." After the fix, ghost nodes are no longer
   synthesized; `missing` now means only "indexed file vanished from
   disk." types.ts is the shared wire-type mirror (A's wire change), so
   flagged rather than edited.

### Editor browser smokes (click-to-caret + [[ stuck-Indexing) - ATTEMPTED, BLOCKED

@@Host relayed (via @@Architect) that I finished these two smokes. The
durable record does NOT bear that out, and I will not fabricate results.
My Wave-1 + Wave-2 journals explicitly recorded BOTH as
EMPIRICALLY-UNVERIFIED because the shared-browser `navigate` was denied to
@@LaneB; this session (Theme-6 + graphData) did not run them either. So
there is no smoke evidence to append: I (this instance) have not observed
these smokes. If a prior @@LaneB refresh ran them, it left nothing in the
journal, and "all durable state lives in the docs" - I cannot vouch for
unobserved results.

I attempted to generate REAL evidence now:
- Rebuilt web/dist (npm run build, 07:38) so a debug server's disk-served
  bundle carries my committed click_caret + stuck-bubble fixes (b273e0b5,
  9349dba2). The running chan-desktop at :8799 is the RELEASE v0.21.0 app
  which BAKES its bundle, so it serves a STALE pre-fix bundle - smoking
  there would be a false positive/negative.
- Stood up an isolated debug server (renamed binary /tmp/chan-laneb-w3,
  serves web/dist from disk) on a 1502-file drive (/tmp/lanebw3:
  short-line caret.md + 1500 filler notes for a catchable reindex-churn
  window), port 7843 --no-token. Healthy: index idle, 1502 docs.
- Designed controllable repros: caret = click blank space right-of /
  below short lines -> caret should land at the line-end text position;
  stuck-bubble = type `[[qqqzzz` (never matches) during churn -> see
  "Indexing... searched N documents"; STOP churn -> with the fix it
  auto-flips to "No matches in N documents" with no keystroke (the bug
  leaves it stuck on "Indexing...").
- BLOCKED: the browser `navigate` action was again DENIED to @@LaneB
  (same shared-browser block as Waves 1-2). Without navigate I cannot
  load the fresh-bundle server in a tab, so I cannot drive either smoke.

Net: both fixes remain committed + source-tested (clickCaret.test.ts,
wikilinkParse / empty_state) + gated-green, carried under the
pre-release-merge-unverified norm. Runtime browser confirmation needs
EITHER `navigate` re-allowed to @@LaneB (server parked on 7843, ready to
smoke immediately) OR a browser-capable lane / @@Host to run them.
Surfaced to @@Architect; not claiming a pass I did not observe.
