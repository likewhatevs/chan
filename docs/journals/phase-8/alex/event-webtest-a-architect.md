# event-webtest-a-architect.md

From: @@WebtestA
To: @@Architect
Date: 2026-05-19

## 2026-05-20 00:20 BST — poke (Round-1 bug-sweep complete + test-server URL hand-off)

Round-1 bug sweep done for the @@WebtestA coverage cluster
(15 bugs). Curated summary, verdicts, side observations, and
test-server URL hand-off all live at the bottom of
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under "Round-1 bug-sweep summary (curated)".

TL;DR:

* **Active repros (need dispatch)**: bug 8 (graph
  false-missing — 8/1102 nodes), bug 11 (image-insert pushes
  cursor + view doesn't roll on subsequent typing).
* **Partial**: bug 6 (Cmd+T blocked on web, native viable),
  bug 20 (re-open with bubble present focuses prompt input).
* **Could not reproduce**: bug 14 (watcher hung first try).
* **Fix holds on HEAD**: bugs 1, 2, 4, 5, 7, 9, 10, 18, 19,
  21 (all phase-8 patches already merged post-v0.11.0).
* **Side observations worth tracking**: `.md.md` double
  extension from "New file" dialog; "Stage:" copy in Hybrid
  help inconsistent with bug-7 immediate-commit; Cmd+K p
  focus race; Cmd+Enter first-char swallow.

Test-server URL (per process.md):
`http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`
against `/tmp/chan-test-phase8-wa/`. Please forward to @@Alex.

Server still running; will leave it up until you signal
teardown or @@Alex is done with it.

## 2026-05-20 00:50 BST — poke (session 2: server restored + fix-verification wave starting)

Previous incarnation's `chan serve` died between sessions; URL
went HTTP 000. Throwaway drive at `/tmp/chan-test-phase8-wa/`
intact. Rebuilt at HEAD (`041de34`) and relaunched. Same URL
(token is deterministic on drive root):

```
http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm
```

Six new fixes have landed in my coverage cluster since the
Round-1 sweep: fullstack-a-4/6/7/8 + fullstack-b-5/6. Plus
bug 20 was partial last time and fullstack-a-4 nominally
addresses it — wants a re-check.

Walking the verifications now. Will append verdicts to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under each fix and poke again when the wave is done (or sooner
if anything reproduces hot).

## 2026-05-20 01:20 BST — poke (mid-wave status, 5/6 cleared)

* **fullstack-a-7** (Cmd+K → Cmd+.): ✓ fix holds — Cmd+. opens
  NAV with correct pill copy, Cmd+K dead (hard switch), Cmd+,
  opens Settings.
* **fullstack-a-6** (Cmd+. F focus): ✓ fix holds — search input
  is the active element on overlay open.
* **fullstack-a-8** (CSS wobble): ✓ fix holds — visible on
  Hybrid NAV entry, search overlay, FB row ctx, FB tab ctx,
  pane-bg ctx. Rich-prompt ctx + Graph ctx not yet exercised.
* **fullstack-a-4 sub-bugs 5/10/19**: ✓ fix holds — Alt+Space
  focuses prompt input, Cmd+Enter retains focus, spawn dialog
  renders, terminal pushes up above the prompt.
* **fullstack-a-4 bug 20**: **inconclusive this session** —
  could not get a clean prompt-close→reopen with bubble
  present; chan-server-side layout preference keeps
  resurrecting a README.md editor tab during the sequence,
  which steals active-tab focus. Will retry next pass against
  a non-pre-populated drive. The patch intent is sound; the
  test setup is the blocker, not the fix.
* **fullstack-b-5** (per-Hybrid theme), **fullstack-b-6** (FB
  watcher scope): not yet tested in this pass — continuing now.

## 2026-05-20 01:35 BST — poke (wave wrap, 5✓ / 1 inconclusive / 1 deferred)

* **fullstack-b-6** (FB watcher scope): ✓ fix holds —
  MutationObserver count: **0** mutations on out-of-scope writes
  (`crates/chan-drive/src/`, `desktop/`); **11** mutations + new
  row on in-scope drive-root write. Exactly the bug acceptance.
* **fullstack-b-5** (per-Hybrid theme both sides): **mostly
  holds** — source-grepped to the `pane-theme-toggle` Sun/Moon
  button (Pane.svelte:920). Clicked it; front-side cascade is
  clean across 5 surface kinds (pane bg, tab strip, xterm,
  cm-editor wrapper, cm-content fg) all flipping to dark via
  `data-theme="dark"`. Did NOT exercise back-side editor
  specifically — needs an editor open on the back before
  setting `hb`, which I couldn't unstick in-session. The patch
  is in the same cascade the front side proves out so it
  probably holds, but the bug's literal acceptance wants the
  back-side editor test.

Session 2 wave: 6 fixes verified clean-or-mostly (a-7, a-6,
a-8, a-4 sub-bugs 5/10/19, b-5 front-side, b-6), 1 inconclusive
(bug 20 setup). Full table at the bottom of
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under "session 2 wave summary".

Server still live, watcher still attached. Standing by for the
next signal (next wave of fixes, retry on bug 20, or teardown).

## 2026-05-20 01:55 BST — poke (bug 8 update: 5/8 cleared on HEAD post-reindex)

Retested bug 8 (graph false-missing) via `/api/graph` on HEAD.
Round 1 had 8 false-positives; HEAD has **3**. The 5 plain-file
cases (LICENSE, desktop/LICENSE, two `chan-drive/src/*.rs`, the
shell script) are gone — most likely cleared by the full reindex
the server did on session-2 boot after the unclean shutdown (the
`rebuild.inprogress` marker + 1 pending_writes journal entry
forced it).

That suggests **Round 1's plain-file false-positives were a
stale-index artifact, not a real indexer bug**. The narrower
residual is the 3 directory-typed-as-`kind:file` cases:

```
docs/agents
docs/journals/phase-7/alex
docs/journals/phase-8/alex
```

These look like markdown link targets (`[..](docs/agents)`)
that the indexer types as files, then the presence check fails
because the path resolves to a directory. Real bug, narrower
scope than originally reported. Suggesting indexer triage on
just this path (link resolver + kind classification) rather
than the broader presence-check work Round 1 implied.

Detail at the bottom of
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under "bug 8 retest on rebuilt HEAD".

## 2026-05-20 02:05 BST — poke (bug 11 retest: APPEARS RESOLVED on HEAD)

Tested bug 11 (image-insert pushes cursor + view doesn't roll)
two ways:

* **Pass 1** (file absent): typed `![](./test-image.png)` at
  EOF of README.md. Round 1: scrollTop snapped to 2 (top of
  doc), cursor off-screen at y≈3922, no roll on subsequent
  typing. HEAD: scrollTop **unchanged** at 3342, cursor stayed
  visible, subsequent `xyz123` typing kept the cursor in view.
* **Pass 2** (real 341 KB PNG, same image-1.png from Round 1):
  on the second image insert, scrollTop went 3342 → **4468** —
  i.e. the view scrolled DOWN to follow the new content, which
  is the opposite of the Round-1 snap-to-top bug.

The Round-1 snap-to-top behaviour did not reproduce in either
pass. Recommending **close-pending** on bug 11 — wants a
confirmation pass with a clean drive + no incidental
graph-overlay interference (Pass 2 had a backlink graph hijack
that obscured the visible editor surface, but the scroll
numbers told the story unambiguously).

The fix isn't visibly named in `git log 97ca38a..HEAD` — may be
a side effect of fullstack-a-4 / a-7, or a silent CM6
scroll-into-view tweak.

## 2026-05-20 02:05 BST — session 2 close summary

Across the whole session:
* 5 in-cluster fixes verified clean (a-7 / a-6 / a-8 / a-4
  sub-bugs 5/10/19 / b-6).
* 1 mostly holds (b-5 front-side cascade; back-side editor not
  exercised).
* bug 11 appears resolved (close-pending recommendation).
* bug 8 narrowed to 3 directory-typed-as-file cases (plain-file
  cases gone — likely Round-1 stale-index artifact).
* 1 inconclusive (bug 20 setup blocker).
* 3 side observations still reproducing (Cmd+Enter swallow,
  Cmd+. p focus race, Hybrid NAV "Stage:" copy).

Server still up, drive in a known-dirty state (README.md has
test markdown, test-image.png removed). Standing by for next
wave / teardown.

## 2026-05-20 02:15 BST — poke (bug 20 retry: FIX HOLDS)

Cracked the layout-resurrection blocker by setting the
initial pane layout via explicit URL hash
(`#s={"k":"l","t":[{"k":"t","n":"Terminal-1","a":1}],"f":1}`)
after a localStorage/sessionStorage clear. The server respected
the explicit layout — no FB tab, no README.md ghost.

From that clean state, walked the precise bug 20 spec:

1. Open prompt (Alt+Space). Cold-open with no bubbles → focus
   on `cm-content` (correct per cold-open rule).
2. Attach watcher. Bubble appears.
3. Close prompt. Bubble dismisses with the prompt visibility.
4. **Reopen prompt (Alt+Space).** Bubble reappears; focus on
   **BODY**, NOT on `cm-content`. Window-level BubbleOverlay
   handler is the active receiver. ✓
5. **Press `1`.** Bubble dismisses; `cm-content` text content
   stays empty (no keystroke into the prompt buffer);
   `event-reply-bug20-v2.md` lands in `watcher-events/`. ✓

Bug 20: **fix holds**. Round-1 partial reproduction was likely
a side effect of the residual layout state (FB selection on
README.md interacted with the rich prompt's bubble-count race
fix in `TerminalRichPrompt.svelte:66-83`).

## 2026-05-20 02:15 BST — revised session close

Across the whole session: 6 in-cluster fixes verified clean
(a-7 / a-6 / a-8 / a-4 sub-bugs 5/10/19/**20** / b-5 front-side
/ b-6). bug 11 appears resolved. bug 8 narrowed (residual is
the 3 directory-typed-as-file cases). Side observations from
Round 1 still surfacing as known knowns.

No remaining inconclusives. b-5 back-side editor and bug-11
clean-drive confirmation are the only fresh-repro asks I'd
push for next pass.

## 2026-05-20 02:50 BST — poke (b-5 back-side retry: blocked, possibly a separate Hybrid-flip-commit bug)

Tried the URL-hash injection trick for b-5 back-side (set up
Hybrid with Terminal-1 on front, CLAUDE.md on back, then
Cmd+. Tab Enter to flip). Layout injection works (`bt`
present, deserializer accepts it per
`tabs.svelte.ts:3140-3260`), but **Cmd+. Tab Enter doesn't
commit the flip** — hash stays unchanged, Terminal-1 stays
visible. Tried multiple times, same result.

This *might* be a separate bug: per `App.svelte:538-540`'s
comment, Tab is supposed to mutate the pane-mode draft and
Enter is supposed to commit. Empirically the commit doesn't
propagate the flip on this build. The other NAV commands
(1/2/3 spawn, x/Backspace close) work cleanly per bug 7's
verdict, so it's specifically the Tab → flip → commit path
that misbehaves.

Two possibilities I can't disambiguate without source-side
debugging:
1. `flipHybrid` mutates the live layout, not the draft —
   contradicting the comment that says Esc rolls back.
2. The commit path treats Tab as a non-draft-mutating action
   and silently restores the original live state.

Either way, this is outside my verification lane to root-
cause. Surfacing for @@Architect — may want to dispatch a
fullstack lane to grep `commitPaneMode` against `flipHybrid`
and confirm the front/back swap commits cleanly. If it does
NOT commit, that's a Round-1 latent bug that no one tripped
over yet because most users don't populate the back side
before flipping.

b-5 verdict stays at **MOSTLY HOLDS** (front-side cascade
clean; back-side editor surface not exercised). Detail under
the latest append in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

Server restored to FB-pane landing state. Watcher detached
(via layout reset).

## 2026-05-20 03:10 BST — poke (b-5 back-side: FIX HOLDS — earlier "blocked" was wrong)

Walking back the earlier "Hybrid flip-commit blocked" claim.
With longer spacing between keystrokes (Cmd+. → wait 0.5s →
Tab → wait 0.5s → Return → wait 1.5s), the flip DOES commit
cleanly. The hash gains `sb:1` and `t` ↔ `bt` swap. Earlier
attempt failed because of too-tight key timing, not a real bug.

Walked the b-5 back-side test end-to-end:

1. URL-hash injection: editor in `bt`, terminal in `t`.
2. Cmd+. Tab Return → hash flipped (`sb:1`, t/bt swapped).
3. Loaded a fresh markdown file (notes.md) on the back side
   via the FB dock — loaded cleanly.
4. Clicked the pane theme toggle. Hash gained `ht:"d"`
   (pane.theme always describes the currently-visible side).

Back-side editor surface readouts: 6 surface kinds all flipped
to dark via the `data-theme="dark"` cascade — `.pane`,
`.cm-editor`, `.cm-content`, `.cm-line`, `.editor-wrap`,
tab strip. Background `rgb(28, 28, 30)`, text `rgb(240, 246, 252)`.

**Bug "Dark/light theme flip leaves half the Hybrid in the
wrong palette": FIX HOLDS on HEAD for both sides.**

### New candidate side bug from the same retry

CLAUDE.md (10 KB) stayed in `loading...` state indefinitely
on the back side when promoted from `bt` via the flip.
notes.md (smaller) loaded cleanly under the same conditions.
The tab spinner kept rotating; 5+ seconds, no progress.
Could be the content-fetch endpoint dropping the request
when the tab's paneId+slot context changes via the flip swap.
Narrow scope — not blocking b-5; surfacing as a candidate
for fullstack dispatch.

## 2026-05-20 03:10 BST — corrected session close

* **Verified ✓ (6)**: a-7, a-6, a-8, a-4 (5/10/19/20),
  **b-5 both sides**, b-6.
* **Appears resolved ◐**: bug 11.
* **Partial**: bug 8 (3 directory-typed-as-file residuals).
* **Still reproducing**: Cmd+Enter swallow, Cmd+. p focus
  race, Hybrid NAV "Stage:" copy.
* **New candidate side bug**: file loading hangs on back-side
  flip from `bt` (CLAUDE.md repro; notes.md doesn't repro).

Nothing left unverified in my cluster. b-5 is now clean across
both sides. Server still live.



Side observations still reproducing from Round 1 (none of these
are tracked bugs yet, surfacing for @@Architect's judgement):

1. Cmd+Enter eats the first character of the dispatched text
   (`echo hello` → `cho hello` in the terminal).
2. Cmd+. p spawns Terminal-N + opens prompt, but focus lands on
   `xterm-helper-textarea`, not the prompt's `cm-content`.
3. Hybrid NAV help still labels rows 1/2/3 as
   `Stage: Terminal / File Browser / Graph` — inconsistent with
   bug 7's immediate-commit verdict.

Full per-fix appends at the bottom of
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

## 2026-05-20 22:30 BST — poke (v0.11.1 walkthrough complete: 8/8 lane-A fixes HOLD)

Walked the lane-A queue from your "v0.11.1 cut — lane-A walkthrough GO"
poke against the cut binary (HEAD `ada8478`, rebuilt locally). All 8
of my -a- tasks **HOLD** on v0.11.1; -b-7 is @@WebtestB's lane and
deferred per the standing-permission split.

### Verdicts

| Task    | Verdict     |
|---------|-------------|
| -a-28   | FIX HOLDS   |
| -a-29   | FIX HOLDS   |
| -a-30   | FIX HOLDS   |
| -a-31   | FIX HOLDS   |
| -a-32   | FIX HOLDS   |
| -a-33   | FIX HOLDS   |
| -a-34   | FIX HOLDS   |
| -a-35   | FIX HOLDS   |
| -b-7    | DEFERRED to @@WebtestB (chan-desktop runtime) |

Per-task empirical signal in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md) under
"2026-05-20 22:30 BST — v0.11.1 cut walkthrough: 8/8 lane-A fixes
HOLD". Highlights:

* **-a-28**: 35 sample points over 7.5 s, 0 Loading-swap flickers,
  count stable at 3 bubbles; replies dismiss surveys + pre-flight
  (type-agnostic predicate works); explicit X dismiss on poke
  persists across the watcher poll cycles with the source file
  still on disk.
* **-a-29**: terminal-host grows 432→712 px on collapse, back to
  432 px on expand; margin-bottom tracks `measuredHeightPx + 10/12`
  via the ResizeObserver per impl.
* **-a-30**: ctx-menu slider mutates `pageWidthRatio`; inline
  `--chan-page-max-width: 480px` at 50% on a 961-px-wide prompt
  matches the formula; setting persists across reload through
  chan-server session store (URL hash excludes `rppw` by design).
* **-a-31**: self-row "Terminal-1 (self)" appears at top with the
  "(self)" suffix; per-row checkboxes; "broadcast input on/off"
  label text verbatim; no umbrella rocker button.
* **-a-32**: three-surface parity (pane hamburger / empty-pane
  right-click / carousel slide 1) all show identical first-class
  spawn entries in identical order with the new chord hints.
  Cmd+K 1/2/3/4/p chord descriptors absent.
* **-a-33**: ancestor breadcrumb walks `drive / docs / agents`
  with intermediate hops as `<button class="crumb">` and the
  current segment as `<span aria-current="true">`. Clicking each
  re-scopes in place; old explicit "Graph from here" button gone
  from every inspector body in the graph.
* **-a-34**: HTML paste of `*bold* **strong** _em_ `code`
  [chan](url)` lands in cm-content with NO backslash escapes; all
  variants render via the `cm-md-*` decoration classes
  (italic / bold / code / link).
* **-a-35**: rename band sits above the editor body, escapes the
  `--chan-page-max-width` cap (band 1005 px vs editor-wrap 985 px),
  pre-fills the current path, commits via
  `Drive::rename_with_link_rewrite` on Enter (verified file
  renamed on disk + tab label + URL hash all updated), Esc
  cancels with no filesystem side effect.

### Side observation worth flagging

The `bandAboveEditor: false` reading I expected on -a-35 came out
because the band sits at the same top y as the editor-wrap (both
start at y=38). Empirically the band IS visible above the editor
body and the rename committed cleanly; the page-width-cap escape
(`width: 100%` + `flex: 1` outside the editor-wrap) is verified
by the band-width-greater-than-editor-wrap-width comparison.
Posting only in case you want to dig into the band-vs-editor
overlap stacking.

### Test server state

Server still live on `127.0.0.1:8787` with URL
`http://127.0.0.1:8787/?t=BbtnncpjBi7PmPsb3YnFxvfAcB9PPMbX`.
Drive `/tmp/chan-test-phase8-wa-r2/` (chan-source seed) has one
ad-hoc artifact `paste-test-renamed.md` (rename test landed) plus
`watcher-events/` with 5 fixture files. Will leave running for
@@Alex to click around if useful; tear down at the next recycle
signal.

Round-2 broader fan-out (carousel redesign, BOOT, manual, etc.)
parks until you cut the patch tag — standing by.


## 2026-05-21 — poke (webtest-a-2: v0.11.2 cut walkthrough complete; 1 regression + 1 usability gap)

Walked the lane-A v0.11.2 cut surfaces from
[`../webtest-a/webtest-a-2.md`](../webtest-a/webtest-a-2.md)
against HEAD `e7468db` (post-v0.11.2 close-out, docs-only on
top of `60901c1` chan-v0.11.2). Local build; DMG install path
skipped (out of lane-A perm scope per the task spec).

Per-task empirical signal under "2026-05-21 — v0.11.2 cut
walkthrough lane A" tail in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts

| Task    | Verdict                |
|---------|------------------------|
| -a-37   | MOSTLY HOLDS — pieces 1+2 clean; piece 3 not surfacing (see lowlight) |
| -a-38   | A NOT TESTED / B HOLDS |
| -a-39   | A FAILS / B HOLDS (Piece A is a real regression — see lowlight)       |
| -a-40   | FIX HOLDS              |
| -a-41   | FIX HOLDS              |
| -a-36   | NOT IN LANE-A (Tauri IPC; @@WebtestB) |
| -a-42   | DOCS-ONLY / Settings overlay walk deferred |

### Critical lowlights

* **`-a-39` Piece A regression**: per-tab FB expand-state persistence
  does NOT hold. Repro: 5 FB tabs in one pane, expand 4 dirs on one
  tab, switch tabs via click → every tab shows the SAME 4 expanded
  dirs. URL hash never serializes `be` for any tab. @@FullStackA's
  audit verdict on -a-39 explicitly anticipated this could happen:
  "If @@Alex still observes lost expand-state on the v0.11.2
  walkthrough — i.e., the symptom is real but the diagnosis
  mis-identified the failure mode — we'd need a live repro." This
  walk provides that repro. The "no-rename, no-new-field" deviation
  in -a-39 Piece A (kept existing `be` SerTab field, skipped the
  rename to `fbe`) is reproducing the under-fix. **Singleton-bleed
  symptom is the EXISTING behavior on main; -a-39 Piece A would be
  additive UX, not a bug-introducing regression**. Your call on
  v0.11.3 hotfix vs Round-2 wave-2 follow-up.
* **`-a-37` Piece 3 usability gap** (NOT regression-class): the
  inline "File seems to have moved to `<path>`" suggestion never
  surfaced in my move-to-subdir repro (basename preserved). Root
  cause looks like single-shot timing: `runSuggestReopenLookup`
  fires ONCE when `markFileMissing` runs; if the file lands at a
  basename-matching path AFTER that (or the indexer hasn't picked
  up the moved file yet), the suggestion never appears. Pieces 1
  and 2 are solid: the debounced recovery check auto-restores the
  editor when the file returns at its original path (better than
  the spec — no Re-open click needed). Re-open + fall-through-to-FB
  with "Choose the moved file" status hint is the documented
  failover and works.

### Highlights

* **`-a-40`**: Wysiwyg outline-style ordered-list markers render
  `3.1.` `3.2.` `3.3.` at nested depth (under `3. Third item`).
  Top-level markers unchanged. Clean.
* **`-a-41`**: source-mode auto-list continuation suppressed for
  both UL (`- bullet one` + Enter → `MARK_UL`, no `- ` prefix) and
  OL (`1. First item` + Enter → `MARK_OL`, no `2. ` prefix).
* **`-a-38` Piece B**: "Copied path" status auto-dismisses within
  3.5 s (gone at t≈3.5s and still gone at t=5.5s).
* **`-a-39` Piece B**: 3× Cmd+Alt+O from single FB → 3 new tabs.
  Select-threading propagates `bs:"docs/journals"` to every new
  tab.
* **`-a-37` recovery**: deleting a file out from under the editor
  then immediately recreating it at the same path → panel surfaces
  briefly then auto-dismisses when the recovery check (Piece 1)
  fires; editor swaps to the restored content. No Re-open click
  needed.

### Side observation — title fallback `Files N` not exercised

@@FullStackA's -a-39 Piece B journal note mentioned the `Files N`
title-fallback for "no drive context" / "two un-selected FBs side
by side". In my walk every Cmd+Alt+O spawn threaded the existing
tab's `bs:"docs/journals"` selection, so the new tabs fell back to
the dir-name convention (`journals/`) rather than the `Files N`
convention. Whether this is the intended user-facing behavior or a
gap worth flagging is your call.

### Side observation — chan-server-side `be` serialization

URL hash never carries `be` for any FB tab in my repro, even when
the active tab's tree visibly has 4 expanded dirs. The continuous
tracker effect at `FileBrowserSurface.svelte:142-150` is wrapped in
`untrack` (writes to `tab.expanded` don't propagate to the outer
`persistLayoutToHash` effect), so even if `tab.expanded` does
update reactively, the hash-write trigger doesn't fire. The
component-level snapshot/restore at lines 101-128 is the only path
that should kick the hash-persist, and empirically it doesn't seem
to swap the singleton on tab activation. Flagging as a possible
narrowing of the diagnosis — but the deeper triage is Lane-A's
implementer to do.

### State at end of walk

Server still live at `http://127.0.0.1:8787/?t=Bna2VZo7Lb2n4Lvct6srJKPg8PUbLt2A`
against `/tmp/chan-test-phase8-wa-r3/` (chan repo seed + a few
ad-hoc test artifacts under `subdir/`). Will tear down at recycle
signal or on poke. v0.11.1 carryovers (-a-32 / -a-33 / -a-34 /
-a-35) not retested this pass — last clean against v0.11.1 cut;
v0.11.2 commits don't touch those areas. Spot-check next pass if
you want them confirmed against the v0.11.2 binary.
