# @@LaneB -> @@LaneA

Append-only. @@LaneB writes here. Most recent entry at the bottom.

Primary use: the KIND route signature for Lane A's Inspector
kind-chip wiring.

## 2026-05-28 14:00 @@LaneB -> @@LaneA
Graph KINDS route signature - YOU ARE UNBLOCKED for kind-chip wiring

**Summary**: `/api/graph` keeps its current `GraphView { nodes,
edges }` response shape and grows a `?kind=` discriminator. The
HTTP surface is internal to the GraphPanel; Lane A wires through
the `openGraphFor*` helpers in
`web/src/state/store.svelte.ts`, not the route. Three calls cover
the four chip kinds (path / language / tag / contact); path
already works via existing helpers, tag via existing
`openGraphForTag`, and Lane B will add `openGraphForLanguage` +
`openGraphForContact` in this slice.

### Tab plumbing - what Lane A imports

From `web/src/state/store.svelte.ts`:

```ts
// kind=path: existing, no change
openGraphForFile(path: string): void
openGraphForDirectory(path: string): void
openFsGraphForFile(path: string): void
openFsGraphForDirectory(path: string): void

// kind=tag: existing, signature unchanged
openGraphForTag(nodeId: string, label: string): void
// nodeId is the same value Inspector chips already carry
// (typically the bare tag name, no leading '#'). Lane B will
// ensure the new tag-lens route accepts the same identifier.

// kind=contact: NEW (Lane B will export in the KIND slice)
openGraphForContact(relPath: string): void
// relPath is the contact file's workspace-relative path
// (e.g. "Contacts/alice.md"). Inspector contact chips already
// know the rel_path from contacts() rows.

// kind=language: NEW (Lane B will export in the KIND slice)
openGraphForLanguage(language: string): void
// language is the canonical chan-report language id
// ("rust", "typescript", ...). Same value the LanguageInfoBody
// chip uses today.
```

All three "open" functions schedule the session save themselves,
spawn a fresh graph tab in the active pane (no dedup; matches
`fullstack-47`), and set the tab title via `graphTitle()` once
Lane B's title-prefix slice lands (`path=…` / `tag=…` /
`lang=…` / `contact=…`). Lane A can wire the chips against the
helpers right now - title formatting is Lane B's problem.

### scopeId convention (FYI - Lane A does NOT touch this)

Lane B carries kind via the `scopeId` prefix Lane A already knows:

| Kind     | scopeId shape           | Example                  |
|----------|-------------------------|--------------------------|
| path     | workspace               | workspace                |
|          | file:<rel>              | file:notes/today.md      |
|          | dir:<rel>               | dir:notes/2026           |
| tag      | tag:<name>              | tag:phase-13             |
| contact  | contact:<rel_path>      | contact:Contacts/alice.md|
| language | language:<lang>         | language:rust            |

`graphTitle()` already groks `tag:` and `contact:`; Lane B will
add the `language:` arm and the kind-prefix on every arm.

### HTTP shape (Lane B internal; Lane A does NOT hit this)

For posterity:

```
GET /api/graph?kind=path&scope=workspace|directory|file&path=<rel>&depth=N
GET /api/graph?kind=tag&tag=<name>&depth=N
GET /api/graph?kind=contact&contact=<rel_path>&depth=N
GET /api/graph?kind=language&language=<lang>
```

`kind` defaults to `path`; the legacy `?scope=workspace&path=` form
keeps working unchanged. `/api/graph/languages` stays for the
existing language-overview lens; the new
`?kind=language&language=` form is the "lens on one language" entry
point.

### What Lane A can ship right now

- KindChip click handler: dispatch on chip kind, call the
  corresponding helper above.
- For tag chips: keep using `openGraphForTag(nodeId, label)`.
- For contact + language chips: import the new helpers once they
  land. If you want to wire the imports before the helpers exist,
  add a TODO and post on this channel; Lane B will land them in
  the KIND slice.

### Cross-lane reminder

Lane B's `WorkspaceInfoBody` reuse in the Dashboard widget set
depends on Lane A's workspace-root parity slice; we'll coordinate
ordering when Lane B's Dashboard slice approaches that piece. No
action needed from Lane A right now.

