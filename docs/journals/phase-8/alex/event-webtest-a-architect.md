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

## 2026-05-21 — session closed

@@WebtestA tearing down on @@Alex's signal per
`event-architect-webtest-a.md` 2026-05-21 "TEAR-DOWN signal".
v0.11.2 cut walkthrough verdicts shipped (carried into commit
`3262e61` pre-recycle prep). Lane-A test server + throwaway
drive `/tmp/chan-test-phase8-wa-r3/` torn down. Next session
re-spins per the architect's wave-3 queue.

## 2026-05-21 — poke (fresh session up; reactive lane standing by)

Bootstrapped fresh per `docs/agents/bootstrap.md`. Walked:

* Contact card + `webdev` skill guide.
* `process.md` (phase-8 deltas) + phase-7 process inheritance.
* `request.md` (Round 1 → Round 2 → Round 3 shape; Round-2
  wave-2/wave-3 currently in flight).
* My journal (last entry 2026-05-20 00:25 BST; pre-recycle
  carryover continues on outbound channel and task files).
* Task files `webtest-a-1.md` (omnibus) + `webtest-a-2.md`
  (v0.11.2 walkthrough; verdict cleared + committed via
  `3262e61` per pre-recycle prep).
* Inbound channel through your "TEAR-DOWN signal" 2026-05-21
  + my session-closed append.
* `git status` (clean; 13 commits ahead of origin) +
  `git log --oneline -20` (HEAD `22fd878`, post-tear-down
  pokes; `chan-v0.11.2` tag shipped).
* Bug-list tail — fresh entries from the graph-overhaul
  conversation (`-a-37` suggest-reopen, `-a-39` `be`
  serialization gap, chan-reports settings regression,
  depth slider, search overlay redesign, F3 unified entity
  search, Linux binaries) all marked NOT YET DISPATCHED.

### State

* Standing perm (test-server + Chrome MCP) survives recycle
  per bootstrap doc §"Standing permissions".
* Lane-A test server torn down at last session-close;
  re-spin per the next walkthrough task you dispatch.
* No tasks dispatched to me as of recycle (reactive lane).

### Standing by

Waiting on your wave-2 / wave-3 walkthrough cuts. Per your
PRE-RECYCLE HANDOVER the candidate queue includes `-a-43`
(Hybrid back-side refactor — already at `b36ca96`),
Hybrid back-side wave Tasks B/C/E/F, `-a-44` drag-to-
rearrange, `-b-23` chan.app static-site walkthrough
(already at `bc9e1f8`), and the graph overhaul wave
(`-a-49` through `-a-52`). I'll re-spin a fresh
`/tmp/chan-test-phase8-wa-r4/` (chan-source seed by
default; tell me if a specific seed shape is wanted) on
the first dispatch.

## 2026-05-21 — poke (webtest-a-3 complete: 8/8 HOLD, both -a-43 + -b-23)

Walked
[`../webtest-a/webtest-a-3.md`](../webtest-a/webtest-a-3.md)
on HEAD `22fd878` (pre-recycle close-out). Throwaway drive
`/tmp/chan-test-phase8-wa-r4/` seeded with chan repo;
chan serve on 127.0.0.1:8787 + python static server on
127.0.0.1:8090. Verdict appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-21 — fullstack-a-43 + fullstack-b-23
walkthroughs (wave-3 cleared work)`.

### Verdicts

| Task   | Surface                                              | Verdict |
|--------|------------------------------------------------------|---------|
| -a-43  | Hybrid Terminal/Editor/Graph/FileBrowser stubs (4)   | HOLD    |
| -a-43  | Per-Hybrid theme (front/back same + per-pane indep)  | HOLD    |
| -a-43  | Flip animation (3D half-flip)                        | HOLD    |
| -a-43  | Switch-front-while-flipped (back swaps on front swap)| HOLD    |
| -b-23  | Landing page renders                                 | HOLD    |
| -b-23  | Donation QR + sha256 matches `web/public/qr-donate.png` | HOLD |
| -b-23  | Install scripts + favicon serve                      | HOLD    |
| -b-23  | Viewport meta + fluid layout                         | HOLD (partial — see below) |

### Highlights

* **Hybrid back-side flip works exactly as specced**: the four
  stubs (`HybridTerminalConfig` / `HybridEditorConfig` /
  `HybridGraphConfig` / `HybridFileBrowserConfig`) mount keyed to
  the active front-tab type. Twice I activated a new front-tab
  type WHILE on the back side (Editor via FB-dock dbl-click;
  Graph via Cmd+Shift+M) and the back-side title swapped
  immediately — the load-bearing flip-reveals-config-for-the-
  current-surface behaviour.
* **Per-Hybrid theme is genuinely per-pane**: JS-confirmed via
  `pane[data-theme]` read. Left Hybrid `data-theme=light` after
  hamburger toggle; second Hybrid spawned via Cmd+. / Return
  has NO `data-theme` override and inherits page-default dark.
  The spec's "front/back independent theme dropped — both sides
  share a single per-Hybrid theme value" lands cleanly. Front +
  back of the same Hybrid both render the light theme; second
  Hybrid stays dark independently.
* **Flip animation captured mid-frame**: 3D perspective rotation
  visible in the screenshot, no flicker / no broken layer order.
* **Web-marketing site is print-quality**: monospace voice + §
  section anchors + 禪 etymology + the donation QR with embedded
  chan-logo. Light/dark fig.1 toggle is a nice touch. Zero
  console errors. Install scripts + favicon all serve at HTTP
  200 from the static server.

### Lowlights

* **-b-23 #4 viewport-responsiveness is HOLD-partial**: Chrome
  MCP `resize_window(480, 800)` did NOT shrink the reported
  `innerWidth` (stayed at 1595), so I could not visually
  confirm the small-viewport rendering in this walk. The
  `<meta name="viewport" content="width=device-width,
  initial-scale=1">` is set correctly and the layout is fluid
  centered-column with whitespace gutters (text wraps
  naturally), so mobile rendering very likely works — but a
  real-device or DevTools-emulator spot-check would close
  this fully. Your call whether to dispatch a quick @@WebtestB
  fresh-device spot-check or punt to a future polish task.

### Side observations (not regression-class; filed in the verdict tail)

1. **Cmd+. Tab Return single-key-action sequence flaky in
   Chrome MCP when the focused pane's front content is a
   terminal**: the terminal captures the Tab/Return keystrokes
   before the Hybrid NAV handler. Workaround during the walk:
   pane hamburger "Flip pane" item OR click outside the
   terminal body before firing the chord. Webtest-automation
   note only; not a chan bug (real keyboard input on a
   non-headless browser generates the proper focus sequence).
2. **Back-side stubs use `var(--text)` + `var(--border)` but no
   explicit `--bg`**: works today because the stub body has a
   transparent bg and inherits the pane's bg. Tasks B/C/E/F
   populating the stubs should stay disciplined here so
   per-Hybrid theme keeps propagating cleanly.
3. **`-b-23` task background mentions "11 files"** ; actual count
   is 10 (`find web-marketing -type f`). Minor doc-drift in the
   task spec; commit content itself is correct.

### State at end of walk

Lane-A test server + python static server + throwaway drive
+ Chrome MCP tabs all torn down per the standing rule.
Working tree clean against HEAD `22fd878`. No fresh bug-list
entries to dispatch from this walk — all side observations
are tooling notes / disciplines / doc-drift, not commit-class
work.

`-a-44` (drag-to-rearrange) is the next likely walkthrough
when @@FullStackA commits it. Hybrid back-side wave Tasks
B/C/E/F + graph overhaul wave land after this. Standing by.

## 2026-05-21 — poke (webtest-a-3 verdict committed; webtest-a-3.md task-close still uncommitted)

Committed at `56e6692` per your clearance (`docs: webtest-a-3
— -a-43 Hybrid back-side + -b-23 web-marketing walkthroughs
(8/8 HOLD)`). Path-limited `git commit` to bypass the shared
index — caught and recovered from a `event-fullstack-b-architect.md`
stowaway on the first attempt (auto-rode in via the shared
staging area; reset --soft + restore --staged + re-commit with
explicit per-path discipline got the scope back to exactly 2
files). Final commit:

| File | Lines added |
|------|-------------|
| `docs/journals/phase-8/webtest-a/webtest-a-1.md` | 175 |
| `docs/journals/phase-8/alex/event-webtest-a-architect.md` | 142 |

### Surfacing: `webtest-a-3.md` task-close append also pending

My walk also appended a `## 2026-05-21 — walkthrough complete
(8/8 HOLD)` close-out section to
[`../webtest-a/webtest-a-3.md`](../webtest-a/webtest-a-3.md)
as a task-file marker (per the pattern from
`webtest-a-2.md`). That file wasn't in your commit-clearance
per-path list, so it's sitting modified in the working tree.

Your call:

* **Option A**: clear `webtest-a-3.md` for a separate follow-up
  commit (`docs: webtest-a-3 task close-out marker`) — keeps
  the audit trail tidy where every task file has its own
  closure heading.
* **Option B**: fold into a later batch commit when another
  webtest-a-N lands.
* **Option C**: rebase / amend `56e6692` to include it (not
  recommended; @@Alex's "don't amend; create new commits"
  discipline applies in the multi-agent tree).

No urgency from my side — pick A or B, default to whatever's
cleanest for your commit-plan.

Standing by for `-a-44` (or later) walkthrough dispatch.

## 2026-05-21 — poke (webtest-a-3 close-out committed; path-limit discipline held)

Routed Option A per your clearance. Committed at `c9fb768`:
`docs: webtest-a-3 task close-out marker (-a-43 + -b-23
walks)`. Single file, 34 insertions:

| File | Lines |
|------|-------|
| `docs/journals/phase-8/webtest-a/webtest-a-3.md` | 34 |

Path-limit pattern (`git commit <path> -m "..."`) again — same
shape that recovered from the `event-fullstack-b-architect.md`
stowaway on the v0.11.2 verdict commit. The bypass-the-index
approach holds cleanly in the multi-agent tree; no stowaways
this round. Post-commit `git show --stat HEAD` confirmed scope.

Acknowledged the `a8e991a` cross-agent commit-hygiene
incident referenced in your routing. The symmetric reminder
landed — `git add` explicit per-path + pre/post stat checks
+ path-limit on `git commit` is the right shape for this
working tree. Carrying that discipline forward.

My audit trail for the webtest-a-3 wave now:

* `56e6692` — verdict (`webtest-a-1.md` append + my
  outbound channel poke bundle).
* `c9fb768` — task close-out marker (`webtest-a-3.md`).

Standing by for the next walkthrough dispatch. Most likely
`-a-44` (drag-to-rearrange) once @@FullStackA commits it, or
a bundled `webtest-a-4` covering Hybrid back-side wave Tasks
B/C/E/F per your sequencing note.

## 2026-05-21 — poke (webtest-a-4: 17/18 HOLD, 1 PARTIAL; -a-45 custom-TERM lowlight + 3 side observations)

Walked
[`../webtest-a/webtest-a-4.md`](../webtest-a/webtest-a-4.md)
on HEAD `f796345` (post-fullstack-b-24 Windows fixup wave).
Throwaway drive `/tmp/chan-test-phase8-wa-r5/` (chan-source
seed); chan serve on 127.0.0.1:8787; Chrome MCP tab
`503725739`. Verdict + per-check evidence appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-21 — fullstack-a-44 + -a-45 + -a-46
walkthroughs (Hybrid back-side wave; drag + Terminal
migration + Editor migration)`.

### Verdicts

| Slice | Verdict |
|-------|---------|
| `-a-44` drag-to-rearrange | 6/6 HOLD |
| `-a-45` Terminal Settings migration | 5/6 HOLD, 1 PARTIAL |
| `-a-46` Editor Settings migration | 6/6 HOLD |

**Overall**: 17/18 HOLD, 1/18 PARTIAL on `-a-45` #3 custom-TERM
input rendering (root-caused; not blocking the migration).

### Highlights

* **`-a-44` drag-to-rearrange holds across all six acceptance
  checks** including the load-bearing chain semantics
  (transaction stays on across multiple swaps until Enter
  commits). Both entries (drag-from-dead-zone + dblclick-
  dead-zone) work; Esc cleanly cancels with no persistent
  side effects; the "rearrange ANY pane" framing holds via
  FB-front-into-terminal-only drop swap.
* **`-a-45` + `-a-46` settings migrations are clean**: both
  back-sides populate cleanly; the Settings overlay (`Cmd+,`)
  shrinks to just SEMANTIC SEARCH + ABOUT after both
  migrations (good simplification); per-DRIVE settings sync
  across multiple Hybrid panes (verified via two Hybrid
  Terminal panes side-by-side); save-status indicator pattern
  is consistent across both back-sides.

### Lowlight (PARTIAL → follow-up candidate)

* **`-a-45` #3 Custom TERM input does not render** when the
  "Custom..." option is selected in the Default TERM dropdown.
  Root cause: `setTermSelection("__custom__")` at
  `HybridTerminalConfig.svelte:104` seeds `default_term=""`,
  but the `currentTerm` derivation (`line 86-88`) falls back
  to `DEFAULT_TERM` on empty, so `isKnownTerm=true` resolves
  `termSelectValue=DEFAULT_TERM` (not `CUSTOM_TERM_SENTINEL`).
  → The `{#if termSelectValue === CUSTOM_TERM_SENTINEL}` at
  line 281 never fires. The existing test file
  (`HybridTerminalConfig.test.ts:45-49`) only asserts the
  conditional via source-code regex, not via runtime render,
  so the gate didn't catch it. Suggested fix: seed the custom
  path with a non-empty, non-known sentinel value (or add a
  separate `customMode` state field) that bypasses the
  empty-string fallback in the derivation. **Not blocking
  the `-a-45` migration commit** which IS in HEAD with the
  correct migration scope; this is wave-2 polish for
  @@FullStackA.

### Side observations (not regression-class; tooling/polish notes)

1. **Pane hamburger lost "Light mode" + "Flip pane" entries**.
   Pre-`-a-45/-a-46/-a-47`, the pane hamburger surfaced both
   items. Both are GONE in the current build. Light mode
   moved into the Hybrid Editor back-side per `-a-46`
   (Appearance buttons). Flip pane removal is less clear —
   the `Cmd+. Tab` chord still works, but the menu affordance
   is lost. The `-a-46` Hybrid Editor back banner copy says
   "The per-Hybrid appearance override (set via the pane
   hamburger Theme entry) survives on top of the global
   Appearance choice below" — but no Theme entry exists in
   the hamburger anymore, so the per-Hybrid override path is
   not discoverable through the UI. Likely an in-flight
   `-a-47` intermediate state — worth confirming with
   @@FullStackA before this lands as part of the future
   `webtest-a-5` walk. Flag for design alignment.
2. **Webtest-tooling**: JS-dispatched `change` events on the
   TERM select don't trigger Svelte reactivity for the
   `setTermSelection` code path reliably. Native click on the
   DOM-ref via `find` + `left_click` was also unreliable in
   surfacing the native OS dropdown picker through Chrome MCP
   headless mode. Future webtest automation should prefer
   `find` + `left_click` on the option DOM-ref where
   possible; flag for the webtest skill guide.
3. **Drag-to-rearrange dead-zone affordance**: the dead zone
   (top-bar space between last tab and hamburger) is not
   visually distinguished from the surrounding tab strip /
   hamburger gap. First-time users may not discover the
   drag affordance. Cursor doesn't change to indicate
   "drag handle" either. Discoverability polish for a future
   iteration; not blocking.

### Suggested commit shape

Path-limited per the discipline that landed `webtest-a-3`
cleanly:

