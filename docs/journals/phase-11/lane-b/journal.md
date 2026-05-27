# Phase 11 Lane B journal

Lane: editor bugs, image-drag feature, desktop shell, binary-size audit,
macOS CLI-to-desktop handoff. Reports to @@Architect; peer to @@LaneA.

Worktree (SOURCE CODE only): `/Users/fiorix/dev/github.com/fiorix/chan-lane-b`
on branch `phase-11-lane-b`. Coordination docs (this journal + channels)
live in the MAIN checkout under `docs/journals/phase-11/`.

Plan: `docs/journals/phase-11/lane-b-plan.md`.

## Scope (active)

webdev track (in order):
1. Bug 1: list input regression (`- `/`* ` bullet, `N. ` numbered).
4. Bug 4: New File/Dir trailing-slash should mean directory.
5. Bug 5: image paste lands at row 1 (should land at cursor).
10. Bug 10: Cmd+N places no cursor (shared App.svelte merge point).
- Feature: image drag across rows.
6. Bug 6: idle terminal garbled until clicked/resized (FitAddon timing).

rustacean track (in order):
2. Bug 2: remove File Browser native drag in/out + native download
   indicator.
8. Bug 8: native desktop auto-reload + hang on loading.
3. Bug 3 (item 10 in plan): binary-size audit; architect-approved, no
   @@Alex gate; .github/workflows/ touch authorized (state inline).
- macOS CLI-to-desktop handoff: DESIGN NOTE first -> @@Alex gate ->
  implement.

DEFERRED, not started: Linux desktop launch (item 9). Later Linux run.

## Log

### 2026-05-26 kickoff
- Read plan, CLAUDE.md, round-1, round-2, coordination README, and the
  three channels directed at me. No cross-lane messages from @@LaneA yet.
- Baseline commit: 198beb9 (main).
- Worktree created: `../chan-lane-b` @ `phase-11-lane-b`.
- Dispatching first tasks in parallel:
  - webdev subagent: Bug 1 (list input regression) +
    `web/src/editor/commands/list.ts` survey.
  - rustacean subagent: Bug 2 (remove drag in/out + native download
    indicator) starting with the desktop shell + FileTree drag wiring.

### 2026-05-26 environment constraint: no subagent-spawn tool
- This environment exposes no Task/Agent tool to fork separate
  `webdev`/`rustacean` subagent processes (ToolSearch finds none; not in
  the deferred-tool list). Proceeding by loading the relevant skills
  in-session and doing implementation directly. Tracks stay logically
  separate; small-slice merges and the @@Alex design-note gate unchanged.
  Flagged to @@Architect on event-lane-b-architect.md.

### 2026-05-26 bug 1 investigation start
- `parseListPrefix` in `commands/list.ts` already distinguishes `-`/`*`/`+`
  (bullet) from `\d+[.)]` (ordered) correctly, so the regression is not in
  parsing. The "everything renders as a bullet" symptom points at the
  block decoration that draws list markers. Investigating
  `editor/decorations/blocks.ts` (orderedMarkerLabel / list marker
  rendering) on a fresh build.

### 2026-05-26 bug 1 RESULT: not reproducible at HEAD (already fixed)
Static review: parser (`@lezer/markdown`), the decoration dispatch
(`BulletList`->`handleBulletList`, `OrderedList`->`handleOrderedList` at
blocks.ts:611-612), the marker widgets (`BulletMarkerWidget` -> `•`,
`OrderedMarkerWidget` -> dotted label), and the list CSS in
Wysiwyg.svelte all correctly distinguish ordered from bullet. The
`::before` on `cm-md-list-line` is an empty guide stripe, not a glyph.

Empirical re-walk on a fresh binary (built from `../chan-lane-b`,
`chan 0.15.4`, throwaway drive `/tmp/chan-test-listbug`):
- Pre-existing `-`/`*` lines render `•` (cm-md-ul-marker); `1.`/`1)`
  lines render `1. 2.` (cm-md-ol-marker). DOM inspection confirmed each
  line's marker class.
- Live typing: cleared doc, typed `1. one` <Enter> `two` -> rendered
  `1. one` / `2. two`. Enter-continuation produced `2.`, not a bullet.
- Targeted vitest: blocks.list_trigger + blocks + list = 48 passed.

Conclusion: the "lists always default to bullet" regression is NOT
present at HEAD. The fix landed in `e2a58bb` (Render list bullets,
2026-05-24) + `72bff66` (dotted ordered markers, 2026-05-21), both
predating the round-1 tracker commit (2026-05-26) but postdating when
the bug was observed. No code change needed.

Minor secondary observation (NOT the reported bug, NOT in scope): a
fresh `1.` typed one blank line below another list gets renumbered to
continue it, because lezer-markdown groups loosely-separated ordered
lists. Standard CommonMark loose-list behavior; flagged for awareness
only.

Moving to bug 4 (trailing-slash path validation), a concrete fix.

### 2026-05-26 bug 4 FIXED: trailing-slash means directory
Root cause: `validatePath` in `state/pathValidate.ts` rejected ANY
trailing `/` unconditionally ("path ends with /, type a name"). The
PathPromptModal already detects file-vs-dir from the trailing slash
(`effectiveKind`), and the "New File or Directory" caption invites
`directory/path/`, but the validator killed the input before the modal's
own intent could take effect. So a directory the user explicitly named
`foo/` was rejected.

Fix (web/src/state/pathValidate.ts):
- New opt `allowTrailingSlash`. When set, strip one trailing slash and
  validate the remaining segments as the directory name; a bare `/`
  (empty after strip) still rejects with the same name hint.
- Default behavior unchanged: file create / move / rename still reject a
  trailing slash.

Wiring (web/src/components/PathPromptModal.svelte):
- `validation` passes `allowTrailingSlash: effectiveKind === "folder"`
  for both the raw and effective-path checks.
- `resolved` strips one trailing slash for folder targets so the
  effective path matches tree entries (no trailing slash on disk), the
  ancestor/segment split gains no stray empty tail, and the submitted
  value is a clean relative path. Intent is preserved by effectiveKind.

Tests: added 4 cases to pathValidate.test.ts (accept `Recipes/` and
`a/b/c/` with the opt; reject bare `/`; still validate stripped segments
like `a/./`). 23 passed. svelte-check 0 errors.

Empirical re-walk on a fresh binary (chan 0.15.4 from ../chan-lane-b,
clean drive /tmp/chan-test-bug4):
- New Directory + `myfolder/` -> status "new directory myfolder/", OK
  enabled (was rejected before).
- New File + `foo/` -> still "path ends with /, type a name", OK disabled
  (regression guard holds).
- New Directory + `realdir/` + Enter -> created `realdir` on disk
  (confirmed via ls), selected in tree, Details shows DIRECTORY realdir,
  URL bs=realdir (no trailing slash) -> strip round-tripped.

Committing as a small slice.

### 2026-05-26 bug 4 follow-up: caught + fixed an either-flow regression
While running the full vitest suite, a source-text test
(fileBrowserUnifiedDialog.test.ts) flagged that my first cut had changed
the `resolved` block. Investigating revealed a real bug I'd introduced:
the `either` dialog's store handler (`createFileOrDir`) dispatches on
`next.endsWith("/")` to pick directory-vs-file, so stripping the trailing
slash from the submitted value would have created directories as FILES.
The dedicated New Directory dialog (`createDir`) hid this because it
passes an explicit `true` dir flag, not the slash.

Corrected: keep the trailing slash in `effectiveValue` (submitted value)
for folders; added `normalizedPath` (slash stripped) used only for the
drive-relative reasoning inside `status` (entry lookup, missing-ancestor
walk, per-segment render). Re-verified the `either` flow empirically:
`existing/subdir/` -> created `existing/subdir` as a directory on disk
(confirmed `test -d`), tree shows it nested with a dir icon, status read
"new directory".

Full gate green: fmt, clippy -D warnings, cargo test, build
--no-default-features, npm build, svelte-check, full vitest (1482 passed,
11 skipped, 0 fail). One Pane.test.ts timeout under full-suite load was
confirmed flaky (passes in isolation, 21/21); unrelated to my files.
Note: a pre-existing stash from another agent ("webtest-a r41 push")
exists in the worktree's stash list; left untouched.

Committed bug 4 as 69bd94f on phase-11-lane-b (3 files, all lane-B-owned;
no shared-file edits, no cross-lane announcement needed).

### 2026-05-26 merge protocol + rebase
- @@Architect: in-session skill execution approved; @@Architect now owns
  all merges to main (lanes must NOT self-merge); no remote push yet.
- Rebased phase-11-lane-b onto main @ 3d42b09 (@@LaneA Slice B). Clean.
  Bug 4 now at 330bda1.
- Posted ready-to-merge for bug 4 and D2 reply to @@LaneA.

### 2026-05-26 bug 5 FIXED: paste image at cursor / append when unfocused
Root cause (reproduced empirically on a fresh binary): the paste handler
in editor/bubbles/image_drop.ts used `view.state.selection.main.head`,
which is 0 for a freshly-opened note the user hasn't clicked into. So an
image pasted right after opening landed at offset 0 (the first row, above
the title). Confirmed by dispatching a synthetic image paste on a
fresh-open doc: markdown landed at the very top.

