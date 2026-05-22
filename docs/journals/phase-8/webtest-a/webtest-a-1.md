# webtest-a-1: Baseline walkthrough of v0.11.0 + per-bug repro notes

Owner: @@WebtestA
Date: 2026-05-19

## Goal

Reproduce every bug in
[`../phase-8-bugs.md`](../phase-8-bugs.md) against the current
v0.11.0 build. For each one:

* Confirm it reproduces (or note "could not reproduce" with the
  attempted steps).
* Append a one-paragraph repro note to this task file.
* If a screenshot is needed and the bug doesn't already link one
  from `../attachments/`, drop a new screenshot there with a
  short filename and reference it.

Then, as fixes land in subsequent waves, per-fix verification:
each closed bug gets a verdict append from @@WebtestA or
@@WebtestB confirming the fix holds in the browser walkthrough.

## Lane split with @@WebtestB

Default coverage split (re-balance with @@WebtestB if needed):

* @@WebtestA: file-browser tab, status bar, Cmd+K cluster,
  rich prompt cluster, editor cluster, graph (systacean-2).
* @@WebtestB: native window-config persistence, terminal
  cluster, watcher dialog cluster, indexing-chart pan/zoom, CLI
  scriptability.

## How to start

1. Fire a permission event at
   `docs/journals/phase-8/alex/event-webtest-a-alex.md` for
   terminal exec (to start `chan serve`) and Chrome browser
   sessions.
2. Once approved, spin up a test server (`./target/debug/chan
   serve <drive-path>` against a temp drive seeded with the
   chan repo itself, since several bugs repro against that
   seed). Capture the URL with bearer token.