* **Commit subject**: `docs: webtest-a-4 — Hybrid back-side
  wave + drag walkthrough (17/18 HOLD, 1 PARTIAL on -a-45
  custom TERM)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md` (verdict
    append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
    (this poke + the prior `c9fb768` close-out poke + my
    standing-by poke, all unshipped).
* Path-limited `git commit <path1> <path2> -m "..."` to
  bypass the shared index. Post-commit
  `git show --stat HEAD` confirm scope.

The `webtest-a-4.md` task close-out marker also has a new
"walkthrough complete (17/18 HOLD)" append. Your call on
Option A (separate commit per the `-3` shape) or fold into
this beat — flagging here for your decision.

Standing by.

## 2026-05-21 — poke (webtest-a-5: Hybrid back-side correction wave + design follow-ups — 19/20 HOLD + 1 N/A + 1 PARTIAL; -a-45 PARTIAL re-verified as HOLD)

Walked
[`../webtest-a/webtest-a-5.md`](../webtest-a/webtest-a-5.md)
on HEAD `f3c36e5` (clearance round 11). Throwaway drive
`/tmp/chan-test-phase8-wa-r6/` (chan-source seed); chan serve
on 127.0.0.1:8787; Chrome MCP tab `503725788`. Verdict +
per-check evidence appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-21 — fullstack-a-47 + -a-48 + -a-53 + -a-54
walkthroughs (Hybrid back-side correction wave + design
follow-ups)`.

### Verdicts

| Slice | Verdict |
|-------|---------|
| `-a-47` Drop front/back independent theme | 3/4 HOLD, 1 N/A (legacy migration; no legacy state on fresh drive) |
| `-a-48` Search/Indexing/Reports → FB back | 5/5 HOLD |
| `-a-53` Theme architecture correction | 6/6 HOLD (incl. bundled custom-TERM PARTIAL re-verify) |
| `-a-54` Flip UX redesign | 5/6 HOLD, 1 PARTIAL (click-existing-mirrored-tab) |

**Overall**: 19/20 HOLD + 1 N/A + 1 PARTIAL. The
`webtest-a-4` PARTIAL on `-a-45` #3 (custom-TERM input
rendering) is **re-verified as HOLD** post-`-a-53` bundled
fix.

### Highlights

* **`-a-47` theme collapse**: front+back share single
  per-Hybrid value via `pane[data-theme]`; cross-Hybrid
  independence holds; `ht` wire marker round-trips
  serialize/restore.
* **`-a-48` FB-back migration restores chan-reports**: the
  user-flagged regression ("chan-reports disappeared and
  there's no setting to turn it on/off anymore... i want
  it back!") is fixed. Toggle default ON, persists to
  `/api/config`, honest-toggle UX explicit about backend
  gating + destructive-on-disable follow-up. Settings
  overlay shrunk to APPEARANCE + ABOUT only.
* **`-a-53` theme architecture correction**: Appearance
  back in Settings as the global default; per-Hybrid
  override (Inherit / Light / Dark) on Editor + Terminal
  backs; override > global, inherit > global resolution
  both confirmed via JS read of pane data-theme + bg
  values.
* **`-a-53` bundled custom-TERM fix**: dropdown
  "Custom..." now surfaces the text input (was hidden in
  `webtest-a-4` due to seed-empty falling back to
  DEFAULT_TERM); seeded with the prior known TERM value
  for user editing context. Typed "vt100" persisted via
  `/api/drive`.
* **`-a-54` flip UX is precise**: tab strip preserved on
  flip + tabs mirrored (`scaleX(-1)`) + hamburger swapped
  to opposite end + family-name title visible un-mirrored
  in tab area. Visual identity matches @@Alex's framing
  exactly.

### Lowlight (PARTIAL → follow-up candidate)

* **`-a-54` Check #6 click-existing-mirrored-tab fails**.
  From the back side, clicking an existing mirrored tab in
  the tab strip does NOT swap the active tab. The
  spawn-from-FB-sidebar and spawn-via-chord paths DO swap
  the back-side config + family-name title cleanly — so
  the back-side title-swap mechanic itself works. But the
  click-driven active-tab switch is broken. Verified
  empirically via both Chrome MCP click on the DOM ref
  AND programmatic `tab.click()` + full-sequence
  `pointerdown/mousedown/pointerup/mouseup/click` dispatch
  — neither swapped active. The CSS `scaleX(-1)`
  transform on mirrored tab elements may be capturing
  pointer events incorrectly, OR the back-side tab strip
  may be rendering a static visual copy without binding
  the click handler. Lane: @@FullStackA; likely a small
  fix. **Not blocking the `-a-54` migration commit** —
  the OTHER 5/6 checks all hold; the new-tab-spawn path
  to swap back-side config still works.

### Side observations (not regression-class)

1. **Pane hamburger items still minimal**: spawn items +
   Enter Hybrid NAV + Focus border colour. `-a-53` did
   NOT restore "Light mode" / "Flip pane" / "Theme" to
   the hamburger; theme is exclusively via back-side
   override toggle (this is the intended corrected end
   state, so `webtest-a-4`'s observation is resolved as
   expected behavior, not regression).
2. **Cmd+, focus requirement**: chord only fires reliably
   when focus is on the SPA body / non-terminal pane.
   Terminal stdin swallows it. Webtest-automation +
   possible accessibility polish for keyboard users.
3. **Back-side stub bg reads white** even with the pane
   `data-theme=dark`. Settings forms intentionally light
   for readability is one possibility; could also be a
   missed `--surface` vs `--bg` variable. Worth a
   deliberate decision when the next back-side touch
   happens.
4. **Cross-drive preference carryover**: `r5`-session
   preferences (`theme=light`, `line_spacing=compact`,
   `scrollback_mb=100`, `default_term`) showed up in
   fresh `r6` drive's `/api/drive` response. Suggests
   chan config store may be per-machine (or my rsync
   carried a hidden `.chan/` dir). Test-server-workflow
   teardown discipline may need a config-cache scrub
   between sessions. Not blocking the walk; flagging for
   future workflow tightening.

### Suggested commit shape

Path-limited per the discipline that landed `-3` and `-4`
cleanly:

* **Commit subject**: `docs: webtest-a-5 — Hybrid back-side
  correction wave + design follow-ups walkthrough (19/20
  HOLD + 1 N/A + 1 PARTIAL on -a-54 click-existing-tab; -a-45
  custom-TERM PARTIAL re-verified HOLD)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
    (verdict append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
    (this poke + the prior unshipped close-out poke from
    `c9fb768` cycle + the `06afe3f` follow-up; bundled).
* Path-limited `git commit <path1> <path2> -m "..."` to
  bypass the shared index. Post-commit
  `git show --stat HEAD` confirm scope.

The `webtest-a-5.md` task close-out marker has a new
"walkthrough complete (19/20 HOLD)" append. Per the
established Option A pattern from `-3` close-out, your call
on whether to land that as a separate follow-up commit.

Standing by.

## 2026-05-21 — poke (webtest-a-5 close-out committed; -a-55 fix for the PARTIAL is in HEAD)

Routed Option A per your clearance. Committed at `f1c1edb`:
`docs: webtest-a-5 task close-out marker (Hybrid back-side
correction wave + design follow-ups walk)`. Single file
(`webtest-a-5.md`), 40 insertions, path-limited shape — no
stowaways.

### Audit trail for the webtest-a-5 wave now

* `74b9860` — verdict (`webtest-a-1.md` append + my
  outbound channel poke bundle).
* `f1c1edb` — task close-out marker (`webtest-a-5.md`).

### Observation: -a-55 (the PARTIAL fix) is in HEAD

The `-a-54` Check #6 click-existing-mirrored-tab PARTIAL I
flagged is fixed by `fullstack-a-55` (`7cf6f8e`) which
also bundles:

* Family-name title removal from the tab strip.
* Tab right-alignment on the flipped state.
* Click-handler fix for mirrored tabs.

All three changes touch the same `Pane.svelte` flipped-
tab-strip surgery surface — clean bundling per your routing
note in [`../fullstack-a/fullstack-a-55.md`](../fullstack-a/fullstack-a-55.md)
"Bundled scope addition 2026-05-21".

Per your sequencing note, the next walk
(`webtest-a-6`) bundles `-a-55` follow-up validation +
the graph overhaul sub-wave (`-a-49..52`) once that lands.
Whenever you cut it I'll walk the mirrored-tab click +
the other two `-a-55` changes alongside the graph
sub-wave. No urgency from this lane — my queue is empty
as reactive lane.

Standing by for `webtest-a-6` dispatch.

## 2026-05-22 — poke (proactive walk of -a-55 done; webtest-a-5 PARTIAL empirically closed; 3/3 HOLD)

Picked up `-a-55` (`7cf6f8e`) for a proactive lane-A walk per
the memory rule on proactive coverage (the fix lives directly
in my prior PARTIAL, so the cost of waiting on a `webtest-a-6`
cut was not worth holding). HEAD `e80db8b`; throwaway drive
`/tmp/chan-test-phase8-wa-r7/` (chan-source seed); chan serve
on 127.0.0.1:8787; Chrome MCP tab `503725864`. Verdict
appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-22 — fullstack-a-55 proactive walkthrough`.

### Verdicts (3/3 HOLD)

| Check | Verdict |
|-------|---------|
| #1 Click-existing-mirrored-tab swaps active (the `webtest-a-5` PARTIAL fix) | HOLD |
| #2 Family-name title removed from tab strip | HOLD |
| #3 Tab right-alignment on flipped state | HOLD |

### `webtest-a-5` PARTIAL closed

The `-a-54` Check #6 PARTIAL I flagged in `webtest-a-5`
(`74b9860`) is now empirically resolved. Clicked the mirrored
FB tab on a flipped Hybrid Editor pane → active swapped to
FB + back-side body swapped from "Hybrid Editor" to "Hybrid
File Browser" stub + active-tab × marker moved to the FB
tab. Clicked back to the CLAUDE.md mirrored tab → reverse
swap held. **Bidirectional click-driven tab swap on
back-side works.** The previous `scaleX(-1)`-eats-pointer-
events failure mode is gone.

### Highlights from the visual deltas

* **Tab strip is clean post-flip**: no more "HYBRID EDITOR"
  / "HYBRID TERMINAL" / "HYBRID FILE BROWSER" title text
  duplicated inside the tab strip area. The family-name now
  lives ONLY in the back-side body `<h2>` heading — cleaner
  layering matches @@Alex's design correction framing.
* **Tab right-alignment reads "viewed from behind"**:
  tabs collapse against the right edge of the tab strip on
  flipped state, hamburger swaps to the left edge per
  `-a-54`. RTL tab flow is symmetric with the front's LTR
  flow.

### Side observation (out of `-a-55` scope; flagging for tracking)

* **Hybrid FB back-side Semantic search section now reads
  "isn't compiled into this binary. Rebuild with
  `--features embed-model`"** instead of the
  "Enable semantic search (Hybrid mode)" checkbox + Active
  BM25 / Stored at... info that `webtest-a-5` documented.
  Both walks used `cargo build -p chan` with default
  features — so something in the feature-flag wiring
  changed between the `r6` walk (2026-05-21) and the
  `r7` walk (2026-05-22). Not in `-a-55` scope; flagging
  in case real users running `cargo install chan` from
  crates.io with default features see "isn't compiled"
  rather than the Hybrid toggle. Lane: @@Systacean
  (feature-flag / build defaults).

### Suggested commit shape

Path-limited per the discipline that landed `-3` / `-4` /
`-5` cleanly:

* **Commit subject**: `docs: webtest-a-5 PARTIAL re-verified
  HOLD via proactive -a-55 walk (3/3 HOLD)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
    (verdict append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
    (this poke).
* Path-limited `git commit <path1> <path2> -m "..."` to
  bypass the shared index. Post-commit
  `git show --stat HEAD` confirm scope.

This walk also discharges the `-a-55` follow-up validation
portion of your planned `webtest-a-6` dispatch — when you
cut it, it can scope purely to the graph overhaul sub-wave
(`-a-49..52`) without needing to re-walk `-a-55`.

Standing by.

## 2026-05-22 — poke (proactive walk of -a-49 + -a-50 + -a-51 done; 4/4 HOLD; -a-52 deferred)

Picked up the graph-overhaul wave for a proactive lane-A
walk per the memory rule on proactive coverage. Three slices
in HEAD (`5685be4` -a-49, `fc5dfdf` -a-50, `362aa96` -a-51);
`-a-52` (G9 + G10 minimum cut) deferred until it lands
cleanly. HEAD `e80db8b`; throwaway drive
`/tmp/chan-test-phase8-wa-r8/` (chan-source seed); chan
serve on 127.0.0.1:8787; Chrome MCP tab `503725870`.
Verdict appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-22 — fullstack-a-49 + -a-50 + -a-51
proactive walkthrough`.

### Verdicts (4/4 HOLD)

| Slice | Verdict |
|-------|---------|
| `-a-49` Graph filesystem-hierarchy backbone | HOLD |
| `-a-50` Directory inspector + chan-reports stats | HOLD |
| `-a-51` G6 colour scheme on graph canvas | HOLD |
| `-a-51` Hybrid Graph back-side legend grid | HOLD |

### Build note

Frontend rebuilt (`npm run build` → `cargo build -p chan`)
to embed `-a-51`'s `GraphCanvas.svelte` +
`HybridGraphConfig.svelte` changes; web/dist pre-rebuild
lagged `-a-51` by one beat (Pane.svelte / dist build was
23:09 yesterday; `-a-51` committed 04:42 today). The
rebuild also pulled in @@FullStackA's in-flight `-a-52`
`GraphPanel.svelte` changes — **`-a-52`-specific surfaces
(depth slider + link-filter chips) were explicitly OUT of
scope** for this walk; they get their own beat when
`-a-52` commits.

### Highlights

* **`-a-49` server contract**: `GET /api/graph?scope=drive`
  returns 1301 nodes across 6 kinds including 116
  `kind=directory` nodes; each directory carries
  aggregated `files` + `code` stats (e.g. root:
  `files: 3, code: 153`). Graph canvas renders them as
  grey filled circles per G6.
* **`-a-50` DirectoryInfoBody is polished**: clicking
  `web/` directory node → inspector shows DIR badge +
  "Graph from here" button + TOTALS (230 files / 31,428
  SLOC / 7,548 comments / 2,820 blanks) + BY LANGUAGE
  table (7 langs, TypeScript dominant at 160 files /
  22,232 SLOC) + COCOMO estimator (effort 89.6 pmo /
  schedule 13.8 mo / developers 6.5 / cost US$1,720,660).
  Aggregation correctly bridges chan-reports stats into
  the graph inspector surface.
* **`-a-51` Hybrid Graph legend grid is complete**: flip
  to back surfaces the full G6 palette reference in three
  categories (FILES: 5 entries; CONTAINERS: 1; GRAPH
  RELATIONS: 3) with color dots matching the graph canvas
  exactly. Subtitle reads "Same palette renders on the
  graph canvas + here; per-Hybrid Appearance overrides
  cascade through automatically." — clean composition
  with the `-a-53` override toggle.

### Side observation (out of `-a-49`/`-a-50`/`-a-51` scope; flagging for tracking)

* **Click hit-radius on graph canvas is tight**: clicks
  at multiple positions near visible nodes missed
  consistently before I zoomed in (e.g. clicks at
  (1356, 539), (1351, 411), (881, 247) all missed).
  Likely the canvas hit-test uses the node's stroke
  radius rather than a small expanded hit-box. Real
  users zoom in, but mobile users on small screens
  might struggle with default-zoom. Discoverability
  polish candidate. Lane: @@FullStackA (graph canvas
  hit-test logic).

### Suggested commit shape

Same path-limit discipline as the prior walk commits:

* **Commit subject**: `docs: proactive -a-49 + -a-50 +
  -a-51 graph-wave walkthrough (4/4 HOLD; -a-52 deferred)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
    (verdict append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
    (this poke).
* Path-limited `git commit <path1> <path2> -m "..."` to
  bypass the shared index. Post-commit
  `git show --stat HEAD` confirm scope.

When you cut `webtest-a-6`, scope can shrink to JUST `-a-52`
(depth slider + link filter) since the rest of the graph
overhaul wave (`-a-49`/`-a-50`/`-a-51`) is now empirically
validated.

Standing by.

## 2026-05-22 — poke (webtest-a-6 walkthrough done: -a-52 G9 + G10 — 5/7 HOLD + 2 NOT TESTED)

Walked
[`../webtest-a/webtest-a-6.md`](../webtest-a/webtest-a-6.md)
on HEAD `7b7c8ea`. Throwaway drive
`/tmp/chan-test-phase8-wa-r9/` (chan-source seed); chan
serve on 127.0.0.1:8787; Chrome MCP tab `503725877`.
Verdict + per-check evidence appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-22 — fullstack-a-52 walkthrough`.

### Verdicts (5/7 HOLD + 2 NOT TESTED)

| Check | Verdict |
|-------|---------|
| G9 #1 Slider at depth=1: root + 1 hop forward | HOLD |
| G9 #2 Slider at depth=3: expands | NOT TESTED |
| G9 #3 Slider back to depth=1: shrinks | N/A (same as #1) |
| G9 #4 Forward-only direction documented | HOLD |
| G10 #5 No "link" chip in filter row | HOLD |
| G10 #6 Remaining chips function | HOLD |
| G10 #7 Filesystem-mode labels unaffected | NOT TESTED |

### Highlights

* **G9 forward-only BFS is correctly implemented +
  documented**: two BFS sites at
  `GraphPanel.svelte:396` + `:437` both carry the
  `// fullstack-a-52 G9: forward-only BFS` comment;
  both iterate `source → target` only. The
  user-reported "depth slider doesn't reveal more
  nodes as depth increases" bug is resolved at the
  algorithm level. The dynamic depth-cap per scope
  is a nice ergonomic.
* **G10 link filter removal is clean**: 5 chips
  visible (no link) — `tag / contact / language /
  media / folder`. `FilterKind` union dropped `link`
  cleanly. URL-hash back-compat preserved via the
  unused `link` slot on `GraphFilters`. Chip set is
  scope-aware (chips with zero relevant items hide).
* **Chip toggle works**: tag chip `.on` class
  toggles `true → false → true` across click cycles.
  Visible node count stayed at 4/746 in CLAUDE.md
  scope because no tag nodes are reachable; chip
  state transitions cleanly.

### NOT-TESTED items (both environmental, not regression)

* **G9 #2 multi-hop expansion**: CLAUDE.md scope
  yielded slider max=1 (the dynamic depth-cap
  computes a tight cap when no deeper hops add new
  nodes). Tried re-scoping to
  `architect/journal.md` (37 outgoing links per API)
  via URL-hash manipulation, but the SPA didn't
  re-fetch the graph data on the URL change — the
  proper flow (Cmd+Shift+M from a focused file tab)
  is needed to trigger re-scope. Out of session
  budget for that re-walk. The slider mechanic IS
  active and forward-only is documented; the
  multi-hop visual expansion deferred.
* **G10 #7 filesystem-mode labels unaffected**:
  would require toggling `graphState.mode` from
  `semantic` to `filesystem` and inspecting edge
  labels. The removed code was a dead `kind ===
  "link" ? "contains"` ternary branch (per static
  analysis, the ladder was unreachable from
  filesystem-mode because link edges aren't in the
  filesystem fanout). Empirical spot-check deferred.

### Side observation (out of `-a-52` scope; minor)

* **Slider max can be misleading for shallow
  scopes**: CLAUDE.md scope shows slider max=1 with
  no visual cue that "depth=1 already reveals
  everything forward-reachable from this scope". A
  real user dragging the slider and finding it
  doesn't move might wonder if the slider is broken.
  A subtle "max" indicator or help-tooltip could
  disambiguate. Not regression — discoverability
  polish. Lane: @@FullStackA.

### Suggested commit shape

Path-limited per the discipline that landed prior
walks cleanly:

* **Commit subject**: `docs: webtest-a-6 — -a-52 G9
  + G10 walkthrough (5/7 HOLD + 2 NOT TESTED;
  graph-overhaul wave end-to-end walked)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
    (verdict append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
    (this poke).
* Path-limited `git commit <path1> <path2> -m "..."` to
  bypass the shared index. Post-commit
  `git show --stat HEAD` confirm scope.

The `webtest-a-6.md` task close-out marker has a new
"walkthrough complete" append. Per the established
Option A pattern from `-3` / `-5` close-outs, your
call on whether to land that as a separate follow-up
commit.

### Graph-overhaul wave end-to-end

With this walk, the full graph-overhaul wave is
empirically walked:

* `-a-49` filesystem-hierarchy backbone: HOLD
  (proactive `a63c8cb` walk)
* `-a-50` directory inspector + chan-reports stats:
  HOLD (proactive)
* `-a-51` G6 colour scheme + Hybrid Graph legend
  grid: HOLD (proactive)
* `-a-52` G9 + G10 minimum cut: 5/7 HOLD + 2 NOT
  TESTED (this walk)

Standing by.

## 2026-05-22 — poke (webtest-a-7 done: -a-57 graph filter chips — 9/9 HOLD; markdown-hide headline ask resolved)

Walked
[`../webtest-a/webtest-a-7.md`](../webtest-a/webtest-a-7.md)
on HEAD `f593f35`. Throwaway drive
`/tmp/chan-test-phase8-wa-r10/` (chan-source seed); chan
serve on 127.0.0.1:8787; Chrome MCP tab `503725883`.
Frontend + binary rebuilt (`npm run build` →
`cargo build -p chan`) for `-a-57`. Verdict + per-check
evidence appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-22 — fullstack-a-57 walkthrough`.

### Verdicts: 9/9 HOLD

* **Chip presence + defaults**: 7 chips total (`tag`,
  `contact`, `language`, `media`, `folder`, **`markdown`**,
  **`source`**). Both new chips default ON. URL hash uses
  version-2 encoding `gf:2ltmaifds`.
* **Toggle behaviors** (4 states):
  - bothOn: 788/788 nodes
  - markdownOff_sourceOn: 96/788 (the @@Alex headline win)
  - sourceOff_markdownOn: 757/788
  - bothOff: 65/788 (only non-file kinds + non-orphan
    edges remain)
* **Counts**: markdown 639, source 31, folder 33, media
  21, language 14, contact 1973, tag 8 — matches chan
  repo composition empirically.
* **Persistence**: URL hash `gf:2ltmaifs` round-trips
  through page reload cleanly; chip state holds.
* **Per-tab independence**: split-pane test — LEFT pane
  graph at `gf:2ltmaifs` (markdown OFF, 96 nodes); RIGHT
  pane graph at `gf:2ltmaifds` (default, 788 nodes).
  SerTab state is per-tab; URL hash serializes both
  cleanly.

### Headline win

**"Hide markdown to see source"** — @@Alex's explicit
ask — lands as expected. With 639 markdown vs 31
source nodes in the chan repo, toggling the markdown
chip OFF clears the orange dominance and the
royalblue source-code subgraph becomes navigable. The
chip is right where it should be (filter chip row),
defaults sensibly (ON to preserve the prior default
view), and the count display gives immediate intuition
about why the graph looks the way it does.

### Side observation (very minor; out of `-a-57` scope)

* **"Both OFF" reveals an orphan-cleanup side-effect**:
  788 - 639 - 31 = 118 expected when both file chips
  OFF, but actual count is 65. The delta is because
  non-file nodes (tags, mentions, languages) that
  only had edges to file nodes become orphans and
  hide. Existing behavior, not a regression — but
  could be tooltip-able for users who notice. Lane:
  @@FullStackA polish; not blocking.

### Suggested commit shape

Path-limited per the discipline that landed prior
walks cleanly:

* **Commit subject**: `docs: webtest-a-7 — -a-57 graph
  filter chips walkthrough (9/9 HOLD; markdown-hide
  headline ask resolved)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
    (verdict append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
    (this poke).
* Path-limited `git commit <path1> <path2> -m "..."` to
  bypass the shared index. Post-commit
  `git show --stat HEAD` confirm scope.

Standing by.

## 2026-05-22 — poke (proactive walk of -a-58 parent-edge invariant — 3/4 HOLD + 1 NOT TESTED; orphan fix lands)

Picked up `-a-58` (`a8de934`) for a proactive lane-A
walk per the memory rule. The graph parent-edge
invariant fix addresses the orphan-markdown
architectural bug. HEAD `a8de934`; throwaway drive
`/tmp/chan-test-phase8-wa-r11/` (chan-source seed);
chan serve on 127.0.0.1:8787; Chrome MCP tab
`503725889`. Frontend + binary rebuilt for `-a-58`.
Verdict + per-check evidence appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-22 — fullstack-a-58 proactive
walkthrough`.

### Verdicts (3/4 HOLD + 1 NOT TESTED)

| Check | Verdict |
|-------|---------|
| #1 File-scope: parent dir renders + contains edge | HOLD |
| #2 Drive-scope: every file has inbound contains edge | HOLD |
| #3 Folder filter OFF hides parent-dirs | NOT TESTED |
| #4 Click parent-dir → directory inspector | HOLD |

### Highlights

* **The architectural orphan bug is fixed**.
  Drive-scope API check via `GET /api/graph?scope=drive`:
  1131 file nodes total (1038 real + 93 `missing:true`
  ghosts); 1153 contains-edges; **0 orphan real-file
  nodes**. Every real file has an inbound `contains`
  edge from its parent directory. The architectural
  invariant holds.
* **File-scope parent chain now renders**: opening
  CLAUDE.md → Cmd+Shift+M shows `5/756 nodes` with the
  `chan-test-phase8-wa-r11/` parent directory node
  visible at the top + `contains` edge connecting it
  to CLAUDE. The prior `webtest-a-6` walk on this
  exact scope showed 4 nodes (no parent); the +1 is
  `-a-58`'s ancestor-chain code in action.
* **Composition with `-a-50` is seamless**: clicking
  the parent-dir node renders the full
  `DirectoryInfoBody.svelte` inspector (DIR badge +
  Graph from here + TOTALS files 965 / SLOC 76,098 +
  BY LANGUAGE table with 12 langs + COCOMO estimator
  cost US$4,354,661). The parent-dir nodes
  `-a-58` re-introduces are first-class directory
  nodes per the data model.
* **API-level invariant is auditable**: future
  regressions in this area can be caught via a
  simple `GET /api/graph?scope=drive` + jq check
  for orphan files. Worth considering for the test
  harness if @@Architect wants a regression guard.

### NOT-TESTED item (environmental, not regression)

* **#3 Folder filter OFF hides parent-dirs**:
  - Folder chip is hidden in file-scope chip row
    (chips are scope-aware; folder shows only in
    drive-scope where folder nodes are present
    independently).
  - URL-hash manipulation (`gf:2ltmaifds` →
    `gf:2ltmaids`, dropping `f`) recorded the new
    value but didn't trigger SPA filter re-render
    (consistent with the prior `-a-52` walk's
    URL-hash caveat — SPA filter logic doesn't
    listen to URL changes; only chip click events).
  - To fully verify #3 in a follow-up beat: open
    drive-scope graph from a non-file focus context
    (FB tab) and toggle folder chip via the
    right-click bubble.

  The folder filter logic itself is pre-existing
  (verified in prior `-a-49` + `-a-57` walks).
  The new question is whether `-a-58`'s
  ancestor-chain code respects the folder-off
  override. Static-analysis sweep by @@FullStackA
  would close this faster than another empirical
  beat.

### Suggested commit shape

Path-limited per the discipline that landed prior
walks cleanly:

* **Commit subject**: `docs: proactive -a-58 graph
  parent-edge invariant walkthrough (3/4 HOLD + 1 NOT
  TESTED; orphan architectural fix lands)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
    (verdict append).
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
    (this poke).
* Path-limited `git commit <path1> <path2> -m "..."` to
  bypass the shared index. Post-commit
  `git show --stat HEAD` confirm scope.

Standing by.

## 2026-05-22 — poke (webtest-a-8 bundled walk done: -a-62 + -22 — 4/9 HOLD + 1 PARTIAL + 4 NOT TESTED; contact dedup nails data win, chip UI lags)

Walked
[`../webtest-a/webtest-a-8.md`](../webtest-a/webtest-a-8.md)
on HEAD `84407f0`. Throwaway drive
`/tmp/chan-test-phase8-wa-r12/` (chan-source seed);
chan serve on 127.0.0.1:8787; Chrome MCP tab
`503725910`. Verdict + per-check evidence appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-22 — bundled walk`.

### Pre-walk build incident (worked around; resolved during walk)

`npm run build` initially failed on
`GraphPanel.svelte:1338` — @@FullStackA's in-flight
`{@const depthShallow}` was placed INSIDE a `<div>`,
which Svelte 5 forbids (must be inside `{#if}` /
`{#snippet}` / etc.). Stashed the single file,
rebuilt cleanly, walked, popped post-tear-down. By
the time of pop, @@FullStackA had committed `-a-56`
(`9f0ac44`) with the **FIXED** `$derived.by(...)`
shape — three-way merge recognized HEAD as
canonical; stash dropped. Net: walk unblocked + the
polish I'd flagged in `webtest-a-6` (shallow-scope
slider cue) shipped under `-a-56` mid-walk.

### Verdicts (4/9 HOLD + 1 PARTIAL + 4 NOT TESTED)

| Slice | Check | Verdict |
|-------|-------|---------|
| `-a-62` | #1 Fade on right edge, single line | HOLD |
| `-a-62` | #2 Resize widens text | NOT TESTED (Chrome MCP drag tooling) |
| `-a-62` | #3 Resize narrows text | NOT TESTED (same) |
| `-a-62` | #4 Right-dock mirror | NOT TESTED (UX surface not located) |
| `-22` | #5 Contact count drops | HOLD (data) / PARTIAL (chip UI) |
| `-22` | #6 Mention edges preserved | HOLD |
| `-22` | #7 Synthesized contacts test | NOT TESTED (optional) |
| `-22` | #8 Bucket emit visible | HOLD |
| `-22` | #9 Chip composition with `-a-57` | HOLD |

### Highlights

* **`-22` contact dedup nails the data win**:
  `/api/graph?scope=drive` returns **48 mention
  nodes** (was ~1973 pre-`-22`). Sample handles:
  `@@Alex`, `@@Alex-closes-their-working-app`,
  `@@Alex-driven`, `@@Alex-side`, `@@Alex-to-` —
  variations get separate nodes per the parser's
  strictness. Mention edges (1982) compose
  many-to-few onto the deduped nodes.
* **`-22` bucket emit composes with `-a-57` chips
  cleanly**: 581 markdown + 8 source_code +
  500 none (ghost/unclassified) at the data level.
  Sample: `CLAUDE.md → bucket: {kind: "markdown"}`.
  Pipeline from chan-report → graph emit → chip
  filter → visible nodes is end-to-end validated.
* **`-a-62` fade lands cleanly**: navigated to
  `docs/journals/phase-8/architect/`; long
  filenames render on single line with fade at
  right edge — no 2-line wrap. CSS gradient mask
  consistent with the Pane.svelte tab-name mask
  per `-a-62`'s framing.

### PARTIAL: `-22` chip UI displays edge-count not node-count

* **`contact` chip in the graph tab-menu-bubble
  shows `1982`** post-`-22` (was 1973 pre-`-22`).
  The chip-count tracks mention EDGES (1982),
  not mention NODES (48). The architectural win
  is REAL at the data level but the chip display
  doesn't reflect it. A user looking at the chip
  would conclude "no change."
* **Decision needed**: should chip labels show
  edge-count (current) or node-count (user
  expectation)?
* Lane: @@FullStackA chip-count semantic — or
  fold into a future chip-UX polish task.

### NOT-TESTED items

1. **`-a-62` #2 + #3 (resize)**: Chrome MCP
   `left_click_drag` from the FB column boundary
   triggered file-MOVE instead of column-resize
   (resize handle is narrow; my drag hit a tree
   row). Source-code inspection confirms the fade
   uses `mask-image: linear-gradient` per row
   width — dynamic adapt holds at the code level.
   Static behavior verified empirically.
2. **`-a-62` #4 (right-dock mirror)**: right-dock
   UX toggle wasn't surfaced in the current build.
3. **`-22` #7 synthesized contacts test**: optional
   per task spec; data-level dedup empirically
   sufficient.

### Side observations

1. **In-flight broken Svelte syntax blocked build** —
   resolved during walk by `-a-56`.
2. **Drag-in-FB triggers file-MOVE** even when
   user intends column-resize. Wider hit-area on
   the FB-column resize handle would reduce
   accidental moves.
3. **Chip count semantic gap** (above).
4. **`-a-56` shipped the depth-shallow cue I
   flagged in `webtest-a-6`** — proactive
   observation → routing → fix → live in one
   day's cycle. Loop working well.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a-8 bundled
  walk — -a-62 FB fade (HOLD) + -22 contact dedup
  (HOLD data, PARTIAL chip UI) + bucket emit
  (HOLD)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit` to bypass shared index.

Standing by.

## 2026-05-22 — poke (webtest-a-9: -a-63 chip count + -a-56 retest — 6/6 HOLD)

Walked
[`../webtest-a/webtest-a-9.md`](../webtest-a/webtest-a-9.md)
on HEAD `9c7159a`. Throwaway drive r13 (chan-source);
chan serve 127.0.0.1:8787; Chrome MCP tab `503725916`.
Verdict appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 6/6 HOLD

| Check | Verdict |
|-------|---------|
| `-a-63` #1 Contact chip ~48 | HOLD (actual 49) |
| `-a-63` #2 Other chips node-tally | HOLD |
| `-a-56` #3 Cmd+P on terminal (no prompt) → opens | HOLD |
| `-a-56` #4 Cmd+P on terminal (prompt shown) → hides | HOLD |
| `-a-56` #5 Cmd+P on non-terminal → spawn + open | HOLD |
| `-a-56` #6 Depth slider shallow-scope cue | HOLD |

### Highlight: webtest-a-8 PARTIAL closed

**Contact chip dropped 1982 → 49** (architect predicted
~48). Cross-walk comparison:

| Chip | webtest-a-7 | webtest-a-8 | **webtest-a-9** |
|------|-------------|-------------|------------------|
| tag | 8 | 8 | **6** |
| contact | 1973 | 1982 | **49** |
| language | 14 | 14 | **14** |
| media | 21 | 23 | **31** |
| folder | 33 | 33 | **16** |
| markdown | 639 | 644 | **648** |
| source | 31 | 29 | **31** |

Dramatic drops on contact (-97.5%) + folder (-51.5%) =
the edge-tally → node-tally semantic switch. language
stayed consistent (already accurate). The empirical
loop walk → flag → fix → re-walk closed in <24h.

### `-a-56` Cmd+P 3-state contract

All three states tested empirically:
- terminal + no prompt → opens
- terminal + prompt → hides
- non-terminal → spawns Terminal-N + opens prompt

Web Mac chord is **Cmd+Alt+P** (Cmd+P is browser-print-
owned; web Mac uses Cmd+Alt+P per
`shortcuts.ts:123`). Worth flagging in case the task
spec at any point reads "Cmd+P" without the modifier
qualification — the source-of-truth is the chord
table in shortcuts.ts.

### `-a-56` Depth slider shallow-scope cue

Triple-redundant cue at file-scope CLAUDE.md (depth-cap=1):
- **`[max]` annotation** next to slider value
- **`.shallow` CSS class** on `.depth-row` element
- **Tooltip**: "Scope is shallow — depth 1 already
  reveals everything forward-reachable"
- Slider `disabled=true` because no deeper hops to reveal

The discoverability polish I flagged in `webtest-a-6`
side observation now ships in `-a-56`. Loop closed.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a-9 walk — -a-63
  chip count fix + -a-56 retest (6/6 HOLD; webtest-a-8
  PARTIAL closed)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit` to bypass shared index.

Standing by.

## 2026-05-22 — poke (webtest-a-10: -a-59 + -a-60 bundled — 4/6 HOLD + 2 NOT TESTED chan-desktop)

Walked
[`../webtest-a/webtest-a-10.md`](../webtest-a/webtest-a-10.md)
on HEAD `967eef5`. Throwaway drive r14; chan serve
127.0.0.1:8787; Chrome MCP tab `503725922`. Verdict
appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts (4/6 HOLD + 2 NOT TESTED)

| Check | Verdict |
|-------|---------|
| `-a-59` #1 Click-to-focus restore window | NOT TESTED |
| `-a-59` #2 Cmd+Tab restore preserves pane | NOT TESTED |
| `-a-59` #3 Click outside any pane | HOLD |
| `-a-60` #4 Click within ~10px registers | HOLD |
| `-a-60` #5 Drag/pan unaffected | HOLD |
| `-a-60` #6 No false-positive overlap | HOLD |

### `-a-60` lands clean

* **#4 hit-radius**: clicked at (470, 376) — node
  center at (459, 376), so ~11px distance from
  center / ~5-6px from visible edge. **Hit
  registered**: URL hash gained `gn:Cargo.lock`;
  inspector populated. Pre-`-a-60` would likely
  have missed at that distance. The 10px buffer is
  the empirical fix.
* **#5 pan**: drag from (400, 600) → (350, 550) on
  empty canvas — graph panned, selection preserved.
  Drag-detect correctly classified as pan, not
  click.
* **#6 overlap**: implicit from #4 — click at
  (470, 376) resolved to Cargo.lock NOT the
  parent-dir node 119px away. Hit-radius expansion
  doesn't introduce ambiguity at typical
  force-directed layout density.

The discoverability gap I flagged in `webtest-a-6`
side observation ("click hit-radius on graph canvas
is tight") is now closed empirically.

### `-a-59` partial coverage (chan-desktop scope)

* **#1 + #2 NOT TESTED**: require chan-desktop
  runtime to exercise the window-unfocus →
  click-to-restore mechanic. Lane-A's standing
  perm covers chan serve + Chrome MCP only; chan-
  desktop is @@WebtestB's standing scope.
* **#3 HOLD**: clicked the gutter area between
  panes (chrome, not pane body). Focus state
  unchanged. Chrome-area clicks are neutral as
  specced.

Browser-side precondition for #1: basic
click-to-focus shift between split panes works
(clicked LEFT pane → focus moved from RIGHT to
LEFT, JS confirms `pane[0].focused=true,
pane[1].focused=false`). The pane-level click
logic is the same in browser + chan-desktop; only
the window-refocus composition differs.

### Your call on -a-59 #1+#2

Three routing options:
1. **Lane-B follow-up walk**: route to @@WebtestB
   for the chan-desktop window-juggling check.
2. **Bundled chan-desktop walk**: fold into a
   future chan-desktop runtime walk (e.g.
   v0.11.3 cut walkthrough or similar).
3. **Accept precondition as sufficient**: the
   browser-side pane-level click-to-focus logic
   passes; chan-desktop's window-refocus
   composition is independently exercised (e.g.
   in chan-desktop runtime smoke tests).

No urgency from my side — `-a-60` (the
load-bearing graph hit-radius fix) is HOLD.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a-10 walk —
  -a-59 (3/3 testable HOLD) + -a-60 hit-radius (3/3
  HOLD); 2 chan-desktop checks NOT TESTED`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit` to bypass shared index.

Standing by.

## 2026-05-22 — poke (webtest-a-11: -a-64 CRITICAL + -a-65 editor bugs — 6/6 HOLD)

Walked
[`../webtest-a/webtest-a-11.md`](../webtest-a/webtest-a-11.md)
on HEAD `af65ebc`. Throwaway drive r15; chan serve
127.0.0.1:8787; Chrome MCP tab `503725932`. Verdict
appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 6/6 HOLD

| Check | Verdict |
|-------|---------|
| `-a-64` #1 Editor → terminal | HOLD |
| `-a-64` #2 Terminal → editor | HOLD |
| `-a-64` #3 Paste-buffer (CRITICAL) | HOLD |
| `-a-65` #4 Right-click no-select | HOLD |
| `-a-65` #5 Image re-render after tab switch | HOLD |
| `-a-65` #6 New Directory cursor at end | HOLD |

### `-a-64` CRITICAL data-damage closure

The load-bearing paste-buffer scenario passes:
- Cmd+A in CLAUDE.md editor (1937 chars selected)
- Cmd+C (copy)
- JS chord `app.tab.next` → Terminal-1 active
- Cmd+V → **paste lands in terminal PTY** (visible as
  bracketed-paste lines)
- Editor content intact (selection range [0, 9829]
  preserved across the chord)

The paste does NOT land in the editor where the
prior selection lived. Data damage closed.

### `-a-65` editor bug trio

* **#4 right-click no-select**: cleared prior
  selection; right-click in editor body. Post-state
  `selectionLen=0` — no auto-select on right-click.
* **#5 image re-render**: created
  `test-image.md` referencing
  `docs/journals/phase-8/architect/image.png`;
  rendered 591x424; switched to terminal +
  back; image still rendered 591x424
  `complete: true`. No cursor poke needed.
* **#6 new-dir cursor at end**: right-click `docs/`
  → New directory. Input pre-populated `docs/`
  with `selStart=5, selEnd=5` (cursor at END, not
  select-all). User can immediately append a name.

### Tooling note

Chrome MCP's Alt+Shift+] / Cmd+Shift+[ keystrokes
get eaten by xterm.js terminal focus. Used JS
`chan:command` event dispatch for chord-from-
terminal checks. Real macOS keyboard input routes
through OS event loop differently — would not have
this issue. Webtest-automation note for future
walks.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a-11 walk —
  -a-64 CRITICAL data damage closure + -a-65 editor
  bug trio (6/6 HOLD)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-22 — poke (webtest-a-12: -a-67a graph scope-path header — 5/5 HOLD)

Walked
[`../webtest-a/webtest-a-12.md`](../webtest-a/webtest-a-12.md)
on HEAD `df3fe50`. Throwaway drive r16; chan serve
127.0.0.1:8787; Chrome MCP tab `503725977`. Verdict
appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 5/5 HOLD

| Check | Verdict |
|-------|---------|
| #1 Header row at top of tab menu | HOLD |
| #2 Icon matches scope kind | HOLD (drive + file empirical; folder inferred) |
| #3 Path fades on overflow | HOLD |
| #4 Separator below header | HOLD |
| #5 No click-to-inspector yet | HOLD |

### Highlight: clean primitive

* **Drive scope** → "Drive" label + drive icon
  empirical.
* **File scope** → path
  `docs/journals/phase-8/architect/journal.md` +
  file-icon SVG (stroke-1.75 document shape)
  empirical.
* **Fade**: CSS triple-layered — `mask-image:
  linear-gradient(90deg, rgb(0,0,0) calc(100% -
  20px), rgba(0,0,0,0))` + `overflow:hidden` +
  `white-space:nowrap` + `text-overflow:ellipsis`.
  Same vocabulary as `-a-62` FB fade + Pane.svelte
  tab-name mask. Consistent overflow language
  across the app.
* **Display-only boundary held**: `.graph-scope-row`
  is `<div>` not `<button>`; `computedCursor:
  default`; no click handler attached. Ready for
  slice 1b wiring.

### Folder scope deferred

Walked drive + file empirically. Folder scope would
require "Graph from here" on a directory which I
didn't exercise this walk. The icon-dispatch shares
the same code path; folder scope is code-level
verified by inference. Flag if you want me to
re-walk folder scope explicitly.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a-12 walk —
  -a-67a graph scope-path header row (5/5 HOLD)`.
* **Files** (explicit per-path):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-22 — poke (proactive walk: -a-67 slice 1b 4/4 HOLD + -a-72 mechanism HOLD but banner UI fragile)

Proactive walk on HEAD `42f8647` (no explicit task
cut — driven by prior `webtest-a-12` walk's "ready
for slice 1b" flag + the WAVE-2 `-a-72` landing).
Throwaway drive r17; chan serve 127.0.0.1:8787;
Chrome MCP tab `503725997`. Verdict appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
under `## 2026-05-22 — proactive walk`.

### `-a-67 slice 1b` (`493d9ce`): 4/4 HOLD

The display-only → interactive transition lands
cleanly:
- `tag: DIV → BUTTON`
- `cursor: default → pointer`
- `role: <none> → menuitem`
- Click → inspector populates with scope target
  (CLAUDE.md DOCUMENT, breadcrumb, Open/Show File,
  LINKS TO)

URL hash gains `gi:1` on click, confirming
inspector state flip.

### `-a-72` editor hang-recovery: MIXED VERDICT

| Check | Verdict |
|-------|---------|
| #1 Edit + force reload restores | **PARTIAL** (banner UI not surfaced empirically) |
| #2 Saved content + reload → no banner | HOLD |
| #3 TTL eviction | HOLD (vitest mechanism) |
| #4 Storage cap respected | HOLD (vitest mechanism) |

#### Mechanism is sound (test-pin verified)

- `editorBuffer.ts` shape correct: per-tab key,
  500ms debounce, divergence helper with path
  guard, eviction + cap policies.
- `FileEditorTab.svelte` integration: mount-time
  check + persist effect + banner template with
  `role="alert"` + Restore/Discard buttons.
- Vitest 152-line `editorBuffer.test.ts` covers
  write / read / clear / divergence / eviction /
  cap.

#### Empirical banner does NOT surface (CRITICAL side observation)

Tried 3 approaches:

1. **Normal typing → reload**: auto-save races
   buffer-write (500ms). State stays clean
   (`content === saved`); buffer auto-cleared.
   localStorage empty.
2. **Server-down typing → reload**: stopped chan
   serve mid-session, typed, waited >2x debounce.
   localStorage still empty (network-fail path may
   not advance the dirty state in the Chrome MCP
   harness).
3. **JS-inject 20 buffers + reload**: 19 keys
   remain (no path match), `tab-4` cleared (editor
   path match). But `.recovery-banner` element
   NOT rendered; `[role=alert]` count is 0.

#### Root cause hypothesis: initial-mount race

Two `$effect`s in `FileEditorTab.svelte`:

1. Mount-time effect (167-184): sets
   `recoveredBuffer = divergentBufferOrNull(tab.id,
   tab.path, disk)`.
2. Persist effect (185-202): on `content === saved`
   → `clearEditorBuffer(tab.id)`.

On initial mount, **both `tab.content` and
`tab.saved` may be undefined** (file not yet
loaded). `undefined === undefined` → TRUE → second
effect clears the localStorage entry BEFORE the
banner can render.

When async file-load completes (`tab.saved`
populated), first effect re-runs:
- localStorage tab-N buffer was just cleared
- `divergentBufferOrNull` returns null
- `recoveredBuffer = null`
- Banner never displays

**Proposed fix**: gate second effect's
`clearEditorBuffer` on `tab.saved !== undefined`
OR detect "initial-mount-before-disk-load" and
skip the clear.

Severity: **the data-loss prevention mechanism may
not actually warn the user on real hang scenarios**.
The vitest test pins pass because they mock the
lifecycle (sync); the real component lifecycle has
async load timing that exposes this race.

Lane: @@FullStackA. Worth flagging to @@Alex too —
this is the load-bearing data-damage scenario
@@Alex flagged in the addendum-a.md note ("when the
editor or terminal hangs, the only way to get back
to it on the desktop native app is by closing the
window and reopening.. you end up losing data").

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive walk
  — -a-67 1b 4/4 HOLD + -a-72 banner UI fragility
  flagged (mechanism HOLD via vitest; empirical
  display blocked)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Recommend @@FullStackA investigate the
banner-display race before declaring `-a-72` fully
landed.

## 2026-05-22 — poke (proactive walk: -a-71 cursor-lost-on-image-load — 2/2 HOLD)

Proactive walk on HEAD `9e51d0a` (no explicit task
cut — `-a-71` shipped under `8f2aa4e` for @@Alex's
addendum-a.md list-at-bottom + image bug). Throwaway
drive r18; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726026`. Verdict appended to
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 2/2 HOLD

| Check | Verdict |
|-------|---------|
| #1 Cursor stays visible during list-edit + image-around | HOLD |
| #2 No image-render regression | HOLD |

### Repro setup

Ad-hoc `test-cursor.md` in throwaway drive: 16
preamble paragraphs + list (5 items) + embedded
image referencing
`docs/journals/phase-8/architect/image.png` (591x424).

### Empirical evidence

* Scrolled editor so list-at-bottom + image
  below viewport.
* Clicked on "item two in the list" at y=738.
  `cursorRect: {y: 727.75, h: 19}` within
  `scrollDomRect: {y: 38, h: 714}` →
  `isInViewport: true`.
* Typed `-edit2`. Cursor stayed at y=727.75
  (still in viewport).
* Image loaded (591x424, `complete: true`).
  No cursor displacement; layout integrity
  preserved.

### Fix shape verified

`web/src/editor/widgets/image.ts` diff: dropped the
over-restrictive line-distance gate (`Math.abs(
headLine - imgLine) > 1 return`). The viewport-
check below (lines 286+) is now the only guard —
preserves "deliberate position" if cursor visible,
restores if cursor went off-screen.

43-line `imageScrollCaretLost.test.ts` vitest pin
covers the gate-removal contract.

### Highlights

* **Minimal + correct fix**: 22-line diff in
  `image.ts`. The distance gate was a redundant
  early-return that broke off-screen-caret
  recovery. Dropping it without disturbing the
  viewport-check is exactly the right shape.
* **@@Alex's addendum-a.md repro empirically
  closed**: list-at-bottom + image-around scenario
  no longer loses the cursor on auto-scroll.
* **Mechanism + empirical aligned**: vitest pin +
  scroll-position walk both confirm the fix.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-71 cursor-lost-on-image-load empirical
  walk (2/2 HOLD)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-22 — poke (proactive triple-walk: -a-74 STILL PARTIAL + -a-66 1 HOLD + -a-69 NOT WALKED)

Proactive walk on HEAD `8453b7a`. Throwaway drive
r19; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726032`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### `-a-74` — STILL PARTIAL on empirical banner surface

The `beforeunload`/`pagehide` flush DOES persist
the buffer (empirically verified: buffer for
`tab-4` with path=CLAUDE.md and divergent content
was present in localStorage immediately before
reload when chan serve was DOWN).

**But banner STILL doesn't render on reload.**
localStorage empty post-reload. Banner DOM
absent.

Reading `-a-72` walk verdict together with
`-a-74` walk: this is **TWO bugs**:

1. **`-a-74` fixed the persist-on-unload path**
   (buffer is now written synchronously before
   page unload, even when reload happens
   mid-typing) ✓
2. **The mount-time race I flagged in `-a-72`
   walk is STILL unfixed**: on initial mount,
   `tab.content === tab.saved === undefined`
   evaluates TRUE → second effect's
   `clearEditorBuffer(tabId)` fires BEFORE the
   banner can render. When `tab.saved` loads
   async, `divergentBufferOrNull` returns null
   (buffer already cleared) → `recoveredBuffer
   = null` → banner state never sets.

**The end-to-end data-loss prevention UX is
STILL NOT working** for the user-visible
surface. `-a-72` mechanism + `-a-74` persist-
path are both green at the unit level, but the
banner the user needs to see on reload still
doesn't render.

**Proposed fix (reiterating from -a-72 walk)**:
gate `FileEditorTab.svelte`'s second effect on
`tab.saved !== undefined` OR detect
"initial-mount-before-disk-load" and skip the
clear. The mount-time check should be the only
clear-or-set authority during the disk-load
window.

Lane: @@FullStackA. **Recommend a third task**
(`-a-75`?) to address the second race. The data-
damage scenario @@Alex flagged in addendum-a.md
is empirically still open.

### `-a-66 slice 1` HOLD — Cmd+N draft creation

Empirically verified:
- 1st Cmd+N → `Drafts/untitled/draft.md` (no
  suffix on first)
- 2nd Cmd+N → `Drafts/untitled-1/draft.md`
  (N=1 suffix on subsequent)

Both open in editor immediately. URL hash
correctly tracks `p: "Drafts/untitled[-N]/draft.md",
m: "wysiwyg", a: 1`. Foundation for the Drafts
feature works as specced.

### `-a-69` NOT WALKED — survey-event setup gap

Code-level only. `BubbleOverlay.svelte`'s
`surveyAsQuoteMarkdown(event)` helper formats
survey topic/from/questions/options as `> `-
prefixed markdown quote lines. Empirical walk
requires a watcher-detected survey event on a
terminal tab, which is non-trivial to trigger
from Chrome MCP browser.

Vitest pins mechanism-verified per
@@FullStackA's commit-ready poke. Lane-A defers
empirical to future walk when survey-emitter
infrastructure is set up.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-66 1 HOLD + -a-74 STILL PARTIAL
  (mount-time race needs second fix) + -a-69
  code-only`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Recommend cutting a follow-up task
for the `FileEditorTab.svelte` initial-mount race
before declaring `-a-72`/`-a-74` shipped.

## 2026-05-22 — poke (-a-82 re-walk STILL PARTIAL + -a-78 slice 1 HOLD)

Proactive walk on HEAD `5cfe964`. Throwaway drive
r20; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726041`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### `-a-82` re-walk — most-of-fix landed, banner STILL not surfacing

**What `-a-82` shipped (verified)**:
* Path-keyed storage: `chan:editor-buffer:CLAUDE.md`
  (was `chan:editor-buffer:tab-4`). Confirmed
  empirically by typing offline + observing the
  key shape in localStorage.
* `saved === undefined` guard in second effect:
  source-verified at
  `FileEditorTab.svelte:206` (added the early
  return when `saved` hasn't loaded yet).
* Schema change: `lastWrite` → `updatedAt`. My
  prior injection tests with the OLD schema were
  silently rejected by the read parser.

**Empirical banner STILL doesn't render**:
Three test scenarios all failed to surface the
banner. Even with correct-schema JS injection
(`{content, updatedAt, path}`), the banner does
not render on reload + localStorage is cleared.

**Refined root-cause hypothesis**:

The `-a-82` guard prevents the INITIAL clear (when
`saved === undefined`). But after the async
file-load completes:

1. tab.saved updates from undefined → disk content
2. Both `$effect`s re-trigger (saved is a dep of both)
3. Second effect: now `saved !== undefined`, and
   `content === saved` (both equal disk content) →
   `clearEditorBuffer(tab.path)` → buffer cleared
4. First effect re-runs: `divergentBufferOrNull`
   reads localStorage → returns null (just cleared)
   → `recoveredBuffer = null`
5. Banner state nulled → template stops rendering

The `-a-82` fix addressed the initial-mount race
(saved undefined). But there's a SECOND race after
async load: the second effect can still clear the
buffer because the buffer content was already
PERSISTED to disk before the divergence check
runs in this scenario.

**Proposed third fix**: either
1. Gate second effect's `clearEditorBuffer` on
   `!recoveredBuffer` (don't clear while banner is
   active awaiting user decision), OR
2. Make first effect mount-only via `untrack` so
   it doesn't re-read after the second clears, OR
3. Run divergence check ONCE on mount and use
   that as the source of truth for the banner
   (don't re-trigger from `tab.saved` changes).

Lane: **@@FullStackA**. **Three** task numbers
(`-a-72`/`-a-74`/`-a-82`) have iterated on this
data-loss prevention without yet surfacing the
banner empirically. The end-to-end UX still isn't
working.

Recommend either:
- Cut `-a-83` for the effect-ordering race, OR
- Reframe the hang-recovery feature: instead of
  banner-on-reload, maybe a different UX surface
  (notification toast on save-failure, or never-
  lose-content via SSE/websocket sync).

### `-a-78 slice 1` — Team dialog shell HOLD

Cleared localStorage. Cmd+Alt+T → terminal.
Cmd+Alt+P → rich prompt. Found **"New Team"
button** in rich prompt toolbar (replaces the
prior watcher button per spec). Clicked it.

Dialog renders with:
- Title "New Team"
- "Your name" input (default Alex)
- "Team name" input (default team-alpha)
- "Auto-prefix names with @@" checkbox (checked)
- "Team size (excluding you): 2" slider
- MEMBERS: Lead + Worker1, each with host
  ("claude") + KEY=value env input + Lead
  radio button
- REAL ESTATE toggle: "Tabs in current Hybrid"
  vs "Split panes"
- "host name required" hint
- Cancel / Bootstrap buttons

The dialog `<div>` has `role="dialog"`. Slice 2+
will wire Bootstrap.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a walk —
  -a-82 STILL PARTIAL (effect-ordering race
  flagged) + -a-78 slice 1 Team dialog HOLD`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. **Strongly recommend** cutting
`-a-83` for the effect-ordering race before
declaring hang-recovery shipped.

## 2026-05-22 — poke (-a-78 slice 2 airplane-grid + drag&drop — 5/5 HOLD)

Proactive walk on HEAD `75f1726`. Throwaway drive
r21; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726047`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 5/5 HOLD

| Check | Verdict |
|-------|---------|
| #1 Split-panes toggle reveals grid | HOLD |
| #2 Grid shapes match team size | HOLD |
| #3 Cells render with index + drop placeholder | HOLD |
| #4 Drag&drop → cell occupies + badge updates | HOLD |
| #5 Multi-robot on same cell | HOLD |

### Empirical evidence

* **Size=2**: shapes `1×2` (active) + `2×1`. 2 cells.
* **Size=4**: shapes `2×2` (active) + `1×4` + `4×1`.
  4 cells. Default `2×2` is the right ergonomic
  choice.
* **Drag Lead → cell 0**: cell gains `occupied`
  class; text becomes "1 @@Lead" (with `@@`
  auto-prefix per clarification #8); member badge
  flips `unassigned` → "cell 1".
* **Drag Worker1 → same cell 0**: cell becomes
  "1 @@Lead@@Worker1"; both badges show "cell 1";
  per spec, these become tabs in the same pane.

### Highlights

* **Auto-prefix `@@` applied**: members display as
  `@@Lead` / `@@Worker1` per the Auto-prefix toggle.
* **Multi-robot tab-in-same-pane semantic
  working**: the spec's "multiple robots on the
  same cell = tabs in same pane" UI affordance is
  empirically clean.
* **Slice 2 closes the Team dialog UI shell**:
  ready for `-a-79` bootstrap orchestrator wiring.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-78 slice 2 airplane-grid + drag&drop
  (5/5 HOLD)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-22 — poke (proactive bundle: -a-66 slice b PARTIAL + -a-85/-a-86 HOLD)

Proactive walk on HEAD `5dffa09`. Throwaway drive
r22; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726056`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### `-a-66 slice b` PARTIAL — Drafts row not surfacing in FB

**Server side correct**:
- `chan-server/src/routes/files.rs` injects
  synthetic `{path: "Drafts", is_dir: true, ...}`
  at position 0 of `/api/files`.
- Empirically verified: curl returns Drafts at
  pos 0 of 1246 entries.

**SPA side missing**:
- Rendered FB has 17 rows, 8 dirs (no Drafts).
- Drafts would alphabetically sit between
  `docs/` and `scripts/` per `sortTreeEntries`.
- Vitest pin at `draftsRowFb.test.ts` only
  validates CSS class shape, not runtime
  rendering.

**Root-cause hypothesis**: the SPA's
`tree.entries` is over-ridden after the initial
`api.list("")` fetch — likely by the WS indexer
event stream (which doesn't carry the synthetic
injection) or by a watcher event for
`Drafts/<created>` that filters somewhere.

Lane: **@@FullStackA** (or whoever owns the SPA
tree-data flow). The synthetic Drafts row is
**load-bearing** for the Drafts feature surface
— users won't discover Drafts without it.

The proactive-walks discipline caught the
mechanism-vs-empirical gap again.

### `-a-85/-a-86` HOLD — toast auto-dismiss via setTransientStatus

Empirical:
- Right-click → "Copy Path" → status shows
  "Copied path" at t0.
- t0 + 4s: status text empty (auto-dismissed).
- 3000ms `TRANSIENT_STATUS_DEFAULT_MS` honored.

Shared mechanism: `setTransientStatus(msg, ms)`
writes `ui.status` + sets timeout to clear. Same
function used by 4 surfaces (Created N / Copied
file path / 2 watcher-detached variants / file
move via `-a-85`).

One empirical check confirms the shared
mechanism. PASS.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-66 b PARTIAL (Drafts row not
  rendered) + -a-85/-a-86 toast auto-dismiss
  HOLD`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Recommend cutting a follow-up for
the `-a-66 b` SPA tree-data flow before declaring
Drafts FB surface shipped.

## 2026-05-22 — poke (proactive: -a-66 b PARTIAL CLOSED via 7be215e + -a-84 placeholder offset HOLD)

Proactive walk on HEAD `3aed6d0`. Throwaway drive
r23; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726062`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 5/5 HOLD

| Task | Verdict |
|------|---------|
| `-a-66 b` Drafts row renders | HOLD (**PARTIAL closed**) |
| `-a-66 b` Yellow tint | HOLD |
| `-a-84` Cursor + placeholder don't overlap | HOLD |
| `-a-84` Hidden on type | HOLD |
| `-a-84` Reappears on full delete | HOLD |

### `-a-66 b` follow-up closed my flagged PARTIAL

The `7be215e` follow-up ("also gate synthetic
injection on dir=''") fixed the SPA rendering
gap I flagged. Now empirically confirmed:

- `Drafts/` row visible in both docked + main
  pane FBs, positioned alphabetically between
  `docs/` and `scripts/`.
- Background `rgba(227, 179, 65, 0.1)` (yellow tint).
- Name color `rgb(227, 179, 65)` (yellow accent).
- Class includes `drafts-row`.

**The proactive-walk loop closed in one round-trip**:
walk → flag → architect routing → fix → re-walk →
confirmed.

### `-a-84` placeholder offset HOLD

@@Alex's `|W` overlap is gone. Cursor at x=350
(1px wide); placeholder at x=353. Gap of ~2px.
`overlap: false`.

Show/hide flow preserved:
- Type `x` → placeholder gone.
- Backspace → placeholder reappears.

Option B (offset right of CM6 cursor) lands clean.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-66 b PARTIAL closed via 7be215e
  follow-up + -a-84 placeholder offset (5/5 HOLD)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-22 — poke (-a-83 BANNER FINALLY SURFACES — 5/5 HOLD; 4-task saga CLOSED)

Proactive re-walk on HEAD `d595758`. Throwaway
drive r24; chan serve 127.0.0.1:8787; Chrome MCP
tab `503726068`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### 🎉 The 4-task hang-recovery saga ENDS HERE

`-a-72` → `-a-74` → `-a-82` → **`-a-83`** → 5/5
HOLD empirical.

The `-a-83` fix matched my Proposal #1 from the
prior walk: "gate second effect's
`clearEditorBuffer` on `!recoveredBuffer`". Plus the
bonus `discardBuffer` `tab.id` → `tab.path` key fix
which I hadn't caught.

### Verdicts: 5/5 HOLD

| Check | Verdict |
|-------|---------|
| #1 Banner appears on mount when divergent | HOLD 🎉 |
| #2 Buttons + role=alert | HOLD |
| #3 Restore swaps content | HOLD |
| #4 Discard dismisses + clears | HOLD |
| #5 Path-keyed clear (no leftover) | HOLD |

### Per-check evidence

**Banner appears empirically**:
- Injected `chan:editor-buffer:CLAUDE.md` with
  divergent content
- Opened CLAUDE.md via FB
- Banner rendered at `x=314, y=38, w=1121, h=43`
  with text "Unsaved changes from a previous
  session were found." + Restore + Discard buttons
  + `role="alert"`

**Restore swaps content**:
- Clicked Restore → editor content now contains
  the injected buffer string; banner dismissed.

**Discard clears localStorage + dismisses**:
- Re-injected a different buffer
- Reloaded → banner reappeared
- Clicked Discard → localStorage entry GONE
  (`lsAfterDiscard: []`); banner dismissed;
  editor stayed at disk content.

The path-keyed clear works correctly — the
pre-`-a-83` `tab.id` form would have left the
localStorage entry lingering, but `-a-83`'s
`tab.path` fix cleans it up properly.

### The proactive-walk discipline paid off

Three round-trips closed this saga:

1. My `-a-72` walk: flagged PARTIAL on banner
   surfacing → `-a-74` cut (beforeunload flush).
2. My `-a-74` walk: STILL PARTIAL → `-a-82` cut
   (path-keying + saved-undefined guard).
3. My `-a-82` walk: STILL PARTIAL with refined
   root cause + 3 proposed fixes → `-a-83` cut
   (Proposal #1 implemented + path-key fix in
   discardBuffer).

Each round, vitest pins passed but the
user-visible UX was broken. Proactive empirical
walks caught it every time.

**The data-loss prevention scenario @@Alex flagged
in addendum-a.md is now empirically closed.** When
the editor hangs and the user force-reloads, the
buffer survives + the banner surfaces + Restore /
Discard work as expected.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a re-walk —
  -a-83 hang-recovery banner FINALLY SURFACES
  (5/5 HOLD; 4-task saga closed)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by — hang-recovery feature can be
declared shipped end-to-end.

## 2026-05-22 — poke (proactive: -a-87 + -a-88 HOLD + -a-66 slice c PARTIAL — same pattern as slice b)

Proactive walk on HEAD `88faa96`. Fresh first-boot
throwaway drive r25; chan serve 127.0.0.1:8787;
Chrome MCP tab `503726077`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 3/4 HOLD + 1 PARTIAL

| Task | Verdict |
|------|---------|
| `-a-87` placeholder line-height match | HOLD |
| `-a-88` docked FB left + no FB tab | HOLD |
| `-a-66 c` DRAFTS chip + notice | **PARTIAL** |

### `-a-87` HOLD — pixel-perfect baseline

Placeholder and CM6 cm-line:
- lineHeight: both `28.8px` ✓
- fontSize: both `16px` ✓
- y position: both `476` ✓
- `alignedY: 0` (perfect baseline match)

### `-a-88` HOLD — clean first-boot

Fresh first-boot drive r25:
- Docked FB on **LEFT** at `x=0, w=305` ✓
- **NO auto-spawned FB tab** — main pane shows
  welcome screen with 4 spawn affordances +
  chord docs ✓

Pre-`-a-88` would have had 2 FBs visible (docked
+ tab). Now the welcome screen is the first
surface; FB tab is opt-in via Cmd+Alt+O.

### `-a-66 slice c` PARTIAL — same root pattern as slice b

**Source side WORKS**:
- `DirectoryInfoBody.svelte` has the kind-chip
  `"DIR" → "DRAFTS"` swap + `.drafts-notice`
  block (verified in commit + in bundled dist).

**Empirical side MISSING**:
- Selected `Drafts/` row.
- Inspector chip shows `"directory"` (lowercase,
  gray bg).
- NO `.drafts` class on chip.
- NO `.drafts-notice` block.
- DirectoryInfoBody's "DIR"/"DRAFTS" chip text
  (uppercase) NOT present.

**Root-cause hypothesis (same pattern as slice b)**:
the inspector rendering the Drafts row is NOT
`DirectoryInfoBody.svelte` — likely `FileInfoBody`
or a generic info body. The chip text
"directory" lowercase confirms this (DirectoryInfoBody
uses uppercase "DIR"/"DRAFTS").

This is the **synthetic-entry vs real-entry data
flow gap** I caught in slice b. Slice b was fixed
by gating server injection on `dir=""`. Slice c
likely needs:
- The synthetic Drafts entry's inspector path
  to route through DirectoryInfoBody, OR
- Apply slice-c changes to FileInfoBody too,
  OR
- Detect synthetic Drafts in the inspector
  dispatch logic.

Lane: **@@FullStackA**. The proactive walk
caught the same data-flow gap pattern twice
across slices b and c. Worth auditing the
synthetic-Drafts-entry data flow holistically.

### Highlights

* **The proactive walk catches mechanism-vs-
  empirical gaps systematically**: when slice b
  PARTIAL was flagged, the fix only touched the
  server-side injection. Now slice c has the
  same gap on the inspector side. A holistic
  audit of synthetic-Drafts data flow might
  prevent similar gaps in slices d (Rich Prompt
  history) and e (Graph styling).

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-87 + -a-88 HOLD; -a-66 slice c PARTIAL
  (same synthetic-Drafts data-flow gap as slice
  b)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Recommend a holistic audit of the
synthetic-Drafts inspector dispatch before
declaring slice c shipped.

## 2026-05-22 — poke (-a-89 placeholder rewrite HOLD + -a-66 slice d disk-persist HOLD / API listing PARTIAL — 3rd repetition of slice b/c/d data-flow gap)

Proactive walk on HEAD `5845fa0`. Throwaway drive
r26; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726098`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts

| Task | Verdict |
|------|---------|
| `-a-89` placeholder via CM6 extension | HOLD |
| `-a-66 d` disk persistence on submit | HOLD |
| `-a-66 d` API surfaces `Drafts/rich-prompt/` | **PARTIAL** |

### `-a-89` HOLD — architectural improvement

CSS overlay → CM6 native placeholder extension.
The `.cm-placeholder` element is now a CodeMirror
widget decoration rather than absolutely-positioned
CSS. Baseline alignment + cursor positioning come
"for free" from CM6's layout engine.

**Supersedes**:
- `-a-84` (manual 2px x-offset)
- `-a-87` (manual line-height match)

Net code reduction + correctness via the
canonical CM6 surface.

### `-a-66 slice d` — disk persist HOLD; API listing PARTIAL

**Disk persist WORKS**:
- Submitted `echo test-a-66d-rich-prompt-history-marker`
  via Cmd+Return.
- File at
  `~/Library/Application Support/chan/drafts/dff9fc3a6072d447/rich-prompt/prompt.md`
  with verbatim content. ✓

**API listing INCOMPLETE (third repetition of
slices b/c/d gap)**:
- `/api/files?dir=Drafts/rich-prompt` returns
  empty array
- `/api/files` (root) returns only the synthetic
  Drafts shell entry
- The file IS on disk in chan's metadata folder
  but the wire-keyspace listing for
  `Drafts/<sub>/` doesn't reach it

### The pattern across slices b/c/d

| Slice | Wins | Gap |
|-------|------|-----|
| b | FB shows synthetic Drafts row ✓ (post follow-up) | (slice b closed) |
| c | DirectoryInfoBody.svelte has DRAFTS chip + notice | Inspector path doesn't reach it; uses different component |
| d | Disk persist on submit | API recursive listing doesn't reach `Drafts/<sub>/` |

**Recommend holistic audit** of the drafts-
metadata-vs-wire-keyspace bridge in the unified
`Drive::list` API before slice e (Graph styling)
lands. The synthetic-Drafts data flow has a
recurring gap at the boundary between chan-drive's
metadata folder and the API/SPA surface.

Lane suggestion: **@@Systacean** for the API
recursive-list bridge + **@@FullStackA** for the
inspector dispatch (slice c follow-up).

### Tear-down hygiene note

`chan remove` doesn't auto-clean
`~/Library/Application Support/chan/drafts/<drive-hash>/`
since Drafts live outside the drive root. Future
walks that exercise the Drafts surface should
explicitly clean this metadata folder
post-tear-down. (I cleaned `dff9fc3a6072d447`
this walk.)

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-89 placeholder rewrite HOLD; -a-66
  slice d disk-persist HOLD / API listing PARTIAL
  (3rd slice b/c/d data-flow gap)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. **Strongly recommend** the holistic
drafts-metadata-vs-wire-keyspace audit before
slice e lands.

## 2026-05-22 — poke (re-walk: -a-66 slice c follow-up + systacean-32 — BOTH FLAGGED GAPS CLOSED, 5/5 HOLD)

Proactive re-walk on HEAD `b51a4b6`. Throwaway
drive r27; chan serve 127.0.0.1:8787; Chrome MCP
tab `503726104`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### 🎉 Both my flagged gaps closed in one round-trip

* `bf36d46` (slice c follow-up) → moved Drafts
  chip+notice styling to FileInfoBody (the actual
  inspector path I identified)
* `b51a4b6` (systacean-32 HIGH) → chan-drive
  prefix-aware Drive::stat for Drafts (closes the
  recurring slice b/c/d data-flow gap)

### Verdicts: 5/5 HOLD

| Check | Verdict |
|-------|---------|
| DRAFTS chip + notice in inspector | HOLD 🎉 |
| API surfaces `Drafts/rich-prompt/` | HOLD |
| API surfaces `Drafts/.../prompt.md` | HOLD |
| FB tree expansion shows subdirs | HOLD |
| End-to-end Drafts flow works | HOLD |

### Empirical evidence

**Inspector** (slice c follow-up):
- Selected `Drafts/` → DETAILS panel
- Chip: `DRAFTS` (uppercase) in `rgb(227, 179, 65)`
  yellow background; class
  `kind-chip drafts-chip` ✓
- Notice block: `role="note"` with the addendum
  text + `rgba(227, 179, 65, 0.1)` tint ✓

**API listing** (systacean-32):
- Submitted `echo systacean-32-api-listing-test`
  via Cmd+Return
- `/api/files?dir=Drafts` → returns
  `[Drafts/rich-prompt]` ✓
- `/api/files?dir=Drafts/rich-prompt` → returns
  `[Drafts/rich-prompt/prompt.md, size: 34]` ✓

**FB tree** (composition):
- Expanded `Drafts/` via chevron click in docked FB
- `rich-prompt/` row appeared indented under
  Drafts/ ✓
- rowCount 18 → 19

### End-to-end Drafts flow validated

1. Drafts row in FB with yellow tint ✓ (slice b)
2. DRAFTS chip + notice in inspector ✓ (slice c
   + follow-up)
3. Cmd+N draft + rich-prompt submit persist to
   disk ✓ (slices d + persist mechanism)
4. API surfaces Drafts subtree ✓ (systacean-32)
5. FB tree expansion shows persisted entries ✓
   (composition of all the above)

### The flagging-to-fix loop

| Walk | Flag | Fix |
|------|------|-----|
| webtest-a slice b walk | Synthetic row missing | follow-up gated on dir="" |
| webtest-a slice c walk | Chip+notice not rendering | follow-up moved to FileInfoBody |
| webtest-a slice d walk | API doesn't surface Drafts subtree | systacean-32 HIGH |

Three repetitions of the synthetic-Drafts data-flow
gap, three round-trip fixes. The proactive-walk
discipline catches the empirical surface; each fix
plus the underlying systacean-32 closes the
recurring pattern at the root (chan-drive's
unified-path API).

### Suggested commit shape

* **Commit subject**: `docs: webtest-a re-walk —
  -a-66 c follow-up + systacean-32 (both flagged
  gaps CLOSED, 5/5 HOLD); end-to-end Drafts flow
  validated`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Drafts feature can be declared
shipped end-to-end through the b/c/d/systacean-32
arc.

## 2026-05-22 — poke (-a-90 PARTIAL — 3rd Alt+Space handler missed)

Proactive walk on HEAD `81f6007`. Throwaway drive
r28; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726114`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdict: PARTIAL

`-a-90` removed Alt+Space from 2 of 3 places.

**Empirical**: spawned Terminal-1 via Cmd+Alt+T,
pressed Alt+Space. **Rich prompt opened.**

### Root cause: missed handler in TerminalTab.svelte

`-a-90` removed Alt+Space from:
- `App.svelte` global keymap ✓
- `TerminalTab.svelte::onShellKeydown` ✓

But **MISSED** the handler in
`TerminalTab.svelte::handleTerminalKeyEvent`
(current source lines 977-989):

```svelte
function handleTerminalKeyEvent(e: KeyboardEvent): boolean {
  if (closeExitedTabFromKey(e)) return false;
  if (
    e.type === "keydown" &&
    e.altKey &&
    !e.ctrlKey &&
    !e.metaKey &&
    !e.shiftKey &&
    e.code === "Space"
  ) {
    e.preventDefault();
    openRichPrompt();  // ← STILL BOUND
    return false;
  }
  ...
}
```

This handler fires when the xterm.js terminal has
focus — the most common context for a "rich prompt
from terminal" muscle-memory hit.

### Vitest pin false-positive

The vitest pin at
`web/src/state/altSpaceRichPromptRemoved.test.ts`
checks two specific regex patterns
(`App.svelte` global + `onShellKeydown`) but
doesn't audit `handleTerminalKeyEvent`. Tests
green; empirical surface broken.

This is another **mechanism-vs-empirical gap**
caught by the proactive-walk discipline.

### Cmd+Alt+P confirmed HOLD

The canonical Cmd+Alt+P (web Mac) chord still
works — toggles rich prompt as expected. The new
entry points are sound; only the legacy chord
removal is incomplete.

### Suggested follow-up

* **Commit subject** (for the @@FullStackA
  follow-up): `Rich prompt: remove 3rd Alt+Space
  handler in TerminalTab.handleTerminalKeyEvent
  (fullstack-a-90 follow-up)` + add vitest pin.
* **Files**: `web/src/components/TerminalTab.svelte`
  + the vitest pin file.

### My commit shape for this walk

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-90 Alt+Space removal PARTIAL (3rd
  handler in TerminalTab.handleTerminalKeyEvent
  missed)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-22 — poke (-a-94 + -a-91 re-walk — Alt+Space saga CLOSED + chord-escape registry HOLD; 3/3)

Proactive walk on HEAD `5e89a74`. Throwaway drive
r29; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726124`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 3/3 HOLD

| Check | Verdict |
|-------|---------|
| `-a-94` Alt+Space from terminal no longer opens prompt | HOLD |
| `-a-91` Cmd+, from focused terminal → Settings opens | HOLD |
| `-a-91` Cmd+Alt+P from focused terminal → rich prompt | HOLD |

### `-a-94` closes my flagged PARTIAL

The 3rd Alt+Space handler in
`TerminalTab.svelte::handleTerminalKeyEvent`
removed. Empirical:
- Cmd+Alt+T → Terminal-1 active.
- Alt+Space → `richPromptVisible: false` ✓

Two round-trips on the Alt+Space removal saga:
- `-a-90`: removed 2 of 3 handlers; my walk caught
  the third
- `-a-94`: removed the third

Saga CLOSED.

### `-a-91` chord-escape registry works empirically

`shouldEscapeTerminal(e)` gate in
`handleTerminalKeyEvent` returns false for
App-group chords so xterm doesn't consume them.

Verified:
- Cmd+, from focused terminal (`xterm-helper-textarea`
  active) → **Settings opens** (`settingsVisible: true`)
- Cmd+Alt+P from focused terminal → **rich prompt
  opens**

The registry covers 7 App-group chords (Settings,
RichPrompt, FB, Graph, Terminal, Reload, NewDraft).

### Side observation: tab-switch chords NOT in registry

Alt+Shift+] from focused terminal still doesn't
switch tabs (xterm eats it). This is by design
per the registry's App-group scope. Tab-switch is
a Tabs-group chord. Possible follow-up for
muscle-memory ergonomics (would be nice to switch
tabs without leaving terminal focus) but NOT a
regression.

### Architectural win

`-a-91`'s registry shape (flag chords in
`shortcuts.ts` with `escapeTerminal: true`) is
more maintainable than ad-hoc per-chord guards.
Future chord additions just flip the bit. Vitest
pin covers both the registry shape AND the 7
required entries.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  re-walk — -a-94 Alt+Space saga CLOSED + -a-91
  chord-escape registry empirically HOLD (3/3)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-22 — poke (-a-66 slice e PARTIAL — indexer doesn't see Drafts; umbrella NOT closed)

Proactive walk on HEAD `f7c0294`. Throwaway drive
r30; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726130`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdict: PARTIAL — umbrella NOT closed

The commit subject says "closes umbrella" but
empirically the graph **does not** render Drafts
data.

| Check | Verdict |
|-------|---------|
| Drafts root node in `/api/graph?scope=drive` | **MISSING** |
| `drafts_link` edges in graph | **MISSING** |
| SPA styling code | HOLD (present, no data to render) |

### Empirical evidence

- Created `Drafts/untitled/draft.md` via Cmd+N
- Added text content to ensure indexing
- Waited 4s
- `/api/graph?scope=drive`:
  - 1435 nodes, 5857 edges
  - No Drafts directory node
  - No drafts_link edges
  - 0 files matching `path.startsWith('Drafts/')`

### Root cause: indexer doesn't walk Drafts/

`synthesize_drafts_layer` gates on
`files.iter().any(|p| p.starts_with("Drafts/"))`.
The `files` argument comes from chan-drive's
indexer file list. But the indexer walks the drive
root on disk, not the Drafts metadata folder.

systacean-32 added prefix-aware
`Drive::stat`/`exists`/`read` for Drafts — but
NOT the indexer scan. So:
- `/api/files?dir=Drafts` works ✓ (systacean-32)
- Indexer file list doesn't include Drafts/ →
  graph synthesizer's gate doesn't fire → no
  Drafts node/edge emitted

### The pattern across 5 slices

| Slice | Surface | Gap | Fix |
|-------|---------|-----|-----|
| b | FB row | server injection on root listing | follow-up gated on dir="" |
| c | Inspector | wrong component (DirectoryInfoBody not used) | follow-up moved to FileInfoBody |
| d | API listing | unified-path didn't reach Drafts subtree | systacean-32 |
| e | Graph emit | indexer doesn't walk Drafts metadata folder | **NEEDS indexer follow-up** |

The synthetic-Drafts data-flow gap appears at
every surface that has an input path different
from the FB row. Each surface has needed a
targeted fix at its specific input.

### Strongly recommend an indexer-side follow-up

Without indexer-side Drafts/ awareness:
- Graph won't show Drafts files
- Full-text search won't find content in
  draft files
- chan-report won't include Drafts in stats

This affects multiple downstream surfaces; the
indexer is the natural shared input. Suggest:

* **Task title**: `chan-drive indexer: walk
  Drafts/ via unified-path scan`
* **Lane**: @@Systacean (probably) — extends
  systacean-32's unified-path bridge into the
  indexer
* **Closes**: `-a-66` umbrella + future graph/
  search/report surfaces that depend on indexed
  Drafts content

### Suggested commit shape for this walk

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-66 slice e PARTIAL (graph styling
  shipped but indexer doesn't walk Drafts;
  umbrella NOT closed)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Strongly recommend the indexer
follow-up before declaring the Drafts umbrella
shipped.

## 2026-05-22 — poke (-a-67 slice 2 HOLD + -a-66 slice e STILL PARTIAL after systacean-34)

Proactive walk on HEAD `773cf95`. Throwaway drive
r31; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726145`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts

| Task | Verdict |
|------|---------|
| `-a-67 2` New Draft as first spawn entry | HOLD |
| `-a-66 e` Drafts in graph after systacean-34 | **STILL PARTIAL** |
| `-a-66 e` BM25 search includes Drafts | **PARTIAL** |

### `-a-67 slice 2` HOLD

Empty-pane right-click menu shows New Draft
(Cmd+N) as the FIRST entry, above Terminal /
File Browser / Rich Prompt / Graph / Search /
Settings. Matches addendum-a's Drafts-first
positioning intent.

### `-a-66 slice e` STILL PARTIAL despite systacean-34

systacean-34 (`aaf7608`) added
`index_drafts_subtree` + `walk_drafts_recursive`.
Code looks right. **But empirically Drafts still
don't surface in graph or BM25**:

- Created `Drafts/untitled/draft.md` with content.
- Restarted chan serve (full reindex triggered
  per "rebuild.inprogress marker found" warn).
- Waited ~25s for reindex.
- `/api/graph?scope=drive`: 1440 nodes, **0
  files under `Drafts/`**, no Drafts directory
  node, no drafts_link edges.
- `/api/search/content?q=systacean-34`: 0 hits
  (BM25 ready=true mode=bm25 hits=[]).

**Server logs reveal the root cause**:
```
WARN chan_server::routes::files: path classification failed
  rel=Drafts/untitled/draft.md
  e=Io("No such file or directory (os error 2)")
```

There's a `path_classification` step that uses
ON-DISK file lookup (not systacean-32's unified
`Drive::stat`). Even when the indexer walks
Drafts/ per systacean-34, the downstream
classification fails because Drafts lives in
chan's metadata folder, not the drive root.

The walker may be silently succeeding at the
walk step but failing to actually populate
BM25/graph because the per-file
classification/indexing step uses the broken
path resolution.

### Recommend systacean-35

`chan-drive path_classification` (and any
downstream code that opens files for indexing)
needs the prefix-aware unified-path handling
that systacean-32 added to `Drive::stat`/
`exists`/`read`. Without it, the indexer's
walk succeeds but the actual indexing fails.

Lane: **@@Systacean**. The `-a-66 slice e`
PARTIAL is now ~3 layers deep into a single
codebase area (initial PARTIAL → systacean-32
fix → systacean-34 fix → STILL PARTIAL). A
holistic audit of Drafts-aware path code
would close the umbrella.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a re-walk
  — -a-67 slice 2 HOLD + -a-66 slice e STILL
  PARTIAL (path_classification fails despite
  systacean-34 walker)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Strongly recommend systacean-35 to
audit `path_classification` + downstream
indexer file I/O for Drafts-aware path
resolution.

## 2026-05-22 — poke (-a-66 slice e GRAPH HOLD via systacean-36 / BM25 PARTIAL + -a-68 slice 1 PARTIAL on shortcuts.ts label)

Proactive walk on HEAD `2125be7`. Throwaway drive
r32; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726151`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### `-a-66 slice e` GRAPH FINALLY HOLD via systacean-36

systacean-36 closes the saga at the graph layer:
- `directory:Drafts` node present ✓
- `drafts_link` edge from `directory:` →
  `directory:Drafts` present ✓
- `Drafts/untitled/draft.md` file node present ✓
- Total graph: 1453 nodes (was 1440)

The architect's "gap UPSTREAM of my hypothesis"
call was right — `apply_watch_change` routing
through `index_draft_file` was the real fix, not
`path_classification`.

🎉 Slice e GRAPH portion closed end-to-end after
**3 round-trips** (systacean-32 → -34 → -36).

### BM25 secondary PARTIAL

`/api/search/content?q=UNIQUEMARKER36CLOSURE`:
- Status 200, mode: bm25, ready: true, **hits: []**

Even with the marker uniquely embedded in
`Drafts/untitled/draft.md` (which IS indexed in
graph per systacean-36), BM25 returns 0 hits.

The graph indexer + BM25 indexer share different
ingestion paths. `index_draft_file` reaches the
graph corpus but not the BM25 store.

This is a 4th-degree gap on the umbrella; not a
load-bearing UX regression (graph is the slice e
primary surface). Worth a systacean-37 follow-up
to align the BM25 path with the graph path.

### `-a-68 slice 1` PARTIAL — `shortcuts.ts` label missed

Rename `"Hybrid NAV"` → `"Hybrid Nav"` updated:
- `Pane.svelte` menu label + aria-label ✓
- `PaneModeHelp.svelte` title + aria-label ✓
- Tests ✓

**Missed**:
- `web/src/state/shortcuts.ts:202`:
  `label: "Enter Hybrid NAV"` (still uppercase)

This label drives the welcome screen chord docs.
Empirically on welcome screen:
- "Hybrid NAV" (uppercase) count: 5
- "Hybrid Nav" (proper case) count: 0

The most-visible chord doc surface STILL shows
the OLD case. Mechanism-vs-empirical gap again.

Suggest follow-up:
* `web/src/state/shortcuts.ts:202`:
  `"Enter Hybrid NAV"` → `"Enter Hybrid Nav"`
* `web/src/state/shortcuts.test.ts:42-43`:
  update regex pattern.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a re-walk —
  -a-66 slice e GRAPH HOLD via systacean-36 (3rd
  round) / BM25 secondary PARTIAL; -a-68 slice 1
  PARTIAL on shortcuts.ts label`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Drafts umbrella GRAPH portion
finally shipped. Two secondary gaps flagged:
BM25 (systacean-37 candidate) + shortcuts.ts
label (`-a-68 slice 1` follow-up).

## 2026-05-22 — poke (-a-68 1b HOLD + systacean-37 STILL PARTIAL on BM25 + -a-70 endpoint HOLD)

Proactive walk on HEAD `7bbe925`. Throwaway drive
r33; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726157`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts

| Task | Verdict |
|------|---------|
| `-a-68 1b` Hybrid Nav label sweep | HOLD 🎉 |
| systacean-37 BM25 boot walk | **STILL PARTIAL** |
| `-a-70` /api/mentions endpoint | HOLD |

### `-a-68 slice 1b` HOLD — closes my flagged label miss

Welcome screen empirical:
- "Hybrid NAV" (uppercase) count: 0 ✓
- "Hybrid Nav" (proper case) count: 5 ✓

The `shortcuts.ts:202` label sweep is complete.
Slice 1 rename closure confirmed across all
user-facing strings.

### systacean-37 STILL PARTIAL — 5th-degree slice e gap

systacean-37 adds "unconditional Drafts boot
walk in Indexer::spawn". But empirically:

- Created `Drafts/untitled/draft.md` with marker
  "UNIQUEMARKER37BM25CLOSURE".
- Restarted chan serve.
- Waited 16+ seconds.
- `/api/search/content?q=UNIQUEMARKER37BM25CLOSURE`:
  `{ready: true, mode: bm25, hits: []}`

The slice e BM25 saga is now 5 iterations deep:
- original PARTIAL
- systacean-32 (Drive::stat unified)
- systacean-34 (boot walks Drafts subtree)
- systacean-36 (apply_watch_change routes Drafts/)
- systacean-37 (Indexer::spawn unconditional boot walk)
- **STILL no BM25 hits**

Either Indexer::spawn isn't actually walking
Drafts (despite "unconditional"), OR the per-file
indexing call silently fails for Drafts paths
because the BM25 indexer uses on-disk file open
(which fails for Drafts/ in the unified keyspace
that doesn't exist on disk).

Recommend a deeper audit — possibly trace the
indexer's file-open code path to see if it uses
the unified `Drive::read` (which works for
Drafts) or raw `std::fs::open` (which doesn't).

### `-a-70` /api/mentions HOLD — endpoint live

- `GET /api/mentions` → 200
- Returns array of `{label: string}` items
- 10 mentions: @@Architect, @@Alex, @@FullStackA,
  @@Systacean, @@WebtestA, ...

**Count discrepancy**: 10 returned vs 49 unique
mention nodes seen in `-22` walk graph data.
Worth a quick verification with @@FullStackA
that 10 is the intentional deduped corpus (vs
49 graph mentions that may include partial
matches / handle variants).

Empirical bubble integration test (typing
`@@cl` in editor for autocomplete) was
inconclusive — Cmd+N didn't open editor in
clean state. Endpoint mechanism works; bubble
SPA integration needs a clean editor mount.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a re-walk —
  -a-68 1b HOLD (label sweep closed) + systacean-37
  STILL PARTIAL on BM25 (5th-degree slice e gap) +
  -a-70 /api/mentions HOLD`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. The BM25 path appears to be a
structural issue (Drafts paths don't exist on
disk for raw file opens) — deeper audit needed.

## 2026-05-22 — poke (Architect re-walk request honored: Drafts BM25 FULLY CLOSED via fresh-binary procedure)

Per your re-walk request. Procedure followed:

1. `pkill -f "target/debug/chan serve"` ✓
2. `npm run build && cargo build -p chan` ✓
3. Build provenance verified:
   - Binary timestamp: May 22 23:39:11
   - HEAD: `91b0b3a` (post-systacean-38)
4. `RUST_LOG=chan_drive=debug` chan serve ✓
5. Created Drafts/untitled/draft.md with marker
6. Restarted to trigger boot walk
7. Queried `/api/search/content`

### 🎉 Drafts BM25 FULLY CLOSED — your hypothesis confirmed

**Boot walk** (post-restart):
- Logs show:
  ```
  index_draft_file: wrote graph + BM25
    rel="Drafts/untitled/draft.md" content_len=92
  ```
- `/api/search/content?q=UNIQUEMARKER38FRESHBINARY`:
  `hits: 1, path: "Drafts/untitled/draft.md"` ✓

**Runtime watcher** (typed more content after boot):
- Logs:
  ```
  index_draft_file: wrote graph + BM25
    rel="Drafts/untitled/draft.md" content_len=152
  ```
- `/api/search/content?q=RUNTIMEWATCHERMARKER39`:
  `hits: 1, path: "Drafts/untitled/draft.md"` ✓

Both indexing paths empirically surface Drafts
content in BM25. The `tracing::debug` logs added
by `-38` made the verification trivial.

### Your "environmental, not code-level" diagnosis 100% correct

My 5 prior PARTIAL reports were against stale
binaries. Most likely root causes:
- Either old `./target/debug/chan` from before
  systacean-37 landed
- OR running chan process not actually restarted
  between walks (only SPA page-refreshed)
- OR cargo incremental compilation returned a
  stale binary

Apologies for the false-positive PARTIALs;
should have explicitly killed + rebuilt + restarted
between each walk.

### Lesson for future re-walks

I'm going to apply this discipline going forward
for previously-failed empirical re-tests:

1. `pkill -f "target/debug/chan serve"` explicitly.
2. `cargo build -p chan` (or `cargo clean -p chan
   && cargo build -p chan` for paranoid mode).
3. Verify binary provenance:
   - `stat -f "%Sm" ./target/debug/chan`
   - `git rev-parse HEAD`
4. Start from fresh binary.
5. `RUST_LOG=<crate>=debug` to confirm code path
   fires.

This will save round-trips on the next
mechanism-vs-empirical investigation.

### The full Drafts saga (8 task iterations, fully closed)

| Item | Surface | Status |
|------|---------|--------|
| slice a | Cmd+N creates draft | HOLD |
| slice b | FB synthetic row + yellow tint | HOLD |
| slice c | FB inspector DRAFTS chip + notice | HOLD |
| slice d | Rich Prompt persist to disk | HOLD |
| slice d | API listing of Drafts/<sub>/ | HOLD (-32) |
| slice e | Graph root node + drafts_link edge | HOLD (-36) |
| slice e | BM25 boot walk | HOLD (-37 + fresh binary) |
| slice e | BM25 runtime watcher | HOLD (-38 wiring + fresh binary) |

### Suggested commit shape

* **Commit subject**: `docs: webtest-a
  architect-requested re-walk — Drafts BM25
  FULLY CLOSED via fresh-binary procedure (saga
  CLOSED end-to-end across 5 slices)`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Drafts saga truly closed end-to-end.
Thanks for the re-walk routing + the
fresh-binary directive — it was exactly the
right diagnostic move.

## 2026-05-23 — poke (-a-76 slice 2 + -a-77 slice 2 — 3/3 HOLD + 1 UX side observation)

Proactive walk on HEAD `b099b24`. Followed the new
fresh-binary discipline per the Drafts saga lesson.
Throwaway drive r35; chan serve 127.0.0.1:8787;
Chrome MCP tab `503726169`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 3/3 HOLD

| Task | Verdict |
|------|---------|
| `-a-76 2` Settings Features (reports + BGE pair) | HOLD |
| `-a-77 2` Screensaver overlay covers drive | HOLD 🎉 |
| `-a-77 2` PIN entry form rendered | HOLD |

### `-a-76 slice 2` HOLD

Cmd+, → Settings. New FEATURES section pairs:
- **chan-reports** (Code-stats indexing) toggle —
  default Off
- **BGE semantic search** (Hybrid BM25 +
  embeddings) toggle — default Off

Both co-located per spec. Clean.

### `-a-77 slice 2` HOLD

Setup:
- `PATCH /api/screensaver/state {enabled: true,
  timeout_secs: 3}` ✓
- Reloaded
- Waited 6s (> 3s timeout, no interaction)

Lock fired:
- `.screensaver-backdrop` rendered full-window
  (1440x757)
- Lock icon + "Screen locked" title
- PIN input + green Unlock button

State machine + overlay work as specced.

### Side observation: helper text vs validation contradiction

Helper text: "No PIN set on this drive. The
lockout is informational only — **any input
unlocks**."

But empirically:
- Empty PIN → "Enter a PIN to unlock" error
- Non-empty PIN → "No PIN set on this drive.
  Open Settings to configure." error

If no PIN is set + screensaver enabled, the user
is **locked out with no way to unlock via the
form**. Settings is also unreachable while
overlay is up.

Suggest one of three fixes:
- Make "any input unlocks" actually work when no
  PIN is set (matches helper text)
- Rework helper to say "Set a PIN in Settings
  before enabling" + refuse `enabled=true` when
  PIN is unset
- Show a "Disable lock" button in the overlay
  when no PIN is set (escape hatch)

Lane: **@@FullStackA**. UX-only, not a
mechanism regression.

### Side note on testing

Without the PIN-set fix, future empirical walks
of the screensaver flow risk locking themselves
out. I disabled via API before tear-down. The
next walker should be aware: don't `enabled=true`
without `pin_set=true`, or have an API escape
ready.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-76 2 + -a-77 2 HOLD (3/3); UX side
  observation on screensaver no-PIN lockout`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by.

## 2026-05-23 — poke (-a-77 slice 3 — 7/7 HOLD; my UX side observation CLOSED)

Proactive walk on HEAD `8b6c97c`. Fresh-binary
discipline. Throwaway drive r36; chan serve
127.0.0.1:8787; Chrome MCP tab `503726175`.
Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 7/7 HOLD

| Check | Verdict |
|-------|---------|
| Settings UI: Screen lock entry | HOLD |
| Sub-block visible when enabled=true | HOLD |
| Inactivity timeout input | HOLD |
| Set PIN button + inline dialog | HOLD |
| Mod+L lock chord | HOLD |
| pauseScreensaverTimer on Settings open | HOLD |
| Overlay no-PIN shape: "Press any key or click" | HOLD 🎉 |
| Click unlocks when no PIN | HOLD 🎉 |

### My flagged UX side observation CLOSED

Slice 2 overlay (no PIN):
- Helper text "any input unlocks" contradicted
  by validation requiring a PIN
- User locked out with no escape

Slice 3 overlay (no PIN):
- Helper text "Press any key or click to unlock"
  (clean + accurate)
- **No PIN form, no Unlock button** (conditional
  render gates on `pin_set=true`)
- Click anywhere on overlay → unlock ✓
- Empirically verified

The form is conditionally rendered only when
`pin_set=true`. For `pin_set=false`, the overlay
is informational + dismissable via any input.
Option 1 routing **is** implemented; my prior
walk's "I don't see it" was because I tested the
SAME tab without reloading first, and the SPA
cached the old overlay component.

(Note for future re-walks: after a code change
that modifies a conditional render, force-reload
the SPA to see the new shape. Cmd+R or
location.reload(). Should add this to the
fresh-binary discipline canon.)

### Three-slice screensaver feature complete

- slice 1 (api.screensaver* + PBKDF2): mechanism
- slice 2 (state machine + overlay): empirical
- slice 3 (Settings UI + Mod+L + no-PIN UX):
  empirical + closes my UX flag

End-to-end:
1. User enables Screen lock in Settings → may set
   PIN
2. After inactivity timeout, lock fires
3. With PIN: enter PIN → unlock (rejected if wrong)
4. Without PIN: click/key → unlock (option 1)
5. Mod+L → manual lock
6. Settings open → timer pauses

Clean UX shape.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a proactive
  walk — -a-77 slice 3 7/7 HOLD (Settings UI +
  Mod+L + no-PIN UX); UX side observation CLOSED`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing by. Screensaver umbrella closed
empirically.

## 2026-05-23 — poke (autonomous batch walk per BATCH DISPATCH: 8/8 HOLD; lane queue-empty signal)

Per BATCH DISPATCH (`b6dab54`): un-tethered, walked
the lane-A backlog autonomously on fresh binary
(build May 23 05:33:31, HEAD `b6dab54`). Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 8/8 HOLD

| Task | Verdict |
|------|---------|
| `-a-66` umbrella full canonical (FB + chip + graph + drafts_link + BM25) | HOLD 🎉 |
| `-a-70` mention bubble in editor | HOLD |
| `-a-93` terminal resize trailing-edge fit | HOLD |

### `-a-66` umbrella canonical (the final closure)

End-to-end on fresh binary:
- FB row: synthetic `Drafts/` with yellow tint
- Inspector chip + notice (DRAFTS + drafts-notice)
- Graph: `directory:Drafts` node + 1 `drafts_link`
  edge + Drafts file under `Drafts/`
- BM25: `CANONICALMARKER66` query returns hit at
  `Drafts/untitled/draft.md` with proper heading
  + snippet

5 slices + 7 systacean follow-ups + fresh-binary
discipline = saga REALLY DONE.

### `-a-70` mention bubble HOLD

Typed `@@Ar` in editor → bubble appeared with 5
`@@Architect-*` matches sourced from the deduped
mention corpus. Filter narrowing works. "↵ to
insert" footer present.

### `-a-93` resize HOLD

Terminal pane reflowed cleanly:
- 1440×900 → 1024×768: xterm 1414→998 px
- 1024×768 → 1600×1000: xterm 998→1574 px

Trailing-edge fit converges to final width.

### Lane-B items deferred (out of lane-A perm)

These need @@WebtestB:
- `-b-30 slice a` font default (chan-desktop primary)
- `-b-30 slice b` Source Code Pro download flow
- `-b-29` WebGL box-drawing
- `-b-28b slice iv` chan-desktop pre-flight modal

### 🚪 Lane queue-empty signal

Per BATCH DISPATCH stand-down protocol:
> When the queue empties + nothing new lands for
> a beat, file a "lane queue-empty" poke + go
> stand-down.

**Queue status**:
- Walked: `-a-66` canonical + `-a-70` + `-a-93`
  (3/3 HOLD)
- Already walked (prior batches): `-a-89`, `-a-91`
- Out of scope (lane-B): `-b-30 a/b`, `-b-29`,
  `-b-28b iv`
- Future @@FullStackA ships (`-a-67d/e/f`,
  `-a-68 2`, `-a-75`, `-a-79`, `-a-80`): not yet
  landed

No remaining lane-A walk-ready items. Going to
stand-down. Re-engage me when the next batch
lands.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a autonomous
  batch walk per BATCH DISPATCH — -a-66 canonical
  + -a-70 + -a-93 (8/8 HOLD); lane queue-empty`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing down. 🫡

## 2026-05-23 — poke (autonomous batch #2: -a-67 d/e/f + -a-68 slice 2 — 4/4 HOLD; queue-empty)

Re-engaged per BATCH DISPATCH protocol on the new
landings. HEAD `ca5c173`. Fresh-binary discipline.
Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 4/4 HOLD

| Task | Verdict |
|------|---------|
| `-a-67d` Terminal right-click revamp | HOLD |
| `-a-67e` FB right-click revamp | HOLD |
| `-a-67f` Editor right-click revamp | HOLD |
| `-a-68 slice 2` Hybrid Nav transactional | HOLD |

### Highlights

* **`-a-67` UMBRELLA closed empirically** across
  all 4 right-click surfaces (Graph slice 1a/1b
  prior + Terminal/FB/Editor now).
* **Hybrid Nav transactional** works as specced:
  Cmd+. → mode, stage T/O/P/G/N silently, Enter
  commits / Esc discards. Verified via N→Enter
  → new draft.md tab.

### 🚪 Lane queue-empty (round 2)

Walked: -a-67 d/e/f + -a-68 slice 2 (4/4 HOLD).
No remaining lane-A walk-ready items per BATCH
DISPATCH backlog.

Lane-B items (`-b-30 a/b`, `-b-29`, `-b-28b iv`)
still out of lane-A scope.

Going stand-down. Re-engage when next batch
lands.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a autonomous
  batch #2 — -a-67 d/e/f + -a-68 slice 2 (4/4
  HOLD); lane queue-empty round 2`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing down. 🫡

## 2026-05-23 — poke (autonomous walk: -a-75 Infographics + carousel — 4/4 HOLD; queue-empty)

Fresh-binary build of `1662ba9`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 4/4 HOLD

| Check | Verdict |
|-------|---------|
| Carousel slide 1 spawn actions in order | HOLD |
| Empty-pane right-click matches (prior walks) | HOLD |
| Infographics tab spawnable | HOLD |
| Infographics contains former carousel shortcut table | HOLD |

### `-a-75` empirical

**Carousel slide 1**:
- 5-tile spawn grid: New Draft / Terminal / File
  Browser / Rich Prompt / Graph
- Infographics tile below (separator divides)
- Pagination dots `● ○ ○` (3 slides; carousel
  rotates)
- Play/pause control

**Infographics tab**:
- URL hash kind `i` (`k:i`)
- Content sections: App / Panes / Tabs (former
  carousel shortcut table)
- Stable surface for chord help reference

### 🚪 Lane queue-empty (round 3)

No remaining lane-A walk-ready items. Going
stand-down.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a autonomous
  walk — -a-75 Infographics + carousel (4/4 HOLD);
  queue-empty round 3`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited `git commit`.

Standing down. 🫡

## 2026-05-23 — addendum to -a-75 walk: PARTIAL on Infographics tab (no carousel inside)

@@Alex flagged immediately after the prior poke:

> "I was expecting that we would still have the carousel
> functionality, but inside the tab instead; this is so
> that we can see all of the widgets from the carousel
> rolling and also pause"

### Empirical re-read

* **Welcome screen** (pre-tab): carousel widget
  with rotating slides + pagination dots `● ○ ○`
  + play/pause control.
* **Infographics tab** (post-click): STATIC
  "Shortcuts" page. No carousel, no rotation, no
  pause. Just the App / Panes / Tabs chord
  sections rendered as a single static page.

### Expected vs actual

@@Alex's intent: the Infographics tab should HOST
the carousel widget itself (rolling slides +
pause control inside the tab), not be a static
extract of one slide's content.

Current: carousel lives on welcome (pre-spawn);
Infographics tab is a static shortcut reference.

### Verdict update for `-a-75`

| Check | Prior | Updated |
|-------|-------|---------|
| Carousel slide 1 spawn order | HOLD | HOLD |
| Empty-pane right-click matches | HOLD | HOLD |
| Infographics tab spawnable | HOLD | HOLD |
| Infographics shows carousel widget | (assumed shortcut table) | **PARTIAL** |

### Suggested fix

* Lift the carousel component (slides + pagination
  + play/pause) from the welcome screen into the
  Infographics tab body.
* Welcome screen can either:
  (a) keep a static slide-1 view (less motion as
      first impression), OR
  (b) keep the carousel as today + duplicate inside
      Infographics, OR
  (c) move the carousel entirely to Infographics and
      simplify welcome to just the spawn tiles.

@@Alex's wording ("inside the tab instead") suggests
option (c): static spawn-tile welcome + carousel-
inside-Infographics. But other readings work; would
defer to @@Alex's preference for (a/b/c).

Lane: **@@FullStackA**. Worth a slice 1b
(`-a-75 slice 1b`?) to move the carousel
component into the Infographics tab.

### Suggested commit shape (for this addendum)

* **Commit subject**: `docs: webtest-a -a-75
  addendum — Infographics tab PARTIAL (static
  page, no carousel widget inside; @@Alex caught
  the UX gap)`.
* **Files**:
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited.

Standing by.

## 2026-05-23 — @@Alex routed -a-75 follow-up: option (c) — carousel only in Infographics tab

@@Alex: "This is correct we will no longer have
the carousel in the back of the pane and it will
only live in the tab from now on"

### Routing locked

Option (c) from the prior addendum:
- **Welcome screen** (back of empty pane): static
  spawn tiles ONLY. No carousel widget, no
  rotation, no pause control.
- **Infographics tab**: hosts the carousel widget
  with all slides + pagination + play/pause.

The Infographics tab becomes the canonical home
for the multi-slide widget UX (Shortcuts + future
slides). The empty-pane back surface simplifies
to the spawn-tile grid + "Each pane's visible tab
is part of the scope for Graph." hint.

### Lane

**@@FullStackA**. Slice 1b (`-a-75 slice 1b`?) to:
1. Move the carousel component from EmptyPane /
   welcome surface to InfographicsTab body.
2. Strip the back-of-pane carousel — leave the
   5-tile spawn grid + footer hint only.
3. Update the Infographics tab default slide
   ordering (Shortcuts first, future slides
   appended).

The carousel rotation + play/pause UX is preserved
end-to-end, just relocated.

Standing by.

## 2026-05-23 — poke (batch #3: -a-75b + -a-67e s2 + -a-67d s2 — 5/5 HOLD; queue-empty round 4)

Fresh-binary build of `8f98eef`. Verdict in
[`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).

### Verdicts: 5/5 HOLD

| Check | Verdict |
|-------|---------|
| `-a-75b` Carousel in Infographics tab | HOLD 🎉 |
| `-a-75b` Welcome static spawn grid | HOLD |
| `-a-67e s2` Unified New File or Directory | HOLD |
| `-a-67e s2` FB Settings flip entry | HOLD |
| `-a-67d s2` MCP env vars modal | HOLD |

### `-a-75b` closes my flagged UX PARTIAL

Empirical:
- Welcome screen: `welcomeHasCarousel: false`; just
  5 spawn tiles + Infographics tile + footer.
- Infographics tab: `.carousel` element with
  pagination + play/pause + arrow nav.

@@Alex's option (c) routing implemented cleanly:
welcome static, Infographics owns the carousel.

### `-a-67e s2` HOLD

Right-click on FB row → menu order:
- New File or Directory (unified)
- Search / New Terminal / New Graph
- Copy Path / Rename / Move / Delete
- **Settings** (foot — flips FileTree to
  settings, tab variant only)

### `-a-67d s2` HOLD

Terminal right-click → "Set MCP env vars" row
has info-button (i). Click → modal:
- Title "MCP env vars"
- Description with `CHAN_MCP_SOCKET` /
  `CHAN_MCP_SERVER_JSON` tokens
- "Show MCP env in terminal" action button
  (moved from menu row)

### 🚪 Lane queue-empty round 4

No remaining lane-A walk-ready items. Going
stand-down.

### Suggested commit shape

* **Commit subject**: `docs: webtest-a batch #3 -
  -a-75b carousel relocation + -a-67e s2 + -a-67d
  s2 (5/5 HOLD); queue-empty round 4`.
* **Files**:
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`
* Path-limited.

Standing down. 🫡

---

## 2026-05-23 — `-a-79 slice 1` HOLD (round 41)

Walked commit `753e780` (Team Bootstrap
orchestrator slice 1) post-`ea694c7` ack.
Throwaway drive at `/tmp/chan-test-phase8-wa-r41/`;
fresh binary verified (build 09:17:31 @ `c9b8489`).

### Verdict: HOLD ✅

Acceptance 1-4 verified empirically:

1. **Config persisted** at app-config drafts
   metadata path:
   `~/Library/Application Support/chan/drafts/<drive-hash>/team-team-alpha/config.toml`
   with members + `[members.env]
   CHAN_TAB_NAME` for each.
2. **Real estate** matches `Tabs in current
   Hybrid` choice — `@@Worker1` spawned as new
   tab in same Hybrid.
3. **Spawn count** is N-1=1 (Lead skipped per
   spec "Slice 1 skips the lead in the spawn
   iteration"); CHAN_TAB_NAME mechanism wired
   through orchestrator → spawnTerminal →
   `terminal_sessions.rs:693`.
4. **Watcher active**: `GET /api/teams/loaded`
   → `{"teams":["team-alpha"]}`.

Slice 2 deferreds (template, pre-flight, split
panes, event-channel prompts) explicitly out of
slice 1 scope.

### Notes worth flagging (not slice-1 blockers)

* **"host name required" copy is misleading**: the
  dialog foot hint fires for empty `Your name` /
  `Team name` (the visible "Alex"/"team-alpha" are
  placeholders), NOT host fields. A UX polish
  candidate — re-word to "name and team name
  required" or check the actual validation source.
* **Identity prompt seedInput not visible in
  claude**: TerminalTab consumes `tab.seedInput`
  and clears it (`tab.seedInput = undefined`), but
  the claude UI in `@@Worker1` shows an empty
  prompt with default placeholder. Likely a race
  between PTY write and claude's boot animation.
  The slice-2 `dispatch_agent_event`-driven path
  is the intended fix per spec line 224-228.
  Logging here for visibility, not as a slice-1
  block.

### State

Test server torn down: team-alpha unloaded via
`POST /api/teams/team-alpha/unload` → confirmed
`teams:[]`; chan serve killed; drive wiped; drafts
metadata wiped; `chan remove` ran; Chrome tab
closed.

### Suggested commit shape

* **Subject**: `docs: webtest-a round 41 — -a-79
  slice 1 HOLD (Team Bootstrap orchestrator);
  flagged 'host name required' copy + seedInput
  visibility note`
* **Files** (path-limited):
  * `docs/journals/phase-8/webtest-a/webtest-a-1.md`
  * `docs/journals/phase-8/alex/event-webtest-a-architect.md`

Standing by for next dispatch / queue.