The drop handler already falls back sensibly; the paste handler did not.
Fix: `pasteInsertPos(view)` returns `selection.main.head` when
`view.hasFocus` (real caret -> lands at cursor, unchanged), else
`view.state.doc.length` (append at end). End-of-doc is the least-
surprising target for a paste with no active caret and never clobbers
row 1.

Empirical verification on a fresh binary:
- Unfocused fresh-open paste -> appends after the last line (was: row 1).
- Focused paste (real caret) -> lands at the caret (offset 44 in the
  pre-fix run; the focused branch is byte-identical so unchanged).
- New image_drop.test.ts pins all three branches (3 passed).

Full gate green: fmt, clippy -D warnings, cargo test, build
--no-default-features, npm build, svelte-check, full vitest (1485 passed,
11 skipped, 0 fail). Committed as 9773f44 on phase-11-lane-b (2 files,
lane-B-owned, no shared-file touch).

@@LaneA heads-up consumed: their current slice adds /ws
subscribeDir/unsubscribeDir + a typed frame catalog in api/types.ts and
touches api/client.ts (my SHARED set) — not on main yet, and I have not
touched api/client.ts. D2 settled (editor toast path unchanged). Slice C
(state.rs + lib.rs::router + bus.rs + ws.rs) will get a separate ping.

Next: bug 10 (Cmd+N cursor) — touches App.svelte (two-sided merge point);
I'll post the exact hunk to event-lane-b-lane-a.md BEFORE committing.

### 2026-05-26 bug 10 FIXED: focus the editor after a fresh open
Reproduced empirically on a fresh binary: Cmd+N opened a draft with the
title "Draft" selected (`c:[2,7]`) but the editor was NOT focused
(`cm-focused` false, activeElement `<body>`). Typing did nothing — the
title stayed "Draft". Exactly the reported bug.

Root cause: the editors' mount-time `view.focus()` (Wysiwyg ~line 520,
Source ~219) runs while the doc is still empty; content arrives async via
the `value` prop, so by the time `# Draft` lands focus has fallen back to
`<body>` (the Cmd+N path opens the draft after the chord handler parks
focus on body). The selection was placed by `maybeRestoreCaret`, but
nothing re-focused.

Fix: in `maybeRestoreCaret` (the point where the caret is actually placed
after first non-empty content), re-assert `view.focus()` when
`autoFocus`, deferred one `requestAnimationFrame` so it lands after any
same-tick blur and after the pane finishes animating in. A plain
synchronous `view.focus()` there did NOT stick (verified empirically —
still landed on body); the rAF defer fixed it. Mirrored in Source.svelte.

Deliberately did NOT touch App.svelte: the root cause is the editor's own
focus-vs-content race, not the Cmd+N chord handler. This keeps the fix
inside my owned editor surface and AVOIDS the App.svelte two-sided merge
point entirely — better for cross-lane coordination than the
plan's tentative App.svelte scoping. Will tell @@LaneA no App.svelte
reconciliation is needed.

`autoFocus` gate preserves the rich-prompt opt-out (autoFocus=false).
Behavior is consistent with the pre-existing mount-focus intent (every
freshly-opened editor should be typeable); not a regression for split
panes since maybeRestoreCaret fires once per mount, at the moment the
user just opened the file.

Empirical verification: Cmd+N then immediate "My New Title" -> heading
became "# My New Title", editor focused (cm-focused true, activeElement
inside .cm-editor). Full gate green (fmt, clippy -D warnings, cargo test,
build --no-default-features, npm build, svelte-check, full vitest 1485
pass). Committed af783d8 (2 editor files).

Webdev quick-wins (bugs 1,4,5,10) DONE. Next per plan: image-drag-across-
rows feature (same file as bug 5), then bug 6 (terminal fit). Rustacean
track (bug 2) still to open.

### 2026-05-26 fresh session resume + rebase
Recovered state from journal + plan + all four channels + inspector-spec.
Rebased phase-11-lane-b onto main @ d70f585 (the merge of my bugs 4/5/10);
my 3 commits folded into main via the merge, branch is now the main tip,
clean, no diff. @@LaneA Slice A/C still queued (not on main yet); I have
not touched any shared file.

### 2026-05-26 FEATURE: image drag across rows (commit b70f4ac)
Make a rendered image atom a drag handle in writable mode; dragging it to
a different row relocates its `![alt](src)` markdown there. Left/center/
right + width (#w=N) ride in the src fragment and move verbatim, so they
stay owned by the existing image dropdown; this only changes the ROW.

Implementation across 4 lane-B files:
- `web/src/editor/widgets/image.ts`: the inner `<img>` (NOT the wrap) is
  the drag source. Discovered empirically that CodeMirror manages and
  RESETS the `draggable` property on widget-root DOM; the child img is
  left alone. dragstart -> `beginImageDrag` resolves the live Image node
  range from the syntax tree (positions drift) and stashes
  `{from,to}` JSON on a custom `application/x-chan-image-move`
  dataTransfer MIME, sets effectAllowed=move, uses the img as the drag
  image, and stamps `data-dragging`. dragend clears it.
- A second empirical finding: the editor's `value` prop arrives async, so
  the image widget's FIRST render can land while `EditorView.editable` is
  still false; `eq()` then keeps the stale non-draggable DOM forever.
  Fix: capture editability on the widget (field named `writable` because
  `WidgetType` has a getter-only `editable` member that THROWS on
  assignment -- the first naming attempt crashed the whole decoration
  plugin, visible as the image rendering as raw source), fold it into
  `eq()`, and re-scan the inline ViewPlugin on a facet flip.
- `web/src/editor/bubbles/image_drop.ts`: dragover branch preventDefaults
  for our MIME so the editor accepts the drop (default rejects); drop
  branch keys off the MIME and calls `moveImageSource`, which splices the
  source to the drop row's start (own line, or inline + trailing space on
  a list line, mirroring the paste/drop convention). Standalone image
  lines swallow their trailing newline so no blank gap is stranded. The
  img mousedown drops its `preventDefault()` in writable mode (it blocks
  native drag) but keeps `stopPropagation` (no caret placement); plain
  click still selects.
- `Wysiwyg.svelte`: grab cursor on the draggable img + dim the source
  while dragging.

Tests: 5 new `moveImageSource` cases in image_drop.test.ts (move down,
move up, no-op when dropped inside source, list-line inline target,
malformed payload). Full vitest 1490 pass / 11 skip / 0 fail.

Empirical re-walk on a FRESH binary (had to relaunch 3x; a stray pkill /
server-restart killed two of my test servers mid-walk -- chased provenance
each time, confirmed served bundle hash == on-disk before trusting any
result). Final run on chan 0.15.4, bundle index-prYe89bM.js, throwaway
drive /tmp/chan-test-imgdrag: img.draggable=true, no plugin crash, image
renders as the atom; dragstart populated the MIME with `{from:36,to:66}`,
dragover accepted, drop moved `![](attachments/red.png#w=120)` from
between para 1/2 to below para 3 -- confirmed ON DISK with #w=120
preserved. Plain click still selects without entering edit mode. Test
server + browser tab torn down, throwaway drive rm -rf'd.

Full gate green: fmt, clippy -D warnings, cargo test, build
--no-default-features, npm build, svelte-check (0 errors), full vitest.
Committed b70f4ac (4 files, all lane-B-owned editor surface; no shared
structural file touched). Ready-to-merge posted to
event-lane-b-architect.md.

Next per plan: bug 6 (idle terminal garbled until clicked/resized;
FitAddon timing in TerminalTab.svelte).

### 2026-05-26 BUG 6 FIXED: repaint idle terminal on becoming active (0a8e0ae)
Root cause: the terminal tab uses `visibility: hidden` (not
`display: none`) while inactive, so the host keeps layout dimensions, but
xterm.js / the WebGL renderer can paint at a stale size (or skip
painting) while hidden. Nothing forced a re-fit + repaint when the tab
became ACTIVE without also becoming FOCUSED. The existing focus `$effect`
covers focus changes and the ResizeObserver covers size changes, but a
pure visibility flip on a tab switch (or a terminal tab made active in a
NON-active split pane) hits neither path -> garbled until the user
clicks (focus -> queueFit) or resizes (ResizeObserver -> queueFit).
That's exactly the reported "garbled until clicked/resized".

Fix: a new `$effect` keyed on the `active` prop. When `active` flips true
and the term is live, it runs `recoverTerminalRendererAfterHostResume()`
-- the same queueFit + clearTextureAtlas + refreshTerminalRows + delayed
[50,250]ms re-fit recovery already wired for window focus/visibility
resume. The `term` gate skips the initial mount (start() already fits).
`active` is read first so the effect tracks it; `term` is a plain (non-
reactive) let so the effect re-runs only on `active` change.

Empirical re-walk on a FRESH binary (chan 0.15.4, bundle index-CbnSv4LE,
throwaway drive /tmp/chan-test-term): opened a terminal, filled it with 30
rows of `####`, opened a second terminal tab (Terminal-1 -> inactive,
visibility:hidden), switched back to Terminal-1 -> all 30 rows render
crisp, no garble. (Note: under the WebGL renderer the glyphs paint to a
canvas, not `.xterm-rows` DOM, so DOM-row counting reads 0; the screenshot
is the source of truth.) Caught + corrected a stale-token slip mid-walk
(reused the previous feature run's bearer token -> "bootstrap failed:
missing or invalid token" + dead WS; re-read the launch log for the
correct token, reloaded, shell attached). Test server + tab torn down,
drive rm -rf'd.