## 2026-05-28 14:11 @@LaneB -> @@LaneA
merge gate ran: both your slices are on main

`phase-13-lane-a@b2ef3f3b` (bugs 1-4) and `phase-13-lane-a@ad184179`
(inspector parity) merged into `main@f97146cc` alongside Lane B's
pane bugs slice (`phase-13-lane-b@ea23a691`). Combined-tree gate
green: cargo fmt / clippy -D warnings / test / build
--no-default-features / npm check / npm build / vitest 1615
passed.

Auto-merge in `web/src/state/tabs.svelte.ts` was clean (your
draft-close fix at line 2044, my `setActivePane` guard at line
3139 - file-disjoint within the same file). No conflicts elsewhere.

`WorkspaceInfoBody` parity from `ad184179` is now available for
my Dashboard widget reuse - the cross-lane note Lane B raised in
turn 1 has its prerequisite landed. I'll grab from your slice
when I get to the Dashboard piece.

Now unblocked for browser-smoke walks of the integrated tree.

The KIND backend slice (the implementation half of the route
signature I posted earlier) is my next slice; will post on this
channel when the new `openGraphFor{Contact,Language}` helpers
land.

No git push performed (per `feedback_merge_is_not_push`); @@Alex
gates that.

## 2026-05-28 14:30 @@LaneB -> @@LaneA
KIND helpers landed - import + wire your chips

`phase-13-lane-b@24f1f31d` is on main. The helpers are now
exported from `web/src/state/store.svelte.ts`:

```ts
import {
  openGraphForFile,        // existing - kind=path file
  openGraphForDirectory,   // existing - kind=path dir
  openFsGraphForFile,      // existing - filesystem mode variant
  openFsGraphForDirectory, // existing - filesystem mode variant
  openGraphForTag,         // existing (signature unchanged) - kind=tag
  openGraphForContact,     // NEW - kind=contact
  openGraphForLanguage,    // NEW - kind=language
} from "../state/store.svelte";
```

For your Inspector kind-chip switch:
- Tag chips: keep calling `openGraphForTag(nodeId, label)`.
- Contact chips: call `openGraphForContact(relPath)`. `relPath` is
  the workspace-relative path of the contact file
  (`Contacts/alice.md`).
- Language chips: call `openGraphForLanguage(language)`.
  `language` is the chan-report language id (`rust`, `typescript`,
  ...).
- Path chips (file / dir): the existing `openGraphFor{File,Directory}`
  /  `openFsGraphFor{File,Directory}` helpers are unchanged.

Tab title shape per the round-1 spec - the strip will read as:

```
path=<basename>    path=workspace    path=notes/
tag=#search        contact=alice.md  lang=rust
```

KNOWN LIMITATION (slice 2b in flight): for `contact:` and
`language:` scopeIds the GraphPanel currently renders the full
workspace graph (not yet a lens). Tab title is right; render is
"workspace graph but with a lens-shaped title". Slice 2b adds the
ScopeOption kinds + BFS-from-center filter so the actual lens
semantics render (contact = backlinks subgraph; language =
bubble + edges to every file of that language). You can wire the
chips against the helpers NOW; the lens semantics land
transparently when 2b merges.

The auto-merge in `tabs.svelte.ts` from the previous round held;
no further surface conflicts surface from this slice (only Lane B
files touched).

## 2026-05-28 15:00 @@LaneB -> @@LaneA
slice 4a merged + 2b live - go on slice 4b

`phase-13-lane-a@39fd3373` (KindChip onClick + path/tag wiring) is
on main at `main@7c936504`. The combined-tree gate hit the
indexer flake on the first run (passed on re-run via
`feedback_fresh_binary_rewalks`); web gate vitest 1619 passed.

`openGraphForContact(relPath)` + `openGraphForLanguage(language)`
have been live since `24f1f31d` (slice 2a) and now actually
render lens-centered subgraphs (slice 2b): contact lens is
bidirectional BFS from the contact file node (captures every doc
that references the contact + everything the contact links out
to); language lens is 1-hop from the bubble (every file of that
language splays around it).

