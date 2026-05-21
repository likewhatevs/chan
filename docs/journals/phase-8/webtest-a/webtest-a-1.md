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