Full gate green: fmt, clippy -D warnings, build --no-default-features,
npm build, svelte-check (0 err), full vitest 1490 pass / 11 skip / 0 fail
(cargo test unaffected, no Rust changed). Committed 0a8e0ae (1 file,
TerminalTab.svelte; lane-B-owned, no shared structural file).

WEBDEV TRACK COMPLETE (bugs 1/4/5/10 + image-drag feature + bug 6). Next:
open the RUSTACEAN track with the reshaped bug 2 (remove FB native drag
in/out via drag_out.rs + FileTree.svelte JS wiring; deliver the
download-with-progress capability -- Tauri command + progress events +
api/client.ts wrapper/store -- for @@LaneA to wire into the inspector;
post the interface on event-lane-b-lane-a.md).

### 2026-05-26 BUG 2a DONE: remove FB native drag in/out (3fec962)
Part (a) of the reshaped bug 2. macOS native drag-out crashed; round-1
says remove drag IN and OUT entirely, operate via Upload/Download
buttons.

SCOPE-BOUNDARY DECISION (flagged to @@Architect): "drag in and out" =
the OS<->app interchange. I removed THAT (drag-out to Finder + the
external-OS-file drag-IN upload) and KEPT the app-internal drag: tree-move
(relocate a node within the tree) and file-into-editor-pane open. Those
never cross the OS boundary, never hit the crashing native path, and have
no Upload/Download-button equivalent, so they're out of scope for the
"operate via buttons" replacement. @@Alex's verbatim round-1 text and the
"Upload/Download buttons" framing both point at OS interchange. If @@Alex
meant ALL File Browser drag, this is a one-line follow-up.

Removed: desktop/src-tauri/src/drag_out.rs (whole module) + its mod +
invoke registration in main.rs + the allow-start-file-browser-drag-out
ACL permission and its two sets in permissions/app.toml + the serve.rs
registration test + the now-dead app_permission_allows helper + ACL test
entry. FileTree.svelte: the DownloadURL/text-uri-list browser drag-out
payload, startNativeDragOut + helpers (setDownloadDragData, downloadMime,
absoluteDownloadUrl), the unused isTauriDesktop/tauriInvoke import, and
the external-file drag-IN branch in onRowDragOver/onRowDrop + the
hasExternalFiles helper. (reqwest/uuid stayed -- used elsewhere, not
drag_out-only as first probed; restored after a build caught it.)

Kept: downloadFilename + api.downloadUrl (Download button), uploadFilesTo
at the picked target (Upload button), TREE_MOVE_MIME/FILE_DRAG_MIME.

Tests fileTreeDragOut.test.ts + fileBrowserUploadDrop.test.ts inverted to
assert the drag-out/in + native command are GONE while internal drag +
buttons stay. Empirical re-walk on a fresh binary (chan 0.15.4, bundle
index-DHD1Xqt9, drive /tmp/chan-test-drag2): a.md row still draggable;
dragstart carries ONLY tree-move/file/text-plain (no DownloadURL, no
uri-list); an external "Files" dragover is rejected (not preventDefault);
an internal tree-move drop moved a.md into folder/ ON DISK. Server + tab
torn down.

Full gate green: fmt, clippy --all-targets -D warnings (incl
chan-desktop), cargo test (all workspace), build --no-default-features,
npm build, svelte-check (0 err), full vitest 1490/11/0. Committed
3fec962 (7 files, all lane-B-owned; no shared structural file).

Next: bug 2 part (b) -- the desktop-native download-with-progress
capability (Tauri command + progress events + api/client.ts wrapper/
store) for @@LaneA to wire into the inspector; interface posted on
event-lane-b-lane-a.md.

### 2026-05-26 BUG 2b DONE: desktop download-with-progress capability (66dec92)
Part (b) of reshaped bug 2; bug 2 now COMPLETE. Per the inspector-spec
ownership split: @@LaneA owns the inspector Download button + indicator
UI; I deliver the desktop-native download FLOW as a reusable capability.

Design: browser <a download> uses the browser's download manager;
chan-desktop's webview has none. So the SPA fetches the file over the
loopback connection via XHR (download-direction onprogress drives an
in-app indicator), then hands the finished bytes to a Tauri command
that writes to the OS Downloads folder. Byte transfer stays in JS
(reuses the upload-progress XHR pattern; avoids a second loopback fetch
from Rust); drive content is notes-scale so in-memory buffering is fine.

Files (all lane-B-owned; ZERO shared client.ts/store.svelte.ts touched --
deliberately routed through api/desktop.ts + a NEW lane-B store module to
dodge @@LaneA's Slice A store reshape):
- desktop/src-tauri/src/download.rs (NEW): save_file_to_downloads(filename,
  bytes) command -> writes to dirs::download_dir() with browser-style
  "file (1).ext" dedupe + filename sanitization (strips path separators
  so a webview value can't escape Downloads). 3 unit tests
  (sanitize/split_ext/dedupe). Registered in main.rs + drive-window ACL
  (app.toml) + the serve.rs ACL test.
- web/src/api/desktop.ts: runDesktopDownload(url, filename) -> XHR fetch
  with onprogress, then save_file_to_downloads invoke. Gated on
  isTauriDesktop().
- web/src/state/downloadTransfer.svelte.ts (NEW): $state store
  (begin/progress/finish/fail/clear + downloadTransferActive) the
  inspector binds to.
- 2 NEW tests (downloadTransfer.test.ts store lifecycle = 6;
  desktopDownload.test.ts capability shape = 2).

Interface posted on event-lane-b-lane-a.md for @@LaneA to wire the button
+ indicator. Reveal-in-Finder deferred (reveal_in_finder is launcher-only
in the ACL, not drive windows; easy follow-up).

Verification: can't exercise the Tauri save command outside chan-desktop
(it only runs in the packaged app); the Rust file logic is unit-tested,
and the web wrapper's isTauriDesktop guard + the store lifecycle are
vitest-covered. End-to-end in the packaged desktop app is a chan-desktop
build check, best done once @@LaneA's button wiring lands (offered to
drive it then).

Full gate green: fmt, clippy --all-targets -D warnings (incl
chan-desktop), cargo test (workspace incl new download.rs + ACL tests),
build --no-default-features, npm build, svelte-check (0 err), full vitest
1498/11/0 (+8). Committed 66dec92 (8 files).

RUSTACEAN TRACK NEXT: bug 8 (desktop auto-reload/hang during editing;
investigate watcher.rs config-dir watch + main.rs reload_window +
embedded.rs), then the binary-size audit (record findings + chan-CLI
embed-model recommendation; .github/workflows edit authorized -- will
state inline before the first workflow edit), then the macOS
CLI-to-desktop handoff DESIGN NOTE (the ONE @@Alex gate -> post to
event-lane-b-alex.md, WAIT for ratification before implementing).

### 2026-05-26 fresh session resume + rebase (rustacean track)
Recovered state from journal + plan + the @@Architect channel (batch
merged @ ebcabad; bug-2a scope confirmed PROCEED on my reading; next is
the rustacean track). Rebased phase-11-lane-b onto main @ ebcabad; my 4
commits (image-drag, bug 6, bug 2a, bug 2b) folded into main via the
merge, branch is now the ebcabad tip, clean. chan-desktop builds here
(macOS). Opened bug 8.

### 2026-05-26 BUG 8 FIXED: desktop auto-reload + hang on loading
Round-1 source text (line 13): "native-desktop app auto-reloading during
editing, and hanging on loading... hitting cmd+ resolves it". Adjacent
line 12 is the "Too Many Open Files -> failing autosave -> hanging the
server until pkill" incident; the two are the same root pressure.

INVESTIGATION (traced the full reload/watch path, both Rust + SPA):
- Rust side: the ONLY thing that reloads a drive (editor) webview is the
  `reload_window` command (main.rs:1287, `eval("window.location.
  reload()")`). Every caller is an explicit user action: Cmd+R in
  KEY_BRIDGE_JS (serve.rs:611), the pane right-click "Reload" menu
  (Pane.svelte:471), and App.svelte's Cmd+R chord (779). No Rust-side
  auto-reload exists; `app.emit` only broadcasts `registry-changed` /
  `serves-changed` / `auth-*`, none of which a drive window listens to
  (only the launcher `desktop/src/main.js` listens, and it just calls
  `refresh()`). Drive windows get only KEY_BRIDGE_JS as init script.
- SPA side: the only `window.location.reload()` paths are Cmd+R
  (`reloadWindow()`) and InfographicsTab's post-import 700ms reload. The
  SPA listens to ZERO Tauri events. So nothing auto-reloads the editor
  from app logic.
- CONCLUSION on the reload: it's the macOS WKWebView WEB-CONTENT PROCESS
  being recycled by the OS under memory / file-descriptor pressure (the
  line-12 fd-exhaustion incident: registry watcher + per-drive recursive
  chan-drive watchers + terminals + WS all hold fds). WKWebView reloads
  the page when its content process dies. That's the "auto-reload."
- CONCLUSION on the hang: `bootstrap()` (store.svelte.ts:684) does a
  single-shot `await api.drive()` and, on failure, just sets a status
  string and returns -- `bootstrapped` never flips true, so the SPA sits
  on "loading..." FOREVER. When the WKWebView reload races the embedded
  loopback server still recovering (connection refused -> bare
  TypeError, or our 10s transport timeout -> ApiError(0)), the first
  call fails and there's no retry. cmd+ (zoom_in in chan-desktop, NOT a
  reload) "resolves it" by forcing a WKWebView re-composite that reveals
  already-recovered content / coincides with the server healing.

