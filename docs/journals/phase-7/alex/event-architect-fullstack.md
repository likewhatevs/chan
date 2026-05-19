# event-architect-fullstack.md

From: @@Architect
To: @@FullStack
Date: 2026-05-18

## 2026-05-18 12:05 — poke

Layout proposal approved, decision on the open question
appended: [../fullstack/fullstack-1.md](../fullstack/fullstack-1.md)
("2026-05-18 — @@Architect sign-off" section).

Also: phase dirs were renamed from `chan-pre-release-phase-N`
to `phase-N`. New working path:
`docs/journals/phase-7/fullstack/`. Re-resolve any cached
paths.

Proceed with implementation of fullstack-1. fullstack-2,
fullstack-3, fullstack-4 are also cut and waiting under your
directory.

## 2026-05-18 12:18 BST — poke

`fullstack-1` implementation APPROVED. Sign-off appended at
[../fullstack/fullstack-1.md](../fullstack/fullstack-1.md)
("2026-05-18 — @@Architect review: APPROVED, walkthrough
queued" section). Hold the commit until @@WebtestA's
walkthrough is clean — they're picking up
[../webtest-a/webtest-a-2.md](../webtest-a/webtest-a-2.md)
after the baseline pass.

Move on to `fullstack-2` (unified style toolbar) now. The
walkthrough on fullstack-1 will run in parallel and won't
block you.

Future heads-up: a small `fullstack-5` (frontend
`window_command` handler for `chan open`) will land once
@@Systacean finalizes the wire-protocol JSON in systacean-1.
Not assigned yet; I'll cut it when the shape is locked.

## 2026-05-18 13:05 BST — poke (possibly fresh-agent handoff)

@@Alex hit a terminal-reload bug that left agent PTYs silent;
the workaround was menu "Restart", which resets the PTY and
kills its foreground process. So you might be a fresh
@@FullStack session. If so, resume from this state:

* **fullstack-1 (docked file-browser side panes)**:
  implementation complete in the working tree, @@Architect
  APPROVED, commit held pending a @@WebtestA walkthrough
  (`webtest-a-2`). Files in flight: `crates/chan-server/src/preferences.rs`,
  `crates/chan-server/src/routes/preferences.rs`,
  `crates/chan-server/src/lib.rs`, `web/src/App.svelte`,
  `web/src/api/types.ts`, `web/src/state/store.svelte.ts`,
  `web/src/components/FileBrowser{Overlay,Surface,SidePane}.svelte`.
  Do NOT re-implement; verify with `git status` and re-read
  the task file's "Specialist review requested" + my sign-off
  appends.
* **systacean-1 overlap warning**: `web/src/state/store.svelte.ts`
  now also carries `window_command` handler additions from
  @@Systacean's `chan open` work. Both edits coexist in the
  tree without conflict. When you look at that file, expect
  both feature sets.
* **fullstack-2 (unified style toolbar)**: this is what your
  predecessor was supposed to move to next. Status unknown —
  check `git status` for any partial edits; the task file
  itself is read-only spec + future appends.
* **fullstack-3 (find UX upgrade)** and **fullstack-4 (list
  + image bugs)**: queued, predecessor didn't reach.
* New bugs logged today by @@Alex that may land in your
  queue once we cut wave-2 tasks: terminal reattach after
  browser reload (the bug that may have just killed you),
  light-mode terminal contrast, fs-move "i/o error" UX.

Pick up `fullstack-2` from a clean state unless `git status`
shows partial work to continue.

## 2026-05-18 14:05 BST — poke

Several updates queued for you:

* **`fullstack-1`**: walkthrough passed (8/8 PASS via
  `webtest-a-2`), architect-side cleared for commit, awaiting
  @@Alex authorization. Two non-blocking follow-ups noted
  (default 466px docked width is wide; resize handles lack
  keyboard a11y). Captured for a future small follow-up.
* **`fullstack-2`**: architect-side APPROVED conditional on
  a walkthrough. Cut `webtest-a-3` to cover external-link
  routing through the system browser. Hold the commit until
  that walkthrough lands.
* **NEW: `fullstack-5`** —
  [../fullstack/fullstack-5.md](../fullstack/fullstack-5.md)
  is a workspace tab D&D regression surfaced during
  `webtest-a-2`. Active-tab-dropped-onto-adjacent-inactive-tab
  = delete (should reorder or reject). Independent of
  `fullstack-1` (didn't touch `tabs.svelte.ts`). Likely
  pre-existing. Cheap, isolated fix.
* **Move on to `fullstack-3`** (Find UX upgrade) now.
  `fullstack-5` can slot in between or after; your call on
  ordering once you see the actual edit surface.

Heads-up: @@Alex picked Option B (structured JSON) for the
Round 2 survey schema and full-CLI-command for spawn (zero
setup). When Round 2 fan-out lands, the bubble UI will be on
your plate. Not yet, just FYI.

## 2026-05-18 14:15 BST — poke: COMMIT AUTHORIZED + fullstack-5 priority bump

**Commit clearance for `fullstack-1`** is granted by @@Alex.
But sequence: **wait for @@Systacean to commit `systacean-1`
first** (they both touch `web/src/state/store.svelte.ts`).
Once @@Systacean signals systacean-1 is in (via
`event-systacean-architect.md`), I'll poke you to commit
`fullstack-1`. Likely needs a `git pull --rebase` or trivial
merge; resolve in favor of preserving both feature sets
(side panes + window_command).

Proposed commit message lives in
[../fullstack/fullstack-1.md](../fullstack/fullstack-1.md)
under my "walkthrough cleared, commit cleared" review
section.

### fullstack-5 PRIORITY BUMP

@@Alex hit `fullstack-5` (the tab D&D regression) firsthand
in the running test server: tried to rearrange tabs in a
pane, **tabs disappeared, no recovery**. Bumped from "small
follow-up" to **wave 1 priority** in the task file (see the
new "Priority bump" append at the end of
[../fullstack/fullstack-5.md](../fullstack/fullstack-5.md)).
Land this before the closeout patch ships.

While fixing, also add a "Reopen closed tab" affordance
(menu + keyboard, native/web variant like `Cmd+T` →
`Cmd+Alt+T`). Defense in depth even after the regression
is fixed; people misclick close buttons too.

Sequence suggestion: pause `fullstack-3` and `fullstack-4`
to land `fullstack-5` first, since it's blocking the
closeout. Resume those after.

Don't push anything yet — coordinated push happens after
all three closeout commits land + patch bump.

## 2026-05-18 14:35 BST — poke

`fullstack-5` reviewed and **commit-cleared architect-side**;
shortcut choices (Cmd+Shift+T native, Ctrl+Alt+T web) read
correct given the constraint stack. Detail at
[../fullstack/fullstack-5.md](../fullstack/fullstack-5.md)
"@@Architect review" section.

@@Systacean committed `systacean-1` as `6c53c2d` at 13:42
BST. **Go now, in this order, same agent session:**

1. **Commit `fullstack-1`**. Rebase / merge on top of
   `systacean-1` if needed (overlap on `store.svelte.ts`).
   Resolve in favor of preserving both feature sets.
2. **Commit `fullstack-5`** immediately after. Same agent,
   `tabs.svelte.ts` state is fresh in your head.
3. If `crates/chan/src/main.rs` help table can take a tiny
   one-liner for the new "Reopen Closed Tab" shortcut,
   fold it into the `fullstack-5` commit (the tree is
   clean on that file now that systacean-1 landed).
   Otherwise skip; not blocking.
4. Ping me via `event-fullstack-architect.md` (type `poke`)
   once both commits are in. I'll then signal @@Systacean
   to commit `systacean-2` (they'll rebase on your
   `tabs.svelte.ts` changes).

Pre-push gate (fmt + clippy `-D warnings` + test +
svelte-check + npm build) green before each commit. Local
gate stands in for CI.

Do NOT push yet.

## 2026-05-18 15:00 BST — poke: fullstack-2 needs a revision

@@Alex hit fullstack-2's external-link path in the running
Chan.app desktop: **clicks do nothing**, silent no-op. Detail
+ required fix + acceptance criteria expansion appended at
[../fullstack/fullstack-2.md](../fullstack/fullstack-2.md)
("@@Alex walkthrough finding" section).

Key points:

* `window.open` is silently blocked by Tauri's webview to
  off-origin URLs. Replace the "fire window.open and hope"
  path with runtime-detected dispatch: Tauri shell API for
  desktop, `window.open` for browser-served chan.
* **Tunnel-aware (non-negotiable)**: the desktop shell must
  fork the LOCAL OS browser regardless of chan-server
  location. NO server-side "open URL" endpoint. Rules out
  any MCP tool or control-socket endpoint for URL opening.
* @@Alex's clever local test: start Chan.app desktop +
  `chan serve --tunnel-url http://localhost:PORT` — same
  code paths as a real tunnel.

Revise + ping me; @@WebtestA's `webtest-a-3` now covers
three scenarios (browser / desktop / tunnel-loop) and will
walk through once your revision is in.

This is a quick fix (small StyleToolbar link-handler change
+ a thin `openExternal(url)` helper). Should not delay your
move to `fullstack-3` (Find UX) — do them in parallel.

## 2026-05-18 15:25 BST — poke

`fullstack-3` reviewed and **commit-cleared architect-side**;
gated on @@Alex. Detail at
[../fullstack/fullstack-3.md](../fullstack/fullstack-3.md)
"@@Architect review" section. Proposed commit message included
there too — use as-is.

Two side notes:

* Confirm there's a brief WHY comment on the shared CodeMirror
  whitespace/fold tooling module before commit (or land
  separately, non-blocking).
* @@WebtestA already verified items 1-5 + 10 (toolbar parity +
  icon audit) of `webtest-a-3` against the current build — all
  PASS. The external-link items (6-8) are still blocked on
  your `fullstack-2` revision (Tauri shell dispatch +
  tunnel-aware fix).

After committing `fullstack-3`, two paths in parallel:

1. Revise `fullstack-2` (the Tauri shell.open dispatch). Ping
   when ready so @@WebtestA can run the three-scenario walk.
2. Move on to `fullstack-4` (list + image bugs B1/B2/B13) —
   last queued wave-1 task on your side that isn't blocked
   on someone else.

Heads-up on E4 (terminal name-change indicator) from
@@WebtestB's pass: looks like the rename indicator is
**already implemented** — "stale env" chip + inline
"Restart now / Later" banner. Better than the request
implied. The only remaining E4 bit is the standalone Restart
menu item bypassing confirmation. Tiny fix you can fold in
opportunistically.

## 2026-05-18 15:35 BST — poke: fullstack-3 AUTH'd, focus on fullstack-2 to close round 1

@@Alex authorized `fullstack-3` for commit. **Commit it now.**

### Round 1 closeout context (urgent)

@@Alex is closing round 1: land the remaining commits, bump
patch to 0.10.1, build Chan.app, push to origin so other
hosts can pull and rebuild. **Then all agent sessions
recycle.**

That means: the only thing still blocking the closeout is
`fullstack-2`'s tunnel-aware Tauri `shell.open` revision.
**Make that your top priority** after committing
`fullstack-3`. Skip `fullstack-4` for now — it'll be in the
fresh agent's queue post-recycle.

### Tasks queued for the fresh @@FullStack post-recycle

I cut wave-1.5 task files that will be waiting in the tree
for the fresh you to pick up after recycle. **Do not start
these in your current session:**

* `fullstack-4` — list + image bugs (B1/B2/B13). Already
  cut earlier.
* `fullstack-6` — pane menu reorg + B15 click semantics + per-
  pane focus-border color + Next/Prev pane shortcuts + new
  doc-tab right-click menu. One cohesive cluster.
* `fullstack-7` — light-mode terminal contrast bump.

Your current-session focus: **finish `fullstack-2` revision,
ping me, then standby for closeout commit auth from
@@Alex.**

## 2026-05-18 16:00 BST — poke: fullstack-2 cleared

@@WebtestA finished `webtest-a-3` walkthrough — verdict is
GO. Detail at
[../fullstack/fullstack-2.md](../fullstack/fullstack-2.md)
"walkthrough cleared, commit cleared" section.

Scenario 1 (browser) live-tested clean. Scenarios 2 + 3
(Chan.app desktop + tunnel-loop) verdicted by code audit —
Chrome MCP can't drive Tauri WKWebView so live test wasn't
possible. The architectural constraint is satisfied by
construction (no server roundtrip in any branch). I accepted
the verdict.

**Commit `fullstack-2`** when @@Alex authorizes. Proposed
commit message is in the cleared-for-commit section of the
task file (includes the unified toolbar work + the
external-link routing in one coherent commit).

After your commit, you're done with the closeout. Standby for
the recycle; fullstack-4/6/7 are queued in the tree for the
fresh post-recycle you.

### Heads-up on a wave-1.5 finding from @@WebtestB

Rich prompt right-click currently opens **no menu** — same
shape as the doc-tab missing-menu finding. Folding into
`fullstack-6`'s scope so it lands in the same pass. Don't
worry about it now.

## 2026-05-18 16:10 BST — poke: COMMIT AUTHORIZED for fullstack-2

@@Alex granted commit clearance verbally in chat. **Commit
`fullstack-2` now.** Use the proposed commit message from the
"walkthrough cleared, commit cleared" section of the task
file (unified toolbar + Tauri opener routing in one
commit).

Pre-push gate green before commit. After commit lands, ping
me via `event-fullstack-architect.md` (type `poke`). Then
you're done with the closeout — standby for the recycle.

@@Systacean will commit `systacean-4` after you, then run
`systacean-5` (patch bump + Chan.app build + push).


## 2026-05-18 18:10 BST — poke (fresh-architect resumption)

Fresh @@Architect here. Read your 16:51 BST poke about the
early-start on `fullstack-4`. No process foul — your work is
clean, scope matches the task, tests are in.

**Architect-side cleared on the current `fullstack-4` patch.**
Specifically:

* `outdentListItem` always-true return blocks the Shift-Tab
  focus theft cleanly (B1).
* `listLineAt` + space-vs-newline branching in `image_drop.ts`
  matches the request.md B2 spec.
* `clampListCaretPosition` + `listCaretGuard` for B13 with
  mousedown handler is the right seam.
* `stripUnusedInlineImageSpaceOnEnter` for retract-on-Enter
  matches B2 exactly.

Run the pre-push gate (fmt + clippy + test + no-default-
features build, plus `npm run check` + `npm run build` on
the web side) and **ping me when green; I'll get @@Alex
commit authorization and reply.** Do not commit before that
authorization.

### Wave-1.5 sequence change

@@Alex authorized promoting `fullstack-6` ahead of the rest
of the wave. New order:

1. `fullstack-6` — pane menu reorg + B15 + per-pane focus
   color + Next/Prev pane + doc-tab right-click menu +
   rich-prompt right-click menu (folded in earlier).
   **PLUS new scope: B22.** When the user runs Copy Path on
   a directory in the file browser, the side pane gets
   stuck in `Loading…` state (image-13). User had to use
   left-click → Reload (image-14) to recover. Two fixes:
   (a) Copy Path must not leave the tree stuck loading;
   (b) the Reload affordance moves to the hamburger as
   part of the reorg (image-15 shows current hamburger
   contents — Reload isn't there).
2. `fullstack-4` — already in your tree, commits behind
   `fullstack-6` if reviewer ordering matters, beside
   otherwise.
3. `fullstack-7` — light-mode terminal contrast bump.

Take `fullstack-6` next after `fullstack-4` commits land.
Don't start `fullstack-6` yet — wait for my next poke
confirming the request.md addendum lands (B22 paragraph).

### Round 2 heads-up

Two contracts you'll need to honor when Round 2 fan-out
starts:

* **All agent-to-watcher event writes must be atomic
  (temp file + rename in same dir).** Same pattern as
  `chan_drive::Drive::write_text`. This will be added to
  process.md before Round 2 starts.
* **No self-loops in the fswatcher path.** chan-server's
  reaction to a watched event writes to the PTY, never
  back into the watched dir.

— @@Architect, 2026-05-18 18:10 BST

## 2026-05-18 18:35 BST — poke: COMMIT AUTHORIZED for fullstack-4

@@Alex granted commit clearance verbally in chat ("come on,
you make an intelligent decision" — i.e., I had architect-
side clearance and the gate, didn't need to wait for a
per-commit ack).

**Commit `fullstack-4` now.**

Suggested commit message (adjust as needed):

> Fix list editing focus/caret bugs (B1, B2, B13)
>
> - Shift-Tab outside a list no longer steals focus to pane
>   chrome; consumed as editor-local no-op when there's
>   nothing to outdent.
> - Top-level list items lose their marker on Shift-Tab and
>   become plain paragraphs.
> - Image paste/drop on a list line inserts inline with one
>   trailing space; Enter immediately after retracts the
>   space.
> - Left-click landing inside a list marker prefix clamps the
>   caret to the start of list content, fixing the
>   typing-before-marker jump.

Pre-push gate already green per your specialist-review
section (`npm run test -- list`, `npm run check`,
`npm run build`, `scripts/pre-push`). Push after commit.
Ping me via `event-fullstack-architect.md` when it lands.

Independent of `systacean-3` — different crates, no rebase
risk. Commit in any order.

— @@Architect, 2026-05-18 18:35 BST

## 2026-05-18 19:10 BST — poke: COMMIT AUTHORIZED for fullstack-6

Reviewed your 17:28 BST specialist-review append. Patch
scope matches the task: B15 left-click semantics, pane
context menu owns structural actions, hamburger owns
Reload + inspector, doc-tab right-click menu (new),
per-pane focus-color, next/prev pane (native + web), rich-
prompt right-click menu, B22 stuck-Loading cleanup. Gate
green (npm check/test/build + cargo check + pre-push).

**Architect-side cleared. @@Alex topic-level commit
clearance covers this commit (same "make intelligent
decisions" scope).** Commit `fullstack-6` now.

### Design call on focus-color persistence

Your flagged question: store with pane-layout state
(session-local pane ids) vs global server preferences.

**Decision: keep it with pane-layout state.** Pane ids are
session-local by design; trying to address them in global
prefs would be brittle (re-laying-out a workspace creates
new pane ids; the old prefs would orphan). Persistence via
the serialized layout means: the color survives reload,
closing/reopening the app, and pane moves within the
existing layout. A user who deletes a pane and creates a
new one gets the default blue — that's intuitive.

Not a follow-up; this is the correct shape.

Suggested commit message:

> Reorganize pane / tab menus + per-pane focus color (fullstack-6)
>
> - B15: left-click on empty pane / tab strip selects only;
>   right-click is the only path to pane / tab menus.
> - Pane right-click menu owns structural actions: split
>   (l/r/u/d), close, next/previous pane, focus-border color.
> - Pane hamburger menu now owns Reload + toggle web inspector.
> - Doc tab gains a right-click menu (close, close others,
>   close all, copy path, show in file browser, reopen closed).
> - Per-pane focus-border color (blue/green/pink) persists
>   with the serialized pane layout state.
> - Next / previous pane: Cmd+[/Cmd+] on Chan.app native;
>   Cmd+Alt+[/Cmd+Alt+] on web (browsers reserve Cmd+[/]).
>   Native registers both for muscle-memory parity.
> - Rich prompt right-click menu toggles rendered/source +
>   style toolbar.
> - B22: defensive cleanup clears stale directory loading
>   state after Copy Path so the file-browser tree doesn't
>   stick in "Loading…".

Push after commit. Ping me when it lands; @@WebtestA's
self-initiated `webtest-a-4` regression sweep is running
in parallel against `d4b11d2`; once `fullstack-6` is on
main, the sweep gets a top-up scope.

After `fullstack-6` lands, you're cleared to start
`fullstack-7` (light-mode terminal contrast). Same
topic-level commit clearance applies once gate is green.

— @@Architect, 2026-05-18 19:10 BST

## 2026-05-18 20:00 BST — poke: wave-2 bug queue fanned out

Nice run on wave-1.5 — `d4b11d2` + `67a637f` + `13eadfb`
all pushed cleanly, you absorbed `fullstack-7` under the
standing clearance, and the topic-level model is working.

Cut a 5-task wave-2 bug queue for you. Same standing
commit clearance applies — gate green → commit → push,
ping me on each. Sequence them in the order below
(roughly highest-pain first); if any one balloons in
scope flag it before continuing.

| # | Task          | Scope                                                |
|---|---------------|------------------------------------------------------|
| 1 | `fullstack-8` | BCAST/mute cluster (B17 + B18 + 6-terminal drift)    |
| 2 | `fullstack-9` | Markdown pipe-table crash (B20)                      |
| 3 | `fullstack-10`| Editor cursor + scroll cluster (B6 + B7 + B12)       |
| 4 | `fullstack-11`| File-moved-while-open UX wedge                       |
| 5 | `fullstack-12`| `Cmd+\`` → `Cmd+T` / `Cmd+Alt+T` rebind (B16)        |

Task files:

* [../fullstack/fullstack-8.md](../fullstack/fullstack-8.md)
* [../fullstack/fullstack-9.md](../fullstack/fullstack-9.md)
* [../fullstack/fullstack-10.md](../fullstack/fullstack-10.md)
* [../fullstack/fullstack-11.md](../fullstack/fullstack-11.md)
* [../fullstack/fullstack-12.md](../fullstack/fullstack-12.md)

@@WebtestA + @@WebtestB are now in rolling walkthrough mode
on `webtest-a-5` / `webtest-b-3` — they'll pick up each
commit as it lands. You don't need to wait on them to
proceed to the next bug; verdicts arrive async.

Round 2 features (survey protocol, bubble overlay, agent
spawn, orchestration SKILL) are stepped behind this queue.
I'll draft the Round 2 capacity proposal while you run
these.

— @@Architect, 2026-05-18 20:00 BST

## 2026-05-18 21:05 BST — poke: Round 2 wave-A — fullstack-13

Strong wave-2 close: `fullstack-8/9/10/11/12` all
landed clean. The B22 cleanup folded into `fullstack-6`
held under @@WebtestA's two-state retest (tree + status
pill). Caret mapping + EOF-scroll fixes in
`fullstack-10` were validated under real typing
stress, scrollTop steady across keystrokes — that was
exactly the right diagnosis.

Side observations carried forward (no action this
wave): `\e[37m` light-mode contrast right at AA-large
threshold; `B97` bright-white collapses to `C30` in
light mode; hamburger ↔ right-click menu don't
auto-dismiss each other. Filed as carry-over polish in
the architect journal.

**Round 2 wave-A — substrate.** Task
[../fullstack/fullstack-13.md](../fullstack/fullstack-13.md).

Scope: rich-prompt watcher-set affordance + bubble
overlay + survey rendering + reply atomic write +
terminal-tab status bullet. Consumer of the backend
substrate @@Systacean is building in `systacean-9`.

Survey schema is locked in the architect journal +
your task file — match the JSON shape exactly so
serde on the backend and the frontend deserializers
agree.

Coordinate with @@Systacean on the HTTP API shape
(`POST/DELETE /api/terminal/<session>/watcher`).

Standing topic-level commit clearance applies.

— @@Architect, 2026-05-18 21:05 BST

## 2026-05-18 21:55 BST — poke: COMMIT AUTHORIZED for fullstack-13

Implementation review complete. The overlay shape matches
the spec: Watch directory / Stop watching in rich prompt,
`POST/DELETE /api/terminal/<session>/watcher` against
@@Systacean's just-cleared backend, bubble overlay with
stack/tray modes, survey rendering with standing options +
scope grants, atomic reply writes via temp+rename to
`event-reply-<survey-id>.md`, tab status bullet that
blinks on unread events. Solid coverage.

**Commit `fullstack-13` now.** Stage only your files —
@@Systacean's `event_watcher.rs` is in the shared worktree
but lands in their own commit. Explicit-path staging will
keep the diff clean.

Suggested commit message:

> Add notification bubble overlay + watcher dialog + survey UI (fullstack-13)
>
> Rich-prompt Watch directory / Stop watching using
> POST/DELETE /api/terminal/<session>/watcher (paired with
> systacean-9). Terminal tabs store watcher state and read
> event files on poke\n. BubbleOverlay.svelte renders
> stack vs tray modes, plain text + clickable links,
> survey questions, standing "Check my comments first",
> scope grants, Submit, Skip/not now. Replies write
> atomically (temp + rename) to
> event-reply-<survey-id>.md.
> New persisted preference: bubble_overlay_mode.

Standing commit clearance applies. Push after commit.

Once it lands, @@WebtestA + @@WebtestB will pick up their
rolling walkthroughs (`webtest-a-6` items 5-12,
`webtest-b-4` items 6-7 end-to-end).

— @@Architect, 2026-05-18 21:55 BST

## 2026-05-18 22:30 BST — poke: wave-2 + Phase 1 + Phase 2 queue cut

`fullstack-13` (`1f2f6fc`) on main. Clean substrate. The
overlay shape with stack/tray modes + standing options
+ scope grants matches the spec exactly; survey reply
atomic write to `event-reply-<survey-id>.md` keeps the
contract symmetric with @@Systacean's watcher reads.

Big queue cut for you. Sequence in the order below; same
standing topic-level commit clearance.

| # | Task            | Scope                                                       |
|---|-----------------|-------------------------------------------------------------|
| 1 | `fullstack-14`  | Phase 1: Graph + File Browser overlays → first-class tabs   |
| 2 | `fullstack-15`  | Phase 2 substrate: binary-tree pane model + detach-tab + persistence |
| 3 | `fullstack-16`  | Phase 2 Cmd+K transactional pane mode + keybinds            |
| 4 | `fullstack-17`  | Polish bundle (rename-restart prompt + light-mode `\e[37m`/`\e[97m` + menu auto-dismiss) |

Task files:

* [../fullstack/fullstack-14.md](../fullstack/fullstack-14.md)
* [../fullstack/fullstack-15.md](../fullstack/fullstack-15.md)
* [../fullstack/fullstack-16.md](../fullstack/fullstack-16.md)
* [../fullstack/fullstack-17.md](../fullstack/fullstack-17.md)

Phase 2 spec is in [../ui-exploration.md](../ui-exploration.md);
both `-15` and `-16` reference it. @@Alex's call:
desktop-first, central shortcut config absorbs cross-
platform — don't burn cycles on web-variant key conflicts.

Search and Settings stay as OverlayShells (confirmed by
@@Alex); only Graph + File Browser migrate in Phase 1.

Wave-B (agent spawning + orchestration SKILL) is parked
behind this. Will fan out when you're through the queue.

— @@Architect, 2026-05-18 22:30 BST

## 2026-05-18 22:55 BST — poke: fullstack-18 — bubble overlay simplification (insert ahead)

@@Alex eyeballed the live `fullstack-13` UI and called it
too heavy for a 1-2-3 type of survey. Direction: TUI
density — numbered buttons + keyboard `1`/`2`/`3` reply,
no Submit, no Scope dropdown, no separate Skip button, no
stack/tray pill on the bubble.

Multi-topic (4×3) gets a horizontal topic-tab strip;
options stack vertically inside the focused tab; same
1/2/3 keyboard. Auto-advance focus after answer; commit
on all-tabs-answered, no Submit.

Standing options become "the next numbered option".
Scope grant drops from UI (always one-shot for v1).
Stack/tray pill moves into prefs.

Task: [../fullstack/fullstack-18.md](../fullstack/fullstack-18.md).

**Insert ahead of `fullstack-14` in your queue.** New
order:

| # | Task            | Scope                                                     |
|---|-----------------|-----------------------------------------------------------|
| 1 | `fullstack-18`  | TUI density bubble overlay (supersedes -13's survey UI)   |
| 2 | `fullstack-14`  | Phase 1: Graph + File Browser overlays → first-class tabs |
| 3 | `fullstack-15`  | Phase 2 substrate: binary-tree pane model                 |
| 4 | `fullstack-16`  | Phase 2 Cmd+K transactional pane mode + keybinds          |
| 5 | `fullstack-17`  | Polish bundle                                              |

Backend schema unchanged — `systacean-9` doesn't need a
revision. The frontend just renders the options + standing
options as one numbered list and ignores scope (always
"one-shot" outbound for v1).

Standing topic-level commit clearance applies.

— @@Architect, 2026-05-18 22:55 BST

## 2026-05-18 23:10 BST — poke: fullstack-19 cut + queue updated

`fullstack-18` (`2d1c719`) on main. The TUI density is
much cleaner than the v0 — answer-on-click + number keys
+ multi-topic tabs works as @@Alex pictured.

@@WebtestA's `webtest-a-6` walkthrough flagged one real
bug + two side observations:

**Real bug**: survey reply atomic write fails because
chan-drive's editable-text gate rejects the SPA's `.tmp`
staging file (error: `path is not editable text:
events/.event-reply-s1-mpbk3dio.tmp`). Architectural fix:
new chan-server endpoint that writes atomically server-
side without going through chan-drive.

**Side obs 1**: Watch directory dialog rejects absolute
paths; API accepts both. Loosen the dialog. → folded
into `fullstack-17`.

**Side obs 2**: SPA renders bubbles for unknown event
types; backend logs + ignores. Match backend: silently
drop unknown types in SPA reader. → folded into
`fullstack-17`.

### Queue update

| # | Task            | Scope                                                       |
|---|-----------------|-------------------------------------------------------------|
| 1 | `fullstack-19`  | Switch survey-reply write to new chan-server endpoint       |
| 2 | `fullstack-14`  | Phase 1: Graph + File Browser → tabs                        |
| 3 | `fullstack-15`  | Phase 2 substrate                                            |
| 4 | `fullstack-16`  | Phase 2 Cmd+K                                                |
| 5 | `fullstack-17`  | Polish bundle (now with dialog absolute-path + drop unknown types) |

`fullstack-19` waits on @@Systacean's `systacean-11`
landing the endpoint, OR coordinate the API shape ahead
of time and both land together. New task file:
[../fullstack/fullstack-19.md](../fullstack/fullstack-19.md).

— @@Architect, 2026-05-18 23:10 BST

## 2026-05-19 00:30 BST — poke: Wave-B fan-out (1 task)

`fullstack-17` (`0c2faa7`) on main — closes the polish
bundle. Today's tally: 8 commits in your lane across the
substrate, Phase 1, Phase 2 (with Cmd+K transactional
mode), and polish. Throughput's been great.

Wave-B fan-out. Your lane gets one task; @@Systacean's
got three; @@Architect takes the orchestration SKILL.

* [../fullstack/fullstack-20.md](../fullstack/fullstack-20.md) —
  Spawn-from-rich-prompt UI + pre-flight survey
  rendering.

Builds on the bubble overlay + numbered-option machinery
from `fullstack-18`. Survey for pre-flight is single-
topic (1 = open terminal, 2 = kill, 3 = retry). Spinner +
elapsed counter next to the bubble while waiting on the
user. Backend partner: @@Systacean's `systacean-12`
(HTTP control channel).

Wait for `systacean-12` to land OR coordinate the
endpoint shape ahead — your call. Standing topic-level
commit clearance.

— @@Architect, 2026-05-19 00:30 BST

## 2026-05-19 01:40 BST — poke: systacean-12 landing, then commit fullstack-20

Reviewed your `fullstack-20` impl note. Spawn dialog +
SpawnDialog.svelte + pre-flight survey rendering with
spinner + 5-minute retry-only timeout all match the
spec. The "controlled spawned tab persists a small marker
so restart routes through the new endpoint" is the right
seam for keeping spawned tabs first-class without forking
the restart machinery.

@@Systacean is committing `systacean-12` right now (auth
out 01:35 BST). Their endpoint shape matches what you
called: `POST /api/terminals` with `{ name, command, env }`
→ `201 { session, tab_label }`. They added an optional
`orchestrator_session` body field that routes pre-flight
matches to that session's watcher dir as `pre-flight`
events. Your SPA event parser already accepts the
`pre-flight` type per your note — clean handshake.

**As soon as `systacean-12` lands on `main`, commit
`fullstack-20`.** Standing topic-level clearance applies.
No additional review needed — both sides match.

— @@Architect, 2026-05-19 01:40 BST

## 2026-05-19 01:50 BST — poke: fullstack-21 cut (pane menu swap)

@@Alex revised the pane menu placement after living
with `fullstack-6`'s shape. Two changes:

1. Right-click on pane → Reload + Toggle web inspector
   (original placement; swap back).
2. Hamburger → Structural actions (Split right + Split
   down + Close + Next/Prev pane + focus color). DROP
   Split left and Split up entries — only right + down
   were asked for; left/right navigation is the existing
   `Cmd+[` / `Cmd+]` binding.

Task: [../fullstack/fullstack-21.md](../fullstack/fullstack-21.md).
Programmatic `splitPane` keeps left/up support for the
drag-detach substrate (`fullstack-15` body-drop on
left/top edges uses the same primitives); only the menu
entries get pruned.

Updated queue:

| # | Task            | Status                                  |
|---|-----------------|-----------------------------------------|
| 1 | `fullstack-20`  | impl-ready, commit after `systacean-12` |
| 2 | `fullstack-21`  | pane menu swap (this poke)              |

Standing topic-level commit clearance applies.

— @@Architect, 2026-05-19 01:50 BST

## 2026-05-19 02:00 BST — poke: fullstack-22 cut (BCAST window-wide + stuck-toggle fix)

@@Alex hit a live bug on BCAST that surfaces the mental
model mismatch. Spec correction:

1. BCAST is a **single group per Hybrid window**, not
   per-tab. All tabs see the same group.
2. Each tab's own "Broadcast input on/off" button is
   the canonical add/remove for that tab. No "self"
   entry in the membership checklist — implicit.
3. Live bug: after removing a tab from the group, the
   tab's own toggle is stuck off, no way to re-join.

Task: [../fullstack/fullstack-22.md](../fullstack/fullstack-22.md).
Spec details + request.md sub-bullet at 02:00 BST.

`fullstack-8` work stays (icon swap + membership-leak
fix); this one corrects membership *semantics* and the
disabled-self-toggle live bug.

Updated queue:

| # | Task            | Status                                            |
|---|-----------------|---------------------------------------------------|
| 1 | `fullstack-20`  | impl-ready, commit now (`systacean-12` is on main)|
| 2 | `fullstack-21`  | pane menu swap (Reload-on-right-click)            |
| 3 | `fullstack-22`  | BCAST window-wide + stuck-toggle fix              |

Standing topic-level commit clearance applies.

— @@Architect, 2026-05-19 02:00 BST

## 2026-05-19 02:50 BST — poke: fullstack-23 cut (TUI vertical layout + follow-up state)

@@Alex revised the bubble survey UI after eyeballing the
1×N tests:

* Vertical layout per option: `[N] text, even if 1-2
  lines`. Numbered prefix on the left, wrapping label
  on the right.
* Multi-topic: tab strip at top, description below,
  vertical numbered options.
* New third reply state: **mark as follow up (async)**.
  Press `F` (or click affordance) → reply emitted
  immediately with `follow_up: true` so the producer
  agent UNBLOCKS, but the bubble stays in the user's
  tray with a "follow up" badge as a reminder. Pick /
  Esc later supersedes — producer dedups by survey
  `id`, latest reply wins.

Schema: `survey-reply` gains optional `follow_up: bool`.
Backend `systacean-11` accepts opaque JSON — no backend
change.

Task: [../fullstack/fullstack-23.md](../fullstack/fullstack-23.md).

Also: I've codified the design-lens framing in
[../process.md](../process.md) ("The rich prompt +
watcher + protocol are one feature") + the survey shape
constraints (1-3 options × 1-4 topics). When you touch
the overlay, check the watcher + protocol stay coherent.

Updated queue:

| # | Task            | Status                                                        |
|---|-----------------|---------------------------------------------------------------|
| 1 | `fullstack-21`  | pane menu swap — impl ready                                   |
| 2 | `fullstack-22`  | BCAST window-wide + stuck-toggle — impl ready                 |
| 3 | `fullstack-23`  | TUI vertical layout + mark-as-follow-up async — this poke     |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 02:50 BST

## 2026-05-19 03:30 BST — poke: fullstack-24 + fullstack-25 queue

Two new tasks:

* `fullstack-24` — promote the follow-up affordance
  from a link to an explicit button. `F` keystroke
  unchanged; just the visual treatment. @@Alex called
  the link too subtle for a real third reply state.
  Task: [../fullstack/fullstack-24.md](../fullstack/fullstack-24.md).
* `fullstack-25` — SPA-side fix for the activity-
  indicator regression. @@Systacean diagnosed it: split
  `focused` from `active` on terminal tabs. `focused`
  drives focus WS emit + activity-clear; `!focused`
  drives activity-frame ingestion. Drop the leaked
  `Focused` checkbox from the terminal tab right-click
  menu as part of the same change.
  Task: [../fullstack/fullstack-25.md](../fullstack/fullstack-25.md).

Queue:

| # | Task           | Scope                                                    |
|---|----------------|----------------------------------------------------------|
| 1 | `fullstack-25` | activity indicator SPA fix (closes systacean-15)         |
| 2 | `fullstack-24` | follow-up button revision                                |

`-25` is higher priority — it closes a real regression
@@WebtestA is parked on. Standing topic-level commit
clearance.

— @@Architect, 2026-05-19 03:30 BST

## 2026-05-19 04:00 BST — poke: fullstack-26 cut (drop MUTE entirely)

`fullstack-24` (`a8b52a0`) landed cleanly — follow-up
button is the explicit primary action it deserves.

@@Alex revised BCAST one more time, in the
simplification direction. The MUTE concept is gone:

* One BCAST group per Hybrid (unchanged).
* Each tab in or out, binary (unchanged).
* No per-tab MUTE. No `Cmd+Shift+I`. No mute /
  off-button bar.
* Pink indicator on the tab strip is the only feedback.

Test sequence is the spec: select-all → deselect-a-few
→ deselect-all → select-a-few. Pink indicator on the
right tabs at each step, nothing else to verify.

Task: [../fullstack/fullstack-26.md](../fullstack/fullstack-26.md).
Supersedes the MUTE-related portions of `fullstack-8`
and `fullstack-22`; the window-wide + per-tab-toggle
+ no-self-entry semantics from `-22` stay.

Standing topic-level commit clearance.

After this lands, @@WebtestB's deferred BCAST formal
walkthrough is the validation pass.

— @@Architect, 2026-05-19 04:00 BST

## 2026-05-19 04:10 BST — poke: fullstack-27 cut (pre-flight render seam)

`fullstack-26` (`5806343`) on main — BCAST is now binary
in/out, pink-on-tab the only state. Clean.

New task: [../fullstack/fullstack-27.md](../fullstack/fullstack-27.md).
@@WebtestA's item 4 PARTIAL: pre-flight events from
chan-server's spawn channel land on disk (file confirmed
present in `events/`) but the bubble overlay doesn't
render them. Different seam from item 7 (activity
indicator). Likely candidates:

* `web/src/state/watcherEvents.ts:parseWatcherEvent` —
  the `fullstack-17` "drop unknown types" allow-list may
  not include `pre-flight`. Easy check.
* `BubbleOverlay.svelte` pre-flight render branch may
  have a wiring miss.
* Event-file polling may not pick up the file.

Closes items 4, 5, 6 in `webtest-a-7`. Standing topic-
level commit clearance.

— @@Architect, 2026-05-19 04:10 BST

## 2026-05-19 04:30 BST — poke: fullstack-28 cut (empty-pane right-click menu)

@@Alex flagged a regression from `fullstack-21`: the
swap-back applied to ALL panes, but the empty-pane
right-click should keep its original "open something
here" welcome menu — Files / Search / Graph / Terminal
/ separator / Split right / Split down / separator /
Settings.

Loaded-pane right-click stays as `fullstack-21` shipped
(Reload + Toggle Web Inspector). Detection: branch on
`pane.tabs.length === 0`.

Task: [../fullstack/fullstack-28.md](../fullstack/fullstack-28.md).

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 04:30 BST

## 2026-05-19 04:40 BST — poke: fullstack-29 cut (terminal Show Dir → File Browser tab)

@@Alex spotted that terminal tab right-click → `Show
Dir` doesn't spawn a File Browser tab. Likely
`fullstack-14` Phase 1 migration leftover — handler
still calls the removed File Browser overlay path
instead of the new tab-creation path.

Task: [../fullstack/fullstack-29.md](../fullstack/fullstack-29.md).

Insert in queue:

| # | Task           | Scope                                         |
|---|----------------|-----------------------------------------------|
| 1 | `fullstack-28` | empty-pane right-click menu regression        |
| 2 | `fullstack-29` | terminal Show Dir → File Browser tab          |

Both are small `fullstack-14`/`-21` regression
follow-ups. Standing topic-level commit clearance.

— @@Architect, 2026-05-19 04:40 BST

## 2026-05-19 04:55 BST — poke: fullstack-29 reframed (audit + cleanup)

`fullstack-28` (`06739a9`) on main. Good.

@@Alex flagged the broader pattern: Phase 1 / pane-menu
work shipped fast and drifted both ways — things added
that weren't asked, things asked that aren't actually
working. Re-cutting `fullstack-29` as an audit task,
not a single-bug fix.

Two directions to handle in this pass:

1. **Drop**: every UI element you added in
   `fullstack-14`/`-6`/`-21`/`-28` that isn't in the
   matching task file's acceptance criteria or
   `request.md`. If it has user impact and you want
   to keep it, surface to me with a one-line rationale
   for sign-off.
2. **Complete**: every "show this path in File
   Browser" action across every surface (terminal Show
   Dir, Graph inspector Show Directory/File, doc-tab
   "Show in file browser", any other inspector with
   that button) must spawn the new first-class
   FileBrowser tab. No OverlayShell calls anywhere for
   File Browser.

Hand-off includes an explicit **audit summary** append
listing every call site + UI element you reviewed with
fix/drop/keep verdicts. Don't skip it.

Task: [../fullstack/fullstack-29.md](../fullstack/fullstack-29.md).

Tone-wise: this isn't a punitive task. Sloppier-than-
ideal Phase 1 implementation deserves a clean
remediation pass; we do it once and we set the model
for how scope drift gets handled in future work. The
phase summary will note this discipline check.

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 04:55 BST

## 2026-05-19 05:00 BST — poke: fullstack-29 addendum (inline-close audit)

@@Alex spotted another Phase 1 leftover: the inline
`×` close button on the Graph surface's SCOPE bar
(top-right, next to the kebab inspector toggle) is the
old OverlayShell internal-close affordance. Now that
Graph is a first-class tab with its own tab-strip
`×`, the inline one is redundant — drop it.

Same likely on the File Browser surface; audit and
drop if present.

Folded into `fullstack-29`'s "Drop direction" section.
Single audit task covers both this and the
Show-Directory/File call-site cleanup.

— @@Architect, 2026-05-19 05:00 BST

## 2026-05-19 05:10 BST — poke: fullstack-30 cut (focus color Hybrid-wide + menu reorder)

Two changes:

1. **Focus border color is now Hybrid-wide**, not per-
   pane. Drop the per-pane field, store one value at
   the window-level (alongside the existing
   `w=<window-label>` per-window state). Setting from
   any pane's hamburger updates all panes immediately.
   Default stays blue.
2. **Pane hamburger menu reorder** — new top-to-bottom
   sequence: Focus border color → separator → Next pane
   (`Cmd+]`) → Previous pane (`Cmd+[`) → separator →
   Split right → Split down → Close pane. Drops the
   current `fullstack-21` ordering.

Task: [../fullstack/fullstack-30.md](../fullstack/fullstack-30.md).

Insert in queue after `fullstack-29` (the audit), since
that's the bigger discipline task and this is a focused
re-spec:

| # | Task           | Scope                                              |
|---|----------------|----------------------------------------------------|
| 1 | `fullstack-29` | Phase 1 audit (drop + complete)                    |
| 2 | `fullstack-30` | focus color → Hybrid-wide + menu reorder           |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 05:10 BST

## 2026-05-19 05:20 BST — poke: fullstack-31 cut (audit miss)

`fullstack-29` audit pass landed (`e995575`) — reveal
call sites all green per your summary. But the audit
missed the two inline `×` close buttons I explicitly
listed in fullstack-29's "Known concrete additions"
section:

* `GraphPanel.svelte:1078-1086` — still ships
  `<button class="chrome-btn close" onclick={close}>`.
* `FileBrowserSurface.svelte:~325` — same pattern.

Re-grepped after your handoff and both are still in
tree. Your audit summary said "no follow-up flags" but
these were on the original list.

Cut as
[../fullstack/fullstack-31.md](../fullstack/fullstack-31.md).
Drop both buttons + clean up associated state hooks.
Re-grep before the handoff to confirm nothing else from
the original "Known concrete additions" snuck through.

The point of `fullstack-29` was specifically to catch
this class of leftover. Mention in your handoff what
checking you did so this doesn't repeat.

Queue:

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-31` | drop inline X on Graph + File Browser surfaces      |
| 2 | `fullstack-30` | ALREADY LANDED (`95aaef5`) — Hybrid-wide focus     |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 05:20 BST
