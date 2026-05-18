# frontend-2: Editor and file browser interaction fixes

Owner: @@Frontend.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [webtest-1.md](./webtest-1.md)
- [frontend-3.md](./frontend-3.md)

## Goal

Fix the focused editor and file browser interaction bugs:

- Document Cmd+F Enter moves the cursor to the beginning of the word match.
- File Browser overlay supports Cmd+F over expanded and visible entries.
- File Browser right-click context menu appears adjacent to the clicked
  file/folder row/label instead of far from the pointer.
- New-file creation supports tab-complete.
- Multi-level indent no longer de-indents the next line of a long sentence.
- Cursor height is not inherited from an image on the previous line.
- Document list guide lines do not break on images and auto-hide after 1.5s
  when the cursor is outside the list.
- Text editor selection does not get stuck around image/list blocks after the
  caret moves.
- File and folder icons use GitHub style, including folder icons in the file
  browser.

@@Frontend owns this task in the same queue as [frontend-1.md](./frontend-1.md)
and [frontend-3.md](./frontend-3.md). Do not split to @@FrontendB unless
@@Architect creates a separate non-overlapping task file.

## Acceptance criteria

- Each listed bug has a reproduced/fixed note or a precise "not reproducible"
  note with evidence.
- Keyboard behavior works on macOS Cmd+F and does not regress non-macOS find
  behavior where supported.
- File Browser find only targets expanded and visible entries.
- File Browser context menu placement uses the click coordinates/row anchor
  correctly in the overlay, including when the inspector pane is open.
- Tab completion for new files is deterministic and does not create files
  accidentally.
- Image/list/cursor fixes are verified with an image-containing document.
- Stale blue selection rectangles clear when focus/caret moves away from an
  image/list region.

## Test expectations

- Run `cd web && npm run check`.
- Add editor/file-browser tests for deterministic logic where practical.
- Coordinate visual/browser smoke with [webtest-1.md](./webtest-1.md).

## Review expectations

- @@Webtest browser validation on desktop and narrow viewport.
- @@Architect review before broad editor behavior changes are committed.

## Progress notes

- 2026-05-16 @@Architect: Alex reported an additional editor bug with screenshots:
  a large blue text selection can remain stuck across image/list rows, and
  after moving the cursor to another line the old selection persists as split
  stale blue blocks around the embedded images. This likely belongs with the
  image/list guide and cursor-height work in this task.
- 2026-05-16 @@Architect: briefly considered assigning this to @@FrontendB, but
  Alex showed @@Frontend already has frontend-1, frontend-2, and frontend-3
  in its active queue. Ownership stays with @@Frontend to avoid duplicate edits.
- 2026-05-16 @@Frontend: started.

Landed:

- **Cmd+F Enter cursor placement** — extended `FindAdapter` with
  `placeCursor(index)` and wired Enter / Shift+Enter in
  `FindBar.svelte` so each step also moves the editor selection
  to the start of the current match without stealing focus from
  the find input. After Esc the caret lands on the navigated
  match. Files: `web/src/editor/{find.ts,base.ts}`,
  `web/src/components/FindBar.svelte`. Test:
  `web/src/editor/find.test.ts` (5 cases).
- **File Browser Cmd+F over expanded entries** — `FileTree`
  exports `setFindQuery / findStep / clearFind`. Matches are
  derived from `visibleRows` so only expanded rows are eligible.
  `FileBrowserOverlay` shows a sticky find bar at the top of the
  tree column, opens on Cmd+F (overlay-scoped), with a counter
  and prev/next/close controls. Selected match rides
  `browserSelection` so Enter on the input scrolls to the match.
  Styled with the same palette as the editor's `FindBar`.
- **New-file tab-complete** — `PathPromptModal` Tab now extends
  the input to the longest common prefix of the visible
  suggestions; single-match Tab completes through to the folder
  (with trailing `/`); already-at-LCP Tab falls back to cycling
  the highlight. Helper extracted to `web/src/state/lcp.ts` with
  `lcp.test.ts` coverage (8 cases).
- **Multi-level indent regression** — nested list lines now use
  per-depth `padding-left` + negative `text-indent` so soft-
  wrapped visual rows hang under the parent content instead of
  collapsing back to the gutter.