Slice 4b (contact + language KindChip wiring) is unblocked. Wire
the contact + language chips' onClick to the helpers; rebase on
`main@7c936504` first so your KindChip slice 4a is the base.

Tab title shape for those lenses: `contact=<basename>` /
`lang=<name>` per slice 2a's `graphTitle()` extension. Scope
header dropdown row renders the contact name with an `@@` prefix
and an `AtSign` icon (Lucide); language renders with the bare
language id and a `Code2` icon.

## 2026-05-28 15:30 @@LaneB -> @@LaneA
slice 4b merged - lane-a roadmap drained

`phase-13-lane-a@08b28da8` is on main at `main@11bf6b19`
alongside Lane B's slice 3a (the InfographicsTab → DashboardTab
internal rename). KindChip is wired end-to-end for path / tag /
mention / contact / language. Combined gate vitest 1625 passed
(+6 from your slice-4b test file). No flakes.

The pending browser-smoke walk you mentioned (KindChip click ->
graph lens render for each kind) is overdue per
`feedback_svelte_static_gate_misses_runtime`. If you want to
drive that walk, go ahead - I'll be deep in the Dashboard widget
rework (slice 3b) so happy to defer the smoke to your lane. If
you'd rather I batch it with the Dashboard slice smoke, also
fine; ping back and I'll pick it up.

If your lane is genuinely drained per your own report and you
have idle bandwidth, the round-1 carryover I'd most welcome help
on is the chan-desktop Shift+Enter smoke that's still pending
from your bug-4 work. Otherwise, I'll close it out as part of
the release-cut verification at round close.

## 2026-05-29 01:01 @@LaneB -> @@LaneA (closing-3 routing)

@@Alex empirically tested main@`4e19d8d2` and reported four
follow-up bugs. Three are mine (C1 + Bug 1 + Bug 2 + C2/C3 polish)
and committed on phase-13-lane-b. One is yours: the missing
separator between the COCOMO section and the NOTES DIRECTORIES
section in the workspace inspector (`WorkspaceInfoBody.svelte`).

@@Alex's words: "I explicitly asked for a separator between cocomo
and NOTES DIRECTORIES, still missing".

Screenshot context: the inspector shows the COCOMO block (effort /
schedule / developers), then NOTES DIRECTORIES heading + the
default notes-directory editor field, with NO visual divider
between them. The user wants a separator there (probably matching
the dashed border the about-fund uses, or the `<div class="msep">`
shape from the graph menus - whichever you've settled on for
inspector body section breaks).

File: `web/src/components/WorkspaceInfoBody.svelte`. Likely a
2-line CSS rule + one `<div class="..."></div>` between the two
sections. Variant gating may apply - the separator is needed in
the `variant="dashboard"` shape (where NOTES DIRECTORIES renders).

No file-disjoint conflict with anything I'm shipping. Queue it on
your closing-3 branch and signal merge-ready on
`event-lane-a-alex.md` when it's gated.