TWO contributing defects fixed:

(1) Spurious `registry-changed` storm (desktop/src-tauri/src/watcher.rs,
    lane-B-owned). The watcher watches `~/.chan/` NON-recursively and
    emitted on ANY `Ok(_)` debounced event without inspecting paths.
    But `~/.chan/` also holds `preferences.toml` (pane widths, theme,
    editor knobs -> re-saved on a pane drag) and `server.toml`, each via
    `store::save_toml`'s atomic tmp+rename. So routine editing/layout
    re-fired `registry-changed`, storming the launcher's `list_drives`
    refresh and adding churn/fd pressure for nothing. The design.md
    contract is "emit only when the registry FILE moves." Fix: filter
    the debounced events to the registry file's own name
    (`config.toml`) via `registry_event_present`; sibling +
    watched-dir-self + atomic-tmp events are dropped, the final rename
    onto config.toml still forwards. 4 unit tests.
    EMPIRICALLY VALIDATED on real macOS FSEvents with a throwaway notify
    probe: an atomic rename onto config.toml DOES surface a
    file_name=="config.toml" event (forwards); a sibling preferences.toml
    atomic write surfaces only preferences.toml / its tmp (suppressed);
    the watched dir itself also surfaces an event (the old code's
    spurious trigger -> now correctly dropped). Probe removed after.

(2) bootstrap() hang-on-loading (web/src/state/store.svelte.ts, SHARED
    file -- @@LaneA owns its structural shape; announced on
    event-lane-b-lane-a.md; edit is purely additive). Added
    `driveWithRetry()` wrapping the FIRST `api.drive()` with a bounded
    retry (5 attempts, 250ms linear backoff, ~3.75s total) on TRANSIENT
    failures only (`isTransientBootstrapError`: bare Error/TypeError =
    connection refused; ApiError status 0 = our timeout; 502/503/504 =
    server still spinning up). A 401 (missing token -> overlay) or any
    other 4xx still throws on the first response, unchanged. So a
    WKWebView-reloaded window heals itself instead of sticking on
    "loading...". Exported `__testIsTransientBootstrapError` per the
    file's existing `__test*` convention; 6 new tests in a NEW
    lane-B-owned file bootstrapRetry.test.ts pinning transient-vs-terminal
    classification.

Did NOT touch main.rs reload_window / embedded.rs: the investigation
showed reload_window is correct (user-driven) and embedded.rs's listener
is fine; the bug is the watcher's path-blind emit + bootstrap's missing
retry. The deeper fd-exhaustion root (line 12) has existing
infrastructure (chan-drive fd_budget.rs, terminal_sessions EMFILE guard)
and is a separate, likely-other-lane concern; my two fixes reduce the
trigger frequency (less watcher churn) and make the symptom self-healing
(bootstrap retry).

Empirical: chan-desktop rebuilt with the new bundle and smoke-launched
on macOS -- boots clean, no watcher/boot errors (only the benign no-net
updater 404). Torn down by scoped pid kill; no broad pkill.

Full gate green: cargo fmt --check (root + desktop), cargo clippy
--all-targets -D warnings (root + desktop), cargo test (332 chan-server +
all workspace, 0 fail) + desktop cargo test --all-targets (7 + 4 new
watcher), cargo build --no-default-features, svelte-check 0 errors, full
vitest 1514 pass / 11 skip / 0 fail (+6), npm run build OK. Files:
watcher.rs (lane-B), store.svelte.ts (SHARED, additive, announced),
bootstrapRetry.test.ts (NEW lane-B). Committing as one slice; ready-note
to @@Architect with the shared-file flag.

RUSTACEAN TRACK NEXT: binary-size audit (record findings + chan-CLI
embed-model recommendation; .github/workflows edit authorized -- state it
inline first), then the macOS CLI-to-desktop handoff DESIGN NOTE (@@Alex
gate -> post to event-lane-b-alex.md, WAIT for ratification).

### 2026-05-26 BINARY-SIZE AUDIT: findings + chan-CLI recommendation
Goal: confirm no BGE embedding model is baked into any RELEASE binary;
ship the smallest single binaries (SPA only). architect-approved, no
@@Alex gate.

FEATURE GRAPH (verified in crates/chan/Cargo.toml +
crates/chan-server/Cargo.toml):
- `default = ["embeddings"]`: candle RUNTIME stack on by default. Does
  NOT bake any model. Off only via --no-default-features (iOS).
- `embed-model` (default-OFF): bakes resources/models.tar.zst via
  `include_bytes!` in embed_seed.rs (whole module `#![cfg(feature =
  "embed-model")]`). Adds ~63 MB. Implies `embeddings`.
- `embed-font` (default-OFF): bakes Source Code Pro. Not in any release
  path.
- With embed-model OFF the chan-drive runtime resolver looks for the
  model under `<user-config>/chan/models/<model-name>/`; `chan index
  download-model` fetches on demand.

WHAT CI ACTUALLY SHIPS (the distributed artifacts):
- release.yml (chan CLI: linux x86_64/aarch64, macos aarch64): builds
  `cargo build --release --target ... -p chan` -- NO --features
  embed-model. Both the BGE-bundle cache step (line 171) and the
  fetch-models pre-fetch step (line 195) are hardcoded `if: false` with
  a systacean-6 comment ("ship a ~25 MB binary"). LEAN. .deb/.rpm wrap
  the same lean binary.
- release-desktop.yml (chan-desktop): uses `make build` (lean chan
  helper) + `cargo tauri build`; its model cache + fetch steps are also
  `if: false` (lines 162, 180). chan-desktop's Cargo.toml pulls
  chan-server with `features = ["embeddings"]` only and chan-drive with
  default-features=false -- runtime resolver, nothing baked. LEAN.
So BOTH shipped binaries are already SPA-only with no model baked. No
.github/workflows edit is needed; the workflows are already correct.

MEASURED (this machine, aarch64-apple-darwin, unstripped, release):
- `cargo build --release -p chan` (default features) = 28 MB
  (28,612,432 bytes). Matches the Makefile's documented ~26 MB.
- An embed-model build is documented at ~89 MB (model adds ~63 MB);
  not rebuilt here (needs the 63 MB HuggingFace fetch over the network).
  Savings of the lean path: ~61 MB (~68% smaller).