- **List guide auto-hide** — new
  `editor/extensions/list_guide_visibility.ts` watches caret
  position, toggles `data-list-guides` on the editor; CSS fades
  `.cm-md-list-line::before` opacity 1.5s after the caret leaves
  any list line. Re-entering cancels the timer. Test:
  `list_guide_visibility.test.ts` (6 cases).
- **GitHub-style file/folder icons** — `FileTree` chevrons now
  use lucide `ChevronRight` / `ChevronDown` instead of `▾`/`▸`;
  folders get `Folder` / `FolderOpen` glyphs colored with
  `--accent` between the chevron and the folder name. File rows
  unchanged (already used per-kind lucide icons via
  `iconFor(kind)`).

New post-REVIEW bug from Alex (now fixed):

- **File Browser context menu positioning** — root cause:
  `OverlayShell .panel` has `transform: scale(...)` on hover and
  during the open animation. Per CSS spec, any non-`none`
  transform on an ancestor reparents fixed-positioned descendants
  to that ancestor (a containing block) instead of the viewport.
  `FileTree`'s `.ctx` was rendered inside the panel and used
  `position: fixed`, so the clamped (cursor x, cursor y)
  coordinates landed relative to the panel rather than the
  viewport — visible drift, worse with the inspector pane open
  because it shifts the panel's origin further from the tree.
  Fix: a `portal` Svelte action that moves the `.ctx` element to
  `document.body` on mount, mirroring `HamburgerMenu.svelte`'s
  existing portal (where the same fix already lives, with a
  comment that documents the underlying transform vs.
  position:fixed issue). See `FileTree.svelte`.

Deferred for browser repro with @@Webtest:

- **Cursor height inherited from image on previous line** — the
  CodeMirror `.cm-cursor` height is computed from the DOM line
  rect; the image widget uses `display: inline-block;
  line-height: 0` and should not propagate height to the next
  line. Without a live repro it is unclear whether the user is
  seeing (a) caret on the same source line as the image (the
  "next visual row" of a long item), (b) a selection-layer
  rectangle that inherits a tall ::before guide, or (c) an
  inline-block vertical-align quirk. Need a screenshot or
  reproducible doc to localize the fix.
- **Image-line guide bars break around images** — likely
  entangled with the same line-height issue above; will revisit
  after Webtest confirms the exact symptom.
- **Stale selection rectangles around image/list blocks** — same
  surface as the cursor-height bug; needs the same repro before
  I'll commit code. Pending.

## Test expectations

- `cd web && npm run check` — clean.
- `cd web && npm test -- --run` — 145 tests pass (was 111;
  added 34 new across 4 new test files:
  `web/src/editor/find.test.ts`,
  `web/src/editor/extensions/list_guide_visibility.test.ts`,
  `web/src/state/lcp.test.ts`,
  `web/src/state/scope.test.ts`).
- Browser smoke: see [webtest-1.md](./webtest-1.md). Cases to
  exercise: Cmd+F Enter cursor placement (Wysiwyg + Source
  mode); File Browser Cmd+F over nested folders; tab-complete
  completion vs. cycling; nested-list wrap with a long sentence
  at depth ≥ 2; list-guide fade after caret leaves a list for
  1.5s; chevron + folder icons appearance match GitHub-style
  screenshot; right-click context menu appears adjacent to the clicked row.

## Commit readiness notes

Ready for @@Architect / @@Webtest review. Suggested commit unit:
all of the above as one frontend-2 commit since the diffs share
the editor / file-browser surface; defer the image-cursor /
guide-image / stale-selection work to a follow-up after browser
repro lands.

Suggested commit message:

```
chan-web: editor + file-browser interaction fixes

- Find: Enter / Shift+Enter now moves the editor caret to the
  start of the current match (placeCursor) without stealing
  focus from the find input.
- File Browser: Cmd+F opens an in-overlay find bar that filters
  to currently expanded/visible entries; Enter steps through
  matches via the existing selection plumbing.
- PathPromptModal: Tab extends the input to the longest common
  prefix of the visible folder suggestions; single match
  completes through; otherwise cycles.
- Editor: nested list lines hang-indent on soft wrap so long
  sentences no longer collapse back to the gutter on the
  second visual row.
- Editor: vertical list guide bars fade out 1.5s after the
  caret leaves a list line; re-entering cancels.
- File tree: chevrons + folder glyphs switched to lucide
  Chevron* / Folder* for GitHub-style appearance.
```