(Bug 3 - clickable Languages - was already fixed by your A5 at
`4280d5f3` and merged in 4e19d8d2; @@Alex's smoke from before that
merge didn't have it. Not flagging that one.)

---- Round 2 ----

## 2026-05-29 @@LaneB -> @@LaneA
OVERLAP DECLARATION: B-slice 2 must edit App.svelte (Cmd+I removal)

Heads up before I touch App.svelte. Bootstrap names App.svelte as
YOUR file this round (the Cmd+P / Team Work flow). B-slice 2 ("move
Dashboard off Cmd+I so the editor can claim Cmd+I for italic")
needs one surgical edit there, because the request file's premise
("no App.svelte change needed") turned out wrong:

- The real Cmd+I -> Dashboard binding is NOT in `shortcuts.ts`
  (that registry is display-only). It's a hardcoded
  `if (... && e.code === "KeyI")` branch in `onWindowKey`
  (App.svelte ~849-853) that calls `openDashboardInActivePane()`.
- CM6's keymap does not stopPropagation, so once I bind Mod-i for
  italic in the editor, Cmd+I in the editor would BOTH italicize
  AND open Dashboard unless that branch is removed.

What I'll do: delete ONLY the `KeyI` branch (App.svelte ~843-853),
replacing it with a one-line comment. I will NOT touch the
`KeyP` / Cmd+P region or anything else in onWindowKey. Please keep
your Cmd+P (KeyP) additions in their own branch region so our
App.svelte diffs stay disjoint; I'll reconcile at the merge gate
either way. The desktop half (serve.rs KEY_BRIDGE KeyI ->
app.dashboard.open) is mine and disjoint from you.

If you'd rather own this App.svelte deletion yourself, say so and
I'll hand you the exact lines; otherwise I proceed surgically.
Also flagged to @@Alex on event-lane-b-alex.md.

Separately: still waiting on the Team Work *label* string for
`app.terminal.richPrompt` (+ the "Rich Prompt" -> "Team Work"
menu/welcome label) before I apply the rename in shortcuts.ts /
Pane.svelte / EmptyPaneWelcome.svelte. Drop it here when ready.

## 2026-05-29 @@LaneB -> @@LaneA
merge gate: combined tree GREEN, but 2 rename/cleanup residuals in YOUR files

Gated phase-13-r2-lane-a@25c81182 + phase-13-r2-lane-b@ae06398b in
../chan-integration. Clean auto-merge (Pane.svelte/tabs.svelte.ts/
App.svelte overlaps merged fine). FULL gate green: cargo fmt/clippy
-Dwarnings/test (0 failed)/build --no-default-features; web svelte-check
0/0/4107 + build + vitest 1570 passed. My "Team Work" label rename
(ae06398b) is in.

But my residual sweep for user-facing "Rich Prompt" caught two in YOUR
surfaces that the rename+deletion missed - and both look like dead/stale
code from your deletion, so they're your rename-vs-REMOVE call, not a
blind string swap:

1. `web/src/state/store.svelte.ts:308` `workspaceWarningLabel`: the
   `broken_rich_prompt` branch returns the user-facing label "Broken
   Rich Prompt". But `broken_rich_prompt` appears ZERO times in
   `crates/` - the backend no longer emits that warning kind after your
   deletion. So this looks like a DEAD frontend branch (also the
   `canDiscardWorkspaceWarning` check at ~315). Recommend: remove the
   broken_rich_prompt handling (+ its type variant) rather than rename.

2. `web/src/components/FileInfoBody.svelte:787`: the Drafts inspector
   notice still reads "Rich Prompt submissions persist as
   Drafts/rich-prompt-N/". Your revamp deleted the rich-prompt-workspace
   archival (submissions now go to the lead terminal; chan-team.toml
   lands in /tmp/new-team-1). So this clause describes behaviour that no
   longer exists - stale, not just a label. Recommend: drop/rewrite the
   clause. (FYI the `rich-prompt-N` draft-dir convention still lingers
   in chan-workspace drafts.rs/workspace.rs/paths.rs comments + tests -
   11 refs - if nothing creates rich-prompt-N dirs anymore that's also
   vestigial; your call.)

These don't break the build/tests (gate is green), so it's your choice:
fix on your branch + re-signal merge-ready, and I'll re-gate + merge; or
tell me to merge as-is and you clean up in a follow-up. I'm holding the
merge to main until you/@@Alex decide. I did NOT touch your files.

## 2026-05-29 @@LaneB -> @@LaneA
UPDATE: @@Alex had me do the cleanup + a full rich-prompt scrub myself

Heads-up so you don't re-fix: @@Alex directed me to clean the residuals
myself AND to leave NO "rich prompt" code anywhere. So at the merge gate
I scrubbed every remaining identifier across the combined tree (the
chord id app.terminal.richPrompt -> app.terminal.teamWork, tab.richPrompt
-> tab.teamWork, .rich-prompt CSS -> .team-work, backend
rich_prompt/rich-prompt -> team_work/team-work, 5 richPrompt* test files
-> teamWork*, all comments). Your internal identifiers that you'd kept
stable are now renamed too - this overrode the "chord id stays stable"
plan per @@Alex's explicit call. Gated green + browser-smoked the Cmd+P
flow. MERGED to main (c4a4adc6, no push). If you pick up follow-up work,
branch fresh off main - your phase-13-r2-lane-a is fully merged.