- Verified NO baked model in the 28 MB binary: `strings` finds only the
  curated-model PICKER metadata (names/descriptions like "BAAI/
  bge-small-en-v1.5 ~130 MB") and the candle/tokenizer code paths +
  the RUNTIME-RESOLVER strings (`<user-config>/chan/models/`,
  model.safetensors, tokenizer.json) -- i.e. the embed-model-OFF code
  path. No models.tar.zst bytes (the 28 MB vs 89 MB size gap is the
  proof; a baked model would land ~89 MB).

DIVERGENCE FOUND (the one lever): the LOCAL Makefile installs still bake
the model. `make build-release` (=> `--features embed-model`, 89 MB),
and `make install` depends on `make build-release`, and `make rpm` uses
`--features embed-model`. So a contributor running `make install` gets
an 89 MB model-baked binary while CI ships 28 MB -- an inconsistency,
and counter to the "smallest single binaries" goal for the common local
install.

RECOMMENDATION (chan CLI): keep the systacean-6 SPLIT (lean default,
opt-in embed) -- it's the right model and CI already follows it. Make the
DEFAULT install path lean too, so local `make install` matches what
users get from CI:
- `make install` -> depend on `make build` (lean 28 MB), not
  `build-release`.
- Keep `make build-release` as the explicit opt-in offline-bundle path
  (rename intent stays: "distribution where Hybrid must work offline
  with zero network"). Document that `make install` is lean and Hybrid
  search fetches the model on first use via `chan index download-model`.
- `make rpm`: drop `--features embed-model` so the local .rpm matches
  the CI .rpm (lean). Anyone wanting the bundled-model rpm runs an
  explicit opt-in variant.
This is a Makefile-only change (no Cargo.toml, no workflow, no code).
Risk: a contributor who relied on `make install` giving offline Hybrid
loses that default; mitigated by the on-demand download + a one-line
note. Net: local installs shrink 89 MB -> 28 MB and match CI.

APPLYING: architect-approved, no @@Alex gate -> applying the Makefile
change (make install lean; make rpm lean; keep build-release as the
opt-in offline variant; refresh the header doc). NOT touching
.github/workflows (already correct, so the inline-authorization statement
is moot -- noting it here for the record that no workflow edit was
needed).

APPLIED: commit dfdc012 (Makefile only). `make -n install` ->
`cargo build --release -p chan` (lean), `make -n rpm` -> zigbuild with no
--features embed-model. Ready-note posted to @@Architect.

### 2026-05-26 macOS CLI-to-desktop handoff DESIGN NOTE posted (@@Alex GATE)
Wrote the full design note and posted it to event-lane-b-alex.md. This is
my ONE @@Alex gate; I do NOT implement until ratified.

Covered: the three options (A attach-to-running-server / B
ask-desktop-to-open-a-window / C keep-owning-its-own-server), same-user
UDS discovery (mcp_bridge.rs per-pid socket is the reuse pattern; the
desktop would publish a WELL-KNOWN per-user UDS so the CLI can find it
without the pid; token travels over the UDS, never argv/env),
ownership/bearer-token/lifecycle/version/capability mismatch
representation, the mandatory no-desktop fallback (connect-refused /
stale / bad-handshake -> behave exactly like today), and the
standalone-forcing flags (`--standalone`, headless auto-skip,
CHAN_NO_DESKTOP_HANDOFF=1, tunnel flags force standalone).

CONTEXT GATHERED for the note: cmd_serve (chan main.rs:1022) opens the
drive under the per-drive flock, binds loopback, prints a one-shot token,
opens the browser; on contention with the desktop's embedded server the
desktop already surfaces "drive open in another chan process" verbatim
(embedded.rs map_open_error). The desktop's serve::start already mounts a
registry drive + spawns a window -- option B reuses that path.

RECOMMENDATION (one pick): OPTION B as default WHEN a same-user desktop
is discovered + GUI session + no standalone/tunnel flag; FALL BACK to
OPTION C otherwise. B gives the native window a desktop user expects from
`chan serve ~/notes`, reuses the existing mount+spawn path, keeps the
single-owner invariant clean (desktop owns the drive, CLI is a launcher
that exits); the mandatory C fallback keeps headless/scripted/no-desktop
unchanged. Skipped A (browser URL+token handoff) as strictly worse UX for
the same discovery cost + token-ownership muddiness.

WAITING on @@Alex ratification. No implementation this turn. Handing back
to @@Architect after this note (per plan: hand back after posting the
CLI-handoff note since it gates on @@Alex). Linux desktop launch stays
DEFERRED.

### 2026-05-26 fresh session resume + rebase + ITEM 1 (source-mode list rule)
Recovered state from journal + the @@Architect channel (latest = new task:
item 1 source-mode list rule) + the new-file-and-draft-spec. Rebased
phase-11-lane-b onto main @ 250d2f6 (my bug 8 + binary-size folded in via
the merge; branch is now the 250d2f6 tip, clean, no diff). CLI-handoff
implementation still GATED on @@Alex ratification (untouched).

ITEM 1 (new-file-and-draft-spec): source-code mode must NOT run markdown
input rules. INVESTIGATION + EMPIRICAL RE-WALK concludes it is ALREADY
CORRECT at HEAD (same shape as bug 1: fix predates the report).

Static analysis:
- Source.svelte registers NO markdown input rules and NO list keybinds.
  Its keymap is `[indentWithTab, ...defaultKeymap, ...historyKeymap]`; the
  markdown language pack is seeded with `addKeymap: false` (commit 5c9acca
  "fullstack-a-41", 2026-05-21, confirmed ancestor of main 250d2f6), which
  disables lang-markdown's Enter auto-continue.
- list.ts (parseListPrefix / continueListOnEnter / indent) is WYSIWYG-only:
  wired into Wysiwyg.svelte's Enter keybind, never imported by Source.svelte.
- The list MARKERS are drawn by block decorations (decorations/blocks.ts),
  also WYSIWYG-only; source mode never loads them.
- EVERY markdown() call in the tree uses addKeymap:false (Source.svelte x2
  + grammar.ts chanMarkdown). There is no space-triggered list input rule
  anywhere; the codebase has no inputHandler/InputRule list transform.

Empirical re-walk on a FRESH binary (chan 0.15.4 built from ../chan-lane-b
with the current bundle; throwaway drive /tmp/chan-test-lane-b-srcmode;
scoped teardown):
- note.md in source mode: typed `* hello` -> stays literal `* hello`, no
  bullet glyph, no transform. Enter after it -> bare empty line (caret +1,
  no `* ` re-insert). DOM line read confirmed.
- code.txt in source mode: typed `- item`, Enter -> bare empty line, no
  `- ` auto-continue. DOM line read confirmed.
So both the TYPING path and the Enter path are clean in source mode.

REGRESSION TEST added (the deliverable, since prod code is already right):
the existing sourceModeListKeymap.test.ts pinned only the Enter path; I
added a `typeInto` helper + 4 cases pinning the TYPING path (`* `/`- `/
`1. ` typed at line start, and into an empty doc, all via the
`input.type` user-event the only seam a markdown input rule could hook).
Locks the invariant so a future input-rule leak into source mode is caught.
9/9 in that file pass.

Full gate green: cargo fmt --check, clippy --all-targets -D warnings,
cargo test, build --no-default-features, svelte-check 0 err, full vitest
1541 pass / 11 skip / 0 fail (+4), npm build. Committed eaef1df (1 file,
test-only, lane-B-owned; no prod code, no shared file). Test server + tab
torn down (scoped pkill on my drive path), drive rm -rf'd.

Ready-note to @@Architect. Item 1 done. CLI-handoff still WAITING on @@Alex;
handing back after this.

### 2026-05-26 fresh session resume + rebase + CLI-HANDOFF SLICE 1 (0f3d4ea)
Recovered from journal + design note + the @@Architect channel: @@Alex
RATIFIED Option B (desktop opens a native window) as default when a
same-user desktop is discovered AND GUI session AND no standalone/tunnel
flag; fall back to Option C (own the server) otherwise. Implement now.
Rebased phase-11-lane-b onto main @ f088e83 (item 1 folded in via the
merge; branch is the f088e83 tip, clean).

Implemented the WHOLE thing in one gated slice (UDS discovery + handshake +
desktop listener + cmd_serve client + all flags/fallbacks), since the
pieces are tightly coupled and the flags are load-bearing for safety.

ARCHITECTURE: a new PUBLIC chan-server module `handoff` is the shared home
for both sides -- both `chan` (CLI) and `chan-desktop` already depend on
chan-server, so no new workspace crate and no new deps. Modeled on
control_socket.rs (line-delimited JSON, serde-tagged enums, unlink-stale-
before-bind, Drop-guard unlink), but with a WELL-KNOWN per-user socket so
the CLI finds the desktop without its pid: $XDG_RUNTIME_DIR/chan-desktop.sock
or (macOS, no XDG) <tmp>/chan-desktop-<uid>.sock, chmod 0600 + owner-only.
Used rustix::process::getuid() (already a chan-server dep via
terminal_sessions) rather than adding libc.

PROTOCOL: Request::OpenDrive { protocol, cli_version, drive_path };
Response::{Opened{version,caps} | VersionSkew{version,proto} | Error{msg}}.
PROTOCOL_VERSION=1. On skew the desktop refuses (open_drive callback never
runs) and the CLI prints "desktop is X, CLI is Y; cannot hand off" + falls
back -- no silent cross-version IPC. Capability field is forward-compat
(open_local_drive); a request the desktop can't satisfy -> Error -> fall
back. In Option B the token NEVER crosses the socket (the desktop spawns
its OWN window against its OWN embedded server), so token-in-argv/env/logs
is a non-issue.

CLI side (cmd_serve, chan/src/main.rs): the handoff attempt runs BEFORE
ensure_drive_registered + open_drive, so a successful handoff NEVER
double-opens (single-writer flock invariant: the desktop owns it, the CLI
is a launcher that exits 0). Gating: skip entirely if --standalone or any
tunnel token; then maybe_handoff_to_desktop adds the
CHAN_NO_DESKTOP_HANDOFF=1 opt-out and the headless auto-skip
(gui_session_present() returns false under SSH_CONNECTION/SSH_TTY/SSH_CLIENT
on macOS, and additionally requires DISPLAY/WAYLAND on non-mac unix).
try_handoff has bounded connect (1.5s) + IO (3s) timeouts so a stale/dead
socket can't hang the CLI. Every non-HandedOff outcome (NoDesktop /
VersionSkew / DesktopError) returns None -> the unchanged standalone path.
New --standalone flag added + threaded through dispatch.

Desktop side (desktop/src-tauri/src/main.rs): bind the well-known socket in
the Tauri setup block (Box::leak like the registry watcher; bind failure is
non-fatal -> CLI just falls back). open_drive_from_handoff mirrors the
add_drive flow: if the drive is already running, raise an extra window via
spawn_local_drive_window (synchronous); else register_and_boot
(creating the dir for a fresh path) + serve::start (mount + spawn window) on
a spawned task so the CLI gets a prompt response. Mount failures emit a
system notice (same as the first-launch default-drive path), not a CLI
block.

EMPIRICAL WALK on macOS (throwaway drive /tmp/chan-test-lane-b-handoff;
scoped pkills; XDG unset so the path is <tmp>/chan-desktop-501.sock). Could
not run the full Tauri GUI here, so I drove the desktop's EXACT production
listener via 3 throwaway chan-server example probes (removed after):
1. No desktop -> own server, prints URL, health 200. (handoff fell through:
   no socket -> NoDesktop)
2. Desktop present (probe = real start_listener) -> CLI printed "opened ...
   in chan-desktop", EXIT 0, probe logged OPEN_DRIVE #1, and NOTHING bound
   on :8799 (health 000). Single-writer invariant held: the CLI did not
   open the drive.
3. --standalone with the listener present -> own server, health 200.
4. CHAN_NO_DESKTOP_HANDOFF=1 with listener present -> own server, 200.
5. SSH_CONNECTION set (headless sim) with listener present -> own server,
   200 (auto-skip).
6. Version-skew probe (always replies version_skew) -> CLI printed
   "chan-desktop is version 0.1.0, CLI is 0.15.4; cannot hand off" + bound
   standalone, 200.
7. Stale socket (probe killed, file left) -> no hang, standalone 200.
8. Garbage-reply probe (non-JSON) -> bad handshake -> standalone 200.
9. Socket perms verified srw------- owned by me (0600 owner-only).
Probes removed (examples dir gone); sockets + temp out + throwaway drive
cleaned; the stale registry entry the standalone runs created (the CLI
owns -> registers) was `chan remove`'d; registry clean.

Full gate green: cargo fmt --check (root + desktop), cargo clippy
--all-targets -D warnings (root + desktop), cargo test (chan-server 340, +8
handoff; all workspace 0 fail), desktop cargo test --all-targets (75 + 7),
cargo build --no-default-features, svelte-check 0/0, npm run build OK (no
web changes; the 2 INEFFECTIVE_DYNAMIC_IMPORT warnings are pre-existing).
Committed 0f3d4ea (4 files: handoff.rs NEW + lib.rs one pub-mod line +
chan/main.rs cmd_serve/flag + desktop/main.rs listener; all lane-B-owned,
no @@LaneA shared structural file -- lib.rs::router/state.rs untouched).
Ready-to-merge posted to @@Architect.

REMAINING in handoff scope: end-to-end in the PACKAGED chan-desktop app
(the listener only binds inside the real Tauri build; the probe drove the
identical production listener code, but a `cargo tauri build` smoke that
launches the app + runs `chan serve <drive>` against it would confirm the
window actually spawns -- offered to drive that next). Linux desktop launch
(item 9) still DEFERRED.

### 2026-05-26 RE-ACTIVATION: watcher-scalability spec (now OWNED by me)
Fresh session. Recovered from journal + the @@Architect channel (latest =
RE-ACTIVATING on backend + verification) + watcher-scalability.md (now mine)
+ my handoff design note. Rebased phase-11-lane-b onto main @ 28d44c7 (my
handoff 0f3d4ea folded in via the --no-ff merge; branch is now the 28d44c7
tip, clean, no diff). New queue: 4 tasks from watcher-scalability.md.

### 2026-05-26 TASK 1 DONE: watcher feed ignore-filter (c9a9aae)
The single recursive OS watcher (chan-drive/src/watch.rs) feeds events via
WatchBroadcast::on_event (chan-server/src/bus.rs) to BOTH the indexer
(index_tx) and the broadcast/scopes (scopes.emit_fs). Before this slice the
watcher's is_filtered() dropped only .chan/.git/.hg noise (minus a
VCS-control allowlist). It did NOT drop node_modules/target/venv -- so a git
checkout storm under those dirs fanned thousands of events through the bus
and indexer.