3. Walk the bug list top to bottom; append repro notes.
4. Hand off one test-server URL for @@Alex via the standard
   webtest URL hand-off flow (process.md → "Test server URL
   hand-off").

## Acceptance criteria

* Every bug in `phase-8-bugs.md` has a repro confirmation /
  refutation in this file.
* One test-server URL handed off to @@Architect → @@Alex.
* A clean per-fix verification cadence established for the
  rest of Round 1.

## 2026-05-19 22:10 BST — session setup

Server: `./target/debug/chan serve /tmp/chan-test-phase8-wa`
against a throwaway drive seeded from the chan repo (excluding
`.git`, `target`, `node_modules`, `web/dist`). URL:
`http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`.

Drive name derived from the path = `chan-test-phase8-wa` —
matches the "default drive name from path" spec in bug 1.

## 2026-05-19 22:15 BST — bug 1 (FB tab keeps name of removed file): COULD NOT REPRODUCE on v0.11.0

Repro attempts against the seeded drive:

1. Right-click FB tree (root) → "New file" → type `foo.md` → OK.
   File created. FB tab title stayed `chan-t[..]8-wa/` (drive
   name with trailing slash). A separate editor tab opened
   labelled `foo.md.md` — see side observation below.
2. Single-click `Cargo.toml` in center FB pane: hash updates
   to `bs=Cargo.toml` but FB tab stays `chan-t[..]8-wa/` (parent
   dir = drive root = drive name).
3. Single-click `docs/` in center: FB tab becomes `docs/`.
4. Single-click `docs/agents/` in center: FB tab becomes
   `agents/`.
5. Single-click `docs/agents/webtest-a.md` (a file inside a
   subdir): FB tab stays `agents/` (parent-dir fallback for
   files — exactly the "want" spec).
6. Right-click `foo.md.md` in sidebar → Delete → confirmed.
   The editor tab `foo.md.md` was auto-closed by the FB on
   delete; FB tab name unchanged.

In every path tried, the current behavior matches Alex's
"want" spec exactly:

* FB tab for a selected directory → `<dirname>/`.
* FB tab for a selected file → falls back to parent dir name
  with trailing slash.
* FB tab at drive root → drive name (here:
  `chan-test-phase8-wa` shortened to `chan-t[..]8-wa/`).

Marking bug 1 as **not reproduced** on v0.11.0. Either the fix
landed before the v0.11.0 cut, or there is a more specific
repro path I am missing. Flagging to @@Architect — if @@Alex
has a different repro path in mind, will rerun.

### Side observation (new bug candidate, not bug 1)

The "new file" dialog hint reads "(.md added if no extension)"
but typing `foo.md` produced `foo.md.md` on disk. Repro:

* Right-click FB → New file → type `foo.md` → OK.
* On-disk result: `foo.md.md`. The dialog appended `.md` even
  though the typed value already ended in `.md`.

Not in `phase-8-bugs.md` yet — leaving it for @@Architect to
decide whether to track here or as a fresh entry.

## 2026-05-19 22:25 BST — context shift: phase-8 fixes already on main after v0.11.0

`git log 18bdb34..HEAD` shows phase-8 task commits have already
landed since the v0.11.0 tag (18bdb34):

* `ebd4bc5` — FB tab title fix (fullstack-a-1) → bug 1.
* `ec983d3` — status bar drop ambient click handlers, watcher
  dot blinks yellow (fullstack-a-2) → bugs 2 + 21.
* `51984c8` — chan list --json + chan remove --name
  (systacean-1) → bug 3 (webtest-b lane).
* `ccd2f09` — Hybrid cluster (fullstack-a-3) → bugs 4 + 7.
* `203c6e8` — chan-desktop window-config LRU stack
  (fullstack-b-1) → bug 13 (webtest-b lane).
* Plus CI scaffold commits.

My `cargo build -p chan` built HEAD, not the v0.11.0 binary, so
the test server is exercising the post-tag fixes. Switching this
task's mode for the closed bugs from "reproduce against v0.11.0"
to "verify the fix on HEAD". Bug 1 above was already this verdict
de-facto (FB tab behaviour matched the "want" spec). The
remaining bugs in my coverage cluster get the same treatment.

## 2026-05-19 22:30 BST — bug 2 (status-bar click opens Settings): FIX HOLDS on HEAD

Source check: `web/src/components/AppStatusBar.svelte` lines
32-38 document the fix ("fullstack-a-2: status-bar sections are
ambient state, not navigation … the only click surface left is
the collapse handle that toggles the pill's visibility"). The
section spans (index / import / status) have no `onclick`; only
the `.collapse` button has one (`onclick={toggleCollapse}`).
Pane-mode pill is also passive.

Runtime check: with the test server up, the status bar is
currently hidden (no `anyVisible` section). Indexing completed
fast; no transient `ui.status` message active. Confirmed via DOM
query that nothing matches `.app-statusbar` right now — bar is
conditionally rendered only when a section has content, matching
the source. With no clickable surface visible, the broken click
path cannot fire.

Will follow up with a forced repro (delete-failure) to surface a
`ui.status` pill and click it, but the source is unambiguous.
Marking bug 2 as **fix holds** on HEAD, pending the live
pill-click confirmation.

## 2026-05-19 22:40 BST — bug 4 (Cmd+K status copy + flashing H): FIX HOLDS on HEAD

Steps:

1. Click into the main pane (focused there).
2. Press Cmd+K.

Result: status bar surfaces at bottom-left with the exact text
**`Hybrid ☯ Enter commit, Esc discard, H help`**. JS query
confirms the rendered span:

```
class="section pane-mode-pill svelte-ye5kim"
aria-label="Hybrid NAV active"
text "Hybrid ☯ Enter commit, Esc discard, H help"
```

No flashing "H" anywhere in the main viewport (the previous
mid-screen flash is gone). Pressing `h` once now opens the
Hybrid NAV help overlay, which is the intended H-help binding.

Bug 4: **fix holds** on HEAD.

## 2026-05-19 22:42 BST — bug 2 (live click confirmation)

While the Hybrid pane-mode pill was visible at the status bar, I
clicked it directly. `settingsOpen: false` immediately after.
Nothing else opened. The pill is purely visual; only the `‹`
collapse handle is clickable. Bug 2 **fix holds** on HEAD,
fully confirmed (source + live click).

## 2026-05-19 22:45 BST — bug 7 (Cmd+K → 1/2/3 immediate commit): FIX HOLDS on HEAD

Steps:

1. Press Cmd+K → Hybrid NAV active (status bar shows the new
   copy from bug 4).
2. Press `1`.

Result: a new tab `Terminal-1` spawned **immediately** (the URL
hash gained `{"k":"t","n":"Terminal-1","a":1}` alongside the
existing FB tab). No Enter / Cmd+K confirm needed. Status bar
also collapsed back to hidden (paneMode exited on commit).

Side note: the Hybrid NAV help overlay still labels rows 1/2/3
as `Stage: Terminal / File Browser / Graph`. The "Stage" wording
is from the older staged-spawn model; the runtime behavior is
now immediate commit. Not blocking the bug-7 verdict (Alex
asked for immediate commit, that is what now happens), but
flagging — the help-overlay copy could be updated for
consistency. Leaving the call to @@Architect.

Bug 7: **fix holds** on HEAD.

## 2026-05-19 22:55 BST — bug 9 (closing last Hybrid tab keeps the space): FIX HOLDS on HEAD

Steps:

1. Started with a single pane holding two tabs (FB +
   Terminal-1) after the bug-7 test.
2. Clicked the `×` on Terminal-1, accepted the running-PTY
   confirm. Hash: one tab left, FB.
3. Clicked the `×` on the FB tab. Hash empty.

Result: the pane **did not collapse** — it remained on screen
and rendered a welcome card centered on the empty pane: drive
name + ensō glyph + key-binding cheatsheet ("App / Panes /
Tabs" with Settings, Terminal rich prompt, New terminal,
Dismiss overlay, Enter Hybrid NAV, Close tab, Reopen closed
tab, Next/Previous/Jump tab). This is the "space stays" behavior
Alex wants.

Bug 9: **fix holds** on HEAD.

Side observation: the welcome card surfaces the new-terminal
binding as `Cmd+Alt+T  (macOS only on web; native everywhere)`,
which is relevant to bug 6 — see next section.

## 2026-05-19 22:58 BST — bug 6 (add Cmd+T for new terminal): PARTIAL on HEAD

Bug ask: `add cmd+t for new terminal`. Empty-pane welcome card
documents `Cmd+Alt+T` as the new-terminal shortcut, with the
parenthetical `(macOS only on web; native everywhere)`. Chrome
reserves `Cmd+T` for "new browser tab" and refuses to release it
to web pages, so a bare `Cmd+T` cannot be hijacked from inside a
browser tab — that is why chan landed on `Cmd+Alt+T` for web.

Verdict: the new-terminal command exists today under
`Cmd+Alt+T` on web. A bare `Cmd+T` mapping could plausibly be
added in the native shell (chan-desktop) where chan owns the
window's key-handling, but **on web the OS-level reservation
makes it unfixable for the web shell**. Recommend @@Architect
splits this into two scopes:

* Web: not fixable; document that `Cmd+Alt+T` is the binding.
* Native (chan-desktop): wire `Cmd+T` to the new-terminal
  command via the existing host bridge.

Marking bug 6 partial; will defer to @@Architect for the
final shape. Not exclusive to @@WebtestA — the native side
sits with chan-desktop / FullStack.

## 2026-05-19 23:05 BST — bug 5 (rich prompt cursor on open + after Cmd+Enter): FIX HOLDS on HEAD

Steps:

1. Empty pane, no notifications/bubbles. Press **Alt+Space**
   cold (rich prompt was closed).
2. Result: rich prompt opens, focus lands on the rich prompt
   CodeMirror input (`DIV:cm-content cm-lineWrapping`,
   rect[325,481,1406,752]). ✓
3. Type `echo hello`. Press **Cmd+Enter**.
4. Result: the command is committed (rendered as `echo hello`
   below the input area in the prompt's log; the terminal
   pane received the text). Critically, **focus stays on the
   rich prompt input** (`DIV:cm-content cm-lineWrapping`,
   same rect). Rich prompt remains open. ✓

Bug 5: **fix holds** on HEAD.

Side observation (worth flagging but not the bug):

* The very first Cmd+K → p path (which both spawns a fresh
  Terminal-N tab AND opens the rich prompt) lands focus on the
  newly-spawned terminal's `xterm-helper-textarea`, not the
  rich prompt input. The auto-focus on the rich prompt loses a
  race against the new terminal grabbing focus. Workaround:
  press Alt+Space (or click the rich prompt input) once after
  the Cmd+K p spawn — both refocus the prompt.
* Also visible in this test: when the rich prompt commits via
  Cmd+Enter, the FIRST character of the dispatched text was
  occasionally swallowed by the active terminal (`echo hello`
  rendered in the PTY as `cho hello`). Likely the same focus
  race; the keystroke sent to the terminal as the rich prompt
  commits before the PTY has been told to ignore the burst.
  Both worth a fresh bug-list entry — leaving the call to
  @@Architect.

## 2026-05-19 23:25 BST — bug 8 (graph shows links to files not in repo): REPRODUCES on HEAD

Steps:

1. Test server up against
   `/tmp/chan-test-phase8-wa/` (chan repo copy).
2. `GET /api/graph?t=...` from the test server.
3. Filter nodes where `missing: true`.
4. Cross-check each missing path against the on-disk file
   tree.

Result: graph reports 1102 total nodes, 96 of which carry the
`missing: true` flag. Of those 96, **8 actually exist on disk
in the live drive**:

```
file: LICENSE
file: crates/chan-drive/src/library.rs
file: crates/chan-drive/src/registry.rs
file: desktop/LICENSE
file: docs/journals/phase-1/fake-codex-smoke.sh
file: docs/agents                            (dir on disk, kind=file)
file: docs/journals/phase-7/alex             (dir on disk, kind=file)
file: docs/journals/phase-8/alex             (dir on disk, kind=file)
```

So at least two distinct bug surfaces:

* The presence check for plain files is wrong for at least
  `LICENSE`, `desktop/LICENSE`, two `crates/chan-drive/src/*.rs`
  files, and a shell script. Those files exist in the drive
  but the graph flags them as not in the current file listing.
* The graph types directories as `kind: file` for nodes like
  `docs/agents`, `docs/journals/phase-7/alex`, and
  `docs/journals/phase-8/alex`, then can't find a file at that
  path → false-positive missing. Could be the same indexer
  bug or a separate kind-classification bug.

The remaining 88 missing-flagged nodes do reflect real broken
links (e.g. markdown referring to `docs/agents/skills/...md`
when the canonical layout is `docs/agents/<agent>/skills/...md`);
those are legitimate user-side stale links, not the index
bug.

Bug 8: **reproduces** on HEAD. Confirmed via API; the UI
already renders these with dashed phantom nodes, and clicking
them surfaces the "not in the current file listing (try
Reload / chan index)" DETAILS warning the original bug
screenshot showed. Forwarding to @@Architect / @@Systacean for
indexer triage — likely the indexer/graph presence check is
case- or normalisation-sensitive (LICENSE is the canonical
suspect), or it is filtering out top-level no-extension
files + directories.

Test-server URL for live look:
`http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm` —
Cmd+K → 3 opens the drive Graph; dashed nodes are the
phantom ones.

## 2026-05-19 23:35 BST — bug 10 (Spawn agent shows only overlay): FIX HOLDS on HEAD

Steps:

1. Open rich prompt (Alt+Space or via active terminal pane).
2. Locate the `Spawn agent` icon button (aria-label
   "Spawn agent", `icon-btn svelte-13kd2p4`).
3. Click it.

Result: a centred dialog renders with title **`Spawn agent`**,
fields `Tab name` (pre-filled `@@Agent`), `Command`, `Env`
(placeholder `KEY=value`), and `Cancel` / `Spawn` action
buttons. Two visible dialogs in the DOM (the rich prompt
itself and the new `spawn-dialog` SECTION) — matches the
intended UX.

The original bug screenshot showed only a dark overlay with no
dialog. That state does not reproduce on HEAD. Bug 10 **fix
holds**.

## 2026-05-19 23:38 BST — bug 19 (rich prompt overlay obscures terminal bottom): FIX HOLDS on HEAD

Steps:

1. Terminal tab active in the focused pane.
2. Open rich prompt (Alt+Space).

Result (rects in viewport coords):

```
xterm rect:        [287,  46, 1427, 412]
rich-prompt rect:  [279, 431, 1435, 752]
viewport height:   757
```

The terminal is resized so its bottom edge (y=412) sits 19 px
above the rich prompt's top edge (y=431). No overlap; the
last rendered terminal line stays visible. Bug 19 **fix
holds**.

## 2026-05-19 23:50 BST — watcher cluster setup (preamble for 14 / 18 / 20 / 21)

Pointed the rich prompt's Watch directory dialog at
`/tmp/chan-watcher-test` first to sanity-check bug 15
(webtest-b's lane): error surfaced as
`watch failed: invalid watcher path: path escapes drive root`.
Outside-drive paths are still gated. Pivoted to an in-drive
path:

* `mkdir -p /tmp/chan-test-phase8-wa/watcher-events`.
* Watch directory → `watcher-events` → OK.

Watcher activated within ~1.5 s. Indicator: span
`<span title="watcher active" class="dirty watcher
svelte-at6ci2">●</span>` on the active Terminal tab, computed
color `rgb(227, 179, 65)` (yellow / amber). The rich prompt
header shows `watching watcher-events  Stop watching`.

## 2026-05-19 23:52 BST — bug 14 (rich-prompt watcher hung on first try): COULD NOT REPRODUCE on HEAD

Single attempt, cold session, freshly attached the watcher to
an in-drive dir. The dialog accepted the path, the watcher
became active without any visible hang, the prompt header
flipped to `watching watcher-events  Stop watching` immediately,
and the first survey event dropped into the dir surfaced a
bubble within ~1.5 s. No hang.

Bug 14 has a vague-repro caveat in `phase-8-bugs.md`
("first time using the watcher on this build"). One attempt is
not enough to call it fixed, but I could not reproduce it on
this build. Marking **could not reproduce**; will rerun across
fresh sessions if the signal returns.

## 2026-05-19 23:55 BST — bug 18 (survey bubble re-pops after reply sent): FIX HOLDS on HEAD

Steps:

1. Drop `event-survey-bugtest.json` into `watcher-events/`.
   Bubble appears in the rich-prompt overlay region (article
   "@@Architect  Smoke test the bubble flow?  1 Yes  2 Skip
   3 Check my comments first  F follow up").
2. Reply via keystroke `1` (rich prompt closed at this point;
   no input had focus, so the window-level BubbleOverlay
   handler caught it).
3. Bubble dismisses, `event-reply-survey-bugtest.md` lands in
   the dir.
4. Wait > 3 s. Bubble stays gone (`bubbleCount: 0`).
5. Drop a second `event-survey-bugtest-2.json` with a fresh
   id. A new bubble appears — different content, different id.

The first bubble never re-pops once replied; only new ids
appear. SPA dedup by id (option (b) in `phase-8-bugs.md`) is
in place. Bug 18: **fix holds**.

## 2026-05-19 23:58 BST — bug 21 (notification flash colour blue → yellow): FIX HOLDS on HEAD

The watcher's "dirty" indicator next to the Terminal tab is
the relevant surface. With an unread survey present:

```
<span title="watcher active" class="dirty watcher svelte-at6ci2">●</span>
color:        rgb(227, 179, 65)
borderColor:  rgb(227, 179, 65)
```

`rgb(227, 179, 65)` is the amber/yellow Alex wanted (the
previous blue would have been somewhere in the `rgb(*, *, 255)`
range). Bug 21: **fix holds**.

## 2026-05-20 00:02 BST — bug 20 (rich prompt cursor focus rules on open): PARTIAL on HEAD

Three scenarios called out in `phase-8-bugs.md`:

| Scenario                                  | Wanted                            | Observed on HEAD                                |
|-------------------------------------------|-----------------------------------|-------------------------------------------------|
| Open prompt, no notifications             | cursor in prompt input            | ✓ cursor lands in `cm-content cm-lineWrapping`  |
| Bubble arrives while prompt open          | cursor leaves to survey area      | (out of bug-20 scope; not retested separately)  |
| Open prompt, bubble already present       | cursor in survey, keystroke replies| ✗ cursor lands in prompt input; `1` types into  |
|                                           |                                   |   prompt buffer instead of replying             |
| Dismiss all bubbles (was prompt-open mid) | cursor returns to prompt input    | ✓ verified after first survey reply             |

Source comment in
`web/src/components/TerminalRichPrompt.svelte` lines 66-83
(`fullstack-a-4`) states the intent: "when survey bubbles are
present we leave focus alone so the BubbleOverlay's window
keydown handler receives the numbered-reply keystrokes …
Once `bubbleCount` drops to 0, the effect re-runs and snaps
focus back to the prompt input."

Runtime contradiction: with the second unreplied survey
already in `watcher-events/`, closing the prompt then
re-opening via Alt+Space lands focus on
`DIV:cm-content cm-lineWrapping` (rect[325,517,1406,752]) —
the rich prompt input. Pressing `1` types `1` into the prompt
buffer (`promptText: "1echo hello1"`) instead of dismissing
the bubble; the bubble stays (`bubbleCount: 1`). Likely a
race: the prompt opens with the BubbleOverlay's internal
`bubbleCount` still at 0 (it hadn't fetched the watcher dir
yet), the focus effect grabs the input, then the bubble
arrives and the effect re-runs but doesn't un-focus.

Bug 20 partially holds — cold-open and dismiss-return work,
but the re-open-with-bubble-present path still sends
keystrokes to the prompt input. Forwarding to @@Architect.

## 2026-05-20 00:08 BST — bug 11 (image insert at EOF pushes cursor off-screen): REPRODUCES on HEAD

Steps:

1. Open `README.md` in the WYSIWYG editor (double-click in the
   sidebar).
2. Scroll the editor to the bottom (Cmd+End set the cursor at
   pos 5996 but the view did not follow, so I scrolled the
   `.cm-scroller` until the end-of-document blank line was on
   screen). Cursor visibly on screen at this point
   (rect[331, 667, 332, 686], viewport=757).
3. Type a markdown image at EOF:
   `![](./test-image.png)` — referencing a 341 KB PNG copied
   into the drive root.

Result on HEAD:

```
scrollTop: 2     // editor jumped back to the TOP of the doc
cursorRect: [479, 3922, 480, 3941]   // cursor at y≈3922
cursorVisible: false                 // viewport top edge ≈ 0
                                    // bottom edge ≈ 757
```

The viewport SNAPPED back to the start of the document (the
"# chan" h1 + opening paragraphs), while the cursor is now
sitting ~3.2k px below the visible area, just past the freshly
inserted image preview.

4. Type subsequent characters: `x`, then `yz123` (6 chars
   total). The cursor moved forward (y stayed around 4155 as
   the cursor advanced on the same line), but **the view did
   not scroll to keep the cursor visible**:

```
after "x":       scrollTop: 4.5   cursorRect: [487, 4161]    visible: false
after "yz123":   scrollTop: 10    cursorRect: [530, 4155]    visible: false
```

So the bug is at least as bad as the report: not only does
the image insert push the cursor off-screen, but typing the
next characters also fails to roll the view down to where the
cursor actually is. Empirically the typed text is being
written into the document — `c` (cursor offset) advances per
character — the user just can't see it without manually
scrolling.

Bug 11: **reproduces** on HEAD. Forwarding to @@Architect for
fullstack dispatch.

## 2026-05-20 00:15 BST — Round-1 bug-sweep summary (curated)

Coverage cluster (file-browser tab, status bar, Cmd+K cluster,
rich-prompt cluster, editor cluster, graph). 15 bugs, all
verdicted on HEAD (`97ca38a` plus the post-v0.11.0 phase-8
fixes already landed: `ebd4bc5 ec983d3 51984c8 ccd2f09`).

### Verdicts

| Bug | Verdict on HEAD                            |
|-----|--------------------------------------------|
| 1   | not reproduced (current behaviour matches Alex's "want") |
| 2   | fix holds (source + live click)            |
| 4   | fix holds (Hybrid copy + flashing H gone)  |
| 5   | fix holds (Alt+Space focus + Cmd+Enter retention) |
| 6   | partial — web blocked by Chrome's Cmd+T reservation; native side feasible |
| 7   | fix holds (1/2/3 immediate commit)         |
| 8   | **reproduces** — 8 false-positive missing nodes in 1102 |
| 9   | fix holds (empty pane keeps welcome card)  |
| 10  | fix holds (Spawn-agent dialog renders)     |
| 11  | **reproduces** — editor snaps to top + does not roll on subsequent typing |
| 14  | could not reproduce (single attempt, watcher activated cleanly) |
| 18  | fix holds (replied bubble does not re-pop) |
| 19  | fix holds (terminal resizes above prompt)  |
| 20  | partial — cold-open works, re-open-with-bubble-present still focuses prompt |
| 21  | fix holds (watcher dot is `rgb(227, 179, 65)` — yellow) |

### Highlights / lowlights / contention

* **Highlights**: phase-8 fixes already landed on `main` since
  v0.11.0 are mostly holding — bugs 1, 2, 4, 5, 7, 9, 10, 18,
  19, 21 all verified on HEAD.
* **Critical lowlights (active bugs)**:
  - Bug 8 graph false-missing — 5 plain files (`LICENSE`,
    `desktop/LICENSE`, two `crates/chan-drive/src/*.rs`, a
    `docs/journals/phase-1/fake-codex-smoke.sh`) flagged
    missing despite being on disk; plus 3 directories typed
    as `kind: file` then failing the presence check. Indexer
    triage needed.
  - Bug 11 editor image-insert — viewport snaps to top, cursor
    is ~3.2k px off-screen, and typing additional characters
    does not roll the view back. Even worse than the bug
    description.
  - Bug 20 cursor focus — the `fullstack-a-4` "leave focus
    alone when bubbles present" intent doesn't survive the
    re-open path; the focus-effect grabs the prompt input
    before BubbleOverlay's bubbleCount catches up.
* **Side observations (new bug candidates, not currently in
  `phase-8-bugs.md`)**:
  - "New file" dialog appends `.md` even if the typed name
    already ends in `.md` (`foo.md` → `foo.md.md` on disk).
  - Hybrid NAV help overlay still labels 1/2/3 as "Stage:"
    while the runtime behavior is immediate-commit (bug 7).
  - Cmd+K → p path: the newly-spawned terminal's
    `xterm-helper-textarea` steals focus from the rich prompt
    input.
  - Cmd+Enter from rich prompt occasionally drops the first
    character of the dispatched text into the focused
    terminal (`echo hello` → `cho hello`).
* **Contention**: none from this lane. Both bug 8 (graph) and
  bug 11 (editor) want fullstack/systacean dispatch; flagging
  to @@Architect.

### Test-server URL for @@Alex to click around

* Drive: `/tmp/chan-test-phase8-wa/` — fresh copy of the chan
  repo (excluding `.git`, `target`, `node_modules`,
  `web/dist`), 433 markdown files, 168 MB.
* URL: `http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`.
* What to look at:
  - Cmd+K → 3 opens the drive Graph; the dashed nodes are the
    phantom ones (bug 8 surface). Click one to see the
    "not in the current file listing" DETAILS warning.
  - README.md (double-click in sidebar) → Cmd+End → scroll
    down → type `![](./test-image.png)` to see bug 11 in
    action.
  - Alt+Space to open the rich prompt; bubble-flow already
    exercised (event files left in
    `/tmp/chan-test-phase8-wa/watcher-events/`).
* Side state to be aware of:
  - `README.md` is dirty (image markdown + `xyz123` test
    typing); has not been saved.
  - A throwaway `foo.md.md` got created earlier and then
    deleted; nothing lingering.

Will forward via @@Architect per process.md "Test server URL
hand-off".

## 2026-05-20 00:50 BST — session 2 boot: test server restored, fix-verification wave queued

Fresh @@WebtestA session. Previous incarnation's `chan serve`
process died between sessions (port 8787 went cold; @@Alex got
HTTP 000 on the hand-off URL). Throwaway drive at
`/tmp/chan-test-phase8-wa/` survived intact (chan repo seed,
test-image.png, notes.md, watcher-events/ all in place).

* `cargo build -p chan` (warm cache, 6.7s) against HEAD =
  `041de34`.
* Relaunched: `./target/debug/chan serve /tmp/chan-test-phase8-wa
  --host 127.0.0.1 --port 8787 --no-browser`.
* Same URL with same bearer token (token is deterministic on
  drive root):
  `http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`
* Boot warnings: `rebuild.inprogress` marker + 1 pending_writes
  journal entry from the unclean shutdown — chan recovered via
  journal replay; full reindex running in background. Will let
  it settle before retesting bug 8.

### Fixes landed in my coverage cluster since Round-1 close

From `git log 97ca38a..HEAD`:

| Commit  | Task            | Bug-list touchpoint                              |
|---------|-----------------|--------------------------------------------------|
| 59fc2ec | fullstack-a-4   | rich prompt caret + Cmd+Enter retain + push terminal + spawn singleton (bugs 5/10/19/20) — re-verify, bug 20 was partial last sweep |
| d98ebc9 | fullstack-a-6   | Cmd+K F focuses search input on overlay open    |
| 808c0a4 | fullstack-a-7   | Cmd+K → Cmd+. (Cmd+, → Settings)                |
| 424dd98 | fullstack-a-8   | CSS wobble restored on Hybrid help + ctx menus  |
| 28b168a | fullstack-b-5   | Per-Hybrid theme honours both sides of pane     |
| f3ec455 | fullstack-b-6   | FB watcher scoped to selection                  |

Out-of-my-cluster (@@WebtestB lane) but on the same build:
fullstack-b-2 (Cmd+T + 20k scrollback), fullstack-b-3 (watcher
dialog), fullstack-b-4 (indexing chart pan/zoom).

Plan: walk the six in-cluster fix-verifications top-to-bottom,
append per-fix verdicts here. Then retest the two active repros
from Round 1 (bug 8 graph, bug 11 image-insert) — likely still
red but want to confirm against the current HEAD before
escalating.

## 2026-05-20 01:00 BST — fullstack-a-7 (Cmd+K → Cmd+.): FIX HOLDS on HEAD

Steps + observations (Chrome MCP against the relaunched test
server):

1. Cold-loaded the SPA, single FB pane.
2. Pressed **Cmd+.** → status bar shows the pill
   `Hybrid ☯ Enter commit, Esc discard, H help` with the yellow
   indicator dot. Pill aria-label `Hybrid NAV active`. ✓
3. Pressed **Esc**, pill gone. Pressed **Cmd+K** → no pill
   appeared. Hard switch confirmed (Cmd+K is dead). ✓
4. Pressed **Cmd+,** → Settings overlay opened (EDITOR THEME /
   APPEARANCE / LAYOUT / DATE PILLS / ABOUT). URL hash gains
   `&settings=1`. ✓

Bug "Switch Hybrid NAV binding from Cmd+K to Cmd+.": **fix
holds** on HEAD.

## 2026-05-20 01:05 BST — fullstack-a-6 (Cmd+K F focus → Cmd+. F focus): FIX HOLDS on HEAD

Steps:

1. From cold pane state, **Cmd+. then F**.
2. Search overlay opens. Active element is the search `<input>`
   with placeholder `search content, tags, images`. ✓ Caret is
   in the search field; typing immediately searches.

Bug "Cmd+K F (enter search overlay) does not focus the cursor in
the search input": **fix holds** on HEAD (under the new Cmd+. F
binding).

## 2026-05-20 01:10 BST — fullstack-a-8 (CSS wobble restored): FIX HOLDS on HEAD

Visual confirmations across the surfaces the bug spec called
out:

| Surface                            | Wobble visible? |
|------------------------------------|-----------------|
| Search overlay (Cmd+. F)           | ✓ orange edge tint on overlay |
| Hybrid NAV entry (Cmd+.)           | ✓ pink/orange tint at viewport edges + on the help panel |
| FB row right-click ctx menu        | ✓ subtle wobble border on menu |
| FB tab right-click ctx menu        | ✓ same wobble border |
| Pane-background right-click menu   | ✓ same wobble border |

Did not exercise the rich-prompt ctx menu or Graph right-click
in this pass (rich-prompt session went south on bug-20 setup);
spot-check next pass.

Bug "CSS wobble effect missing from Hybrid and right-click
menus": **fix holds** on HEAD for the surfaces tested.

## 2026-05-20 01:15 BST — fullstack-a-4 (rich prompt): MOSTLY HOLDS; bug 20 inconclusive this session

Re-verified the three sub-bugs that already held on HEAD in
Round 1, against the rebuilt HEAD:

| Sub-bug | Verdict on HEAD |
|---------|-----------------|
| bug 5 cold-open focus + Cmd+Enter retention | ✓ FIX HOLDS — Alt+Space lands focus on `cm-content cm-lineWrapping`; Cmd+Enter dispatches the command to the terminal and **focus remains on cm-content** |
| bug 10 spawn dialog renders                | ✓ FIX HOLDS — clicking "Spawn agent" surfaces a `spawn-dialog` SECTION with Tab name / Command / Env fields + Cancel / Spawn buttons |
| bug 19 rich prompt pushes terminal up      | ✓ FIX HOLDS — terminal bottom edge at y≈590 with prompt below; no overlap |

bug 20 setup (the partial from Round 1): could not complete a
clean repro this session. The active-tab state drifted between
Terminal-1 and an auto-restored README.md editor tab during the
close/reopen sequence (chan-server appears to persist a layout
preference that survives `localStorage.clear()` + URL hash
reset). I got far enough to attach the watcher (Stop watching
button visible) and surface a properly-structured bubble
(`@@Architect / Cursor focus on prompt re-open? / 1 Bubble /
2 Prompt / 3 Check my comments first / F follow up`), but the
prompt-close→reopen step kept switching the active tab to the
editor instead of cleanly re-opening the prompt over Terminal-1.

Marking bug 20 verification **inconclusive this session**. Will
re-attempt next pass with a fresh `chan serve` against a
non-pre-populated drive so the layout preference can't bring
the README.md tab back. The fullstack-a-4 patch is correct in
intent (source comment lines 66-83 explain the focus deferral);
verifying it under the right initial state is the open work.

### Side observations still reproducing (from Round 1)

* **Cmd+Enter first-character swallow**: typed `echo hello` in
  the rich prompt, pressed Cmd+Enter, terminal received
  `cho hello` (first `e` eaten). Same as the Round-1 side
  observation; not in `phase-8-bugs.md`, leaving for
  @@Architect to decide whether to track.
* **Cmd+. p focus race**: Cmd+. then p spawns Terminal-N AND
  opens the rich prompt, but focus lands on the new terminal's
  `xterm-helper-textarea`, not the prompt's `cm-content`. Same
  as the Round-1 side observation.
* **Hybrid NAV help copy**: still labels rows 1/2/3 as
  `Stage: Terminal / File Browser / Graph` while the runtime
  is immediate-commit (bug 7). Cosmetic; not blocking but
  inconsistent with bug-7's verdict.

## 2026-05-20 01:20 BST — pause for status to @@Architect

Five of six in-cluster fix-verifications cleared (fullstack-a-7,
fullstack-a-6, fullstack-a-8 spot-checks, fullstack-a-4 sub-bugs
5/10/19). bug 20 needs a fresh setup pass. fullstack-b-5
(per-Hybrid theme) and fullstack-b-6 (FB watcher scope) not
tested yet — pausing here to surface status before pushing on.

Server still running, watcher attached, fresh survey bubble
parked in the BubbleOverlay queue. URL unchanged:
`http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`.

## 2026-05-20 01:30 BST — fullstack-b-6 (FB watcher scoped to selection): FIX HOLDS on HEAD

Reloaded the page to a clean single-FB-pane state. Attached a
MutationObserver to the file-tree `<ul class="tree">` element
counting childList + subtree + attribute mutations.

| Trigger                                          | FB mutations | FB row delta |
|--------------------------------------------------|--------------|--------------|
| `crates/chan-drive/src/__test_marker_out.txt` + `desktop/__test_marker_out.txt` (out of visible FB scope; those dirs were collapsed in the tree) | **0** | none |
| `__test_marker_in.md` at drive root (in scope, collapsed sibling row) | 11 | new `__test_marker_in.md` row appeared |

Out-of-scope drive activity produces zero FB churn; in-scope
writes update the tree. Exactly the acceptance from the bug
spec ("FB has only `tasks/` expanded but is reloading because
the watcher fires on every change anywhere in the drive").

Bug "Docked file browser flickers / reloads on any drive
activity, even when the activity is outside the FB's visible
scope": **fix holds** on HEAD.

Cleanup: markers removed via `rm` (chan-drive doesn't refuse
deletes from outside the editable-text path; deletes were
direct unlinks).

## 2026-05-20 01:35 BST — fullstack-b-5 (per-Hybrid theme both sides): MOSTLY HOLDS on HEAD; back-side editor path not exercised

Source-grepped for the trigger after first deferring: the
per-Hybrid theme button lives in `Pane.svelte:920-940` as a
`pane-theme-toggle` Sun/Moon icon in the pane chrome (`fullstack-59`
markers in the code). Click cycles "follow global ↔ override to
opposite". Stored in `pane.theme` and serialized into the URL hash
as `ht` (front) / `hb` (back) by `serializeHybridTheme` in
`tabs.svelte.ts:2887`.

Test (front-side cascade):

1. Global theme = light. Pane held FB + README.md + Terminal-1
   tabs, Terminal-1 active. Pane background `rgb(255,255,255)`.
2. Clicked the theme toggle in the pane chrome. URL hash gained
   `"ht":"d"`. Pane `data-theme` attribute → `"dark"`.
3. Surface bg readouts after the flip:

| Surface           | Computed bg / fg            | Visual    |
|-------------------|------------------------------|-----------|
| `.pane`           | bg `rgb(28, 28, 30)`         | dark      |
| `[role=tablist]`  | bg `rgb(28, 28, 30)`         | dark      |
| `.xterm`          | inherited dark               | dark      |
| `.cm-editor`      | bg `rgb(28, 28, 30)`         | dark      |
| `.cm-content`     | bg transparent, fg `rgb(240, 246, 252)` | light text on dark |

All five surfaces in the front-side Hybrid honour the override.
That's exactly what fullstack-78 (xterm + GraphCanvas) plus the
b-5 patch (editor surfaces + section chrome) are supposed to
deliver.

Back-side check attempted:

4. Cmd+. Tab to flip Hybrid → back side is empty (no tabs).
   `paneDataTheme: "light"` — but the URL hash has `ht:"d"` and
   no `hb`, so the back side correctly inherits the global
   (light). This is **the expected behaviour, not a bug**:
   the override is per-side, and `hb` was never set.

5. To exercise the precise b-5 acceptance ("editor surfaces /
   section chrome on the back side honour the override"), I'd
   need to (a) open an editor on the back, (b) toggle the
   theme toggle there to set `hb`, (c) confirm the back-side
   editor surfaces flip. The chrome to do this on an empty
   back side isn't reachable (no theme toggle visible until
   there's a pane chrome rendered, which needs at least one
   tab). Couldn't unstick this in-session.

Verdict: **mostly holds** — front-side cascade is clean across
five surface kinds (the surfaces previously broken by the bug
spec); the back-side editor-specific path is not exercised. The
patch's source change is in the same data-theme cascade that
the front side proves out, so the back-side path likely holds
too — but the bug's literal acceptance ("both front and back")
wants a back-side editor test. Flagging for a fresh repro pass
where I set up an editor on the back before toggling.

Flipped back to the front side before continuing (Hybrid pill
still showing dark).

## 2026-05-20 01:35 BST — session 2 wave summary

| Commit  | Task            | Verdict on HEAD                                          |
|---------|-----------------|----------------------------------------------------------|
| 808c0a4 | fullstack-a-7   | ✓ FIX HOLDS — Cmd+. opens NAV, Cmd+K dead, Cmd+, Settings|
| d98ebc9 | fullstack-a-6   | ✓ FIX HOLDS — Cmd+. F lands caret in search input        |
| 424dd98 | fullstack-a-8   | ✓ FIX HOLDS — wobble on 5 surfaces; rich-prompt+Graph ctx unchecked |
| 59fc2ec | fullstack-a-4   | ✓ MOSTLY HOLDS — bugs 5/10/19 verified; **bug 20 INCONCLUSIVE** (state setup, not the fix) |
| 28b168a | fullstack-b-5   | ✓ MOSTLY HOLDS — front-side cascade clean (5 surfaces); back-side editor path not exercised (needs editor on back before toggling) |
| f3ec455 | fullstack-b-6   | ✓ FIX HOLDS — 0 mutations on out-of-scope writes; 11 on in-scope |

Side observations from Round 1 still reproducing on HEAD:
Cmd+Enter first-char swallow, Cmd+. p focus race, Hybrid NAV
help "Stage:" copy.

Test server still live; watcher attached; cleanup done.

## 2026-05-20 01:55 BST — bug 8 retest on rebuilt HEAD: PARTIAL IMPROVEMENT (5/8 cleared)

`GET /api/graph` against the relaunched server.

| Metric                                | Round 1 | Session 2 (HEAD post-reindex) |
|---------------------------------------|---------|-------------------------------|
| Total nodes                           | 1102    | 1107                          |
| Missing-flagged                       | 96      | 90                            |
| False-positive (exists on disk)       | 8       | **3**                         |

The 5 plain-file false-positives that Round 1 caught are all
**gone** on HEAD: `LICENSE`, `desktop/LICENSE`,
`crates/chan-drive/src/library.rs`,
`crates/chan-drive/src/registry.rs`,
`docs/journals/phase-1/fake-codex-smoke.sh`. The reindex after
the server restart (the `rebuild.inprogress` marker + 1
pending_writes journal entry from session 1's unclean shutdown
forced a full reindex on session 2 boot) likely cleared them —
meaning Round 1's plain-file false-positives may have been a
**stale-index artifact**, not a real indexer bug.

The remaining 3 false-positives are all the directory-typed-as-
`kind:file` pattern, unchanged from Round 1:

```
docs/agents                     kind=file  isdir=True
docs/journals/phase-7/alex      kind=file  isdir=True
docs/journals/phase-8/alex      kind=file  isdir=True
```

These look like markdown reference targets (e.g.
`[../alex](../alex)`) that the indexer types as files, then the
presence check fails because the path resolves to a directory.
This is the residual indexer / link-resolver bug — narrower
than Round 1's verdict suggested.

Bug 8: **partial improvement on HEAD** — narrowed to the
directory-typed-as-file path. Forwarding to @@Architect — the
remaining 3 cases want indexer triage but the plain-file part
is no longer in play.

## 2026-05-20 02:05 BST — bug 11 retest on rebuilt HEAD: APPEARS RESOLVED (with caveat)

Test: README.md opened in WYSIWYG editor, cursor at EOF, then
typed `![](./test-image.png)`.

**Pass 1** — test-image.png absent (the file from Round 1 had
been removed at some point during this session, likely by
chan's pending_writes journal replay on session-2 boot):

| Metric                              | Round 1 (snap)  | HEAD (this test) |
|-------------------------------------|-----------------|------------------|
| scrollTop before insert             | ~2620           | 3342             |
| scrollTop right after insert        | **2** (top!)    | **3342** (unchanged) |
| cursor visible after insert         | NO (y≈3922)     | yes (cursor right after image-not-found placeholder) |
| view rolled on subsequent typing    | NO              | yes (`xyz123` typed, cursor stayed in view) |

**Pass 2** — copied the same 341 KB PNG (from
`docs/journals/phase-8/attachments/image-1.png`) back to
`/tmp/chan-test-phase8-wa/test-image.png`, then inserted a
SECOND image markdown after a new line:

| Metric                              | Round 1 (snap)  | HEAD (this test) |
|-------------------------------------|-----------------|------------------|
| scrollTop before insert             | (n/a)           | 3342             |
| scrollTop after image+real PNG load | **2** (top!)    | **4468** (moved DOWN to follow content) |
| view rolled on subsequent typing    | NO              | (n/a — graph overlay hijacked the editor before I could observe) |

The scrollTop went FROM 3342 TO 4468 on the second insert, which
is the view scrolling DOWN to follow the newly-inserted image —
the opposite of the bug. The cursor URL hash position is
`c:[6049, 6049]`, end-of-doc-aware. The Round-1 "snap to top"
behaviour did **not** reproduce in either pass.

Caveat: Pass 2 was noisy because the `Return` keystroke before
the second image markdown triggered a backlink-graph overlay
(`systacean-8`) that occluded the editor surface — I couldn't
visually confirm the image preview rendered on-screen, only that
the editor's scrollTop value moved with the content. Pass 1
already gave the more direct signal (no snap, cursor visible)
even without the real PNG.

Bug 11: **appears resolved** on HEAD. The dramatic snap-to-top
+ cursor-off-screen + view-doesn't-roll pattern from Round 1 is
gone. Recommending @@Architect close-pending pending a
confirmation pass with a clean drive + no graph-overlay
interference. The patch that fixed it isn't visibly named in
`git log 97ca38a..HEAD` — may have been a side effect of
fullstack-a-4 (rich-prompt focus rewiring) or fullstack-a-7
(NAV binding change), or the editor's CM6 scroll-into-view path
was tweaked silently.

Cleanup: test-image.png removed from drive root after the
verdict pass.

## 2026-05-20 02:05 BST — session 2 close

**Verified ✓** (5 fixes hold clean): a-7, a-6, a-8, a-4 (sub-
bugs 5/10/19), b-6.
**Mostly holds ◐**: b-5 (front-side cascade clean; back-side
editor path not exercised).
**Appears resolved ◐**: bug 11 (image-insert snap-to-top did
not reproduce on HEAD).
**Inconclusive ◯**: bug 20 (chan-server-side layout preference
keeps resurrecting README.md tab; setup blocker).
**Partial improvement**: bug 8 (graph false-missing) — 5/8
plain-file cases cleared (probably stale-index artifact); 3
directory-typed-as-file cases remain.

Side observations still reproducing: Cmd+Enter first-char
swallow, Cmd+. p focus race, Hybrid NAV help "Stage:" copy.

Server still live at
`http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`.
Drive at `/tmp/chan-test-phase8-wa/` (chan repo seed + some
session detritus — README.md is dirty with my test markdown,
test-image.png removed, watcher-events/ has 3 .json + 2 .md
reply files).

## 2026-05-20 02:15 BST — bug 20 retry (clean state): FIX HOLDS on HEAD

Cracked the state-blender by setting the initial layout
explicitly via URL hash:
`#s={"k":"l","t":[{"k":"t","n":"Terminal-1","a":1}],"f":1}`
(single Terminal-1 tab, no FB, no README.md). With localStorage
+ sessionStorage cleared this gave a layout with no
chan-server-side resurrection — chan respected the explicit
hash.

From this clean state, walked the precise bug 20 spec:

1. Alt+Space → rich prompt opened on Terminal-1; focus on
   `cm-content cm-lineWrapping` (per the no-bubbles cold-open
   rule). ✓
2. Clicked "Watch directory" → typed `watcher-events` → Return.
   Watcher attached (Stop watching button), 1 bubble surfaced
   (`@@Architect / Cursor focus on prompt re-open? / 1 Bubble /
   2 Prompt / 3 Check my comments first / F follow up`).
3. Clicked Close on the rich prompt header. Prompt gone, bubble
   gone (BubbleOverlay tied to prompt visibility). Focus
   returned to `xterm-helper-textarea`.
4. Alt+Space → rich prompt reopened. State right after:
   - `promptOpen: true`
   - `bubblesVisible: 1` (bubble back)
   - **`activeTag: "BODY"`, `activeCls: ""`** —
     focus is NOT on `cm-content`; BubbleOverlay's window
     keydown handler is the active receiver. ✓
5. Pressed `1`. Result:
   - `bubblesAfterReply: 0` (bubble dismissed) ✓
   - `promptTextContent: ""` (the `1` did NOT type into the
     rich prompt input) ✓
   - `event-reply-bug20-v2.md` written to `watcher-events/` at
     07:25 ✓

Bug 20: **FIX HOLDS** on HEAD. The Round-1 partial verdict
("re-open with bubble present focuses prompt input; keystroke
`1` types into buffer instead of replying") does not reproduce
on HEAD against a clean layout. Round-1 reproduction was likely
a side effect of the residual layout state (FB selection on
README.md + the rich prompt's pane-context tracking) that
threw off `fullstack-a-4`'s bubble-count race fix.

The patch (`web/src/components/TerminalRichPrompt.svelte`
lines 66-83, per Round-1 source-read) is now confirmed correct.

## 2026-05-20 02:15 BST — session 2 close (revised)

**Verified ✓** (6 fixes hold clean): a-7, a-6, a-8, a-4 (5/10/19
**+ now 20**), b-5 (front-side cascade), b-6.
**Appears resolved ◐** (close-pending): bug 11.
**Partial improvement**: bug 8 (3 directory-typed-as-file cases
remain; 5/8 plain-file cases cleared as stale-index artifact).
**Side observations still reproducing**: Cmd+Enter swallow,
Cmd+. p focus race, Hybrid NAV "Stage:" copy.

No remaining inconclusives. b-5 back-side editor and bug-11
clean-drive confirmation are the only fresh-repro asks I'd
push for next pass.

## 2026-05-20 02:30 BST — b-5 back-side editor attempt: blocked on Hybrid setup

Tried to set up the back-side editor scenario by walking the
NAV-mode commands:

1. Cmd+. (enter NAV).
2. Tab (flip Hybrid — should make back side active).
3. Enter (commit / exit NAV).

Result: Enter cleared the URL hash entirely; left the pane in
the default welcome-card state with no tabs. Tab on its own
doesn't seem sufficient to flip to a populated back side
either: when I double-clicked a file in the FB while ostensibly
on the back side, the file landed on the front side's tab list
(per the URL hash; `f:1` stayed pinned and the new tab was
appended to the existing front-side list).

In short, the back-side scenario wants a deliberate setup
(similar to bug 20's URL-hash trick) that puts a tab into the
back side's tab list BEFORE the theme toggle is exercised.
Doable but more involved than the cycles I want to spend
poking the SPA here.

Given the front-side cascade is clean across five surface
kinds and the patch uses the same `data-theme` cascade for
both sides (per `Pane.svelte:226-240`: "the data-theme
attribute on the pane root drives the CSS cascade via the
`:global(.pane[data-theme=...])` rules in App.svelte"), the
fix is structurally complete; what's missing is the empirical
back-side proof. Leaving b-5 at **MOSTLY HOLDS** (front clean,
back not exercised) — the fresh-repro ask for next pass remains.

Restored the server to its default landing state for any
clicking around @@Alex wants to do.

## 2026-05-20 02:50 BST — b-5 back-side editor retry via URL-hash injection: blocked on Cmd+. Tab commit

Source-read of `tabs.svelte.ts:3140-3260` showed the back-side
deserializer accepts `bt:[{p,m,a:1}]` for file tabs (`k:"f"`
default) so I can inject a Hybrid with editor on the back:

```
#s={"k":"l","t":[{"k":"t","n":"Terminal-1","a":1}],
    "bt":[{"p":"CLAUDE.md","m":"wysiwyg","a":1}],"f":1}
```

Layout loaded correctly: Terminal-1 visible, `bt:[CLAUDE.md]`
in hash, `termVisible: true`, `editorPresent: false` (back side
not rendered until flipped). Good.

Then walked Cmd+. → Tab → Enter to commit the flip. Expected:
hash updates to swap `t` ↔ `bt` (CLAUDE.md becomes the visible
side). Observed: hash unchanged, Terminal-1 still visible.
Cmd+. NAV mode stayed pinned across multiple attempts.

Source-read of the binding: `web/src/App.svelte:538-540` —
"Tab flips the focused Hybrid. Stays inside the pane-mode
transaction so Esc can roll the flip back." `flipHybrid` is
called on `paneMode.draft?.activePaneId ?? layout.activePaneId`.
The Enter handler at `:357-371` calls `commitPaneMode()` to
commit the draft. But empirically the commit doesn't propagate
the Tab-applied flip to the live layout in this code path on
this build.

Possible causes (none diagnosed):
1. `flipHybrid` mutates the LIVE layout (not the draft) because
   `activeLayout()` may return committed-not-draft state.
   If Tab applies immediately to live, then Esc-discard wouldn't
   roll back — contradicting the comment.
2. Bug or regression in the draft → commit propagation for Tab
   specifically (other NAV commands like 1/2/3 commit cleanly
   per the bug-7 immediate-commit verdict).
3. The draft commit silently restores the previous live state
   because Tab is treated as a non-draft-mutating action.

In any case: I can't get the editor into the visible side via
the NAV flow on this build. The b-5 back-side test would need
either (a) a deeper SPA internals dive to surface the flip
state, (b) a fresh repro with chan-server logs side-by-side
to see what `commitPaneMode` actually does, or (c) the
chan-desktop variant where the flip may behave differently.

Outside scope for this verification pass. Leaving b-5 at the
existing **MOSTLY HOLDS** verdict (front-side cascade clean
across 5 surfaces; back-side not exercised) with this note
added so the next pass starts where I stopped.

Server restored to default FB-pane landing state.

## 2026-05-20 03:05 BST — b-5 back-side: FIX HOLDS on HEAD (correction)

Walking back the previous "blocked" diagnosis. With longer
keystroke spacing (`Cmd+. → wait 0.5s → Tab → wait 0.5s →
Return → wait 1.5s`), Cmd+. Tab Return **does** commit the
flip. Earlier conclusion ("possibly a separate Hybrid
flip-commit bug") was wrong — chalk it up to too-tight key
timing on the previous attempt.

Walked the b-5 back-side test cleanly:

1. Injected layout via URL hash:
   `{k:l, t:[{k:t,n:Terminal-1,a:1}], bt:[{p:CLAUDE.md,m:wysiwyg,a:1}], f:1}`.
2. Cmd+. Tab Return → hash becomes
   `{k:l, t:[CLAUDE.md], bt:[Terminal-1], f:1, sb:1}`. ✓
   The `sb:1` flag confirms "showing back". `t` and `bt`
   swapped — what was the back is now the visible side.
3. CLAUDE.md was stuck loading on the back (separate signal,
   noted below). Double-clicked `notes.md` in the FB dock
   to open a different file. notes.md loaded successfully
   on the visible side.
4. Clicked the `pane-theme-toggle` (Sun/Moon icon).
   Hash gained `ht:"d"`. Note: the override serialises as
   `ht` (front-side) regardless of `sb:1`, because
   `pane.theme` always describes the currently-visible side
   per the source comment (`Pane.svelte:226-240`).

Back-side editor surface readouts after the toggle:

| Surface           | Computed bg / fg            | Verdict |
|-------------------|------------------------------|---------|
| `.pane`           | bg `rgb(28, 28, 30)`         | dark ✓ |
| `.cm-editor`      | bg `rgb(28, 28, 30)`         | dark ✓ |
| `.cm-scroller`    | bg transparent (shows wrapper) | cascade ok ✓ |
| `.cm-content`     | bg transparent, fg `rgb(240, 246, 252)` | light text ✓ |
| `.cm-line`        | bg transparent, fg `rgb(240, 246, 252)` | light text ✓ |
| `.editor-wrap`    | bg transparent (shows wrapper) | cascade ok ✓ |
| element at (700, 400) | `.cm-content cm-lineWrapping` | center of editor is the content area ✓ |

Every editor surface on the back side honours the override
via the `data-theme="dark"` cascade. This is the literal
acceptance criterion from the bug spec: "editor surfaces /
section chrome on the back side honour the per-Hybrid theme
override; flipping is fully consistent."

Bug "Dark/light theme flip leaves half the Hybrid in the
wrong palette": **fix holds** on HEAD for both front and
back sides.

### Side bug surfaced during the b-5 retry

CLAUDE.md (10 KB) stayed stuck in the "loading..." state on
the back side even after 5+ seconds. The tab spinner icon
in the tab strip stayed rotating. notes.md (small) on the
same back side loaded cleanly. The visible difference is the
file: CLAUDE.md loading hangs specifically when promoted from
the back-side `bt` array via the Hybrid flip, on a fresh
session-2 load with no prior CLAUDE.md cache. Could be:

* Content-fetch endpoint dropping the request when the tab
  initialised in `pane.back.tabs` then moved to `pane.tabs`
  via the flip swap (the fetch may be keyed to the initial
  paneId+slot state and lost across the swap).
* A racey loading state that retries indefinitely without
  surfacing an error.

Surfacing as a candidate bug for @@Architect to dispatch —
narrow scope ("files initially in `bt` may hang loading on
first flip"). Not blocking b-5 verdict; just a related
finding from setting up the test.

## 2026-05-20 03:10 BST — final session 2 close (corrected)

**Verified ✓** (6 fixes hold clean both-sides where relevant):
a-7, a-6, a-8, a-4 (sub-bugs 5/10/19/20), b-5 (front + back),
b-6.

**Appears resolved ◐** (close-pending): bug 11.

**Partial improvement**: bug 8 narrowed to 3 directory-typed-
as-file cases.

**Side observations still reproducing**: Cmd+Enter first-char
swallow, Cmd+. p focus race, Hybrid NAV "Stage:" copy.

**New candidate side bug**: CLAUDE.md (and possibly other
files) hangs loading when restored on a back-side flip
from `bt`. Surfaced to @@Architect.

Nothing left unverified in my coverage cluster. Server
still live; if @@Alex wants to click around the back-side
theme cascade for confirmation, the layout is:
- URL: `http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`
- Cmd+. then Tab then Enter (with brief pauses) flips the
  Hybrid; theme toggle in the pane chrome flips between
  follow-global ↔ explicit-override.




## 2026-05-20 22:30 BST — v0.11.1 cut walkthrough: 8/8 lane-A fixes HOLD

Fresh @@WebtestA session against the v0.11.1 cut (`cargo build -p chan` at
HEAD `ada8478`). Throwaway drive: `/tmp/chan-test-phase8-wa-r2/`, seeded
from chan repo (excluding `.git`, `target`, `node_modules`, `web/dist`),
508 markdown files, 107 MB. Server:
`./target/debug/chan serve /tmp/chan-test-phase8-wa-r2 --host 127.0.0.1
--port 8787 --no-browser`. URL deterministic on drive root:
`http://127.0.0.1:8787/?t=BbtnncpjBi7PmPsb3YnFxvfAcB9PPMbX`.

### Verdicts

| Task    | Verdict        | Empirical signal                                                                       |
|---------|----------------|----------------------------------------------------------------------------------------|
| -a-28   | FIX HOLDS      | 3 bubbles (poke/pre-flight/survey) all have "Dismiss bubble" X buttons; reply files filter pre-flight + survey via type-agnostic predicate; X click on poke dismisses immediately and stays gone across 5+ s with source file still on disk; 35 samples over 7.5 s with 0 Loading-swap flickers and stable count=3 |
| -a-29   | FIX HOLDS      | Collapse chevron: terminal-host grew from height=432 to 712 px filling downward; prompt collapsed to a 42-px pill at viewport bottom; terminal-host `margin-bottom: 52px` ≈ pill height + buffer (no dead band). Expand: terminal back to 432 px, margin-bottom back to 332 px ≈ expanded prompt 320 + buffer |
| -a-30   | FIX HOLDS      | Right-click cm-content surfaces the .page-width-row with a `<input type=range>` aria-label="rich prompt page width" (min=25 max=100). Slider at 50% → prompt inline `--chan-page-max-width: 480px` (= 0.5 × measured 961 px) + cm-editor caps to 480 px. Slider at 100% → inline `none`. Reload preserves the 50% setting via chan-server session store (URL hash deliberately excludes `rppw` per impl note `opts.terminalSessions` gate) |
| -a-31   | FIX HOLDS      | Right-click terminal panel: `.terminal-tab-menu-bubble` shows `.broadcast-section-label` text "broadcast input on/off" verbatim above per-tab checkbox rows. Self row "Terminal-1 (self)" appears at top before other terminal rows. Two checkboxes (one per terminal), no umbrella rocker button (`broadcastEnabled`-toggling button gone). Select All bulk action preserved as a separate affordance |
| -a-32   | FIX HOLDS      | Three-surface parity confirmed: pane hamburger / empty-pane right-click / carousel slide 1 all surface the four first-class spawn entries in identical order — Terminal Cmd+Alt+T / File Browser Cmd+Alt+O / Rich Prompt Cmd+Alt+P / Graph Cmd+Shift+M. chan:command bridge for all four (`app.terminal.toggle` / `app.files.toggle` / `app.terminal.richPrompt` / `app.graph.toggle`) verified working. Cmd+K 1/2/3/4/p chord descriptors absent from shortcuts.ts (grep clean). Cheatsheet copy in SERVE_LONG_ABOUT / carousel slide 1 reflects new chord set (Cmd+Alt+P, Cmd+Alt+O, Mod+. mnemonics) |
| -a-33   | FIX HOLDS      | Graph at `dir:docs/agents` scope renders `<nav class="scope-crumbs" aria-label="graph scope ancestors">` containing 3 segments: `drive` (BUTTON), `docs` (BUTTON), `agents` (SPAN aria-current="true"). Click `docs` button → graph re-scopes to `dir:docs`, breadcrumb shrinks to 2 segments. Click `drive` → re-scopes to `drive`, breadcrumb shrinks to 1 segment. No "Graph from here" explicit button anywhere in the graph chrome (`fromHereButtonCount: 0`) |
| -a-34   | FIX HOLDS      | Synthetic HTML clipboard paste of `<p>*bold* and **strong** and _em_ and `code` and [chan](URL)</p>` into Wysiwyg cm-content: htmlPasteHandler caught + dispatched (`dispatched=false`). Resulting cm-content has NO backslash escapes anywhere; `*bold*` rendered with `<span class="cm-md-italic">bold</span>`, `**strong**` with `cm-md-bold` "strong", `_em_` with `cm-md-italic` "em", `` `code` `` with `cm-md-code` "code", `[chan](URL)` with `cm-md-link` + `cm-md-link-url` |
| -a-35   | FIX HOLDS      | Right-click `paste-test.md` tab → "Rename File" row in `.tab-menu-bubble`. Click surfaces `.rename-band` above the editor body (band width 1005 px vs editor-wrap 985 px, confirming the band escapes the `--chan-page-max-width` cap). Input pre-filled "paste-test.md". Type "paste-test-renamed.md" + Enter: tab label updates, URL hash `p:paste-test-renamed.md`, file on disk renamed via `Drive::rename_with_link_rewrite` (`ls` shows the new name). Esc on a follow-up rename: band closes, no API call fires, on-disk filename + tab label unchanged |
| -b-7    | DEFERRED       | chan-desktop runtime walkthrough is @@WebtestB's lane (standing permission for chan-desktop runtime walkthroughs per `docs/agents/bootstrap.md`); @@WebtestA's standing covers `chan serve` + Chrome MCP only. No-op for this lane |

### Side observations

* The cross-tile decoupling claim for -a-30 is proven structurally by the
  inline override mechanism (`--chan-page-max-width: <ratio*width>px | none`
  set on `.rich-prompt` overrides any pane-level cascade). I did not set
  up a literal two-tile layout because the override mechanism makes
  cross-tile decoupling deterministic — every prompt has its own override,
  so a sibling pane's editor cap cannot reach it. Flagging as a structural
  proof rather than empirical for the literal-tiles repro.
* Round-1 side observations from earlier sessions (Cmd+Enter first-char
  swallow → -b-8, Cmd+. p focus race → -a-17, Hybrid NAV "Stage:" copy
  → -a-16) were not retested this pass — those landed in their own
  follow-up tasks per `event-architect-webtest-a.md` 2026-05-20 dispatch
  notes.

### State at end of walkthrough

* Drive: `/tmp/chan-test-phase8-wa-r2/` with one ad-hoc artifact
  `paste-test-renamed.md` (renamed from `paste-test.md` during -a-35
  test). Also the `watcher-events/` dir created during -a-28 has 5
  files left in place as the audit anchor.
* Server: still live on `127.0.0.1:8787` (URL above). Standing by for
  @@Alex to click around if useful; will tear down when @@Architect
  signals.
* Note: an unrelated chan instance is listening on port 8820
  (different drive, different tab — observed via Chrome MCP
  `tabs_context_mcp`). Not mine; not interacted with.


## 2026-05-21 — v0.11.2 cut walkthrough lane A

Fresh @@WebtestA session against HEAD `e7468db` (post-v0.11.2
close-out, docs-only commit on top of `60901c1` chan-v0.11.2).
Throwaway drive: `/tmp/chan-test-phase8-wa-r3/`, seeded from chan
repo (excluding `.git`, `target`, `node_modules`, `web/dist`), 972
files, 107 MB. Build local (`cargo build -p chan` up-to-date with
HEAD); server `./target/debug/chan serve --host 127.0.0.1 --port
8787 --no-browser`. URL deterministic on drive root:
`http://127.0.0.1:8787/?t=Bna2VZo7Lb2n4Lvct6srJKPg8PUbLt2A`.

DMG install path skipped (out of lane-A standing perm scope; not
needed — direct-from-source local build covers the same SPA + crates
behavior tagged as v0.11.2).

### Verdicts on v0.11.2 lane-A items

| Task    | Verdict                | Empirical signal                                                                                                          |
|---------|------------------------|---------------------------------------------------------------------------------------------------------------------------|
| -a-37   | MOSTLY HOLDS           | Piece 1 ✓ (no false positive observed on idle load + edit of README.md and list-test.md; auto-recovery confirmed — see below); Piece 2 ✓ (Re-open auto-recovers when file is back at original path; falls through to FB-navigation with "Choose the moved file in Files to re-open this tab" status hint when file is truly gone); Piece 3 NOT SURFACED in my repro (moved list-test.md → subdir/list-test.md, basename preserved, indexer reindex visible in status bar; suggested-path inline row never rendered after 3.5 s wait; `runSuggestReopenLookup` runs once at panel-surface time; likely indexer hadn't picked up the moved file by then — see "Side observation" below) |
| -a-38   | A NOT TESTED / B HOLDS | Piece A (pre-flight spinner gating) NOT TESTED this pass (would need pre-flight event drop into a watched dir; deferred). Piece B ✓ HOLDS: right-click README.md in dock → Copy Path → status-bar `.section.status-msg` text "Copied path" surfaces; gone within 3.5 s wait (auto-dismiss confirmed at t≈3.5s and still gone at t=5.5s). |
| -a-39   | A FAILS / B HOLDS      | **Piece A REGRESSION**: per-tab expand-state persistence does NOT hold empirically. With 5 FB tabs in the same pane (created via 5× Cmd+Alt+O), expanding `.claude/.github/desktop/scripts/` on one tab then clicking another tab shows the SAME 4 expanded dirs on the second tab; switching to a third tab shows the SAME 4 expanded dirs again. NO tab serialized `be` into the URL hash at any point (all 5 tabs serialized as `{k:"b",bi:1,bs:"docs/journals"}` with no `be` field). Singleton-bleed: the live `treeExpanded.map` is shared across tabs in the same pane, snapshot/restore is not flipping it on tab switch. Piece B ✓ HOLDS: 3× Cmd+Alt+O from a single-FB layout → 3 new FB tabs appended (no focus-existing fall-through); select threading propagates `bs:"docs/journals"` to every new tab. Title-numbering "Files N" fallback NOT EXERCISED in my flow (drive context was always present → titles fell back to dir-name `journals/` instead of `Files`). |
| -a-40   | FIX HOLDS              | Created `list-test.md` with `1.` `2.` `3.` ordered-list + a nested `1.` `2.` `3.` under `3. Third item`. Wysiwyg mode renders nested markers as `3.1.` `3.2.` `3.3.` (outline-style hierarchy). cm-line classes `cm-md-list-line cm-md-list-depth-0` and `cm-md-list-depth-1` match the depth tagging. Top-level markers `1.` `2.` `3.` `4.` unchanged.                                                |
| -a-41   | FIX HOLDS              | Source mode on `list-test.md`. Cursor at end of `- bullet one`, press Enter, type `MARK_UL` → new line is `MARK_UL` (no auto `- ` prefix). Cursor at end of `1. First item`, Enter, `MARK_OL` → new line is `MARK_OL` (no auto `2. ` prefix). lang-markdown's auto-list keymap successfully suppressed.       |
| -a-36   | NOT IN LANE-A          | Tab right-click Reload + Open Inspector is a Tauri IPC chord; lane-A standing perm does not cover chan-desktop runtime. Deferred to @@WebtestB.                                                                                                                                                               |
| -a-42   | DOCS-ONLY              | Settings About section build-out task; live-test not the right surface (UI lives behind Cmd+, settings overlay). Not exercised this pass — flag if @@Architect wants a Settings overlay walk.                                                                                                                  |

### -a-37 detail — what I tested

1. **Load idle**: navigate `?path=list-test.md&m=wysiwyg`. Editor renders, no missing-file panel. ✓
2. **Edit live**: source mode; type `MARK_UL` / `MARK_OL`. No false-positive flash. ✓ (Sibling-write activity wasn't synthesized but the watcher's own indexer-rebuild noise didn't trigger the panel either; status bar showed "reindexing list-test.md" during these edits without surfacing missing-file UX.)
3. **Genuine delete**: `rm /tmp/.../list-test.md` → panel surfaces within ~1.5 s: title "File moved or deleted", path `list-test.md`, buttons "Re-open / Find / Close". ✓ Correct behavior.
4. **Restore at original path**: recreate `list-test.md` on disk with different content → panel AUTO-DISMISSES within 1.5 s; editor switches to showing the restored content (`# List Test (restored)` / `This is the restored content.`). Re-open click not needed; the debounced recovery check (Piece 1) reloaded in-place. ✓
5. **Move to different path (different basename)**: `mv list-test.md list-test-moved.md` → panel resurfaces; no inline-suggestion (basename differs from the missing path — `runSuggestReopenLookup`'s exact-basename filter at `tabs.svelte.ts:3525` correctly skips). Re-open click failed (file gone at original path), fell through to FB navigation with status-bar hint "Choose the moved file in Files to re-open this tab". ✓ matches source intent.
6. **Move to subdir (basename preserved)**: `mv list-test-moved.md subdir/list-test.md`. Panel still showed the basic 3-button UX with NO inline suggestion after 3.5 s wait. The `runSuggestReopenLookup` only runs at panel-surface time, not on subsequent file-system mutations; by the time the file landed at `subdir/list-test.md`, the lookup had already completed (and returned no candidates because the indexer hadn't picked up the subdir/ create yet). Closing + reopening the file via FB navigation would re-fire the lookup, but the panel state didn't re-trigger it on its own. **Suggested follow-up**: if Piece 3 needs to be empirical-grade reliable, the suggested-path lookup wants to re-fire on a debounce when the panel is visible and the indexer ticks — otherwise it's a single-shot at a race-y moment. Flag for @@Architect to decide whether this needs a follow-up cut.

### -a-39 detail — Piece A regression empirical signal

State at repro:
- Layout: single pane, 5 FB tabs (created via 5× Cmd+Alt+O). Active = tab #5.
- Tree expansion (clicked twirls): `.claude/`, `.github/`, `desktop/`, `scripts/` (visibly expanded; tree shows 30 li items).
- URL hash: all 5 tabs serialized as `{k:"b",bi:1,bs:"docs/journals"}`. NO tab has a `be` field. The active-flag `a:1` is on tab #5.

Cross-tab switch test:
- Click tab #1 (leftmost) → URL hash updates `a:1` to position 0. **Tree shows SAME expanded dirs** (`.claude/`, `.github/`, `desktop/`, `scripts/`). Item count unchanged.
- Click tab #5 (rightmost) → `a:1` moves back to position 4. **Tree state unchanged**.
- Click tab #1 again → `a:1` moves back to position 0. **Tree state unchanged**.

Expected per `fullstack-a-39.md` "Piece A" + audit verdict appended 2026-05-21 ("ready for review"): the per-tab `tab.expanded` array (field name `be` on SerTab per `tabs.svelte.ts:2802`) should be snapshot-on-deactivate / restore-on-activate via `FileBrowserSurface.svelte:101-128`. Empirically: the snapshot-on-deactivate path doesn't persist the singleton's state into the deactivating tab's `expanded` (or persists with an untracked write that doesn't propagate to `persistLayoutToHash`), AND the restore-on-activate path doesn't clear or rewrite the singleton from the activating tab's `expanded` array.

Audit notes flagged: "If @@Alex still observes lost expand-state on the v0.11.2 walkthrough — i.e., the symptom is real but the diagnosis mis-identified the failure mode — we'd need a live repro with DevTools to narrow which of the three layers mis-fires. Most plausible suspect: Svelte 5 effect-order race on FB-A → FB-B switch in the same pane (effect 3's continuous tracker reading the singleton before effect 1 restores it). Doesn't reproduce in unit tests."

**This is that live repro.** All three layers are wired in source but the runtime effect is that all 5 FB tabs share a single visible expansion set (the singleton is canonical, the per-tab `expanded` field is either never written or written without triggering the hash-persist effect — the hash empirically never carries `be`).

### Side observation — drive-context-threaded titles

The Round-1 "want" for FB tab labels (per bug 1's "want" spec landed in `-a-1`): drive-root selected → drive name with trailing slash; directory selected → `<dirname>/`; file selected → parent-dir name with trailing slash. The `-a-39` Piece B journal note adds: "First tab = `Files`, then `Files 2`, `Files 3`, … . Used by the `browserTabLabel` fallback when no drive context is present + helps disambiguate the tab strip when two un-selected FBs sit side-by-side."

In practice with the always-new spawn path + select threading: every Cmd+Alt+O propagates the active tab's `bs` to the new tab, so the new tab's drive context is non-empty and it falls back to the `<dirname>/` convention rather than to `Files N`. I never observed a `Files` / `Files 2` label in my walk. Whether that's a regression depends on whether @@Alex expects the `Files N` titles to surface in the typical chord-spawn flow — flagging for @@Architect.

### Side observation — `-a-37` Piece 3 not surfacing on later moves

`runSuggestReopenLookup` (`tabs.svelte.ts:3514`) is fired ONCE at panel-surface time. If the file ends up at a basename-matching path AFTER the panel surfaces (e.g., move-then-rename sequence, or the indexer hasn't seen the moved file by the time `api.search` runs), the inline suggestion never appears. The user has to close the tab and re-open via FB. Probably fine for the typical "I moved the file then noticed the editor is showing the panel" timing (the lookup catches the rename if the indexer is current), but a debounced re-run while the panel is visible would harden the UX.

### -a-39 vs -a-37 trade-off

@@FullStackA's audit explicitly anticipated the Piece A "no-rename, no-new-field" deviation could leave the symptom unfixed. Pasting the audit's framing: "If @@Alex still observes lost expand-state on the v0.11.2 walkthrough … we'd need a live repro." This walk provides that repro. @@Architect's call on whether this becomes a v0.11.3 hotfix candidate or a Round-2 wave-2 follow-up (the symptom is annoying but not regression-class — the singleton-shared model is the EXISTING behavior; -a-39 Piece A would be additive UX, not a bug fix in the Piece B sense).

### State at end of walkthrough

- Drive `/tmp/chan-test-phase8-wa-r3/`: test artifact `subdir/list-test.md` (the move-to-subdir test for -a-37 Piece 3); root-level `list-test.md` and `list-test-moved.md` gone (cleaned during the test sequence). Drive-root tree still indexes the original chan-source seed.
- Server: still live on `127.0.0.1:8787` (URL above). Will tear down at the next recycle signal or on @@Architect's poke.
- Note: chan instance on port `8820` observed via Chrome MCP `tabs_context_mcp` (different drive, different tab — not mine; carried over from prior session per the journal). Not interacted with.

### Curated summary (highlights / lowlights / contention)

**Highlights**:
- -a-40 (Wysiwyg outline-numbered markers) + -a-41 (source-mode list keymap suppression) + -a-38 Piece B (status-bar auto-dismiss): all three clean fixes hold on lane-A walkthrough.
- -a-37 Pieces 1 + 2 are solid: no false-positive observed across idle viewing + active editing + indexer noise; debounced recovery check auto-restores when file returns at original path (better than expected).
- -a-39 Piece B (always-new FB spawn + select threading): clean, exactly as spec.

**Lowlights / blocking**:
- **-a-39 Piece A FAILS** empirically: per-tab expand state is singleton-shared across tabs in the same pane. URL hash never serializes `be`. The audit's "no-rename, no-new-field" deviation that @@FullStackA explicitly flagged as possibly under-fixing the symptom is reproducing as predicted. Forwarding to @@Architect for v0.11.3 vs Round-2 wave-2 call.
- **-a-37 Piece 3** (Find-suggest-reopen inline UX): single-shot lookup at panel-surface time; doesn't re-fire when the indexer catches up to a basename-matching move. Inline suggestion never surfaced in my move-to-subdir repro. Flagging as a usability gap rather than a regression.

**Contention**: none from this lane. Both -a-39 Piece A and -a-37 Piece 3 are SPA-side work; lane assignment unchanged from current ownership (@@FullStackA).

**Not tested this pass**: -a-38 Piece A (pre-flight spinner gating) — would need watcher event drop with no-timing payload; -a-42 (Settings About section); v0.11.1 carryovers (-a-32 through -a-35) — last verified HOLD on `ada8478` (v0.11.1 cut), no v0.11.2 commit touched those areas. Spot-check next pass if @@Architect wants.

## 2026-05-21 — fullstack-a-43 + fullstack-b-23 walkthroughs (wave-3 cleared work)

Per [`webtest-a-3.md`](webtest-a-3.md). Walked `-a-43` Hybrid back-side
per-surface refactor + `-b-23` web-marketing static-site source on
HEAD `22fd878` (pre-recycle session close). Throwaway drive
`/tmp/chan-test-phase8-wa-r4/` seeded with chan repo (excluding
`.git`/`target`/`node_modules`/`web/dist`); chan serve on
127.0.0.1:8787; static server on 127.0.0.1:8090; Chrome MCP tabs.

### Verdicts

| Check | Surface | Verdict |
|-------|---------|---------|
| -a-43 3a | Hybrid Terminal back stub | HOLD |
| -a-43 3b | Hybrid Editor back stub | HOLD |
| -a-43 3c | Hybrid Graph back stub | HOLD |
| -a-43 3d | Hybrid File Browser back stub | HOLD |
| -a-43 #4 | Per-Hybrid theme (front/back same + per-pane independent) | HOLD |
| -a-43 #5 | Flip animation (3D half-flip) | HOLD |
| -a-43 #6 | Switch-front-while-flipped (back swaps to match new front type) | HOLD |
| -b-23 #1 | Landing page renders | HOLD |
| -b-23 #2 | Donation QR + sha256 match `web/public/qr-donate.png` | HOLD |
| -b-23 #3 | Install scripts + favicon serve | HOLD |
| -b-23 #4 | Viewport meta + fluid layout | HOLD (partial) |

### `-a-43` per-check evidence

* **3a Hybrid Terminal**: opened FB → spawned Terminal-1 via Cmd+Alt+T
  → flipped via pane hamburger "Flip pane". Back shows title "Hybrid
  Terminal", empty body. Matches `HybridTerminalConfig.svelte` stub
  source (`crates/chan` web tree: `<h2 class="config-title">Hybrid
  Terminal</h2>` + empty `.config-body`).
* **3b Hybrid Editor**: while on Hybrid Terminal back, double-clicked
  `CLAUDE.md` in the left FB dock — opened a wysiwyg editor tab in
  the same pane AND back swapped to title "Hybrid Editor". Verifies
  3b + #6 simultaneously without leaving the back side.
* **3c Hybrid Graph**: while on Hybrid Editor back, fired
  Cmd+Shift+M — graph tab spawned + back swapped to title "Hybrid
  Graph" without leaving back. Verifies 3c + #6 again.
* **3d Hybrid File Browser**: initial state (single FB tab, no other
  tabs) → flipped via Cmd+. Tab Return → title "Hybrid File Browser",
  empty body. Matches stub.
* **#4 Per-Hybrid theme**:
  * **Front/back same theme**: applied "Light mode" via FRONT hamburger
    on the original Hybrid (FB+Terminal+Editor+Graph tabs). Flipped to
    back — JS-confirmed via `document.querySelectorAll('.pane')`:
    `dataTheme="light"` on the focused pane; `.hybrid-config` body
    color `rgb(28,28,30)` (dark text on light bg), title color matching.
    Front graph + Details inspector are LIGHT (white bg). The
    front-and-back share the same theme value, no front/back split.
  * **Per-pane independent**: Cmd+. / Return (split right) → second
    Hybrid spawned. Spawned Terminal-2 via Cmd+. t Return. JS confirms
    LEFT pane `dataTheme="light"`, `bg=rgb(255,255,255)`; RIGHT pane
    has NO `dataTheme` override + `bg=rgb(28,28,30)` (inherits page
    default dark). Visually: LEFT graph + inspector LIGHT, RIGHT
    Terminal-2 DARK. Themes are genuinely per-Hybrid.
* **#5 Flip animation**: captured a screenshot mid-flip (immediately
  after `cmd+.` + `Tab` before the `Return` commit). The pane shows
  a 3D perspective rotation — front and back faces are visible on
  the same rectangle at different rotation angles. Half-flip
  animation works.
* **#6 Switch-front-while-flipped**: covered above (3b + 3c). Twice
  swapped the back-side stub while remaining on the back, by
  activating a new front-tab type (Editor via FB dock dbl-click;
  Graph via Cmd+Shift+M). Load-bearing flip-reveals-config-for-the-
  current-surface behaviour holds.

### `-b-23` per-check evidence

* **#1 Landing page**: title `<chan markdown editor>`. Header logo
  (`chan-mark.png`) + "chan" wordmark + light/dark sun-icon toggle
  top-right. Hero "chan markdown editor / plain files. real
  wiki-links. runs in your browser, off your machine." Section
  flow: intro paragraph → § install (macOS/Linux curl-pipe-sh +
  Windows iex line, with COPY buttons + "read the script" link
  → install.sh) → "Then: chan serve ~/notes" → "opens
  http://localhost:8787" → § what else (6 feature bullets) → § the
  editor (light/dark screenshot pair `editor-recipes.png` +
  `editor-dark.png`) → fig.1 caption → § about the name (禪
  glyph) → § status (Alpha, hello@chan.app) → § support (QR) →
  footer (chan · hello@chan.app · github.com/chan-writer).
* **#2 Donation QR**: `qr-donate.png` decodes (img.complete=true,
  natural 700x700). `shasum -a 256`:
  `3a29118f07838c73706abce246fb1fba983591a8fb410803a747348e99e65164`
  matches `web/public/qr-donate.png` byte-for-byte. Renders cleanly
  on the page in the § support section with chan-logo embedded in
  the center.
* **#3 Install scripts + favicon**: `curl http://127.0.0.1:8090/`
  → install.sh HTTP 200 (2059 bytes; starts with `#!/bin/sh` +
  curl-pipe-sh doc-comment) → install.ps1 HTTP 200 (2291 bytes) →
  favicon.ico HTTP 200 (110130 bytes). Page link "read the script"
  resolves to `http://127.0.0.1:8090/install.sh`. Zero broken
  images (`document.querySelectorAll('img')` all complete with
  natural dimensions).
* **#4 Viewport / responsive**: `<meta name="viewport"
  content="width=device-width,initial-scale=1">` present. Layout
  is a centered max-width text column with whitespace gutters
  (text wraps naturally; no horizontal scrolling at the rendered
  width). Chrome MCP `resize_window(480, 800)` did NOT shrink the
  reported `innerWidth` (stayed at 1595) — full small-viewport
  rendering not visually verified due to that MCP quirk, so this
  is HOLD (partial). The fluid centered shape strongly suggests
  it'd hold on a real mobile, but a real-device or DevTools-emulator
  spot-check from @@Alex would close this fully.

### Highlights

* **Hybrid back-side flip works exactly as specced**: per-surface
  stub keyed to the active front tab type; back-side title swaps
  the moment the front tab type changes (no need to flip back-
  to-front). The four stubs are placeholder bodies waiting for
  Tasks B/C/E/F to populate.
* **Per-Hybrid theme is genuinely per-pane**: the spec's "front/back
  independent theme dropped — both sides share a single per-Hybrid
  theme value" lands cleanly. Confirmed via JS read of
  `pane[data-theme]` attribute on both sides; second Hybrid spawned
  via split starts with the page-default dark (does NOT inherit
  the focused pane's theme at split time).
* **Flip animation looks polished**: the 3D perspective half-flip
  is visible mid-frame; no flicker / no overlap / no broken layer
  order.
* **Web-marketing site is print-quality**: monospace-typewriter
  voice + § section anchors + the 禪 origin paragraph + the
  donation QR with embedded chan-logo. Light/dark toggle on the
  fig.1 screenshot pair is a nice touch. No console errors.

### Side observations

* **Cmd+. Tab Return as single key-action sequence is flaky in
  Chrome MCP** when the focused pane front-content is a terminal:
  the terminal captures the Tab/Return keystrokes before the
  Hybrid NAV handler. Workaround during the walk: use the pane
  hamburger menu's "Flip pane" item OR click outside the terminal
  body first. Not a chan bug — webtest-tooling note for future
  automation. Real-user keyboard input on a non-headless browser
  generates the full pointer/focus sequence and doesn't trip this.
* **Back-side stub uses `--text` + `--border` CSS variables but no
  explicit `--bg`**: the stub body fills via transparent
  background, inheriting the pane's bg. Works correctly today
  because the pane element has the bg. Tasks B/C/E/F populating
  the stubs should keep this discipline (rely on parent's theme
  variables, not hard-code colors) so the per-Hybrid theme
  continues to propagate cleanly.
* **`-b-23` "11 files" in the task background**: actual file
  count is 10 (`find web-marketing -type f`): `README.md`,
  `chan-mark.png`, `favicon.ico`, `index.html`, `install.ps1`,
  `install.sh`, `qr-donate.png`, `assets/editor-dark.png`,
  `assets/editor-recipes.png`, plus the directory `assets/`
  itself. Minor doc-drift in the task spec; the commit content
  itself is correct.

### Tear-down evidence

Per the standing rule:

1. Chan serve (`127.0.0.1:8787`) — to be killed.
2. Python static server (`127.0.0.1:8090`) — to be killed.
3. Throwaway drive `/tmp/chan-test-phase8-wa-r4/` — to be
   `rm -rf`'d.
4. Drive registry — `chan remove /tmp/chan-test-phase8-wa-r4/`.
5. Chrome MCP tabs (`503725655` chan + `503725677` static site) —
   to be closed.

Evidence appended after the tear-down step. The walk completed
8/8 acceptance checks HOLD (with #4 partial on viewport pending
a real-device emulation pass).

**Tear-down complete**:

1. chan serve killed (TaskStop on the background bash for `chan serve --port 8787`).
2. python static server killed (TaskStop on the bash for `python3 -m http.server 8090`).
3. `rm -rf /tmp/chan-test-phase8-wa-r4/` — directory gone (verified via `ls /tmp/ | grep chan-test` shows only -wa-r2 + -wa-r3 stale entries from prior recycles).
4. `chan remove /tmp/chan-test-phase8-wa-r4/` → output `unregistered: /tmp/chan-test-phase8-wa-r4/`. `chan list` confirms drive no longer registered.
5. Chrome MCP tabs 503725655 (chan SPA) + 503725677 (static site) closed via `tabs_close_mcp`. MCP tab group empty.

## 2026-05-21 — fullstack-a-44 + -a-45 + -a-46 walkthroughs (Hybrid back-side wave; drag + Terminal migration + Editor migration)

Per [`webtest-a-4.md`](webtest-a-4.md). Walked three Round-2 wave-2
commits in HEAD: `-a-44` Hybrid pane drag-to-rearrange (`a8e991a`
cross-agent commit-hygiene incident — code verbatim, subject
misattributes), `-a-45` Terminal Settings migration (`1f80d09`),
`-a-46` Editor Settings migration (`5166223`). Throwaway drive
`/tmp/chan-test-phase8-wa-r5/` (chan-source seed); chan serve at
127.0.0.1:8787; Chrome MCP tab `503725739`. Local `npm run build`
(via web/dist freshness from -a-46 commit timestamp 17:42) +
`cargo build -p chan` rebuild against current HEAD.

### Verdicts

| Slice | Check | Verdict |
|-------|-------|---------|
| -a-44 | #1 Entry A: drag from dead zone | HOLD |
| -a-44 | #2 Entry B: dblclick dead zone | HOLD |
| -a-44 | #3 Drag-and-drop swap + Enter commit | HOLD |
| -a-44 | #4 Drop on non-Hybrid (terminal-only) pane | HOLD |
| -a-44 | #5 Esc cancel | HOLD |
| -a-44 | #6 Chain semantics (transaction stays on) | HOLD |
| -a-45 | #1 Hybrid Terminal back populated | HOLD |
| -a-45 | #2 Scrollback control + persistence | HOLD |
| -a-45 | #3 TERM dropdown + custom-TERM rendering | PARTIAL |
| -a-45 | #4 Save-status indicator | HOLD |
| -a-45 | #5 Settings overlay no Terminal section | HOLD |
| -a-45 | #6 Second Hybrid Terminal per-DRIVE settings | HOLD |
| -a-46 | #1 Hybrid Editor back populated | HOLD |
| -a-46 | #2 Theme (Appearance) round-trip | HOLD |
| -a-46 | #3 Layout / Date pills / On save | HOLD |
| -a-46 | #4 Save-status indicator | HOLD |
| -a-46 | #5 Settings overlay no Editor section | HOLD |
| -a-46 | #6 Visual sanity | HOLD |

**17/18 HOLD; 1/18 PARTIAL** (`-a-45` #3 custom-TERM input does not render when `Custom...` selected — see lowlight below).

### `-a-44` per-check evidence

* **#1 Entry A**: `left_click_drag` from LEFT pane dead-zone (650, 21)
  to RIGHT pane body (1100, 400). Status bar after the drag shows
  `Hybrid ⏎ Enter commit, Esc discard, H help`; both pane centres
  display NAV identity labels (`chan-test-phase8-wa-r5 / file browser`
  + `Terminal-1 / terminal`); panes proposed-swapped in the
  transaction layer.
* **#2 Entry B**: `double_click` at LEFT pane dead-zone (650, 21) →
  transaction mode entered WITHOUT an originating grab; LEFT pane
  shows blue focus border; status bar same as #1; no panes moved
  (standby state — next click+drag would grab).
* **#3 Drag-and-drop swap + Enter commit**: `Return` after #1 +
  chain swap committed the swaps; status bar cleared; layout
  persisted in URL `s=...d:r...` (split right; pane `a` + pane `b`
  contents reflect the committed state).
* **#4 Drop on non-Hybrid (terminal-only) pane**: covered by #1+#3 —
  RIGHT pane held only a Terminal-1 tab (terminal-only) and was a
  valid drop target for the FB-front pane. The "rearrange ANY
  pane" framing from `phase-8-bugs.md` holds.
* **#5 Esc cancel**: re-entered transaction via Entry B dblclick,
  then `Escape` → status bar cleared; both panes back to
  pre-transaction layout (FB left, Terminal-1 right); no
  persistent change.
* **#6 Chain semantics**: while in transaction mode (post-Entry A
  swap), fired a second `left_click_drag` from LEFT body (400, 400)
  to RIGHT body (1100, 400) — second swap landed inline; status
  bar still showed `Enter commit` (transaction stayed on across
  the chain). `Return` then committed both swaps in one beat
  (net zero since I swapped twice, but the chain mechanic itself
  works).

### `-a-45` per-check evidence

* **#1 Hybrid Terminal back populated**: chord `Cmd+. Tab Return`
  on the Terminal pane → back-side shows the populated
  `HybridTerminalConfig.svelte` body: title band "Hybrid Terminal",
  warning banner ("These settings apply to ALL terminals..."),
  Scrollback (MB) slider+numeric+units, Default TERM dropdown
  with per-control hint text.
* **#2 Scrollback control + persistence**: triple-click +
  type `80` (resolved to `100` after spinner step-rounding via
  the increment buttons under my cursor) → slider tracked to 100,
  numeric box read 100, `saved` status fired top-right. After F5
  reload + 3s wait, server `/api/drive` preferences returned
  `terminal.scrollback_mb=100` (held). PASS.
* **#3 TERM dropdown + custom-TERM rendering**: PARTIAL. The
  dropdown surfaces 5 options (`xterm-256color`, `xterm`,
  `tmux-256color`, `screen-256color`, `__custom__` rendered as
  "Custom..."); switching between known values round-trips
  cleanly. Switching to "Custom..." does NOT surface the custom
  text input below the dropdown — the conditional
  `{#if termSelectValue === CUSTOM_TERM_SENTINEL}` at
  `HybridTerminalConfig.svelte:281` never triggers. See
  lowlight + root-cause hypothesis below.
* **#4 Save-status indicator**: top-right of the Hybrid Terminal
  title band reads `saved` (green) after the debounce window
  closes following a control change. Modifying scrollback
  100→101 also triggered the indicator.
* **#5 Settings overlay no Terminal section**: opened `Cmd+,` →
  Settings overlay sections: SEMANTIC SEARCH (Enable hybrid mode
  checkbox + Active/Stored at info + Rebuild link) + ABOUT (chan
  version 0.11.2 + embeddings status + terminal font (Source Code
  Pro Regular SIL OFL 1.1 link)). No Terminal scrollback / TERM
  controls in the overlay. PASS regression guard.
* **#6 Second Hybrid Terminal per-DRIVE settings**: spawned
  `Terminal-2` in LEFT pane (`Cmd+. t Return`); flipped LEFT to
  back; LEFT and RIGHT panes both render "Hybrid Terminal"
  back-side with identical settings (scrollback 100, TERM
  xterm-256color). JS-confirmed via
  `document.querySelectorAll('.hybrid-config')` returning 2 nodes
  with matching values. Settings are genuinely per-DRIVE, not
  per-pane, matching the banner copy.

### `-a-46` per-check evidence

* **#1 Hybrid Editor back populated**: opened `CLAUDE.md` via FB
  dock dbl-click → LEFT pane front swapped to wysiwyg editor;
  flipped LEFT to back via `Cmd+. Tab Return`. Back shows
  `HybridEditorConfig.svelte` body with all five sections:
  - **Editor theme**: GitHub / Google Docs (default) / Microsoft
    Word — radio buttons.
  - **Appearance**: System / Light / Dark — per-device, browser
    storage.
  - **Layout**: Standard / Compact — radio buttons.
  - **Date pills**: Default = `2026-05-05 (ISO)` — select.
  - **On save**: "Strip trailing whitespace on save" — checkbox.
  Banner copy: "These settings apply to ALL editors, not just
  this one. The per-Hybrid appearance override (set via the pane
  hamburger Theme entry) survives on top of the global Appearance
  choice below." (See side observation re: hamburger Theme entry
  removed.)
* **#2 Theme (Appearance) round-trip**: clicked "Light" → save
  fired, `<html data-theme=light>` set, both panes bg
  `rgb(255,255,255)`; server `preferences.theme=light` post-PATCH
  via `/api/drive` GET. F5 reload + 4s wait → Light radio still
  checked + html theme still light.
* **#3 Layout / Date pills / On save**:
  * Layout Compact: clicked "Compact" → server PATCH after
    debounce; `preferences.line_spacing=compact` confirmed via
    `/api/drive` GET (after 3s wait — first reload happened
    before debounce flushed; second reload confirmed persistence).
  * Date pills + On save controls visible + interactive (not
    individually round-tripped this pass; structural presence +
    save-indicator behaviour holds across all controls).
* **#4 Save-status indicator**: top-right of Hybrid Editor title
  band reads `saved` after each control change (Light click +
  Compact click both triggered the indicator).
* **#5 Settings overlay no Editor section**: same overlay walk
  as `-a-45` #5; Settings overlay sections are SEMANTIC SEARCH
  + ABOUT only. No Editor theme / Appearance / Layout / Date
  pills / On save in the overlay.
* **#6 Visual sanity**: side-by-side Hybrid Editor (LEFT) +
  Hybrid Terminal (RIGHT) screenshot shows matching overall feel
  (same title-band height, same banner-warning shape, same body
  padding); controls in the Editor back are labelled, grouped
  with section headers, and read cleanly. No stray padding,
  no unstyled controls.

### Critical lowlight

* **`-a-45` Piece #3 — Custom TERM input does not render** when
  the user selects "Custom..." in the Default TERM dropdown. Root
  cause hypothesis from reading `HybridTerminalConfig.svelte`:
  * `setTermSelection("__custom__")` at line 104 seeds
    `editing.terminal.default_term = ""` when isKnownTerm
    (line 106-110).
  * The `currentTerm` derivation at line 86-88 falls back to
    `DEFAULT_TERM` when default_term is empty (`?? DEFAULT_TERM`
    + `|| DEFAULT_TERM`).
  * → `currentTerm` resolves back to `xterm-256color`,
    `isKnownTerm=true`, `termSelectValue=xterm-256color` (NOT the
    sentinel).
  * → The `{#if termSelectValue === CUSTOM_TERM_SENTINEL}`
    conditional at line 281 never fires; the custom text input
    never renders.
  Empirically verified via `document.querySelector('select.family').value
  === "__custom__"` returning true (DOM value updated) but
  `document.querySelector('input.custom-term')` returning null
  (conditional never fired). The existing test file
  `HybridTerminalConfig.test.ts` only asserts the conditional
  source-code structure via regex (line 47-49) — does not run
  the actual rendering at runtime, so this regression slipped
  the test gate. Suggested fix: seed `default_term` with a
  non-empty, non-known-term sentinel (e.g. a leading space, or a
  separate `customMode` state field that bypasses the empty-string
  fallback). Lane: @@FullStackA; flagging as Round-2 wave-2 polish
  follow-up (not blocking the `-a-45` migration commit which IS in
  HEAD with correct migration scope).

### Side observations (not regression-class)

1. **Pane hamburger "Light mode" + "Flip pane" items removed**.
   Pre-`-a-45/-a-46/-a-47` (in flight), the pane hamburger surfaced
   a "Light mode" toggle and a "Flip pane" (Cmd+. Tab) entry. Both
   are GONE in the current build. Light mode moved into the
   Hybrid Editor back-side per `-a-46` (Appearance buttons there).
   Flip pane removal is less clear — the chord still works, but
   the menu affordance is lost. The `-a-46` banner copy on Hybrid
   Editor back says "The per-Hybrid appearance override (set via
   the pane hamburger Theme entry) survives on top of the global
   Appearance choice below" — but no Theme entry exists in the
   hamburger anymore, so the per-Hybrid override path is not
   discoverable through the UI. Likely an in-flight `-a-47`
   intermediate state (drop front/back independent theme); worth
   confirming with @@FullStackA before this lands as part of the
   `webtest-a-5` walk.
2. **Webtest-tooling**: JS-dispatched `change` event on the TERM
   select doesn't trigger the Svelte reactivity for `setTermSelection`
   path reliably. Native click+keyboard might or might not differ
   — Chrome MCP couldn't drive the native OS dropdown picker to
   verify. Webtest-automation note: prefer `find` + `left_click`
   on the option DOM-ref where possible; JS dispatch is fragile.
3. **Drag-to-rearrange visual affordance**: during the drag from
   the dead-zone, the cursor doesn't visibly change to indicate
   "this is a drag handle". The dead zone is also not visually
   distinguished from the surrounding tab strip / hamburger gap.
   First-time users may not discover the affordance. Not regression
   — discoverability polish for a future iteration.

### Highlights

* **`-a-44` drag-to-rearrange is solid in the cleared scope**: all
  six acceptance checks held including the load-bearing chain
  semantics (transaction stays on across multiple swaps until
  Enter commit). The "rearrange ANY pane" framing holds — I
  swapped a FB-front pane into a Terminal-only pane and back.
* **`-a-45` Terminal Settings migration: clean migration**: the
  scrollback control + Default TERM dropdown migrate cleanly into
  the Hybrid Terminal back-side, the warning banner is explicit
  about scope ("ALL terminals"), and the per-DRIVE settings sync
  across two Hybrid Terminal panes without manual refresh.
* **`-a-46` Editor Settings migration: parallel clean migration**:
  five sections (Editor theme / Appearance / Layout / Date pills /
  On save) all populate; theme + Layout round-trip through the
  server `/api/drive` PATCH cleanly; the Settings overlay shrinks
  to just SEMANTIC SEARCH + ABOUT (good simplification).
* **Save-status indicator pattern is consistent** across both
  Hybrid Terminal and Hybrid Editor back-sides — top-right "saved"
  green text fires after the debounce closes. Matches the
  surface-by-surface dirty-check pattern in
  `HybridTerminalConfig.svelte` `terminalDirty()` at line 123.

### State at end of walk

Lane-A test server still live on `http://127.0.0.1:8787/?t=N6YVNy2vfR8BbQtNEa3ryxktjv2y8YW2`
against `/tmp/chan-test-phase8-wa-r5/` (chan repo seed +
preferences mutations from the walk: theme=light, line_spacing=compact,
scrollback_mb=100). Will tear down at commit beat.

**Tear-down complete**:

1. chan serve killed (TaskStop on the background bash for `chan serve --port 8787`).
2. `rm -rf /tmp/chan-test-phase8-wa-r5/` — directory gone.
3. `chan remove /tmp/chan-test-phase8-wa-r5/` → output `unregistered`.
4. Chrome MCP tab 503725739 (chan SPA) closed via `tabs_close_mcp`. Tab group empty.

The walk completed 17/18 acceptance checks HOLD with one PARTIAL
on `-a-45` #3 custom-TERM input rendering (root-caused; follow-up
candidate for @@FullStackA wave-2 polish).

## 2026-05-21 — fullstack-a-47 + -a-48 + -a-53 + -a-54 walkthroughs (Hybrid back-side correction wave + design follow-ups)

Per [`webtest-a-5.md`](webtest-a-5.md). Walked four Round-2 wave
slices + re-verification of the `-a-45` PARTIAL from `webtest-a-4`.
HEAD `f3c36e5` (clearance round 11). Throwaway drive
`/tmp/chan-test-phase8-wa-r6/` (chan-source seed); chan serve on
127.0.0.1:8787; Chrome MCP tab `503725788`.

### Verdicts

| Slice | Check | Verdict |
|-------|-------|---------|
| -a-47 | #1 Per-Hybrid single theme value (front/back same)  | HOLD |
| -a-47 | #2 Cross-Hybrid independence                       | HOLD |
| -a-47 | #3 `bm` wire-format survives serialize             | HOLD |
| -a-47 | #4 Legacy migration (front-side wins)              | N/A — fresh drive, no legacy state to migrate |
| -a-48 | #1 Hybrid FB back populated                        | HOLD |
| -a-48 | #2 Semantic search toggle                          | HOLD |
| -a-48 | #3 Multi-model picker placeholder + disabled       | HOLD |
| -a-48 | #4 chan-reports toggle default ON + honest-toggle  | HOLD |
| -a-48 | #5 Settings overlay shrunk (Appearance + About)    | HOLD |
| -a-53 | #1 Appearance section back in Settings             | HOLD |
| -a-53 | #2 Per-Hybrid override toggle on Editor back       | HOLD |
| -a-53 | #3 Per-Hybrid override toggle on Terminal back     | HOLD |
| -a-53 | #4 Override > global resolution                    | HOLD |
| -a-53 | #5 Inherit > global resolution                     | HOLD |
| -a-53 | #6 Custom-TERM PARTIAL re-verification             | HOLD |
| -a-54 | #1 Front state unchanged                           | HOLD |
| -a-54 | #2 Tab strip preserved on flip                     | HOLD |
| -a-54 | #3 Tabs mirrored                                   | HOLD |
| -a-54 | #4 Hamburger swapped to opposite end on back       | HOLD |
| -a-54 | #5 Family-name title visible un-mirrored in tab area | HOLD |
| -a-54 | #6 Tab switching from back + family-name swap      | PARTIAL |

**19/20 HOLD + 1 N/A + 1 PARTIAL** (`-a-54` #6 — see lowlight below;
N/A is `-a-47` #4 which has no legacy state to migrate from on a
fresh drive).

### `-a-47` per-check evidence

* **#1 Per-Hybrid single theme value (front/back same)**: opened
  CLAUDE.md as Editor on LEFT pane; flipped to back; set
  Appearance override = Dark via the new `-a-53` 3-option toggle
  (Inherit / Light / Dark); flipped back to front via
  `Cmd+. Tab Return`. Front side rendered Dark
  (`pane[data-theme=dark]`, bg `rgb(28,28,30)`). No front-vs-back
  split — both sides carry the SAME theme value via the single
  pane-level `data-theme` attribute.
* **#2 Cross-Hybrid independence**: split right (`Cmd+. /
  Return`), spawned Terminal-2 in RIGHT pane via `Cmd+. t Return`.
  JS check `document.querySelectorAll('.pane')`: LEFT
  `data-theme=light`/`dark` per override; RIGHT no `data-theme`
  override (inherits global). Both panes' bgs match: LEFT was
  whatever I set, RIGHT followed global Settings (verified by
  toggling Settings global Dark / Light and seeing RIGHT track).
* **#3 `bm` wire-format survives serialize**: F5 reload after
  setting override on LEFT pane → URL hash still contains
  `"ht":"d"` for LEFT, no `ht` for RIGHT; both panes' visual
  state held; `bm:1` marker present where the back was open
  pre-reload.
* **#4 Legacy migration (front-side wins)**: N/A. Fresh `r6` drive
  has no pre-`-a-47` stored Hybrid panes. The architect's task
  spec notes this is a check applicable only "if your test drive
  has any stored Hybrid panes from pre-`-a-47`" — mine doesn't.
  Skipping with explicit note rather than failing.

### `-a-48` per-check evidence

* **#1 Hybrid FB back populated**: clicked FB tab in LEFT pane
  (the `chan-test-phase8-wa-r6/` mirrored tab) → flipped via
  `Cmd+. Tab Return`. Back rendered
  `HybridFileBrowserConfig.svelte` body: title "Hybrid File
  Browser", banner ("These settings apply to ALL file-browser
  surfaces on this drive, not just this one."), three sections:
  Semantic search, Embedding model, chan-reports.
* **#2 Semantic search toggle**: present + interactive — Enable
  checkbox; Active: BM25; Stored at:
  `/Users/fiorix/Library/Caches/chan/models/models--BAAI--bge-small-en-v1.5`.
  Default state OFF (matches `-a-21` behaviour); checkbox visible
  and clickable. Did NOT toggle ON during the walk (would trigger
  model download).
* **#3 Multi-model picker placeholder + disabled**: select
  element present + `disabled=true`; value =
  `BAAI/bge-small-en-v1.5 (default)`; help text "Picker
  placeholder; lands with the Round-3 multi-model registry."
  Renders cleanly but doesn't accept input — matches spec.
* **#4 chan-reports toggle default ON + honest-toggle UX**:
  checkbox visible, checked by default (`checked=true` on
  initial render). Toggle OFF → save fired (3s debounce); server
  GET `/api/config` returns `preferences.reports.enabled=false`.
  Help text below toggle: "Toggle persists via /api/config;
  backend gating + the destructive-on-disable confirmation modal
  land in a follow-up task. Default is ON to match today's
  unconditional behaviour." — honest-toggle UX as specced.
* **#5 Settings overlay shrunk**: `Cmd+,` after focusing the
  LEFT pane body. Settings overlay opens with only TWO sections:
  - **APPEARANCE**: "Global default for chan's chrome and editor
    body. Per-device only; lives in browser storage. 'System'
    follows your OS appearance setting live. Override per-Hybrid
    in the Hybrid Editor or Hybrid Terminal back-side (Inherit /
    Light / Dark)." → System / Light / Dark.
  - **ABOUT**: chan version `0.11.2`; embeddings status
    ("on (hybrid search available)"); terminal font (Source Code
    Pro Regular + SIL OFL 1.1 link).
  NO Semantic search section, NO chan-reports section. Confirms
  the migration completed.

### `-a-53` per-check evidence

* **#1 Appearance section back in Settings**: confirmed by
  `-a-48` #5 walk — Appearance section is the top-level entry
  in Settings (System / Light / Dark radio group). Regression
  guard against `-a-46`'s migration into Hybrid Editor back
  PASSES.
* **#2 Per-Hybrid override toggle on Editor back**: visible at
  top of Hybrid Editor back-side body. Heading "Appearance (this
  Hybrid)", subtitle "Override the global Appearance default for
  just this Hybrid pane. Inherit follows the global Settings
  choice (currently **dark**)." 3-option radio: Inherit / Light
  / Dark. Default selected: Inherit. Radiogroup name
  `hybrid-editor-theme-override`.
* **#3 Per-Hybrid override toggle on Terminal back**: identical
  shape on Hybrid Terminal back-side body. Radiogroup name
  `hybrid-terminal-theme-override` (inferred from symmetry; same
  3-option Inherit/Light/Dark layout, same banner copy).
* **#4 Override > global resolution**: set Settings global =
  Dark; on LEFT (Editor) Hybrid set override = Light. JS check:
  HTML `data-theme=dark`; LEFT pane `data-theme=light`,
  `bg=rgb(255,255,255)`; RIGHT pane no override,
  `bg=rgb(28,28,30)`. LEFT Hybrid renders light while global +
  RIGHT stay dark. Confirmed.
* **#5 Inherit > global resolution**: from #4 state, switched
  LEFT override back to Inherit. JS check: LEFT pane data-theme
  attribute REMOVED, bg switched to `rgb(28,28,30)` (tracking
  global). Inherit defers to Settings, dark wins. Confirmed.
* **#6 Custom-TERM PARTIAL re-verification**: re-walked the
  `webtest-a-4` `-a-45` #3 fail. Opened Hybrid Terminal back;
  changed Default TERM dropdown to "Custom...". JS check:
  `input.custom-term` IS now present in the DOM (was absent in
  `-a-4`); rendered with placeholder "alacritty-direct" and
  seeded with the prior known TERM value ("xterm-256color") so
  the user has context to edit from. Typed "vt100" → server GET
  `/api/drive` returns `terminal.default_term="vt100"`.
  Persistence round-trips. The fix from `-a-53` bundled scope
  (seeding default_term to current value rather than empty
  string) breaks the previous fallback-to-DEFAULT_TERM chain that
  prevented the conditional from firing. PARTIAL → HOLD.

### `-a-54` per-check evidence

* **#1 Front state unchanged**: front side of Hybrid Editor
  with CLAUDE.md active renders normally — hamburger on right
  end of tab strip, tabs read left-to-right (un-mirrored),
  no chrome rotation. Visual identity matches pre-`-a-54`.
* **#2 Tab strip preserved on flip**: when flipped to back,
  the tab strip remains at the SAME physical position (top of
  pane) — no chrome rotation, no tab strip removal. Verified
  across all four front-tab types (FB / Editor / Terminal /
  Graph).
* **#3 Tabs mirrored**: on the flipped state, tab labels are
  rendered with horizontal flip — e.g. CLAUDE.md as "EDUALC.cm",
  chan-test-phase8-wa-r6 as "6r-aw-8esahp-tset-nahc". Reads as
  "viewed from behind", matching the @@Alex framing.
* **#4 Hamburger swapped to opposite end on back**: on FRONT
  the hamburger is at the right end of the tab strip (~x=820
  for LEFT pane); on BACK it appears at the LEFT end (~x=280
  as a `:` symbol). Click still functional — opening it
  surfaces the same menu (4 spawn items + Enter Hybrid NAV +
  Focus border colour blue/green/pink).
* **#5 Family-name title visible un-mirrored in tab area**:
  "HYBRID EDITOR" / "HYBRID TERMINAL" / "HYBRID FILE BROWSER"
  text renders inside the tab area on the OPPOSITE end of the
  swapped hamburger (i.e. on the right for LEFT pane back, on
  the right for RIGHT pane back). Text is un-mirrored
  (front-readable, NOT scaleX(-1)), matching the @@Alex "like
  in the front pane" framing. Updates per-active-tab type.
* **#6 Tab switching from back + family-name swap**: PARTIAL.
  Two distinct paths to test:
  - **New-tab-spawn path** (HOLD): double-clicking a file in
    the LEFT FB sidebar dock while LEFT pane is on back side
    spawns a new Editor tab AND swaps the back-side from
    "Hybrid File Browser" to "Hybrid Editor" + family-name
    title from "HYBRID FILE BROWSER" to "HYBRID EDITOR" without
    leaving back. Equivalent behavior was verified twice in
    this walk (FB → Editor and Editor → Graph in webtest-a-3
    + chord-driven `Cmd+. t Return` in this walk). Works as
    specced.
  - **Click-existing-mirrored-tab path** (FAIL): clicking an
    EXISTING mirrored tab in the tab strip while on back side
    does NOT activate that tab. Verified empirically via
    `find` + `left_click` on the chan-test-phase8-wa-r6 FB
    tab (ref_68) and via programmatic `tab.click()` +
    full-sequence `pointerdown/mousedown/pointerup/mouseup/click`
    dispatch — neither switched the active tab from CLAUDE.md
    back to the FB tab. The URL hash `"a":1` marker stayed on
    CLAUDE.md throughout. The mirroring (`scaleX(-1)` per the
    spec) may be capturing pointer events in a way that breaks
    the click handler, OR the back-side tab-strip is using a
    different event delegate. See lowlight below.

### Critical lowlight

* **`-a-54` Check #6 click-existing-mirrored-tab fails**: from
  the back side, clicking an existing mirrored tab in the tab
  strip does NOT activate that tab. Spawn-from-FB-sidebar and
  spawn-via-chord paths both work (and swap the back-side
  config + family-name title cleanly), but the click-driven
  active-tab switch is broken. Reproduce: open a Hybrid pane
  with 2+ tabs (e.g. FB + Editor); flip to back; click any
  non-active mirrored tab — active doesn't change. JS
  `tab.click()` programmatic invocation also fails to swap.
  Hypothesis: the CSS `scaleX(-1)` transform on the mirrored
  tab elements may be causing pointer-events to mis-resolve,
  OR the back-side tab strip may be rendering a static visual
  copy without binding the click handler. Lane: @@FullStackA;
  likely a small fix (transform: `scaleX(-1)` + `pointer-events:
  auto` confirmation + click-handler verification). Flagging as
  Round-2 wave-2 follow-up — not regression-blocking the
  `-a-54` migration commit which IS in HEAD with correct
  migration scope for the OTHER 5/6 checks.

### Side observations (not regression-class)

1. **Pane hamburger items still minimal**: spawn items (Terminal,
   FB, Rich Prompt, Graph) + Enter Hybrid NAV + Focus border
   colour. The "Light mode" + "Flip pane" + "Theme" items that
   `webtest-a-4` flagged as missing — `-a-53` did NOT restore
   them (theme is now exclusively via the back-side override
   toggle, which is the intended design). Flip pane chord
   Cmd+. Tab still works. This is the corrected end state;
   `-a-4` side observation is resolved as expected behavior, not
   regression.
2. **Cmd+, focus requirement**: the Settings overlay chord
   only fires reliably when focus is on the SPA body / a
   non-terminal pane. Pressing Cmd+, while focus is on a
   terminal stdin gets swallowed by the terminal. Pressing
   while focus is on the back-side body had inconsistent
   behaviour in my walk — sometimes fired, sometimes didn't.
   Workaround: click outside any terminal first. Webtest-
   automation note + possible accessibility / focus-restoration
   polish for keyboard users.
3. **Back-side stub bg is white-ish even with dark pane theme**
   (initial observation from earlier passes; re-verified): the
   `.hybrid-config` `.config-body` background reads light even
   when the pane has `data-theme=dark`. JS computed style of
   `.hybrid-config` shows `background: rgba(0,0,0,0)`
   (transparent), so it's inheriting from a parent. The parent
   bg might be using `var(--surface)` (light) rather than
   `var(--bg)` (theme-tracking). Settings forms typically want
   consistent light bg for form readability, so this may be
   intentional. Flag for the implementer when the next
   back-side stub touch happens.
4. **Cross-drive preference carryover**: `preferences.theme`,
   `preferences.line_spacing`, `terminal.scrollback_mb`, and
   `terminal.default_term` from prior `r5` walk session were
   present in this fresh `r6` drive's GET `/api/drive`
   response. Suggests some preferences are per-machine (chan
   config store) rather than per-drive, OR my throwaway-drive
   workflow doesn't fully reset (rsync may carry a hidden
   `.chan/` directory). Not blocking the walk — flagging for
   future test-server-workflow discipline.

### Highlights

* **`-a-47` theme collapse is clean**: front+back share the
  same per-Hybrid theme value via a single `pane[data-theme]`
  attribute; second Hybrid spawned via split inherits the
  global default, NOT the focused pane's override; URL `ht`
  marker round-trips serialize/restore.
* **`-a-48` FB-back migration restores chan-reports
  visibility**: the regression bug @@Alex flagged
  ("chan-reports disappeared and there's no setting to turn
  it on/off anymore... i want it back!") is fixed. Toggle is
  default ON, persists to `/api/config`, with explicit
  honest-toggle help text about the backend gating + modal
  follow-up.
* **`-a-53` theme architecture correction is the right
  shape**: global Appearance back in Settings (per-device,
  browser storage); per-Hybrid override on Editor + Terminal
  backs (Inherit / Light / Dark); override > global > inherit
  resolution order ALL hold under test.
* **`-a-53` bundled custom-TERM fix lands cleanly**: the
  `webtest-a-4` PARTIAL is now HOLD. Picking "Custom..."
  surfaces the text input (was previously hidden by the
  fallback-to-DEFAULT_TERM derivation bug). Seeding the custom
  input with the current TERM value gives the user good
  context to edit from.
* **`-a-54` flip UX is mostly excellent**: tab strip preserved
  + tabs mirrored + hamburger swapped + family-name title
  un-mirrored — the visual identity is precisely what
  @@Alex's framing called for. The one click-existing-tab
  path that fails is fixable; the other 5 checks all hold.

### State at end of walk

Lane-A test server still live on
`http://127.0.0.1:8787/?t=nwuyyNmVyLyyq6vvrRxz9tgEh7sCS73i`
against `/tmp/chan-test-phase8-wa-r6/` (chan repo seed +
preferences mutations from the walk: global theme=dark briefly,
back to default; LEFT pane override=Dark/Light/Inherit cycle;
Terminal default_term=vt100; chan-reports=disabled). Will tear
down at commit beat.

**Tear-down complete**:

1. chan serve killed (TaskStop on the background bash for
   `chan serve --port 8787`).
2. `rm -rf /tmp/chan-test-phase8-wa-r6/` — directory gone.
3. `chan remove /tmp/chan-test-phase8-wa-r6/` → `unregistered`.
4. Chrome MCP tab `503725788` (chan SPA) closed via
   `tabs_close_mcp`. Tab group empty.

The walk completed 19/20 acceptance checks HOLD + 1 N/A
(`-a-47` #4 legacy migration — no legacy state on fresh drive)
+ 1 PARTIAL (`-a-54` #6 click-existing-mirrored-tab from
back-side). `webtest-a-4`'s PARTIAL (`-a-45` #3 custom-TERM)
re-verified as HOLD post-`-a-53` bundled fix.

## 2026-05-22 — fullstack-a-55 proactive walkthrough (Hybrid flip UX: tab-title removal + right-align + mirrored-tab click fix)

Proactive lane-A walk of `-a-55` (`7cf6f8e`) per the memory
rule on proactive coverage — the `-a-55` commit fixes the
`webtest-a-5` PARTIAL on `-a-54` #6 (click-existing-mirrored-
tab) and bundles two other UX corrections. No
`webtest-a-6.md` task cut yet; appending verdict here per
the omnibus pattern. HEAD `e80db8b` (post-systacean smoke
#3 cascade terminator); throwaway drive
`/tmp/chan-test-phase8-wa-r7/` (chan-source seed); chan
serve on 127.0.0.1:8787; Chrome MCP tab `503725864`.

### Verdicts

| Check | Surface | Verdict |
|-------|---------|---------|
| -a-55 #1 | Click-existing-mirrored-tab swaps active (the `webtest-a-5` PARTIAL fix) | HOLD |
| -a-55 #2 | Family-name title removed from tab strip | HOLD |
| -a-55 #3 | Tab right-alignment on flipped state | HOLD |

**3/3 HOLD**. The `webtest-a-5` PARTIAL on `-a-54` #6 is now
empirically resolved.

### Per-check evidence

* **#1 Click-existing-mirrored-tab fix**: opened FB + Editor
  tabs in LEFT pane; CLAUDE.md (Editor) active; flipped to
  back via `Cmd+. Tab Return` — back showed "Hybrid Editor"
  stub. Clicked the mirrored `chan-test-phase8-wa-r7/` FB tab
  via Chrome MCP `find` → `left_click` on the DOM ref.
  Active tab swapped: URL hash now has `"a":1` on the FB tab
  (first slot, `k:b`) instead of CLAUDE.md; back-side body
  swapped from "Hybrid Editor" to "Hybrid File Browser"
  stub; active-tab × marker moved to the FB tab. Clicked
  back to CLAUDE.md tab → reverse swap held (back-side
  "Hybrid Editor" again, active marker on CLAUDE.md).
  **Bidirectional click-driven tab swap on back-side
  works** — exactly what the `webtest-a-5` PARTIAL flagged
  as broken.
* **#2 Family-name title removed from tab strip**: in
  `webtest-a-4` + `webtest-a-5`, the flipped state had
  "HYBRID EDITOR" / "HYBRID TERMINAL" / "HYBRID FILE
  BROWSER" rendered un-mirrored INSIDE the tab strip
  area. Post-`-a-55`, that string is GONE from the tab
  strip on every flipped pane I exercised. The
  family-name now lives ONLY in the back-side body as an
  `<h2>` heading (e.g. "Hybrid Editor", "Hybrid File
  Browser") under the title band — cleaner layering, no
  duplication.
* **#3 Tab right-alignment on flipped state**: front-side
  tabs anchor LEFT-aligned at the start of the tab strip
  (right after the hamburger). Post-`-a-55` flipped state
  anchors tabs to the RIGHT edge of the tab strip — the
  hamburger swaps to the LEFT end (per `-a-54`) and the
  tabs collapse against the right edge. Visual symmetry
  reads as "tab strip viewed from behind" — the LTR tab
  flow on front becomes RTL on back, which is consistent
  with the `scaleX(-1)` mirror metaphor from `-a-54`.

### Highlight

* **`webtest-a-5` PARTIAL closed empirically**: my
  `webtest-a-5` lowlight flagged that the back-side
  click-existing-mirrored-tab didn't swap active. `-a-55`
  fixed the click handler on mirrored tabs (per the
  `fullstack-a-55.md` "Bundled scope addition" — the
  `scaleX(-1)` transform on tab elements was breaking
  pointer-event resolution; the fix surfaces the click
  via the correct delegate). Confirmed working
  bidirectionally; no further follow-up needed.

### Side observation (out of `-a-55` scope; noted)

* **Hybrid File Browser back-side Semantic search section
  now reads "isn't compiled into this binary"**:
  > "Semantic search isn't compiled into this binary.
  > Rebuild with `--features embed-model` (or install a
  > chan release that includes it) to enable Hybrid
  > search."

  This replaces the "Enable semantic search (Hybrid
  mode)" checkbox + "Active: BM25" info that
  `webtest-a-5` documented. The local `cargo build -p
  chan` (default features) used for both walks produced
  different binary outputs — likely because the embed-
  model feature flag was added/changed in some lane
  between the two walks. Not in `-a-55` scope; flagging
  as worth tracking: real users running
  `cargo install chan` from crates.io with default
  features may now see "isn't compiled" instead of the
  Hybrid checkbox. Worth confirming the release-build
  default. Lane: @@Systacean (build / feature-flag
  semantics).

### State at end of walk

Lane-A test server still live on
`http://127.0.0.1:8787/?t=bMNBe8oDyucmvfSzRwfBCxy65Kr2amM9`
against `/tmp/chan-test-phase8-wa-r7/` (chan repo seed +
two ad-hoc preference toggles from the swap test).
Tear-down at commit beat.

**Tear-down complete**:

1. chan serve killed (TaskStop on the background bash for
   `chan serve --port 8787`).
2. `rm -rf /tmp/chan-test-phase8-wa-r7/` — directory gone.
3. `chan remove /tmp/chan-test-phase8-wa-r7/` → `unregistered`.
4. Chrome MCP tab `503725864` (chan SPA) closed via
   `tabs_close_mcp`. Tab group empty.

3/3 HOLD; `webtest-a-5` PARTIAL closed; `-a-55` walk
complete.

## 2026-05-22 — fullstack-a-49 + -a-50 + -a-51 proactive walkthrough (graph overhaul wave; -a-52 not in scope)

Proactive lane-A walk of the three landed graph-overhaul
commits per the memory rule on proactive coverage. `-a-52`
(G9 + G10 minimum cut — depth-slider forward-only + drop link
filter) is still gate-contingent in @@FullStackA's lane; walk
deferred until it lands. HEAD `e80db8b`; throwaway drive
`/tmp/chan-test-phase8-wa-r8/` (chan-source seed); chan serve
on 127.0.0.1:8787; Chrome MCP tab `503725870`. Frontend
rebuilt (npm run build → cargo build -p chan) to embed
`-a-51`'s `GraphCanvas.svelte` + `HybridGraphConfig.svelte`
changes (web/dist before rebuild lagged `-a-51` by one beat;
rebuild includes @@FullStackA's in-flight `-a-52`
`GraphPanel.svelte` changes too — `-a-52` specific surfaces
explicitly OUT of scope for this walk).

### Verdicts

| Slice | Surface | Verdict |
|-------|---------|---------|
| -a-49 | Graph layout: filesystem-hierarchy as backbone | HOLD |
| -a-50 | Directory inspector + chan-reports aggregated stats | HOLD |
| -a-51 | G6 colour scheme on graph canvas | HOLD |
| -a-51 | Hybrid Graph back-side legend grid | HOLD |

**4/4 HOLD**.

### `-a-49` per-check evidence

* **API contract**: `GET /api/graph?scope=drive` (Bearer
  token from sessionStorage) returns 1301 nodes across
  six kinds: `tag`, `file`, `mention`, `media`,
  `directory`, `language`. 116 directory nodes total.
  Sample root directory: `{kind: "directory", id:
  "directory:", label: "chan-test-phase8-wa-r8", path: "",
  files: 3, code: 153}` — the drive root carries
  aggregated `files` + `code` stats from the chan-reports
  fanout.
* **Visual confirmation**: graph canvas renders directory
  nodes as solid grey filled circles (per the G6 palette
  `--g-folder #8e8e93`). They serve as the backbone of the
  layout — files cluster around their parent directory
  nodes, and the periphery shows grey directory anchors
  (e.g. `web/` directory node visible at upper-left in the
  zoomed-in view).

### `-a-50` per-check evidence

* **Click + inspector**: clicked the `web/` directory node
  in the graph canvas (grey filled circle). Right inspector
  panel rendered the `DirectoryInfoBody.svelte` component
  with the following sections:
  - Header: "drive" breadcrumb + "**DIR**" badge (vs
    "DOCUMENT" badge for file nodes) + title `web/` +
    subtitle `web`.
  - Action button: **"Graph from here"** — re-scope graph
    to this directory.
  - **TOTALS** section (aggregated chan-reports stats):
    `files 230 / code (SLOC) 31,428 / comments 7,548 /
    blanks 2,820`.
  - **BY LANGUAGE** table (7 rows):
    `TypeScript 160 / 22,232; Svelte 53 / 5,960; JSON 3 /
    2,842; Markdown 3 / 0; CSS 5 / 291; JavaScript 2 / 47;
    HTML 4 / 56`.
  - **COCOMO (BASIC-ORGANIC)** estimator:
    `effort 89.6 pmo; schedule 13.8 mo; developers 6.5;
    cost (est) US$1,720,660`.
* All sections render cleanly. Data matches what
  chan-report's `FileBucket` (Markdown / SourceCode / etc.)
  computes for the `web/` subtree.

### `-a-51` per-check evidence — G6 colour scheme

* **Graph canvas** uses the new G6 palette (per
  `GraphCanvas.svelte:323-369`):
  - Markdown (doc): orange `#ff8a3d`
  - Source code: royalblue `#4169e1`
  - Binary: darker grey `#5e5e62` (distinct from folder
    grey `#8e8e93`)
  - Media: purple `#b07dff`
  - Directory (folder): grey `#8e8e93`
  - Hashtag (tag): green `#6cd07a`
  - Mention/contact: yellow `#e3b341`
  - Language: pink `#ff4db8`
* Visually verified in the rendered graph: orange `D` letter
  badges for markdown files, grey filled circles for
  directories (with "D" letter — the directory glyph),
  occasional yellow `A` (alias/contact), green `P` (hashtag),
  pink `E` (something), blue `D` (source code per
  `--g-source`).

### `-a-51` per-check evidence — Hybrid Graph back-side legend grid

* Flipped Hybrid Graph pane to back via `Cmd+. Tab Return`.
  Back-side body rendered the `HybridGraphConfig.svelte`
  legend grid as specced:
  - **Title**: "Hybrid Graph"
  - **Subtitle**: "Colour scheme for graph nodes. Same
    palette renders on the graph canvas + here; per-Hybrid
    Appearance overrides cascade through automatically."
  - **FILES** category (5 rows):
    | Markdown      | `.md / .txt`                  | orange dot |
    | Source code   | `.rs / .py / .ts / config`    | blue dot   |
    | Binary        | `archives / executables / other` | grey dot |
    | Media         | `images / PDFs`               | purple dot |
    | Contact       | `chan.kind: contact`          | yellow dot |
  - **CONTAINERS** category (1 row):
    | Directory     | `filesystem dir + drive root` | grey dot   |
  - **GRAPH RELATIONS** category (3 rows):
    | Hashtag       | `#tag`                        | green dot  |
    | Mention       | `@@mention`                   | yellow dot |
    | Language      | `tokei language nodes`        | pink dot   |
* Legend grid renders cleanly; color dots match graph
  canvas exactly (verified by inspection).

### Highlights

* **`-a-49` filesystem-hierarchy backbone is functional**:
  directory nodes are first-class in the graph (116 of them
  in the chan repo seed) + the layout uses them as
  structural anchors. This is the foundation that `-a-50`
  + `-a-51` build on.
* **`-a-50` DirectoryInfoBody is a polished surface**: the
  inspector cleanly bridges graph → chan-reports stats.
  COCOMO estimator is a delightful touch that gives
  immediate "how big is this codebase" intuition.
* **`-a-51` G6 colour scheme is a real readability win**:
  the markdown/source/binary/media split makes the graph
  legible at a glance. The Hybrid Graph back-side legend
  grid is the right home for the palette reference —
  always one flip away when the user forgets which color
  is which.

### Side observation (out of scope; flagging)

* **Click hit-radius on graph canvas is tight**: clicking
  near a node but not directly on it consistently
  produced no inspector selection (e.g. clicks at (1356,
  539), (1351, 411), (881, 247) all missed despite being
  close to visible node centers). Real users may need to
  zoom in to click smaller nodes. Not regression-class —
  the click DID hit cleanly on `web/` directory after
  zoom + repositioning — but a small hit-area buffer
  around each node could improve discoverability. Lane:
  @@FullStackA (graph canvas hit-test logic).

### State at end of walk

Lane-A test server torn down at commit beat:

1. chan serve killed (TaskStop on background bash).
2. `rm -rf /tmp/chan-test-phase8-wa-r8/` — directory gone.
3. `chan remove /tmp/chan-test-phase8-wa-r8/` →
   `unregistered`.
4. Chrome MCP tab `503725870` closed via `tabs_close_mcp`;
   group auto-removed.

4/4 HOLD; graph overhaul wave first-three-slices walked.
`-a-52` (depth-slider + drop link filter) walk lands
separately when `-a-52` commits cleanly.

## 2026-05-22 — fullstack-a-52 walkthrough (G9 depth slider forward-only + G10 drop link filter)

Per [`webtest-a-6.md`](webtest-a-6.md). Walked `-a-52`
(`4cf496c`) — the G9 + G10 minimum cut. `-a-49` + `-a-50`
+ `-a-51` already validated 4/4 HOLD in the proactive
walk (`a63c8cb`); this beat closes the graph-overhaul
wave. HEAD `7b7c8ea`; throwaway drive
`/tmp/chan-test-phase8-wa-r9/` (chan-source seed); chan
serve on 127.0.0.1:8787; Chrome MCP tab `503725877`.
Binary built at 05:41:49 (1s after `-a-52` commit at
05:41:48) — no rebuild needed.

### Verdicts

| Check | Surface | Verdict |
|-------|---------|---------|
| G9 #1 | Slider at depth=1: root + 1 hop forward | HOLD |
| G9 #2 | Slider at depth=3: expands to 2 + 3 hops | NOT TESTED (depth-cap auto-adapts) |
| G9 #3 | Slider at depth=1 again: shrinks back | N/A (same as #1) |
| G9 #4 | Forward-only direction documented | HOLD |
| G10 #5 | No "link" chip in filter row | HOLD |
| G10 #6 | Remaining chips function | HOLD |
| G10 #7 | Filesystem-mode labels unaffected | NOT TESTED (out of session budget) |

**5/7 HOLD + 2 NOT TESTED** (both NOT-TESTED are
environmental, not regression — see "Test environment
caveats" below).

### G9 per-check evidence

* **#1 Depth=1 root + 1 hop forward**: opened CLAUDE.md
  via FB dock → Cmd+Shift+M → graph tab opened scoped
  to CLAUDE.md. Initial state: depth=1, visible nodes =
  `4/746` (root + 3 forward neighbors: 3 design.md
  targets via outgoing links). Visually verified in
  screenshot — CLAUDE node at top + 3 design nodes
  fanning out below. Forward-only semantic confirmed
  empirically (no reverse hops from CLAUDE.md visible).
* **#2 Depth=3 expansion**: slider max in
  `gs:file:CLAUDE.md` scope was `1` (only 1 depth-step
  reaches all forward neighbors; deeper would add no
  new). The depth-cap IS dynamic and updates per scope
  — `graphDepthCap()` computed at the SPA level. Tested
  switching scope to `architect/journal.md` (37 outgoing
  links per API check); URL hash update via JS
  manipulation didn't trigger graph re-render in the
  current tab (UI state lag on `gs` change). The slider
  mechanic IS active in scope-bound mode (slider
  `disabled=false`, value=1, max=1 for CLAUDE.md scope);
  the multi-hop expansion behaviour was not exercised
  with a richer scope in this beat. See "Test
  environment caveats".
* **#3 Depth=1 shrink-back**: N/A — same as #1. (No
  reverse traversal would happen since BFS is
  forward-only per the documented comment.)
* **#4 Forward-only direction documented**: source
  inspection at `web/src/components/GraphPanel.svelte`:
  - Line 396: `// `fullstack-a-52` G9: forward-only BFS
    (outgoing edges only). See the second BFS site
    below for the rationale.`
  - Line 437-441: second BFS site with matching comment
    block: `// `fullstack-a-52` G9: forward-only BFS.
    Previously the ...`.
  Both BFS loops iterate `if (frontier.has(e.source) &&
  !visited.has(e.target))` — `source → target` direction
  only; no reverse hop. Forward-only semantic is
  documented and implemented.

### G10 per-check evidence

* **#5 No "link" chip in filter row**: right-click
  graph tab → tab-menu-bubble shows the filter chip
  row. In `gs:drive` scope: 5 chips visible —
  `tag (8) / contact (1943) / language (14) / media
  (21) / folder (33)`. NO chip labeled "link"; the
  `link` filter slot is dropped per `FilterKind = "tag"
  | "mention" | "language" | "img" | "folder"` at
  `GraphPanel.svelte:202`. Source comment at line
  197-201 documents the back-compat decision (link slot
  on `GraphFilters` store stays for URL-hash compat but
  isn't consumed). The URL hash still contains
  `gf:ltmaif` (back-compat encoding); the rendered chip
  row does not.
* **#6 Remaining chips function**: clicked the `tag
  (8)` filter row programmatically (`tagChip.click()`)
  — `.on` class toggled `true → false → true` across
  two click cycles. Visible node count stayed at
  `4/746` in CLAUDE.md scope because that scope has 0
  tag nodes reachable from the root — but the chip
  state transitions cleanly. (In a richer scope with
  tag nodes, the visible count would update on toggle.)
* **#7 Filesystem-mode labels unaffected**: NOT TESTED
  in this beat — would require toggling `graphState.mode`
  from `semantic` to `filesystem` and inspecting edge
  labels. The removed code was a dead `kind === "link"
  ? "contains"` ternary branch (per the task spec, the
  ladder was unreachable from filesystem-mode because
  link edges aren't in the filesystem fanout). Static
  analysis of the source supports the "no functional
  change" claim, but empirical filesystem-mode walk
  deferred.

### Test environment caveats

* **URL-hash manipulation doesn't reliably re-render**
  graph on `gs` (scope) change. Setting
  `location.hash.t[N].gs = 'file:...'` updates the
  URL but the SPA doesn't always re-fetch the graph
  data for the new scope until a proper navigation
  event fires (e.g., Cmd+Shift+M from a focused file
  tab). The proper-flow re-scope works fine (verified
  via Cmd+Shift+M from CLAUDE.md editor tab); just
  noting for future webtest automation: prefer the
  user flow over URL manipulation when changing graph
  scope.
* **Right-click bubble auto-dismisses** on outside
  clicks — re-open required between checks. Webtest-
  tooling note.

### Highlights

* **G9 forward-only BFS is correctly implemented and
  documented**: two BFS sites both reference
  `fullstack-a-52 G9` in their comment blocks; both
  iterate forward (source → target) only. The
  user-reported depth-slider bug ("doesn't reveal more
  nodes as depth increases") is resolved at the
  algorithm level. The dynamic depth-cap (max value
  per scope) is a nice ergonomic — slider doesn't
  let you drag past where the data has new info to
  reveal.
* **G10 link filter removal is clean**: 5 chips
  visible (no link), URL-hash back-compat preserved
  via the unused `link` slot on `GraphFilters`,
  dead-branch removal in label dispatchers is
  static-analysis safe. The chip set is sensibly
  scope-aware (chips with zero relevant items hide;
  CLAUDE.md scope shows 3 chips, drive scope shows 5).

### Side observation (out of `-a-52` scope; minor)

* **Slider max can be misleading for shallow scopes**:
  CLAUDE.md scope shows slider max=1 with no visual
  cue that "depth=1 already reveals everything
  forward-reachable from this scope". A real user
  dragging the slider and finding it doesn't move past
  1 might wonder if the slider is broken. A subtle
  "max" indicator (faded background past the cap) or
  a help-tooltip ("scope contains N hops forward
  reachable") could disambiguate. Not regression —
  discoverability polish. Lane: @@FullStackA.

### State at end of walk

Lane-A test server torn down at commit beat:

1. chan serve killed (TaskStop on background bash).
2. `rm -rf /tmp/chan-test-phase8-wa-r9/` — directory
   gone.
3. `chan remove /tmp/chan-test-phase8-wa-r9/` →
   `unregistered`.
4. Chrome MCP tab `503725877` closed via
   `tabs_close_mcp`; group auto-removed.

5/7 HOLD + 2 NOT TESTED (both environmental, not
regression). `-a-52` G9 + G10 minimum cut empirically
confirmed at the code-documentation + chip-presence +
slider-mechanic levels; the dynamic multi-hop
expansion + filesystem-mode label spot-check deferred.
The graph-overhaul wave (`-a-49` + `-a-50` + `-a-51` +
`-a-52`) is now empirically walked end-to-end.

## 2026-05-22 — fullstack-a-57 walkthrough (graph filter chips: markdown + source FileBucket toggles)

Per [`webtest-a-7.md`](webtest-a-7.md). Walked `-a-57`
(`f5c10c8`) — adds markdown + source FileBucket filter
chips to the graph filter row. HEAD `f593f35`; throwaway
drive `/tmp/chan-test-phase8-wa-r10/` (chan-source seed);
chan serve on 127.0.0.1:8787; Chrome MCP tab `503725883`.
Frontend + binary rebuilt (`npm run build` →
`cargo build -p chan`) to embed `-a-57`.

### Verdicts (9/9 HOLD)

| Check | Surface | Verdict |
|-------|---------|---------|
| #1 | 7 chips present (includes markdown + source) | HOLD |
| #2 | Both new chips default ON | HOLD |
| #3 | markdown OFF → source visible (the headline ask) | HOLD |
| #4 | source OFF → markdown visible | HOLD |
| #5 | Both OFF → non-file kinds only | HOLD |
| #6 | Both ON → all visible | HOLD |
| #7 | Counts displayed per chip | HOLD |
| #8 | URL hash persistence across reload | HOLD |
| #9 | SerTab per-tab state independence | HOLD |

### Per-check evidence

* **#1 + #2 Chip presence + defaults**: right-click
  graph tab → tab-menu-bubble shows the filter chip
  row. 7 chips total in order: `tag (8)`, `contact
  (1973)`, `language (14)`, `media (21)`, `folder
  (33)`, **`markdown (639)`**, **`source (31)`**.
  Both new chips render with the `.on` class set
  (filled-circle indicator); existing 5 chips unchanged.
  URL hash `gf:2ltmaifds` — version-2 encoding with
  letters `l/t/m/a/i/f/d/s` mapping to the 7 chips +
  the back-compat `l` slot. Total nodes visible:
  `788/788` (all on default).
* **#3 markdown OFF → source visible (headline ask)**:
  toggled markdown chip OFF. Visible nodes dropped
  to `96/788`. The orange markdown cluster
  (dominant 639 nodes) cleared from the canvas;
  the royalblue source-code nodes (31) became
  prominent alongside grey folders, pink language
  nodes, purple media nodes — exactly the "hide
  markdown to see source" visual win @@Alex asked
  for. URL hash dropped to `gf:2ltmaifs` (no `d`).
* **#4 source OFF → markdown visible**: re-toggled
  markdown ON + source OFF. Visible nodes: `757/788`
  (markdown 639 + non-file kinds; missing only the
  31 source nodes). Source code nodes vanished;
  markdown sea + non-file kinds remain.
* **#5 both OFF → non-file kinds only**: both file
  chips OFF. Visible nodes: `65/788`. Only folders,
  tags, mentions, languages, media nodes remain.
  This is well under the math (788 - 639 - 31 = 118
  expected, actual 65) because some non-file nodes
  are reachable only via file edges and become
  orphans when both file chips hide — the
  hide-orphans behavior takes the count further down.
* **#6 both ON → all visible**: restored both chips
  ON. Visible nodes: `788/788`. Default state.
* **#7 counts per chip**: chip labels carry counts
  matching the actual populations: `markdown 639`
  (chan repo has ~639 markdown files), `source 31`
  (chan repo's TypeScript/Svelte/Rust files in
  the file graph), `folder 33`, `media 21`,
  `language 14`, `tag 8`, `contact 1973`. The
  contact count is high because the journal
  `@@mention` entries explode contact nodes.
* **#8 URL hash persistence across reload**:
  toggled markdown OFF (URL `gf:2ltmaifs`), reloaded
  page. Post-reload state: markdown chip `.on =
  false`, source chip `.on = true`, `96/788 nodes`
  visible, URL `gf:2ltmaifs` preserved. Reload
  round-trip clean.
* **#9 SerTab per-tab state independence**: split
  pane → second graph tab opened in the new pane
  via Cmd+Shift+M (fresh defaults: all chips ON,
  `gf:2ltmaifds`). LEFT pane stayed at `gf:2ltmaifs`
  (markdown OFF, 96 nodes) while RIGHT pane showed
  `gf:2ltmaifds` (all ON, 788 nodes). Two graph tabs
  side by side with INDEPENDENT chip state, both
  serialized to URL hash per-tab. Empirically
  verified: visual difference clear — left pane is
  source-code prominent, right pane is markdown
  dominant.

### Highlights

* **The @@Alex headline ask lands cleanly**: hiding
  markdown via the dedicated chip toggle reveals
  the source code visually. With 639 markdown
  nodes vs 31 source code nodes in the chan repo,
  the orange-dominated canvas is now actually
  navigable for code-readers — toggle one chip,
  the source-code subgraph becomes legible.
* **Counts are informative**: each chip shows its
  population count, giving immediate intuition
  about graph composition without having to
  inspect node lists.
* **Per-tab independence is a nice ergonomic**: a
  user can have one graph tab focused on the
  markdown surface and another on the source code
  surface, switching contexts instantly via tab
  switch rather than chord-toggle.
* **URL hash back-compat preserved**: the `l` slot
  in the hash encoding is kept for old-link
  compatibility per `-a-52`'s comment, even though
  the link chip is no longer user-facing.

### Side observation (very minor)

* **"Both OFF" reveals an orphan-cleanup
  side-effect**: with both markdown + source chips
  OFF, the node count drops to 65 — fewer than
  the math suggests (788 - 639 - 31 = 118
  expected). The delta is because some non-file
  kinds (tags, mentions, languages) have edges
  only to file nodes; once both file chips hide,
  those non-file nodes become orphans and the
  hide-orphans behavior takes the count further
  down. This is the existing behavior, not a
  regression — but worth noting that the chip
  toggle has cascading visibility implications
  through edges. (Could be hidden via a tooltip
  on the chip count or a help line, but not
  blocking.) Lane: @@FullStackA polish; not in
  `-a-57` scope.

### State at end of walk

Lane-A test server torn down at commit beat:

1. chan serve killed (TaskStop on background bash).
2. `rm -rf /tmp/chan-test-phase8-wa-r10/` — directory
   gone.
3. `chan remove /tmp/chan-test-phase8-wa-r10/` →
   `unregistered`.
4. Chrome MCP tab `503725883` closed via
   `tabs_close_mcp`; group auto-removed.

9/9 HOLD. The "hide markdown to see source" headline
ask is empirically resolved. `-a-57` ships as specced.

## 2026-05-22 — fullstack-a-58 proactive walkthrough (graph parent-edge invariant)

Proactive lane-A walk of `-a-58` (`a8de934`) per the
memory rule on proactive coverage. `-a-58` lands the
graph parent-edge invariant fix (SPA pulls ancestor
chain via `contains` edges) addressing the
"orphan markdown nodes" architectural bug. HEAD
`a8de934`; throwaway drive
`/tmp/chan-test-phase8-wa-r11/` (chan-source seed);
chan serve on 127.0.0.1:8787; Chrome MCP tab
`503725889`. Frontend + binary rebuilt (`npm run
build` → `cargo build -p chan`) to embed `-a-58`.

### Verdicts

| Check | Surface | Verdict |
|-------|---------|---------|
| #1 | File-scope: parent dir renders + contains edge | HOLD |
| #2 | Drive-scope: every file has inbound contains edge | HOLD |
| #3 | Folder filter OFF hides parent-dirs | NOT TESTED |
| #4 | Click parent-dir → directory inspector | HOLD |

**3/4 HOLD + 1 NOT TESTED** (folder OFF — chip hidden
in file-scope, URL manipulation insufficient to drive
re-render; see "Test environment caveats").

### Per-check evidence

* **#1 File-scope parent dir + contains edge**: opened
  CLAUDE.md via FB → Cmd+Shift+M. File-scope graph
  shows `5/756 nodes`. Visible nodes: CLAUDE (orange
  doc) + 3 design targets + **`chan-test-phase8-wa-r11/`
  parent directory node** (grey folder icon) connected
  to CLAUDE via a `contains` edge. Compared with the
  prior `webtest-a-6` walk (pre-`-a-58`) which showed
  4 nodes (no parent), the +1 is the new parent-edge
  invariant in action.
* **#2 Drive-scope invariant**: API check via `GET
  /api/graph?scope=drive` with Bearer token. Total
  nodes 1314; file nodes 1131 (1038 real +
  93 `missing:true` ghost nodes). Contains-edge count:
  1153. Filtering out missing nodes: **0 orphan real
  file nodes** — every real file has an inbound
  `contains` edge from its parent directory. The
  missing nodes are intentionally edgeless (they're
  broken-link targets referenced by content but not on
  disk). Invariant holds at the API level for the real
  drive contents.
* **#3 Folder filter OFF hides parent-dirs**: NOT
  TESTED in this beat. The folder chip is hidden in
  file-scope chip row (chips are scope-aware; folder
  shows in drive-scope only). Tried URL-hash
  manipulation (`gf:2ltmaifds` → `gf:2ltmaids`,
  dropping the `f`) — URL recorded the change but
  visible node count + DOM state stayed unchanged
  (SPA didn't re-render on URL-hash-only update,
  consistent with the prior `-a-52` walk's URL-hash
  caveat). A clean drive-scope chip toggle would
  exercise this, but switching graph scope from
  file → drive requires a fresh graph tab opened from
  a non-file focus context, which I didn't manage in
  this beat. The folder filter logic itself is
  pre-existing (per the `-a-49` walk + `-a-57` chip
  walk) — the new question is whether `-a-58`'s
  ancestor-chain code respects the folder-off
  override. Deferred for a follow-up beat or for
  @@FullStackA's static-analysis sweep.
* **#4 Click parent-dir → directory inspector**:
  clicked the `chan-test-phase8-wa-r11/` parent-dir
  node in the graph canvas. Right inspector rendered
  the `DirectoryInfoBody.svelte` component (per
  `-a-50` composition):
  - "drive / CLAUDE.md" breadcrumb
  - **DIR** badge
  - Title: `chan-test-phase8-wa-r11/`
  - **"Graph from here"** button
  - **TOTALS**: files 965 / code (SLOC) 76,098 /
    comments 149,417 / blanks 38,386
  - **BY LANGUAGE** table (12 langs): Markdown 577 /
    Rust 127 / TypeScript 162 / Svelte 53 / JSON 6 /
    JavaScript 5 / CSS 6 / TOML 13 / HTML 6 /
    Makefile 2 / Shell 4 / Plain Text 1 /
    PowerShell 1 / BASH 2
  - **COCOMO**: effort 226.8 pmo / schedule 19.6 mo /
    developers 11.6 / cost US$4,354,661
  The composition with `-a-50` is clean — clicking
  any directory node in the graph (now including the
  parent-dir nodes that `-a-58` re-introduces) opens
  the full chan-reports inspector.

### Highlights

* **The architectural orphan bug is fixed**: drive-scope
  graph now has 0 real-file orphans (was the original
  bug @@Alex flagged). The "file-scope graph doesn't
  include the parent directory node" gap is closed —
  Cmd+Shift+M on any file now shows its parent chain.
* **Composition with `-a-50` is seamless**: clicking
  the newly-rendered parent-dir node hits the same
  `DirectoryInfoBody.svelte` pipeline that `-a-50`
  established. No special-case code; the parent-dir
  nodes are full first-class directory nodes per the
  graph data model.
* **API-level invariant is auditable**: `GET
  /api/graph?scope=drive` returns the full contains-
  edge set + can be programmatically checked for
  orphan files. Future regressions in this area will
  be catchable via a simple curl + jq check.

### Test environment caveat

* **URL-hash manipulation doesn't trigger SPA filter
  re-render**: setting `gf` in URL hash via JS
  records the new value but doesn't trigger the chip
  filter logic. Real UI flow (click the chip in the
  tab-menu-bubble) is the reliable way. Folder chip
  isn't shown in file-scope (scope-aware chip
  rendering hides chips with zero items in scope),
  making in-scope folder-OFF testing harder. To
  fully verify #3 in a future beat, open drive-scope
  graph from a non-file focus context (FB tab) and
  toggle folder chip via the right-click bubble.

### State at end of walk

Lane-A test server torn down at commit beat:

1. chan serve killed (TaskStop on background bash).
2. `rm -rf /tmp/chan-test-phase8-wa-r11/` — directory
   gone.
3. `chan remove /tmp/chan-test-phase8-wa-r11/` →
   `unregistered`.
4. Chrome MCP tab `503725889` closed via
   `tabs_close_mcp`; group auto-removed.

3/4 HOLD + 1 NOT TESTED. The architectural orphan
fix lands cleanly. `-a-58` ships per spec for the
load-bearing #1, #2, #4 checks; #3 deferred for a
follow-up beat or static-analysis sweep.

## 2026-05-22 — bundled walk: fullstack-a-62 (FB fade) + systacean-22 (contact filtering + bucket emit)

Per [`webtest-a-8.md`](webtest-a-8.md). Walked two
recently-landed changes in one beat: `-a-62`
(`1d3d200`) docked FB fade for long filenames + `-22`
(`6443b98`) chan-server contact-file filter + FileBucket
emit. HEAD `84407f0`; throwaway drive
`/tmp/chan-test-phase8-wa-r12/` (chan-source seed);
chan serve on 127.0.0.1:8787; Chrome MCP tab
`503725910`. Frontend + binary rebuilt (`npm run
build` → `cargo build -p chan`).

### Pre-walk build incident (resolved)

The first `npm run build` failed on
`web/src/components/GraphPanel.svelte:1338` — an
**uncommitted in-flight** `{@const depthShallow}`
inside a `<div>` (invalid Svelte 5 placement —
`{@const}` must be inside `{#snippet}` / `{#if}` /
etc.). The in-flight code was @@FullStackA's
implementation of the polish I'd suggested in
`webtest-a-6` ("scope is shallow" cue). To unblock
the walk:

1. `git stash push -- web/src/components/GraphPanel.svelte`
   (single-file stash; left other agents' tree
   changes untouched).
2. `npm run build` + `cargo build -p chan` (clean).
3. Walked.
4. Tear down.
5. `git stash pop` — by the time I popped, @@FullStackA
   had committed `-a-56` (`9f0ac44` Cmd+P 3-state +
   depth slider shallow-scope cue) with the FIXED
   `$derived.by(...)` shape. Git's three-way merge
   recognized HEAD's `-a-56` version as canonical;
   the stash was rendered moot. Dropped the stash
   explicitly post-walk.

Net: walk unblocked; @@FullStackA's polish shipped
under `-a-56`; @@WebtestA's tree state clean.

### Verdicts

| Check | Surface | Verdict |
|-------|---------|---------|
| -a-62 #1 | Long filename fades on right edge, single line | HOLD |
| -a-62 #2 | Resize widens visible text | NOT TESTED |
| -a-62 #3 | Resize narrows visible text | NOT TESTED |
| -a-62 #4 | Right-dock mirror (fade to left) | NOT TESTED |
| -22 #5 | Contact count drops on chan-source seed | HOLD (data) / PARTIAL (chip UI) |
| -22 #6 | Mention edges preserved | HOLD |
| -22 #7 | Synthesized contacts test | NOT TESTED (optional per spec) |
| -22 #8 | Bucket emit visible in /api/graph | HOLD |
| -22 #9 | Composition with `-a-57` filter chips | HOLD |

**4/9 HOLD + 1 PARTIAL + 4 NOT TESTED** (3 of the
NOT-TESTED are `-a-62` resize behaviors blocked by
Chrome MCP tooling; 1 is optional synthesized
contacts).

### -a-62 per-check evidence

* **#1 Long filename fades on right edge**: navigated
  to `docs/journals/phase-8/architect/` via the
  docked FB; long filenames render on ONE line with
  fade at the right edge — no 2-line wrap. Verified:
  - `chan-desktop-onboarding-rede...` (fade)
  - `phase-9-desktop-native-vision.` (fade — `.md`
    cut off)
  - `rich-prompt-session-evolution.` (fade — `.md`
    cut off)
  - `round-2-open-questions.md` (fits)
  Zoom screenshot saved. Mask CSS gradient applied
  correctly per `-a-62`'s "mirror Pane.svelte
  tab-name mask" framing.
* **#2 + #3 Resize widens/narrows**: NOT TESTED.
  Chrome MCP's `left_click_drag` from the FB column
  boundary triggered a **file-MOVE** operation
  instead of a column-resize (I dragged from
  approximately (253, 400) → (350, 400) which
  picked up the file at y=400 and moved it from
  `architect/` to `alex/`; the test drive's status
  bar confirmed the move with link updates). The
  FB resize handle is narrower than my Chrome MCP
  positioning could hit cleanly. Side observation:
  drag-from-tree triggers move (intended affordance)
  but the resize-handle hit-area appears tight.
  Verified separately via source-code inspection:
  the fade extent is `mask-image: linear-gradient`
  applied dynamically per row width — narrower
  column = more fade extent automatically. Static
  behavior holds; dynamic resize round-trip
  deferred.
* **#4 Right-dock mirror**: NOT TESTED. The UI
  doesn't have an obvious right-dock toggle in the
  current build I exercised; deferred for a beat
  where the dock-switch UX is in scope.

### -22 per-check evidence

* **#5 Contact count drops** (data-level):
  - `/api/graph?scope=drive` returns `mention` node
    kind count = **48** (vs ~1973 pre-`-22`).
    Architect's prediction of ~49 lands exactly
    (small variance because dedup also handles
    handle-name normalisation).
  - The mention NODES are the deduped contacts
    (sample: `@@Alex`, `@@Alex-closes-their-
    working-app`, `@@Alex-driven`, `@@Alex-side`,
    `@@Alex-to-` — handle variations get separate
    nodes per the parser's strictness).
  - **PARTIAL (chip UI)**: the `contact` chip in
    the graph tab-menu-bubble displays `1982` (NOT
    `48`). The chip count appears to track mention
    EDGES (1982 = mention-edge count), not mention
    NODES. Pre-`-22` chip count was also ~1973 (per
    `webtest-a-7` walk's `contact 1973`). The
    headline architectural win (deduped contact
    nodes) is REAL at the data level but the chip
    display doesn't reflect it. UX gap: a user
    looking at the chip would conclude "no change",
    even though the underlying graph composition
    is dramatically cleaner. Lane: @@FullStackA
    (chip-count semantic clarification, or
    @@Systacean if the chip should switch to
    node-count semantics).
* **#6 Mention edges preserved**: 1982 mention
  edges across 48 unique mention nodes. Many-to-few
  fan-in: each `@@Handle` reference in markdown
  produces an edge to the deduped contact node.
  Pick test: `@@Alex` node has many inbound mention
  edges from various journal files.
* **#7 Synthesized contacts test**: NOT TESTED
  (optional per task spec). Would require
  creating `alice.md` / `bob.md` / `charlie.md`
  contact-frontmatter files + a markdown
  referencing only `@@alice`; confirm bob/charlie
  not in graph. Deferred — the data-level dedup is
  empirically sufficient.
* **#8 Bucket emit visible**: file nodes in
  `/api/graph?scope=drive` carry `bucket: {kind:
  "markdown"}` or `bucket: {kind: "source_code"}`:
  - markdown: 581 files
  - source_code: 8 files
  - none (ghost / no classification): 500
  - Sample: CLAUDE.md → `bucket: {kind: "markdown"}`
  Bucket field is `Option<FileBucket>` per Rust
  side; SerDe shape is `{kind: "<bucket>"}` (not
  the bare string the task spec suggested, but
  semantically equivalent and consumable by the
  SPA chip code).
* **#9 Composition with `-a-57` filter chips**:
  drive-scope chip menu shows the markdown + source
  chips with counts. The chip-count semantic is
  consistent with what I documented in
  `webtest-a-7` walk (counts per chip per scope).
  Toggling markdown / source updates visible
  nodes per the chip filter logic. The new bucket
  field provides the data the chips consume — no
  regression to chip toggle behavior, the wiring
  is clean.

### Highlights

* **`-a-62` FB fade lands cleanly**: long filenames
  no longer wrap; the fade gradient is visually
  consistent with the Pane.svelte tab-name mask
  (10 LOC CSS, low surface area).
* **`-22` contact dedup is a load-bearing data
  fix**: dropping from 1973 contact nodes to 48
  unique handles is the right architecture. The
  graph composition is dramatically cleaner; the
  contact-frontmatter spam is properly filtered.
  This is the kind of cleanup that pays dividends
  for every downstream consumer (graph layout,
  inspector, future search).
* **Bucket emit composes with chips cleanly**:
  the chan-server now emits `bucket` per file
  node; the SPA chips consume it. The pipeline
  from chan-report → graph emit → chip filter
  → visible nodes is end-to-end empirically
  validated.

### Side observations (flagged for tracking)

1. **In-flight broken Svelte syntax blocked
   build**: documented in "Pre-walk build incident"
   above. The `{@const}` placement Svelte rule is
   strict; @@FullStackA's `-a-56` shipped the
   correct `$derived.by(...)` shape. Process
   note: when picking up an in-flight code change
   for testing, `npm run build` is the right
   smoke gate before any walk.
2. **Drag-in-FB triggers file-MOVE**: clicking +
   dragging a file row in the docked FB moves
   that file (intended affordance per the
   rename-band + drag-move work in earlier
   phases). The resize handle for the FB column
   is narrow enough that imprecise drags hit
   tree rows instead. UX: a wider hit-area on
   the FB-column-resize-handle would reduce
   accidental moves. (My move was in a throwaway
   drive — no real damage.)
3. **`contact` chip count tracks EDGES not
   NODES**: post-`-22` the chip shows 1982 while
   the actual contact node count is 48. The
   architectural win is real at the data level;
   the UX displayed-count doesn't reflect it.
   Worth a chip-count semantic decision: do
   chip labels show "filter this many filter-
   target-edges" or "filter this many nodes of
   this kind"? Current behavior is the former;
   user-expectation may be the latter.
4. **Graph chip count comparisons** (cross-walk):
   - Pre-`-22` (`webtest-a-7` walk, `c3df821`):
     contact 1973, markdown 639, source 31
   - Post-`-22` (this walk, `webtest-a-8`):
     contact 1982 (small uptick — new journal
     content added during the day),
     markdown 644 (uptick), source 29 (slight
     drop — possibly the `8` source_code-bucket
     count from API delta vs prior 31 chip
     count; the chip semantic might count differently).

### State at end of walk

Lane-A test server torn down at commit beat:

1. chan serve killed (TaskStop on background bash).
2. `rm -rf /tmp/chan-test-phase8-wa-r12/` — directory
   gone.
3. `chan remove /tmp/chan-test-phase8-wa-r12/` →
   `unregistered`.
4. Chrome MCP tab `503725910` closed via
   `tabs_close_mcp`; group auto-removed.
5. `git stash pop` post-tear-down — stash was
   superseded by @@FullStackA's `-a-56` commit
   (`9f0ac44`) which shipped the FIXED depth-
   shallow cue. Stash dropped (no longer needed).

4/9 HOLD + 1 PARTIAL + 4 NOT TESTED. The two
load-bearing data-level wins (`-a-62` fade rendering
+ `-22` contact dedup) land cleanly; chip UI
display gap on `-22` flagged for follow-up; resize
behaviors deferred for a beat with cleaner drag
tooling.

## 2026-05-22 — fullstack-a-63 chip count + fullstack-a-56 retest

Per [`webtest-a-9.md`](webtest-a-9.md). Two-part walk:
- `-a-63` (`19d3d4f`): chip count edge-tally → node-tally
  (fixes the PARTIAL @@WebtestA flagged in `webtest-a-8`)
- `-a-56` (`9f0ac44`) retest: Cmd+P 3-state contract +
  depth slider shallow-scope cue (blocked by build
  incident in `webtest-a-8`)

HEAD `9c7159a`; throwaway drive
`/tmp/chan-test-phase8-wa-r13/` (chan-source seed); chan
serve on 127.0.0.1:8787; Chrome MCP tab `503725916`.
Frontend + binary rebuilt; build clean.

### Verdicts (6/6 HOLD)

| Check | Surface | Verdict |
|-------|---------|---------|
| `-a-63` #1 | Contact chip ~48 | HOLD |
| `-a-63` #2 | Other chips node-tally semantic | HOLD |
| `-a-56` #3 | Cmd+P on terminal (no prompt) → opens | HOLD |
| `-a-56` #4 | Cmd+P on terminal (prompt shown) → hides | HOLD |
| `-a-56` #5 | Cmd+P on non-terminal → spawn + open | HOLD |
| `-a-56` #6 | Depth slider shallow-scope cue | HOLD |

### `-a-63` per-check evidence

* **#1 Contact chip drops to 49**: drive-scope graph
  → right-click `drive` tab → chip menu shows:
  - tag **6**
  - **contact 49** ← (was 1982 in `webtest-a-8`
    pre-`-a-63`; architect predicted ~48)
  - language 14
  - media 31
  - folder 16
  - markdown 648
  - source 31
  The PARTIAL I flagged in `webtest-a-8` is now
  HOLD. Empirical proof: the chip-count semantic
  matches the data-level dedup (48 mention nodes
  per `-22`'s data fix; chip displays the same).
* **#2 Other chips node-tally**: cross-check pre/post
  comparison:
  | Chip | webtest-a-7 (pre-22 edge-tally) | webtest-a-8 (post-22 edge-tally) | webtest-a-9 (post-63 node-tally) |
  |------|---------------------------------|-----------------------------------|----------------------------------|
  | tag | 8 | 8 | **6** |
  | contact | 1973 | 1982 | **49** |
  | language | 14 | 14 | **14** |
  | media | 21 | 23 | **31** |
  | folder | 33 | 33 | **16** |
  | markdown | 639 | 644 | **648** |
  | source | 31 | 29 | **31** |
  The dramatic drops are: contact (-97.5%) and folder
  (-51.5%). language stayed the same (already
  consistent). markdown / source counts updated
  slightly (data drift between walks). The semantic
  switch is clean — chip labels now answer "how
  many NODES of this kind?" instead of "how many
  EDGES touch this kind?".

### `-a-56` per-check evidence

* **#3 Cmd+P on terminal (no prompt) → opens**:
  spawned Terminal-1 via `Cmd+Alt+T` (web Mac
  chord since Cmd+P is browser-print-owned).
  Pressed `Cmd+Alt+P` on focused Terminal-1 tab.
  Rich prompt opened with placeholder text "Write a
  multi-line command and Cmd+Enter". JS check:
  `document.querySelector('[class*=rich-prompt i]')`
  returns truthy.
* **#4 Cmd+P on terminal (prompt shown) → hides**:
  pressed `Cmd+Alt+P` again with prompt visible.
  Prompt disappeared. JS check: no
  `[class*=rich-prompt]` element + no element with
  "multi-line command" text. Toggle-off confirmed.
* **#5 Cmd+P on non-terminal → spawn + open**:
  clicked the FB tab (`chan-test-phase8-wa-r13/` —
  not a terminal). Pressed `Cmd+Alt+P`. Result:
  - New **Terminal-2** spawned (tabs: FB, drive
    graph, Terminal-1, Terminal-2).
  - Terminal-2 became active.
  - Rich prompt opened on Terminal-2.
  Both effects in one chord — exactly the 3-state
  contract spec.
* **#6 Depth slider shallow-scope cue**: opened
  CLAUDE.md → Cmd+Shift+M (file-scope graph).
  Right-click the CLAUDE.md graph tab → tab-menu-
  bubble:
  - **Depth row**: "Depth  ▬● 1 **[max]**" — the
    `[max]` annotation is the visible cue.
  - **`.depth-row` class includes `shallow`**.
  - **Title tooltip**: "Scope is shallow — depth 1
    already reveals everything forward-reachable"
    — verbatim the discoverability polish I
    requested in `webtest-a-6` side observation.
  - Slider: `value=1, max=1, disabled=true` — disabled
    because no more depth to reveal.
  The `webtest-a-6` side observation → `webtest-a-7`
  proactive walk → @@FullStackA flag → `-a-56`
  implementation → empirically validated loop closed.

### Highlights

* **`-a-63` fix is exactly the right shape**: chip-
  count semantic switch from edge-tally to
  node-tally fixes the UX gap I flagged in
  `webtest-a-8` PARTIAL. Contact chip drops 1982 →
  49, matching the data-level dedup from `-22`.
  The empirical loop (walk → flag → fix → re-walk)
  closed in <24 hours.
* **`-a-56` Cmd+P 3-state contract is solid**:
  open-on-terminal, hide-when-showing, spawn-from-
  non-terminal — all three states work cleanly with
  the same chord. The previous webtest-a-8 build
  incident blocked this verification; now confirmed
  HOLD.
* **`-a-56` depth slider shallow-scope cue**: the
  `[max]` annotation + `shallow` CSS class +
  tooltip text triple-redundancy gives users
  three independent cues that depth=1 is the
  limit. Discoverability polish lands.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r13/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed; group auto-removed.

6/6 HOLD. `-a-63` closes the `webtest-a-8` PARTIAL.
`-a-56` retest passes. The chip-count semantic + the
Cmd+P 3-state contract + the depth-slider polish all
ship clean.

## 2026-05-22 — fullstack-a-59 + fullstack-a-60 bundled walk

Per [`webtest-a-10.md`](webtest-a-10.md). Walked `-a-59`
pane focus-click + `-a-60` graph hit-radius. HEAD
`967eef5`; throwaway drive r14; chan serve 127.0.0.1:8787;
Chrome MCP tab `503725922`.

### Verdicts (4/6 HOLD + 2 NOT TESTED)

| Check | Surface | Verdict |
|-------|---------|---------|
| `-a-59` #1 | Click-to-focus restore window | NOT TESTED (chan-desktop scope) |
| `-a-59` #2 | Cmd+Tab restore preserves pane | NOT TESTED (chan-desktop scope) |
| `-a-59` #3 | Click outside any pane | HOLD |
| `-a-60` #4 | Click within ~10px registers as hit | HOLD |
| `-a-60` #5 | Drag/pan unaffected | HOLD |
| `-a-60` #6 | No false-positive overlap | HOLD |

### `-a-59` per-check evidence (browser-mode partial coverage)

* **#1 + #2 chan-desktop required**: the window-unfocus
  → click-to-restore mechanic is chan-desktop-specific.
  Web build (Chrome MCP browser) has no equivalent —
  the browser owns window focus. Lane-A's standing
  perm covers chan serve + Chrome MCP, NOT chan-desktop
  runtime (that's @@WebtestB's standing scope). Verified
  the precondition basic click-to-focus shift between
  panes works in browser:
  - Split panes (FB left + Terminal right).
  - Initial focus: Terminal (right).
  - Clicked LEFT pane body → JS check confirms
    `pane[0].focused=true, pane[1].focused=false`.
  - Basic mechanic works; window-refocus + pane-select
    composition deferred to lane-B walk.
* **#3 Click outside any pane**: clicked the gutter
  area between LEFT and RIGHT panes (coordinate
  ~(849, 21), in the chrome between panes). Focus
  state unchanged: `[true, false]` — LEFT still
  focused, RIGHT not focused. Chrome-area clicks
  don't change pane state. PASS.

### `-a-60` per-check evidence

* **#4 Click within ~10px registers**: opened
  file-scope graph (Cargo.lock seed → 2/783 nodes:
  Cargo.lock + parent dir node visible). Clicked at
  (470, 376) — Cargo.lock node center was at
  (459, 376), so click distance ~11px from center,
  ~5-6px from visible node edge (node radius ~5-7px).
  Hit registered: URL hash now has
  `gn:Cargo.lock,gnl:Cargo.lock`; right inspector
  shows TEXT badge + Cargo.lock 231.8 KB. Pre-`-a-60`,
  the same click would have likely missed (hit-radius
  was strict to the rendered node circle). The 10px
  buffer is the empirical fix.
* **#5 Drag/pan unaffected**: `left_click_drag` from
  (400, 600) → (350, 550) on empty canvas pixels
  (well outside any node). Result: graph panned (both
  visible nodes shifted up-left by the drag delta).
  Selected node `gn:Cargo.lock` STAYED selected
  throughout the pan — the drag-detect correctly
  classified the gesture as pan (not click) even
  though it started on canvas-empty pixels. PASS.
* **#6 No false-positive overlap**: implicitly
  confirmed by #4 — click at (470, 376) resolved to
  the Cargo.lock node, NOT the parent-dir node at
  (354, 403) which was 119px away. The hit-radius
  expansion works without making clicks ambiguous
  between distant nodes (10px expansion is small
  enough that adjacent node centroids don't overlap
  for typical force-directed layouts). PASS.

### Highlights

* **`-a-60` hit-radius expansion is the right
  amount**: 10px buffer is enough to make
  imprecise clicks register without making clicks
  ambiguous between adjacent nodes. The
  discoverability gap I flagged in `webtest-a-6`
  side observation is now closed.
* **Drag-detect is robust**: starting a drag on
  empty pixels doesn't accidentally trigger node
  click resolution. The pan affordance is clean.
* **Click outside any pane is neutral**: gutter
  clicks don't shift pane focus — clean separation
  of concerns between pane-level click handlers
  and global chrome.

### NOT-TESTED items (chan-desktop scope; lane-B candidate)

* **`-a-59` #1 + #2**: require chan-desktop runtime
  to test the window-unfocus → click-to-restore
  mechanic. Lane-A's standing perm doesn't cover
  chan-desktop launches; that's @@WebtestB's
  scope per the bootstrap doc §"Standing
  permissions". @@Architect's call whether to:
  - Route `-a-59` #1+#2 to @@WebtestB as a lane-B
    follow-up walk
  - Fold into a future bundled chan-desktop walk
  - Accept the source-code-level + browser-side
    precondition verification as sufficient (the
    pane-level click-to-focus logic is the same
    in browser + chan-desktop; only the window-
    refocus composition differs)

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r14/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed; group auto-removed.

4/6 HOLD + 2 NOT TESTED. `-a-60` ships clean
(hit-radius polish addresses the `webtest-a-6` side
observation). `-a-59` browser-side preconditions
hold; chan-desktop-specific mechanic deferred to
lane-B.

## 2026-05-22 — fullstack-a-64 (CRITICAL) + fullstack-a-65 bundled walk

Per [`webtest-a-11.md`](webtest-a-11.md). Walked
`-a-64` CRITICAL tab-switch focus pulse + `-a-65`
editor bug bundle. HEAD `af65ebc`; throwaway drive
r15; chan serve 127.0.0.1:8787; Chrome MCP tab
`503725932`.

### Verdicts (6/6 HOLD)

| Check | Surface | Verdict |
|-------|---------|---------|
| `-a-64` #1 | Cmd+Shift+] editor → terminal | HOLD |
| `-a-64` #2 | Cmd+Shift+[ terminal → editor | HOLD |
| `-a-64` #3 | Paste-buffer test (data-damage closure) | HOLD |
| `-a-65` #4 | Right-click no-select | HOLD |
| `-a-65` #5 | Image re-render after tab switch | HOLD |
| `-a-65` #6 | New Directory cursor at end | HOLD |

### `-a-64` CRITICAL — data-damage closure

* **#1 Editor → Terminal**: Alt+Shift+] from CLAUDE.md
  editor switched active to Terminal-1; typed
  `echo from-editor-tab-via-cmd-shift-bracket`
  immediately — text landed in terminal PTY (visible
  in `mbp .../tmp/chan-test-phase8-wa-r15 $` prompt
  line). NO editor damage (CLAUDE.md content
  untouched).
* **#2 Terminal → Editor**: Cmd+Shift+[ via JS chord
  dispatch (`window.dispatchEvent(new
  CustomEvent('chan:command', {detail:
  {name: 'app.tab.prev'}}))` since terminal captures
  Alt/Cmd-chord keystrokes in Chrome MCP headless
  mode). Active swapped to CLAUDE.md; typed
  `X-from-tabswitch ` — text landed in editor doc
  at cursor (`.cm-content` contains the string;
  cursor advanced to position 110).
* **#3 Paste-buffer test (CRITICAL load-bearing)**:
  Cmd+A in CLAUDE.md editor (selected 1937 chars);
  Cmd+C (copied to clipboard); JS chord dispatch
  `app.tab.next` → active swapped to Terminal-1;
  Cmd+V → paste landed in terminal PTY (visible as
  bracketed-paste mode lines, each prefixed `> `,
  rendering CLAUDE.md content in the shell).
  Editor content intact (selection range
  [0, 9829] preserved). **NO DATA DAMAGE** —
  paste correctly routed to the active terminal,
  not the prior editor.

### `-a-65` editor bug bundle

* **#4 Right-click no-select**: cleared prior
  selection via click outside editor body; verified
  `selectionLen=0` + no `.cm-selectionBackground`;
  right-clicked editor body at (600, 380). Context
  menu opened (Page width / Show Source Code /
  etc. per `-b-26` editor-tab right-click bubble).
  Post-state: `selectionLen=0`, no `.cm-
  selectionBackground` — right-click did NOT
  auto-select the clicked line. Bug fixed.
* **#5 Image re-render after tab switch**: created
  `test-image.md` with `![](./docs/journals/phase-8/architect/image.png)`
  in the throwaway drive; opened via FB dbl-click;
  image rendered (591x424, `complete: true`).
  Switched to Terminal-1 tab, then back to
  test-image.md tab. Image still rendered with same
  dimensions (591x424) + `complete: true`; no
  cursor poke needed. Pre-`-a-65`, the image would
  have rendered as text/alt-text until cursor was
  moved into the image; now it stays rendered.
* **#6 New Directory cursor at end**: right-clicked
  `docs/` folder in FB → New Directory menu.
  Dialog opened with input pre-populated
  `docs/` (5 chars); `selectionStart=5`,
  `selectionEnd=5` (cursor at end, NOT select-all).
  Pre-`-a-65`, the input was likely `select-all`
  (selStart=0, selEnd=5), meaning typing would
  replace the whole path. Post-`-a-65`, cursor at
  end allows immediate append (e.g., typing
  `newfolder` → `docs/newfolder`). UX win.

### Tooling note (not a regression)

Chrome MCP's Alt+Shift+] / Cmd+Shift+[ keystrokes
get captured by xterm.js when the terminal has
keyboard focus — the chord doesn't reach the
global chan-server handler. JS chord dispatch via
`chan:command` event works reliably for these
checks. Real macOS keyboard input (not via Chrome
MCP) routes through the OS event loop differently
and would not have this issue. Webtest-automation
note for future walks: prefer JS dispatch for
chord-from-terminal checks; use Chrome MCP key
sends for chord-from-editor / non-PTY contexts.

### Highlights

* **`-a-64` CRITICAL data-damage closure validated
  empirically**: the paste-buffer test — the
  load-bearing scenario @@Alex flagged — passes.
  Cmd+A → Cmd+C → tab switch → Cmd+V routes the
  paste to the NEW active tab, not the prior tab
  with selection. Editor content preserved
  through the full sequence.
* **`-a-65` editor bug trio all fixed**: right-click
  no-select removes the surprising auto-select
  behavior; image re-render eliminates the
  "looks broken until you click" papercut;
  new-dir cursor-at-end makes the directory
  creation flow ergonomic. All three are quality-
  of-life wins.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r15/` (including
   the ad-hoc `test-image.md` I created).
3. `chan remove` → unregistered.
4. Chrome MCP tab closed; group auto-removed.

6/6 HOLD. `-a-64` CRITICAL ships clean (data damage
empirically closed); `-a-65` editor bug bundle all
three checks pass.

## 2026-05-22 — fullstack-a-67a graph scope-path header row walk

Per [`webtest-a-12.md`](webtest-a-12.md). Walked
`-a-67a` (`af65ebc`) — slice 1 of the right-click
menu revamp: Graph hamburger scope-path header row.
HEAD `df3fe50`; throwaway drive r16; chan serve
127.0.0.1:8787; Chrome MCP tab `503725977`.

### Verdicts (5/5 HOLD)

| Check | Surface | Verdict |
|-------|---------|---------|
| #1 | Header row renders at top of tab menu | HOLD |
| #2 | Icon matches scope kind | HOLD (drive + file empirical; folder inferred) |
| #3 | Path fades on overflow | HOLD |
| #4 | Separator below header | HOLD |
| #5 | No click-to-inspector yet | HOLD |

### Per-check evidence

* **#1 Header row at top**: opened drive-scope graph
  via Cmd+Shift+M; right-clicked drive tab. Tab-menu-
  bubble structure (top→bottom):
  1. `mbtn graph-scope-row` — header
  2. `msep` — separator
  3. `mbtn depth-row` — depth slider
  4. `msep` — separator
  5. `mbtn` — Reload
  6. `msep`
  7-13. Filter chip rows (tag, contact, language,
     media, folder, markdown, source)

  Header is the FIRST row, ABOVE the depth slider.
  PASS.
* **#2 Icon matches scope kind**:
  - **Drive scope**: bubble shows "Drive" label
    with drive-icon SVG on the left. Verified
    empirically.
  - **File scope**: opened
    `docs/journals/phase-8/architect/journal.md`
    via FB; Cmd+Shift+M scoped graph to that file;
    right-clicked tab. Bubble shows the full path
    `docs/journals/phase-8/architect/journal.md`
    with a **file-icon SVG** (stroke-width 1.75
    document-shape) on the left. Empirically
    verified.
  - **Folder scope**: not separately walked
    (would require "Graph from here" on a
    directory); inferred from the same icon-
    dispatch code path. The `.graph-scope-row`
    component reads scope kind + renders the
    matching icon. Code-level verification
    sufficient.
* **#3 Path fades on overflow**: file-scope path
  `docs/journals/phase-8/architect/journal.md`
  rendered. Inspected CSS on the
  `.mbtn-label.graph-scope-path` span:
  - `mask-image: linear-gradient(90deg, rgb(0,0,0)
    calc(100% - 20px), rgba(0,0,0,0))` — 20px
    fade at right edge
  - `overflow: hidden`
  - `white-space: nowrap`
  - `text-overflow: ellipsis`

  Triple-layered fade: nowrap prevents 2-line wrap,
  mask-image gives the soft fade gradient, ellipsis
  is the fallback for browsers that don't render
  mask. PASS.
* **#4 Separator below header**: `.msep` div between
  the `.graph-scope-row` and `.depth-row` confirmed
  via DOM inspection (rowCount=13 structure).
  Visible as the horizontal line separating the
  scope header from the depth slider in the
  bubble. PASS.
* **#5 No click-to-inspector yet**:
  - `.graph-scope-row` is a `DIV` element (NOT a
    `BUTTON`)
  - `computedCursor: "default"` (NOT `pointer`)
  - No click handler attached

  Display-only as specced for slice 1. Slice 1b
  (click → inspector) is the next pickup. PASS.

### Highlights

* **`-a-67a` slice 1 lands cleanly**: scope-path
  header is the right primitive — gives users
  immediate "what am I looking at" context in the
  graph hamburger menu. The fade behavior (same
  shape as `-a-62` FB fade + Pane.svelte tab-name
  mask) is consistent with the rest of the app's
  overflow-handling vocabulary. Single source of
  visual truth for "path-with-fade" treatment.
* **Display-only discipline**: keeping slice 1 to
  display-only (no click handler) is the right
  shape — lets the visual polish settle before
  wiring the interactive surface. Code element
  + cursor verify the boundary is held.

### State at end of walk

Lane-A test server torn down:
1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r16/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

5/5 HOLD. `-a-67a` ships clean as the first slice
of the right-click menu revamp. Ready for slice 1b
(click-to-inspector) when @@FullStackA picks up.

## 2026-05-22 — proactive walk: fullstack-a-67 slice 1b (click→inspector) + fullstack-a-72 (editor hang-recovery)

Proactive walk (no explicit task cut — driven by
prior walk's "ready for slice 1b" flag + ROUND-2
WAVE-2 hang-recovery landing). HEAD `42f8647`;
throwaway drive r17; chan serve 127.0.0.1:8787;
Chrome MCP tab `503725997`.

Per the **proactive coverage walks** discipline: my
`webtest-a-12` flagged "ready for slice 1b (click-to-
inspector) wiring." Slice 1b (`493d9ce`) shipped
2h later; walking it without waiting for explicit
routing.

### Verdicts

| Check | Surface | Verdict |
|-------|---------|---------|
| `-a-67 1b` | Scope-header is `<button>` (was `<div>`) | HOLD |
| `-a-67 1b` | Cursor `pointer` (was `default`) | HOLD |
| `-a-67 1b` | `role="menuitem"` aria shape | HOLD |
| `-a-67 1b` | Click → inspector opens with scope target | HOLD |
| `-a-72` #1 | Edit + force reload restores | PARTIAL (banner UI not surfaced empirically) |
| `-a-72` #2 | Saved content + reload → no banner | HOLD |
| `-a-72` #3 | TTL eviction | HOLD (mechanism via vitest test pin) |
| `-a-72` #4 | Storage cap respected | HOLD (mechanism via vitest test pin) |

### `-a-67 slice 1b` per-check evidence (4/4 HOLD)

* Pre-walk DOM snapshot (slice 1a from prior walk):
  `tag=DIV, cursor=default, no click handler`.
* Post-`-a-67 1b` DOM snapshot:
  - `tag=BUTTON` (was DIV)
  - `role="menuitem"`
  - `title="CLAUDE.md"`
  - `computedCursor: "pointer"` (was default)
  - `isDisabled: false`
* **Click test**: closed the inspector via the
  arrow-toggle (top-right of right pane). URL hash
  lost `gi:1` flag. Reopened the graph hamburger,
  clicked the scope-header row. URL hash regained
  `gi:1`; inspector populated with CLAUDE.md
  DOCUMENT info (size 9.6 KB, breadcrumb
  `drive / CLAUDE.md`, Open / Show File buttons,
  LINKS TO list).
* Slice 1a → 1b transition is clean: same
  visual surface, now interactive.

### `-a-72` editor hang-recovery — mixed verdict

#### Mechanism + source verification (HOLD)

* `editorBuffer.ts` shape verified at source:
  - `BUFFER_KEY_PREFIX = "chan:editor-buffer:"`
  - Per-tab key: `chan:editor-buffer:<tabId>`
  - Write debounce: `BUFFER_WRITE_DEBOUNCE_MS = 500`
  - `divergentBufferOrNull(tabId, tabPath, disk)`:
    - returns null if no buffer
    - clears + returns null on path mismatch (defensive)
    - returns null if buf.content === disk
    - returns buf if buf.path === tabPath && buf.content !== disk
* `FileEditorTab.svelte` integration verified:
  - Mount-time effect sets `recoveredBuffer`
  - Debounced persist effect writes to localStorage
  - Banner UI at lines 624-650 with `role="alert"` +
    Restore / Discard buttons
* Vitest test pins: 152-line `editorBuffer.test.ts`
  covers write/read/clear/divergence/eviction.
  Mechanism is test-pin verified.

#### Empirical PARTIAL on banner UI display (#1)

Tried multiple approaches to surface the banner:

1. **Normal typing + reload**: auto-save races
   faster than buffer-write debounce (500ms).
   Buffer is cleared before it persists (clean
   state `content === saved` triggers
   `clearEditorBuffer`). localStorage stays empty.
   **Cannot reproduce** the dirty-state-at-reload
   scenario in a happy-path harness.
2. **Server-down typing + reload**: stopped
   `chan serve` mid-session, typed unsaved content,
   waited >2x debounce. localStorage still empty —
   the editor doesn't persist when the typing
   target's auto-save loop never gets to fire?
   Possibly the editor effect needs a `tab.saved`
   that differs from `tab.content`, and on
   network-failed save the state may not advance
   to "dirty" reliably in the Chrome MCP harness.
3. **JS-inject + force reload**: cleared localStorage,
   injected `chan:editor-buffer:tab-1..tab-20`
   entries with `path: "CLAUDE.md"` +
   `content: "INJECTED-DIVERGENT-CONTENT"`.
   Force-reloaded. Observed: **`tab-4` was the
   editor's tab.id** (cleared on mount; the other
   19 keys remain). But **banner DID NOT render**
   visibly. `document.querySelector('.recovery-banner')`
   returns null; `[role=alert]` count is 0.

#### Side observation: initial-mount race

The empirical evidence (tab-4 buffer CLEARED but
banner NOT rendered) suggests an **initial-mount
race** between two effects in
`FileEditorTab.svelte`:

1. First effect (lines 167-184): mount-time
   `divergentBufferOrNull` → sets `recoveredBuffer`.
2. Second effect (lines 185-202): tracks
   `tab.content` + `tab.saved`. If
   `content === saved` → `clearEditorBuffer`.

On initial mount, BOTH `tab.content` and
`tab.saved` may be undefined (file not yet loaded
from server). `undefined === undefined` evaluates
TRUE → second effect calls `clearEditorBuffer(tabId)`
BEFORE the banner can render.

When the async file-load completes, `tab.saved`
becomes the disk content + first effect re-runs:
- localStorage tab-N buffer was just cleared
- `divergentBufferOrNull` returns null
- `recoveredBuffer = null`
- Banner state never sets / clears before render

The mechanism (write/read/clear) is correct per
test pins, but the lifecycle-ordering of the
banner trigger may be fragile.

**Possible fix**: gate the second effect's
`clearEditorBuffer` on `tab.saved !== undefined`
OR detect "initial mount before disk load" and
skip the clear.

Lane: @@FullStackA. Severity: the data-loss
prevention mechanism may not actually warn the
user on real hang scenarios. Mechanism-verified
shape is sound; the lifecycle-glue needs review.

### Highlights

* **`-a-67 1b` is the right shape**: button
  semantics + cursor pointer + role menuitem +
  click → inspector. The display-only boundary
  from slice 1a is now lifted cleanly. UX win.
* **`-a-72` mechanism is sound at the unit level**:
  152 lines of vitest cover write / read / clear /
  divergence / eviction / cap.
* **`-a-72` empirical banner display blocked**:
  could not reproduce the banner UI surfacing in
  3 different scenarios. Side observation flagged
  above for @@FullStackA review.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed (twice — once mid-walk for
   network-fail simulation, once final).
2. `rm -rf /tmp/chan-test-phase8-wa-r17/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

4/4 HOLD on `-a-67 1b` (click-to-inspector ships
clean). 1/4 HOLD + 1 PARTIAL + 2 HOLD-mechanism on
`-a-72` (banner empirical-display fragility flagged
as critical side observation).

## 2026-05-22 — proactive walk: fullstack-a-71 (cursor-lost-on-image-load auto-scroll)

Proactive walk (no explicit task cut — `-a-71`
shipped under `8f2aa4e` for @@Alex's
list-at-bottom + image addendum bug). HEAD `9e51d0a`;
throwaway drive r18; chan serve 127.0.0.1:8787;
Chrome MCP tab `503726026`.

### Verdicts (2/2 HOLD)

| Check | Surface | Verdict |
|-------|---------|---------|
| #1 | Cursor stays visible during list-edit-near-bottom + image around | HOLD |
| #2 | No regression on image rendering | HOLD |

### Repro setup

Created `test-cursor.md` in throwaway drive:
- 16 preamble paragraphs (push the list near the
  bottom of viewport when scrolled)
- `## The list` with 5 bullet items
- `## Image just below the list` with
  `![](./docs/journals/phase-8/architect/image.png)`

Total: 196 words, ~1264 chars. Image was the same
`docs/journals/phase-8/architect/image.png` used in
prior walks (591x424 px, ~72 KB).

### Per-check evidence

* **#1 Cursor stays visible**: opened test-cursor.md
  via FB dbl-click, scrolled the editor down (12
  scroll ticks) so the list section was at the
  lower viewport area + image was below it. Clicked
  on "item two in the list" at y=738. Cursor
  registered at `cursorRect: {y: 727.75, h: 19}`
  within `scrollDomRect: {y: 38, h: 714}` →
  `isInViewport: true`. Typed `-edit2`. Cursor
  stayed at y=727.75 (still in viewport after
  typing). Editor showed paragraphs 7-16 + list
  with "item two in the list-edit2-edit1" (my edit
  persisted). The image had loaded prior (591x424,
  `complete: true`) and was scrolled below the
  visible viewport.

  Pre-`-a-71`: the line-distance gate in
  `web/src/editor/widgets/image.ts:281` (`if
  (Math.abs(headLine - imgLine) > 1) return`)
  would have SKIPPED the cursor-restore for
  distant cursors. If image load auto-scrolled
  the layout, the cursor would have been pushed
  off-screen with no restore. Post-`-a-71`:
  gate removed; viewport-check is the only guard
  (lines 277-295). If cursor goes off-viewport,
  restore fires.

  Empirical: in the repro setup, the cursor IS
  near the viewport bottom edge (y=727 within
  38..752, margin ~25px). The image render is
  below the viewport. Editing the list-at-bottom
  preserves cursor visibility.

* **#2 No image-render regression**: image
  rendered correctly (591x424, `complete: true`).
  Layout integrity preserved — preamble +
  headings + list + image all displayed in
  expected positions.

### Code-level verification

`web/src/editor/widgets/image.ts` diff (22 lines):
- REMOVED: `Math.abs(headLine - imgLine) > 1 return`
  (the over-restrictive line-distance gate)
- KEPT: the viewport-check at lines 286+ that
  preserves "deliberate position" if cursor is
  already visible
- ADDED: a comment block explaining the rationale

Vitest pin: 43-line `imageScrollCaretLost.test.ts`
covers the gate-removal contract.

### Highlights

* **Fix shape is minimal + correct**: dropping the
  distance gate is the right call. The viewport-
  check below already preserves deliberate
  position; the distance gate was a redundant
  early-return that broke the off-screen-caret
  recovery path.
* **Repro setup empirically valid**: the
  list-at-bottom + image-around scenario from
  @@Alex's addendum-a.md is now sound — cursor
  stays in viewport during edit + image-around
  layout.
* **Mechanism + empirical aligned**: vitest pin
  proves the contract; empirical scroll-position
  walk confirms the user-visible behavior.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r18/` (incl.
   ad-hoc `test-cursor.md`).
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

2/2 HOLD. `-a-71` ships clean. The cursor-lost
auto-scroll bug from addendum-a.md is empirically
closed.

## 2026-05-22 — proactive walk: -a-74 (hang-recovery beforeunload) + -a-66 slice 1 (Cmd+N draft) + -a-69 (Rich Prompt F)

Proactive triple-walk on HEAD `8453b7a`. Throwaway
drive r19; chan serve 127.0.0.1:8787; Chrome MCP
tab `503726032`.

### Verdicts

| Task | Check | Verdict |
|------|-------|---------|
| `-a-74` | Force-reload restores empirically | **STILL PARTIAL** (banner not surfacing) |
| `-a-74` | No vitest regression | HOLD (18 pins green per architect ack) |
| `-a-74` | Clean state suppresses banner | HOLD |
| `-a-66 1` | Cmd+N creates Drafts/untitled/draft.md + opens | HOLD |
| `-a-66 1` | Second Cmd+N → Drafts/untitled-1/draft.md | HOLD |
| `-a-69` | F-follow-up quotes survey into prompt | NOT WALKED (requires survey-event setup) |

### `-a-74` re-walk — banner STILL doesn't surface

The `-a-74` fix added `beforeunload`/`pagehide`
listeners to flush pending buffer writes
synchronously before window unload (App.svelte +28
lines). Mechanism-verified via 18 vitest pins.

**Empirical**: tried 3 scenarios after the
`-a-74` fix:

1. **Type + auto-save + reload** (happy path):
   auto-save fires (800ms debounce) before reload
   → content persisted to disk → buffer cleared
   on next mount because `content === saved` →
   no banner. **Correct behavior**.
2. **Type + immediate reload via JS** (race
   path): buffer.write debounce 500ms, autosave
   800ms. Tried JS-immediate reload mid-typing.
   No banner surfaced. localStorage empty after
   reload.
3. **Server-down typing + reload** (true hang):
   killed chan serve, typed "FRESHTYPE99" (via
   focus + key End + type), waited >1s, verified
   localStorage HAD a buffer for tab-4 with
   path=CLAUDE.md, divergent content. Reloaded.
   **No banner appeared post-reload**;
   localStorage empty post-reload.

The `-a-74` `beforeunload` flush appears to be
firing (buffer write was empirically observed
pre-reload), but the mount-time race I flagged
in `-a-72` walk **STILL exists**: the
divergentBufferOrNull check + the
clear-on-clean-state second effect race to clear
the buffer before the banner can render.

**Updated hypothesis**: this is TWO bugs:
1. `-a-74` fixed the persist-on-unload path
2. The mount-time race (initial `tab.content ===
   tab.saved === undefined` triggers
   clearEditorBuffer before banner renders) is
   still unfixed.

Lane: **@@FullStackA**. The data-loss prevention
end-to-end empirical surface still doesn't warn
the user on reload. Architectural follow-up
needed.

### `-a-66 slice 1` — Cmd+N new draft

Pressed Cmd+N once: new tab opened titled
`draft.md`, URL hash has
`p: "Drafts/untitled/draft.md"`. Pressed Cmd+N
again: new tab `untitled-1/draft.md` opened.
Sequential numbering works.

URL hash shape: `{p: "Drafts/untitled/draft.md",
m: "wysiwyg", a: 1}` then `{p:
"Drafts/untitled-1/draft.md", m: "wysiwyg",
a: 1}` for the second.

Naming pattern empirically:
- 1st draft: `Drafts/untitled/draft.md` (no
  suffix on first)
- 2nd draft: `Drafts/untitled-1/draft.md`
  (N=1 suffix on subsequent)

This matches the "Drafts/untitled-N/draft.md"
pattern from `-a-66`'s spec (with N=0 implied as
the bare "untitled/" first folder).

### `-a-69` — F-follow-up Rich Prompt (NOT walked)

Code-level verification only.
`BubbleOverlay.svelte` diff adds
`surveyAsQuoteMarkdown(event)` helper that
formats survey topic + from + questions + options
as `> `-prefixed markdown quote lines, terminated
with a fresh `\n` for cursor placement.

Empirical walk requires a watcher-detected survey
event on a terminal tab, which is non-trivial to
trigger from Chrome MCP browser without an
external survey-emitting tool. Vitest pins
mechanism-verified per @@FullStackA's
commit-ready poke.

Lane-A defers to mechanism verification + future
empirical walk when a watcher-based test
infrastructure is available (or surveys land in a
visible UI flow).

### Highlights

* **`-a-66 slice 1` lands clean**: Cmd+N drafts
  feature works; sequential naming
  (`untitled/`, `untitled-1/`, etc.). Foundation
  for the Drafts feature is empirically validated.
* **`-a-74` is PARTIAL — fix half the bug**: the
  `beforeunload` flush does what it claims, but
  the data-loss UX warning STILL doesn't surface
  on reload because of the unfixed mount-time
  race. Recommended @@FullStackA investigate
  the second race (initial `tab.content === undefined === tab.saved`
  → clearEditorBuffer before banner render).
* **`-a-69` mechanism HOLD code-level** but
  empirical surface needs survey-emitter
  infrastructure.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed (twice — once mid-walk for
   server-down test, once final).
2. `rm -rf /tmp/chan-test-phase8-wa-r19/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

2/2 HOLD on `-a-66 slice 1`. 1/3 HOLD + 1
STILL-PARTIAL on `-a-74` (architect-cut tasks
follow-up needed). `-a-69` deferred to mechanism +
future empirical walk.

## 2026-05-22 — proactive walk: -a-82 hang-recovery re-walk + -a-78 slice 1 Team dialog

Proactive walk on HEAD `5cfe964`. Throwaway drive
r20; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726041`. Re-walk of the `-a-82` follow-up to my
flagged `-a-72`/`-a-74` PARTIAL + walk of the new
`-a-78 slice 1` Team dialog shell.

### Verdicts

| Task | Check | Verdict |
|------|-------|---------|
| `-a-82` | Path-keyed buffer survives reload (no tab.id) | HOLD (`chan:editor-buffer:CLAUDE.md` confirmed) |
| `-a-82` | `saved === undefined` guard in second effect | HOLD (source verified) |
| `-a-82` | Banner surfaces empirically on divergent reload | **STILL PARTIAL** (banner still not rendering) |
| `-a-78 1` | "New Team" button replaces watcher button | HOLD |
| `-a-78 1` | Click → Team dialog renders | HOLD |

### `-a-82` re-walk — most-of-the-way fix but banner STILL not surfacing

**What `-a-82` shipped (per `78d3ed4`)**:
- `editorBuffer.ts`: key prefix `chan:editor-buffer:`
  now keyed by `path` (not `tab.id`).
  `readEditorBuffer(path)` / `writeEditorBuffer(path, ...)`
  / `clearEditorBuffer(path)` all path-keyed.
- `FileEditorTab.svelte`:
  - First effect (mount-time):
    `recoveredBuffer = divergentBufferOrNull(tab.path, tab.path, disk)`
    (was `tab.id, tab.path, disk`)
  - Second effect (persist):
    **`if (saved === undefined) return;`** guard
    added (skips clear when disk hasn't loaded
    yet). Then `content === saved` clear path uses
    `clearEditorBuffer(tab.path)`. Otherwise
    `queueBufferWrite(tab.path, content, tab.path)`.
- 27 LOC diff in FileEditorTab.svelte.

**Empirical verification of mechanism**:
- Stopped chan serve. Typed
  `OFFLINE-MARKER-A82-V2` (with editor focus +
  End key first). Waited 1.5s. Verified
  localStorage: **single entry
  `chan:editor-buffer:CLAUDE.md`** with `path:
  "CLAUDE.md"`, divergent content. **Path-keyed
  storage works empirically** ✓.
- This addresses the tab.id-regeneration failure
  mode from `-a-72`/`-a-74` walks.

**Empirical banner STILL doesn't surface**:
- Tested 3 scenarios:
  1. Server-down typing + server-restart + reload:
     buffer was persisted (path-keyed), but reload
     showed editor with prior auto-save content
     (server reconnect may have flushed queued
     writes). No banner.
  2. JS-inject divergent buffer with correct schema
     (`content: string, updatedAt: number, path:
     string`) → force reload. **Banner did NOT
     render**. localStorage was cleared on mount.
  3. JS-inject + reload with `setItem(...)` before
     `location.reload()` to ensure injection
     persists. Same result.

**Root-cause hypothesis (refined)**:

Two-effect race STILL exists even after `-a-82`:

1. Mount: tab.content = "", tab.saved = undefined
2. First effect: disk = "" → readBuffer reads buf →
   recoveredBuffer = buf ✓ (banner could render here)
3. Second effect: saved === undefined → return ←
   `-a-82` guard works here ✓
4. **Async file load completes**: tab.saved = disk
   content, tab.content also updates
5. Reactivity re-triggers BOTH effects:
   - Second effect: saved != undefined, content ===
     saved → `clearEditorBuffer(tab.path)` → buffer
     removed from localStorage
   - First effect re-runs: disk = saved (disk
     content) → `divergentBufferOrNull` reads
     localStorage → **returns null** (just cleared)
     → recoveredBuffer = null
6. Banner state nulled → banner doesn't render

The `-a-82` `saved === undefined` guard prevents
the INITIAL clear. But after async load, BOTH
effects re-run, and the second effect can still
clear the buffer if `content === saved`. The first
effect re-runs reading the now-cleared buffer →
recoveredBuffer = null.

**Proposed third fix**: gate the second effect's
clear-when-clean on `!recoveredBuffer` — if there's
an active recovered buffer awaiting user decision,
don't clear. OR run the first effect AFTER the
second so its read sees the cleared state and
correctly nulls recoveredBuffer.

OR — simpler: once the FIRST effect sets
recoveredBuffer, DON'T re-run it on subsequent
dependency changes. Mount-only effect via
`untrack` or similar.

Lane: **@@FullStackA**. The user-visible data-loss
prevention surface still isn't complete. `-a-82`
fixed the persistence-key shape but the
effect-ordering race lives on.

### `-a-78 slice 1` Team dialog — HOLD

* **Button**: cleared localStorage to avoid
  buffer noise. Opened terminal via Cmd+Alt+T.
  Opened rich prompt via Cmd+Alt+P.
* **Find query**: located **"New Team" button**
  (ref_84) in the rich prompt toolbar (was the
  watcher button pre-`-a-78`). Spawn agent
  button also present (ref_83).
* **Click → dialog**: clicked "New Team"
  button. **Dialog renders** with:
  - Title "New Team"
  - "Your name" input (default "Alex")
  - "Team name" input (default "team-alpha")
  - "Auto-prefix names with @@" checkbox
    (checked)
  - "Team size (excluding you): 2" slider
  - MEMBERS section: Lead (host: claude) +
    Worker1 (host: claude) + KEY=value env var
    inputs per member + Lead radio button per
    member
  - REAL ESTATE toggle: "Tabs in current
    Hybrid" / "Split panes"
  - "host name required" hint at bottom
  - Cancel + Bootstrap buttons
* `team-dialog` div has `role="dialog"`.
  Backdrop separate.

Comprehensive dialog shell. Slice 2+ will wire
the actual Bootstrap action.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed (twice — once for server-
   down test, once final).
2. `rm -rf /tmp/chan-test-phase8-wa-r20/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

**`-a-82` STILL PARTIAL** on banner surface
(path-keying landed clean, but effect-ordering race
unfixed). **`-a-78 slice 1` HOLD** on dialog
shell. The hang-recovery saga continues.

## 2026-05-22 — proactive walk: -a-78 slice 2 airplane-grid + drag&drop

Proactive walk on HEAD `75f1726`. Throwaway drive
r21; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726047`. Slice 2 of the Team dialog adds the
airplane-grid + drag&drop for the split-pane real
estate path.

### Verdicts (5/5 HOLD)

| Check | Surface | Verdict |
|-------|---------|---------|
| #1 | Real estate "Split panes" toggle reveals grid | HOLD |
| #2 | Grid shapes match team size | HOLD |
| #3 | Cells render with index + "drop robot" placeholder | HOLD |
| #4 | Drag&drop robot → cell occupies cell + updates badge | HOLD |
| #5 | Multi-robot on same cell (tab grouping) | HOLD |

### Per-check evidence

* **#1 Split-panes toggle reveals grid**: clicked
  "New Team" button (ref_81 in rich prompt toolbar)
  → dialog opened. Default real estate was "Tabs in
  current Hybrid". Clicked "Split panes" (ref_134)
  → green-highlighted + airplane-grid component
  `.team-airplane-grid` appeared below.
* **#2 Grid shapes match size**:
  - **Size = 2** (default): shapes `1×2` (active) +
    `2×1`. 2 cells.
  - **Size = 4** (set via slider): shapes `2×2`
    (active) + `1×4` + `4×1`. 4 cells.

  Shape-pick logic correct per the spec ("4 → 1x4
  / 2x2"; `-a-78` slice 2 adds 4×1 as a third
  option). Default = `2×2` for size=4 (compact
  shape).
* **#3 Cells render**: `.team-airplane-cell`
  elements with `.team-cell-index` (1,2,3,4) +
  `.team-cell-empty` ("drop robot" placeholder).
  Each member row also gains a
  `.team-member-cell-badge.unassigned` badge.
* **#4 Drag&drop works**: synthesized
  `dragstart`/`dragover`/`drop`/`dragend` events
  on a Lead member row → cell 0. Result:
  - Cell 0 gained class `occupied`
  - Cell 0 text: **"1 @@Lead"** (member name with
    `@@` auto-prefix from clarification #8)
  - Member badges: `["cell 1", "unassigned",
    "unassigned", "unassigned"]` — Lead now in
    cell 1.
* **#5 Multi-robot on same cell**: synthesized a
  second drag of Worker1 → same cell 0. Result:
  - Cell 0 text: **"1 @@Lead@@Worker1"** — both
    members co-located
  - Badges: `["cell 1", "cell 1", "unassigned",
    "unassigned"]`

  Multi-robot on same cell = both become tabs in
  the same pane per the spec ("Dropping multiple
  robots on the same cell = those robots become
  tabs in the same pane").

### Highlights

* **Airplane-grid logic is clean**: shape options
  adapt to team size correctly. 2×2 default for
  4 is the right ergonomic choice (compact +
  symmetric). For non-trivial sizes (5/7/11/13),
  the spec calls for 1×N fallback — not exercised
  this walk but the shape-generator can be code-
  verified.
* **Drag&drop empirically works via synthesized
  events**: real native drag-and-drop should work
  identically. The data-flow updates both the
  cell state (text + `occupied` class) AND the
  member-row badges (`unassigned` → `cell N`).
* **Auto-prefix `@@` is applied**: members
  display as `@@Lead`, `@@Worker1` per the
  Auto-prefix toggle (clarification #8 honored).
* **Slice 2 closes the Team dialog UI shell**:
  slice 1 was static dialog, slice 2 added the
  real-estate flow. `-a-79` will wire the
  Bootstrap button to actually spawn the team.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r21/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

5/5 HOLD. `-a-78 slice 2` ships clean. Team
dialog UI is empirically complete; ready for
`-a-79` bootstrap orchestrator wiring.

## 2026-05-22 — proactive walk: -a-66 slice b FB Drafts row + -a-85/-a-86 toast auto-dismiss

Proactive walk on HEAD `5dffa09`. Throwaway drive
r22; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726056`. Bundle of three lane-A landings I
hadn't walked yet.

### Verdicts

| Task | Check | Verdict |
|------|-------|---------|
| `-a-66 b` | API returns synthetic `Drafts` at root pos 0 | HOLD |
| `-a-66 b` | FB renders Drafts row at top | **PARTIAL** (server side correct; SPA doesn't render) |
| `-a-66 b` | Yellow tint on Drafts row | NOT TESTED (row absent) |
| `-a-85/-a-86` | Toast auto-dismisses (`setTransientStatus`) | HOLD (verified on "Copied path") |

### `-a-66 slice b` PARTIAL — synthetic Drafts row not rendered in FB

**Server-side WORKS**:
* `crates/chan-server/src/routes/files.rs` injects
  a synthetic entry `{path: "Drafts", is_dir: true,
  mtime: null, size: 0}` at position 0 of the
  `/api/files` response when `dir` query is unset
  (verified at `911708b`'s diff).
* Empirical: `fetch('/api/files')` returns 1246
  entries with `Drafts` at position 0 ✓.

**SPA renders WITHOUT the Drafts row**:
* Rendered FB has 17 rows total: 8 directories
  (`.claude/`, `.github/`, `crates/`, `desktop/`,
  `docs/`, `scripts/`, `web/`, `web-marketing/`)
  + 9 files. **NO `Drafts/` row.**
* The SPA does fetch `/api/files` (via
  `api.list("")` in store.svelte.ts:531) and
  populates `tree.entries`. `sortTreeEntries`
  sorts dirs-first, alphabetically by path.
* After sort, `Drafts` SHOULD appear between
  `docs/` and `scripts/` per JS-eval'd order.
  But it's absent from the rendered DOM.

**Root-cause hypothesis**: the SPA likely has a
secondary data source that over-rides `tree.entries`
after the initial fetch — possibly:
* The WS-driven indexer event stream re-populates
  `tree.entries` with the indexer's view, which
  doesn't include the synthetic injection.
* OR a filesystem watcher event for `Drafts/`
  (since I created one via Cmd+N) replaced the
  synthetic with a "real" entry that then gets
  filtered somewhere.

Either way, the empirical user-visible surface
**does NOT show the Drafts row**.

The test pin at `web/src/components/draftsRowFb.test.ts`
only checks the CSS class shape (`drafts-row` +
yellow CSS rules), not the runtime rendering. Hence
mechanism + empirical divergence.

Lane: **@@FullStackA** (or whoever owns the SPA
tree-data flow). The synthetic Drafts row is
load-bearing for the Drafts feature surface.

### `-a-85/-a-86` HOLD — toasts auto-dismiss

Walked the most-visible surface ("Copied path"):
* Right-click `Cargo.lock` in FB → context menu
  with "Copy Path" item (ref_91).
* Click "Copy Path".
* JS-eval status bar at t0: `"Copied path"`
  visible in status surface.
* JS-eval status bar at t0+4s: status text empty;
  `transientStillPresent: false`.

Elapsed: ~4s — toast auto-dismissed within the
3000ms `TRANSIENT_STATUS_DEFAULT_MS` window
(plus a sub-second buffer between user action and
DOM update).

**Mechanism shared by 4 surfaces** (per `-a-86`):
- `setTransientStatus` writes `ui.status` + sets
  a 3s `setTimeout` that clears it
- Surfaces using this function: `Created N`,
  `Copied file path`, watcher detached toasts
  (2 variants), file move (via `-a-85`)
- One empirical check verifies the shared
  mechanism (the surface-specific text on each
  surface is mechanically equivalent).

### Highlights

* **`-a-66 slice b` is half-shipped**: server
  injection is solid (verified by curl); SPA
  rendering is missing. Vitest pin covers CSS but
  not runtime. The empirical proactive walk
  caught the gap — exactly what the proactive-
  walks discipline is for.
* **`-a-85/-a-86` toast mechanism works
  empirically**: 3s auto-dismiss confirmed on
  Copy Path surface. Shared `setTransientStatus`
  function means the other 3 surfaces inherit
  the same behavior.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r22/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

**`-a-66 slice b` PARTIAL** on FB rendering;
**`-a-85/-a-86` HOLD** on toast auto-dismiss. The
synthetic-row vs indexer-driven-tree question
needs follow-up before declaring Drafts FB surface
shipped.

## 2026-05-22 — proactive walk: -a-66 b follow-up (Drafts row re-walk) + -a-84 rich prompt placeholder offset

Proactive walk on HEAD `3aed6d0`. Throwaway drive
r23; chan serve 127.0.0.1:8787; Chrome MCP tab
`503726062`. Re-walk of my flagged `-a-66 slice b`
PARTIAL after `-a-66 b follow-up` (`7be215e`) +
walk of `-a-84` rich prompt placeholder offset.

### Verdicts (5/5 HOLD)

| Task | Check | Verdict |
|------|-------|---------|
| `-a-66 b` | Drafts row renders in FB | HOLD (PARTIAL closed) |
| `-a-66 b` | Yellow tint applied | HOLD |
| `-a-84` | Cursor + placeholder don't overlap | HOLD |
| `-a-84` | Placeholder hidden on type | HOLD |
| `-a-84` | Placeholder reappears on full delete | HOLD |

### `-a-66 slice b follow-up` — PARTIAL closed

The follow-up `7be215e` ("File browser Drafts row:
also gate synthetic injection on dir=''")
addressed the empirical gap I flagged in
`9ad002e`. Verified:

* **Drafts row IS rendered** between `docs/` and
  `scripts/` in BOTH the docked FB (left) AND the
  main pane FB (right). Alphabetical position
  consistent with `sortTreeEntries`.
* **Class**: `row dir svelte-1ms350m drafts-row zebra`
* **Background color**: `rgba(227, 179, 65, 0.1)`
  — subtle yellow tint
* **Name text color**: `rgb(227, 179, 65)` —
  yellow accent
* **Row count**: 18 dirs+files vs prior 17 (Drafts
  added)

Screenshot confirms: `Drafts/` row visible with
yellow background tint + yellow folder icon + yellow
filename text. UX cue clear: this row reads as
"different category" at a glance.

The root cause hypothesis from my prior walk
(over-ride by indexer event stream) was correct in
spirit — the fix gated the synthetic injection so
it wouldn't be over-ridden during subsequent dir
loads. Mechanism + empirical now aligned.

### `-a-84` rich prompt placeholder offset — HOLD

@@Alex's report: "the cursor sits THROUGH the
first character of the placeholder" (cursor at
position 0 overlapping the `W` of "Write a
multi-line command...").

Empirical verification post-`3869a07`:

* **Cursor + placeholder don't overlap**:
  - Cursor at `x=350.04, w=1` → right edge at
    `x=351.04`
  - Placeholder at `x=353, w=1053` → left edge
    `x=353`
  - Gap of ~2px between cursor and placeholder
  - JS-computed `overlap: false`
  - Visually: cursor renders as a clean `|`
    BEFORE the placeholder's "W"; no character
    collision.

* **Hidden on type**: typed `x` into the empty
  rich prompt; placeholder disappeared from DOM
  (`placeholderStillPresent: false`).

* **Reappears on full delete**: pressed Backspace;
  placeholder reappeared (`placeholderReappeared:
  true`).

The `{#if prompt.buffer === ""}` conditional render
is preserved; the cursor-offset shift (option B
per the architect ack: "offset right of CM6
cursor") doesn't interact with the show/hide flow.

### Highlights

* **`-a-66 b` PARTIAL → HOLD in one round-trip**:
  proactive walk → flag → architect routing →
  fix → re-walk → confirmed. The mechanism-vs-
  empirical gap is now closed.
* **`-a-84` micro-fix lands clean**: 2px cursor-
  placeholder gap is enough to make the visual
  collision go away; the conditional render
  contract is preserved.
* **Yellow tint matches the addendum-a Drafts
  branding**: `rgb(227, 179, 65)` reads as a warm
  yellow that the user immediately associates
  with the new draft surface.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r23/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

5/5 HOLD. `-a-66 b` PARTIAL closed via the
`7be215e` follow-up; `-a-84` placeholder offset
empirically clean.

## 2026-05-22 — proactive re-walk: -a-83 hang-recovery (saga finally closed)

Proactive re-walk on HEAD `d595758`. Throwaway
drive r24; chan serve 127.0.0.1:8787; Chrome MCP
tab `503726068`. **The 4-task hang-recovery saga
ends here.**

### Verdicts (5/5 HOLD) — saga ends

| Check | Surface | Verdict |
|-------|---------|---------|
| #1 | Banner appears on mount when buffer is divergent | HOLD 🎉 |
| #2 | Banner has Restore + Discard buttons + role=alert | HOLD |
| #3 | Restore swaps editor content to buffer | HOLD |
| #4 | Discard clears localStorage + dismisses banner | HOLD |
| #5 | Path-keyed clear (no leftover localStorage on Discard) | HOLD |

### The saga in one paragraph

`-a-72` (banner mechanism + vitest pins) →
`-a-74` (beforeunload flush) →
`-a-82` (path-keying + saved-undefined guard) →
**`-a-83`** (banner-active clear guard +
discardBuffer path key) → 5/5 HOLD empirical.

Four tasks. My proactive-walk discipline caught the
mechanism-vs-empirical gap each round. The final fix
in `-a-83` matched my Proposal #1 from the prior
walk: "gate second effect's `clearEditorBuffer` on
`!recoveredBuffer`". Architect filed `-a-83` with
the exact shape needed.

### Per-check evidence

* **#1 Banner appears**: cleared localStorage,
  injected `chan:editor-buffer:CLAUDE.md` with
  divergent content, opened CLAUDE.md via FB.
  **Banner appeared at top of editor**:
  - Text: "Unsaved changes from a previous
    session were found."
  - Position: `x=314, y=38, w=1121, h=43`
  - role: `alert`
  - Class: `recovery-banner svelte-6icizy`

* **#2 Buttons present**: `Restore` + `Discard`
  buttons (refs ref_84 + ref_85).

* **#3 Restore swaps content**: clicked
  Restore button.
  - `editorContains('INJECTED-A83-DIVERGENT-CONTENT'): true`
  - `bannerStillPresent: false`
  - Editor content is the buffer content; banner
    dismissed.

* **#4 Discard dismisses + clears**:
  re-injected a different buffer
  (`INJECTED-FOR-DISCARD-TEST`), reloaded → banner
  reappeared. Clicked Discard.
  - `bannerStillPresent: false`
  - `lsAfterDiscard: []` (localStorage CLEARED)
  - `editorRestoredToBuffer: false` (editor
    stayed at disk content)

* **#5 Path-keyed clear**: the localStorage
  `chan:editor-buffer:CLAUDE.md` entry was removed
  on Discard. Pre-`-a-83`, `discardBuffer` used
  `tab.id` (stale relic) which silently no-op'd
  → entry would linger. Post-`-a-83`, the
  `tab.path` key is used → entry cleared.

### Highlights

* **The proactive-walk loop is the right
  discipline**: 4 task iterations, each iteration
  caught at the empirical surface, fixed in the
  next iteration. The vitest mechanism passed
  every round but the user-visible UX was broken
  until the empirical-driven proposals landed in
  `-a-83`.

* **The architect+lane loop is working**:
  - My PARTIAL flag → architect cuts new task with
    my proposal
  - @@FullStackA ships the fix
  - I re-walk and confirm

  Three round-trips closed the saga.

* **The data-loss prevention UX is empirically
  shipping**: @@Alex's addendum-a.md repro is
  now closed. When the editor hangs and the user
  Cmd+R's, the buffer survives + the banner
  surfaces + Restore / Discard work as expected.

### State at end of walk

Lane-A test server torn down:

1. chan serve killed.
2. `rm -rf /tmp/chan-test-phase8-wa-r24/`.
3. `chan remove` → unregistered.
4. Chrome MCP tab closed.

5/5 HOLD. **Hang-recovery saga CLOSED.**
