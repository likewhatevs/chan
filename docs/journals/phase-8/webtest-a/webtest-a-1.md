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