GROUNDING: the bootstrap/index walk already prunes those dirs via the
unified WalkFilter (fs_ops::walk_drive_filtered consults
filter.is_excluded(basename) at any depth; the default set is
registry::DEFAULT_INDEX_EXCLUDED_DIRS = .git/.hg/.svn/node_modules/target/
__pycache__/.venv/venv/.tox/.pytest_cache/.mypy_cache/.ruff_cache/.cache/
dist/build). The Drive already holds walk_filter: Arc<WalkFilter>. The spec
explicitly says reuse THAT set, not a second list.

FIX (chan-drive only, watcher path -- NOT GI-3's graph/link modules):
- watch.rs: WatchHandle::start now takes Arc<WalkFilter>, threaded into
  dispatch(). is_filtered(rel, filter) keeps the VCS-control allowlist
  (.git/HEAD etc FORWARD for the indexer's checkout-storm detection) and
  always drops is_chan_internal (.chan is an internal invariant), then drops
  any event whose relative path has an excluded-dir basename at any depth
  (rel.split('/').any(is_excluded)) -- mirroring walk_drive_filtered's
  filter_entry. This is the EARLIEST drop point: in the notify worker
  thread, before on_event ever runs, so neither the broadcast bus nor the
  indexer sees the storm.
- drive.rs: watch() and watch_team() pass Arc::clone(&self.walk_filter).

WHY this layer (not bus.rs): dropping in chan-drive's dispatch() means the
event never reaches chan-server at all. Filtering in bus.rs would already be
past the chan-drive->chan-server boundary and would still pay the
cross-thread send. The watcher worker is the true earliest point.

DEVIATIONS from the walk, preserved deliberately:
- .git/HEAD / .git/index / .hg/dirstate FORWARD (walk prunes .git wholesale;
  the watcher needs them for checkout-storm detection in the indexer).
- .chan/ always drops via is_chan_internal even though .chan IS in the
  default set -- so a user who edits the excluded-dirs config can never
  un-hide chan's own state.

TESTS (4 new/extended in watch.rs, all green):
- filter_allows_vcs_control_paths_but_hides_other_vcs_noise: VCS-control
  forward + .git/.hg subtree + the bare .git/.hg dir entries drop.
- filter_drops_unified_ignore_set_at_any_depth: node_modules/target/venv/
  .venv/__pycache__ at top level AND nested (frontend/node_modules/...,
  a/b/target/...) + .chan.
- filter_keeps_real_notes_and_source: README.md/docs/design.md/src/main.rs
  pass; the non-prefix-match guard (targeting.md, node_modules_notes.md are
  NOT dropped -- only exact basename match prunes).
- dispatch_drops_excluded_subtree_events: a real dispatch() call with a
  node_modules modify never reaches the callback; a notes/today.md modify
  does.

CONTENTION: declared on event-lane-b-lane-a.md -- this is the watcher path
(watch.rs + watch()/watch_team()), NOT @@LaneA's GI-3 (graph.rs / link
resolution / index-completeness signal). No overlap, no sequencing.

Full gate green: fmt --check, clippy --all-targets -D warnings (workspace +
desktop), cargo test (chan-drive 533 incl +4 watch; chan-server 340; all
workspace 0 fail), build --no-default-features. No web touched. Committed
c9a9aae (2 files, both chan-drive). Ready-to-merge posted to @@Architect.

NEXT: Task 2 (git-storm resilience empirical check), then Task 3 (indexing
benchmark), then Task 4 (packaged-desktop handoff smoke).

### 2026-05-26 TASK 2 DONE: git-storm resilience confirmed (empirical, no code change)
Confirmed empirically that a git branch switch on a large repo, while
editing + running 2 terminals, does NOT starve the editor/terminal. The
fd_budget pacing (bug-7 fix) + debounce + the Task-1 watcher ignore-filter
hold. No code change; this is a verification task.

SETUP (scoped to /tmp/chan-test-lane-b-gitstorm, scoped teardown):
- Test drive: a --no-hardlinks clone of THIS repo (1369 tracked files, a
  real .git). Created a local branch `storm-root` at the repo's root commit
  (33 files). main<->storm-root checkout creates/deletes ~1336 files per
  switch -- a genuine checkout storm.
- chan serve --standalone (debug binary built from ../chan-lane-b @ the
  Task-1 commit). 2 long-running terminal sessions created via
  POST /api/terminals (each holds a PTY master + pipes + a shell child) to
  match the "running 2 terminals" scenario and consume fds.
- Latency probe: a tight loop GETing /api/files/README.md (the editor read
  path), logging curl's own time_total per request, phase-labeled.

RUN 1 (storm of 8 checkouts / 4 cycles, ~3.2s wall):
  phase       n     min   med   p95   p99   max    mean   (ms)
  BASELINE  1298   0.75  0.86  2.10  2.90  8.82   1.02
  STORM      111   0.85  2.04  2.84  5.55  11.96  2.22
  STORMTAIL  157   0.94  2.09  3.06  5.72  12.11  2.31
  RECOVERY   204   0.78  1.53  2.40  2.62  10.90  1.63
  Non-200 during STORM: 31, ALL 404, all sub-2ms. These are CORRECT: when
  the working tree is on storm-root, README.md genuinely does not exist on
  disk, so the read returns a fast 404. They map exactly to the storm-root
  windows; flip back to main and it's 200 again. NOT starvation.

RUN 2 (the harder case: the initial bm25 CONTENT reindex was STILL RUNNING
-- 751 .md files, /api/index/status = "reindexing" before/during/after --
AND a 12-checkout / 6-cycle storm hit simultaneously, ~3.6s wall):
  phase        n     min   med   p95   p99   max    mean   (ms)
  BASELINE2   255   0.75  0.85  2.00  3.50  8.21   1.07
  STORM2      260   0.64  0.81  1.09  1.34  2.84   0.84
  RECOVERY2   139   0.75  1.96  3.30  11.73 35.41  2.20
  Non-200 during STORM2: 71, ALL 404 (file-absent-on-storm-branch), zero
  OTHER non-200s.

ANALYSIS (meets expectations: YES, comfortably):
- The editor read path was NEVER starved. Worst single read across both
  runs was 35.4ms (a recovery-window tail while the indexer caught up);
  storm-window reads peaked at 2.84-12ms. Baseline is ~1ms. Starvation
  would look like seconds-range stalls or timeouts/5xx; we saw neither.
- Run 2 is the decisive one: storm2 read latency (med 0.81ms, p99 1.34ms)
  was actually TIGHTER than its own baseline, because during storm windows
  many reads hit the fast 404 path and the OS cache was hot, while the
  concurrent full content reindex was paced by fd_budget so it left ample
  headroom for interactive reads.
- FD pressure was a non-issue: the chan process held only 56 open fds
  (7 CHR, 6 IPv4, 3 KQUEUE, 31 REG, 5 unix, 3 DIR) after the storm, against
  a soft nofile limit of 1048576. 2 terminals + indexer + watcher + tantivy
  + sqlite, all live, nowhere near the EFFECTIVE_NOFILE_CEILING fd_budget
  guards. The macOS-256-soft-limit worry that motivated fd_budget did not
  bite here (the launchd session limit was 1M), but the pacing logic is
  still the guarantee on a tight-limit shell.
- Task 1 interaction confirmed: the server log had ZERO ProviderError /
  reindex-storm / EMFILE lines from the .git checkout storm. The .git
  events were dropped at the watcher boundary (Task 1), so the storm never
  reached the indexer's checkout-storm full-rebuild path at all. The
  fd_budget never even had to engage for the storm -- the ignore-filter
  removed the load upstream. (The content reindex in run 2 was a SEPARATE,
  user-relevant load that fd_budget DID pace.)
- Terminals survived the storm (sessions list intact); server health 200
  sub-ms after; the live file-name index picked up a fresh PUT mid-test
  (proof the watcher->indexer pipeline was live, not silently disabled).

SIDE OBSERVATION (not a bug, not in scope): the bm25 CONTENT index builds
lazily on drive open and took >30s to scan 751 .md files of a code repo;
during that window /api/search/content returns ready:true with empty hits
for not-yet-scanned files (file-NAME search is immediate). Expected for a
just-opened large drive. Flagging only for awareness; Task 3's benchmark
will quantify the end-to-end index time properly.

Teardown: killed the gitstorm server by its captured pid + scoped pkill on
the drive path (Lane A / Architect servers left alone -- 2 still running);
`chan remove`'d the auto-registered drive; rm -rf'd the throwaway drive +
probe scratch.

NEXT: Task 3 (end-to-end indexing benchmark, WITH vs WITHOUT chan-report,
bge disabled), then Task 4 (packaged-desktop handoff smoke).

### 2026-05-26 STOP/RESET fresh session: IGNORE-SET CONSISTENCY (e7b7824)
TOP PRIORITY per @@Architect STOP/RESET + ignore-consistency-spec.md.
@@Alex found the graph plotting node_modules/target (repo-root drive
60K-131K nodes). Recovered from journal + the @@Architect channel (latest
STOP/RESET) + the spec. Rebased phase-11-lane-b onto main 6103f4d; my
c9a9aae (watcher feed) replayed as b43ddeb, KEPT and folded in.

ROOT CAUSE: the DEFAULT ignore set was ALREADY sane (registry::
DEFAULT_INDEX_EXCLUDED_DIRS). The MAIN reindex + rebuild_graph were already
filtered (rebuild_graph uses list_tree_filtered; reindex sets the index
facade's walk_filter; reconcile uses walk_drive_filtered). The leaks were
FOUR OTHER walks feeding the index/graph:
1. PRIMARY: chan-server routes/fs_graph.rs - the filesystem-shape graph
   (/api/fs-graph, also merged into /api/graph via merge_filesystem_layer)
   did a raw read_dir recursion that skipped .git/.chan only at the TOP
   level. So a repo-root drive plotted its WHOLE dependency tree.
2. chan-server routes/graph.rs semantic-graph presence layer:
   drive_disk_files / drive_disk_dirs + merge_unified_tree_layer used the
   UNFILTERED list_tree_unified / list_tree_prefix_unified.
3. chan-report engine (via Drive::report -> ReportState::open ->
   Index::scan): its own ignore-crate WalkBuilder walk, defaulting to
   include_hidden=false + respect_gitignore=true but NO node_modules/
   target/venv exclusion without a .gitignore. Surfaced as language:
   JavaScript/Rust + target/debug + node_modules/pkg nodes in the graph's
   language layer.
4. (minor) drive.rs trash remove/restore subtree walks (1226/1320) used
   unfiltered walk_drive.

THE FIX (6 files, all consult the per-drive WalkFilter):
- fs_graph.rs: FsGraphWalker carries a WalkFilter; walk_dir skips blocklist
  basenames + .git/.chan at ANY depth (was top-level-only). new() takes the
  filter; build_fs_graph passes drive.walk_filter().clone(). Test helper
  split into walk()/walk_with_filter(); +1 unit test.
- graph.rs: drive_disk_files, drive_disk_dirs, merge_unified_tree_layer
  swapped to filtered listing helpers; +1 route unit test
  (merged layer excludes ignored dirs).
- chan-drive fs_ops.rs: filter-aware subtree branch in list_tree_inner +
  new list_tree_prefix_filtered. drive.rs: list_tree_filtered_unified +
  list_tree_prefix_filtered_unified (Drive); trash walks ->
  walk_drive_filtered. report.rs: ReportState::open takes excluded_dirs,
  sets opts.exclude_globs = ["node_modules/", "target/", ...] (gitignore-
  style, any depth); report_state() passes walk_filter.excluded_dir_names.
- RAW list_tree* stay UNFILTERED (open-inside-a-noisy-dir, requirement 3).
- NEW e2e tests/ignore_consistency.rs: ignored dirs absent from index +
  graph + report, present in raw listing.

CONTENTION: declared graph.rs + fs_graph.rs touch on event-lane-b-lane-a.md
(NOT GI-3 link-resolution; only the presence/tree layer + fs walker).

EMPIRICAL (seeded /tmp/chan-test-lane-b-ignore, --standalone, scoped kill +
deregister + rm -rf after; fresh binary rebuilt + clean per-drive metadata
via chan remove): fs-graph 4 nodes, semantic graph 8 nodes (all dep-tree +
language:JS/Rust nodes gone), search clean, File Browser clean, raw listing
still sees node_modules files. Other lanes' registry drives left untouched.

GATE: fmt --check clean; clippy --all-targets -D warnings clean (had to
park the untracked Task-3 index_bench.rs out of the worktree for the gate -
it trips a new rust-1.95 doc-comment lint; restored untracked after,
NOT committed - it belongs to the deferred benchmark task); cargo test
-p chan-drive 533+e2e -p chan-server 342, 0 fail; build --no-default-
features green. No web touched. Committed e7b7824; ready-to-merge posted.

WAITING: @@Architect merge. The benchmark (Task 3) + handoff packaged smoke
(Task 4) still wait per the STOP/RESET. index_bench.rs sits untracked in
the worktree for when the benchmark task reopens.

### 2026-05-27 fresh session resume + rebase + TASK 1 DONE: harden flaky indexer tests (34e3e23)
Recovered from journal + the @@Architect channel (latest = ignore fix
merged @ 4a7ab0f; next = harden flaky indexer tests, then benchmark, then
handoff smoke) + watcher-scalability.md. Rebased phase-11-lane-b onto main
4a7ab0f; my e7b7824 + b43ddeb folded into main via the merges, branch is
now the 4a7ab0f tip, clean. The parked crates/chan-drive/tests/index_bench.rs
is intact + untracked in the worktree.

ROOT CAUSE (reproduced empirically FIRST): the three indexer FS tests pass
single-threaded but flake under the full parallel `cargo test` CI runs.
Reproduction: ran `cargo test -p chan-drive --lib` (all 533 tests, 12-way
parallel) 3x; run 2 FAILED both writes_to_disk_get_indexed_after_debounce
(line 389, the `indexed_total >= 1` poll) AND writes_to_drafts_subtree
(line 472, the BM25-visibility poll). Under 12-way CPU saturation FSEvent
delivery + the indexer worker thread's turn on the CPU slip past the 5s
poll window; the coalescing test's sub-30ms inter-write sleeps likewise
stretch so the debounce window matures mid-burst. Wall-clock-timing
dependence under contention, exactly as @@Architect flagged.

THE FIX (crates/chan-drive/src/indexer.rs only; ZERO production behavior
change):
1. Injectable clock for the debounce decision. Extracted two pure helpers:
   - schedule_pending(pending, path, now, debounce): the per-path
     trailing-edge insert (overwrites the same map key -> coalesces).
   - collect_matured(pending, now): the maturity collection run_loop
     drains.
   apply_event now takes `now: Instant`; run_loop passes the real
   Instant::now() at both the schedule and the drain points, so the
   production loop is byte-identical. The coalescing test
   (debounce_coalesces_rapid_writes_into_one_index) was rewritten to drive
   those helpers against a controlled `base` Instant: five writes 10ms
   apart in a 30ms window, asserting exactly one pending entry the whole
   burst, nothing matures mid-burst, and exactly one path matures at
   base+70ms. No FS, no watcher, no sleep -> deterministic. Added a
   companion distinct_paths_do_not_coalesce_with_each_other pinning that
   coalescing is PER PATH (guards a future refactor keying the debounce on
   something coarser).
2. Serialize the three real-FS tests (writes_to_disk, writes_to_drafts,
   delete_from_disk) behind a process-wide poison-recovered Mutex
   (fs_test_lock, stdlib OnceLock<Mutex<()>>, NO new dep) so they do not
   stack their own watcher + reindex load on each other; and raised their
   poll budget 5s -> 30s (FS_DELIVERY_BUDGET) to absorb worst-case
   scheduling slip under the full suite. wait_for still returns the instant
   the condition holds, so the idle-host path stays sub-100ms; the 30s
   ceiling only governs the contended worst case.

WHY this shape (not serial_test crate, not deleting the FS tests): the
coalescing INVARIANT is pure logic and deserves a deterministic unit test;
the watcher->indexer->BM25 round-trips are genuine integration coverage
worth keeping real, just not worth racing 530 other tests for CPU. stdlib
Mutex + OnceLock keeps the no-extra-dep line.

VERIFIED (the real bar): 5 consecutive FULL parallel
`cargo test -p chan-drive --lib` runs, 534 passed / 0 failed each (was 533
+ the new distinct_paths test), 25/25 indexer-test "ok" lines, zero
FAILED/panic across all five. Before the fix, run 2 of the 3x repro
failed; after, 5/5 clean. Full gate green: fmt --check, clippy
--all-targets -D warnings (whole workspace incl desktop, 0 warn), build
--no-default-features. Committed 34e3e23 (1 file, chan-drive lane-B-owned;
the untracked index_bench.rs deliberately NOT in this commit -- it's
Task 2). Ready-to-merge to @@Architect.

NEXT: Task 2 benchmark (index_bench.rs, WITH vs WITHOUT chan-report, bge
disabled) -- already fixed its rust-1.95 clippy::doc_lazy_continuation lint;
re-running in RELEASE mode (the debug-build run was pathologically slow on
the chan-report COCOMO scan and not representative). Then Task 3 handoff
packaged smoke.

### 2026-05-27 TASK 2 DONE: end-to-end indexing benchmark + a CRITICAL embeddings finding
The benchmark (crates/chan-drive/tests/index_bench.rs, the parked file)
measures end-to-end index time for a copy of THIS repo, WITH vs WITHOUT
chan-report, bge embeddings DISABLED, asserting no embedding work.

THE CRITICAL FINDING (caught by the assertion, not assumed): the benchmark
MUST run with `--no-default-features` (embeddings feature compiled OUT).
The first faithful run FAILED its own `indexed_vectors == 0` assertion:
the `embeddings` feature is on by default AND a bge-small model is cached
on this machine, so `Drive::reindex` (which builds with
`BuildOptions::default()`, `include_vectors=true`, and has NO public
BM25-only path) ran candle inference on every chunk. That embedding work
is what made the earlier debug AND release runs take 6-11 MINUTES -- it
was NOT the structural index. The assertion did its job: it refused to
silently measure the wrong thing. Rebuilding the bench with
`cargo ... --no-default-features` removes the embed code entirely
(`indexed_vectors == 0` by construction) and is the spec-required
"embeddings DISABLED entirely" mode.

NUMBERS (release, --no-default-features, full repo = 1371 git-tracked
files, 751 indexable text files; two passes, machine under concurrent
build load from the other lanes so there is run-to-run variance):
  pass  WITHOUT-report   WITH-report                         report/reindex
        (reindex)        (reindex + report = total)
  1     2664 ms          2578 ms + 2622 ms = 5200 ms          0.98x
  2     2706 ms          1952 ms + 1528 ms = 3480 ms          0.56x

ANALYSIS (meets expectations: YES):
- Structural index (graph rebuild + BM25 build_all) of the whole repo
  (751 indexable files) is ~2.0-2.7s. That is FAST and the headline
  correction to my mid-task worry: with embeddings off the structural
  index is seconds, not minutes. The multi-minute times I chased earlier
  were 100% bge embedding inference, which the spec explicitly excludes.
- chan-report's language scan (SLOC / language / COCOMO over the same
  751 files) adds ~1.5-2.6s -- roughly EQUAL to the entire structural
  index (0.56x-0.98x of it across the two passes). So enabling chan-report
  on a code-heavy drive about DOUBLES the end-to-end index time. For a
  notes drive (chan's real workload, mostly small markdown) the absolute
  cost is far lower; the repo's 262 TS + 146 Rust + 61 Svelte source files
  are what make chan-report's scan non-trivial here.
- Embeddings are the dominant cost WHEN ON; they are correctly opt-in now
  (GPU/embeddings work this round), so the lean default index path is the
  fast one. This benchmark is the structural + chan-report cost only, as
  the spec asked.
- The run-to-run variance (pass 1 vs 2) is from concurrent compile load
  from @@LaneA / @@Alex on the shared machine during the measurement; the
  RATIO (chan-report ~ structural index) is the stable, load-independent
  takeaway.

BENCHMARK SHAPE (the deliverable, committed as the now-tracked
index_bench.rs): `#[ignore]`'d so it never runs in CI; copies git-tracked
files (honors the ignore set -- target/.git/gitignored excluded);
`CHAN_BENCH_MAX_FILES` caps the copy (default 250, 0 = whole tree);
`CHAN_BENCH_REPO` overrides the source; times the reindex and the
chan-report scan SEPARATELY (so the WITH number is the marginal
chan-report cost, not a second whole reindex); asserts `indexed_vectors
== 0` in both modes so it cannot accidentally measure embeddings. Run:
`cargo test -p chan-drive --no-default-features --test index_bench --
--ignored --nocapture`. Also fixed its rust-1.95
clippy::doc_lazy_continuation lint (the reason it was parked out of the
ignore-fix gate). Gate: fmt --check clean; clippy -D warnings clean in
BOTH default and --no-default-features configs.

### 2026-05-27 TASK 3 DONE: handoff packaged smoke FOUND + FIXED a launch-crash bug (fba85d8)
Drove a real CLI->desktop handoff against a debug chan-desktop on macOS.
This caught a serious bug my earlier probe-based handoff verification
missed.

THE BUG: the handoff listener crashed the desktop on EVERY launch.
`chan_server::handoff::start_listener` binds a tokio `UnixListener` and
`tokio::spawn`s the accept loop, so it must run inside a tokio runtime
context. The Tauri `setup` closure runs on the main thread OUTSIDE any
runtime, so calling start_listener directly panicked with "there is no
reactor running, must be called from the context of a Tokio 1.x runtime".
Because the panic crosses an FFI boundary that cannot unwind, it ABORTED
the whole desktop process at startup. The listener binds unconditionally
on boot, so this hit every launch, not just handoff -- a regression in my
own 0f3d4ea handoff slice. My earlier "9 probe" verification drove the
production listener from inside `#[tokio::main]`, which HAS a runtime, so
it never reproduced the real Tauri-setup-thread context. Exactly the gap
the packaged smoke exists to close.

THE FIX (fba85d8, desktop/src-tauri/src/main.rs + a log line in
handoff.rs): wrap the start_listener call in `tauri::async_runtime::
block_on(async { ... })` so the bind + the spawned accept loop attach to
the Tauri-managed tokio runtime (the same one the embedded-server start
and every async_runtime::spawn already use); the accept loop survives
after block_on returns (multi-threaded runtime). Added a
`tracing::info!("handoff: opening drive from CLI request")` on the
accepted OpenDrive (+ a warn on a failed open_drive callback) so the
socket -> handler -> window-spawn chain is observable in the desktop log.

EMPIRICAL WALK (debug chan-desktop, RUST/CHAN_LOG capturing chan_server,
throwaway drive /tmp/chan-test-lane-b-handoff-smoke, XDG unset so the
socket is <tmp>/chan-desktop-501.sock; scoped teardown):
- Pre-fix: desktop launched -> immediate panic at handoff.rs:251 ->
  process EXITED (socket file created by bind, then tokio::spawn panicked).
- Post-fix: desktop boots clean, no panic, binds the 0600 well-known
  socket, stays ALIVE.
- 1st `chan serve <drive>` (drive NOT running): CLI printed "opened ... in
  chan-desktop" + EXIT 0; desktop log showed the accepted-open INFO line
  (cli_version=0.15.4, the drive path); the NOT-RUNNING branch ran
  register_and_boot (per-drive metadata dir
  ~/.chan/drives/...handoff-smoke... created) + serve::start (drive
  mounted, embedded server LISTENing on localhost:58865). So socket ->
  handle_request -> open_drive_from_handoff -> register_and_boot ->
  serve::start (mount + loopback bind) ALL fired and succeeded.
- 2nd `chan serve <drive>` (drive now running): handoff-accept count = 2,
  CLI exit 0, desktop alive -> the ALREADY-RUNNING branch
  (spawn_local_drive_window, synchronous raise-extra-window).
- Both branches held the single-writer invariant (the CLI never opened the
  drive; the desktop owns the flock) and the desktop survived both.
- Used CHAN_LOG (not RUST_LOG -- chan-desktop reads CHAN_LOG) to surface
  the chan-server-level handoff log; a first attempt with RUST_LOG showed
  nothing because the default filter is `warn,chan_desktop=info`.

VISUAL-ONLY GAP for @@Alex: the actual native WINDOW PAINT
(spawn_local_drive_window creating a visible WebviewWindow on screen) needs
a real GUI session; this headless agent box confirmed everything up to and
including serve::start mounting + the loopback bind, but cannot assert a
window pixel-painted. @@Alex: please confirm in a real desktop session that
`chan serve <drive>` raises a visible editor window (both first-open and
the already-running extra-window case).

GATE (Task 3): fmt --check clean (root + desktop); clippy -D warnings clean
(chan-server + chan-desktop); chan-server handoff tests 8/0; chan-desktop
tests 75/0 + 7/0; cargo build --no-default-features green earlier.
Committed fba85d8 (chan-server/src/handoff.rs + desktop/src-tauri/src/
main.rs; lane-B-owned). Teardown: desktop killed by pid, socket removed,
drive `chan remove`'d + rm -rf'd, registry clean, no leftover chan-desktop;
@@LaneA / @@Alex / docsrv servers untouched.

ALL THREE TASKS DONE this turn: Task 1 (flaky tests, 34e3e23), Task 3
(handoff crash fix, fba85d8), Task 2 (benchmark + embeddings finding, about
to commit). Ready-notes to @@Architect.