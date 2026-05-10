# Overlays revamp plan

Scope: settings, file browser, search overlays. Inspector becomes
the shared detail surface across browser / search / graph. Desktop
only for v1; mobile click-vs-inspector disambiguation deferred.

Touches: `web/src/components/{SettingsPanel,FileBrowserOverlay,SearchPanel,FileTree,Inspector,FileInfoBody,GraphPanel,PromptModal}.svelte`,
`web/src/state/store.svelte.ts`, `web/src/api/client.ts`, plus
new `web/src/components/PathPromptModal.svelte` and
`web/src/components/{TagInfoBody,ImageInfoBody}.svelte` (split out
from FileInfoBody).

Server: extend `/api/search/content` to also return tag and image
hits (or add `/api/search/tags`, `/api/search/images`). TBD below.

---

## 1. Settings: drop "Notes" section, lead with Assistant

`SettingsPanel.svelte:587-601` (the `<section><h3>Notes>` block
with `Name` + `Folder`) goes away. The assistant section starting
at `:603` becomes the first section.

- The folder readout (drive root path) is dropped from settings;
  it remains visible in the file browser header
  (`FileBrowserOverlay.svelte:151`, `title={drive.info?.root}`).
- `editedName`, the name dirty-tracking, and the
  `api.updatePreferences({ name })` call in `save()`
  (`SettingsPanel.svelte:405-408`) move out. They get re-wired
  into the new file-browser hamburger (section 2). The
  per-device-global config save path stays put.
- `dirty()` and `snapshot()` lose the `editedName` term.
- `loadGlobalConfig`, `loadLlmStatus`, font preview, build info,
  index reset all stay where they are.

Open question: do we also drop the read-only "Folder" line, or
move it into the same hamburger as a non-editable "Drive root:
…"? Recommend: include in hamburger, read-only, copy-to-clipboard
on click. Cheap and answers "where is this on disk?" without
opening settings.

Answer: yes, move it to the hamburger

## 2. File browser: hamburger replaces the "+" popover

The current "+" popover in `FileBrowserOverlay.svelte:78-143,
210-233` becomes a single hamburger (`☰`) that opens a menu with:

1. New file
2. New folder
3. ─
4. Rename drive… (opens prompt pre-filled with `drive.info.name`;
   PATCH `/api/drive` with `{ name }`. Pure metadata, the on-disk
   directory is untouched. This is the same call that lived in
   settings.)
5. Copy folder path (writes `drive.info.root` to clipboard)

Implementation notes:
- Replace `triggerEl`/`newMenuOpen`/`POPOVER_*` plumbing with a
  generic hamburger menu component or keep the inline popover and
  just add three more `<li>` rows.
- Add `fileOps.renameDrive()` in `store.svelte.ts` next to the
  other fileOps; it calls `uiPrompt("drive name", drive.info.name
  ?? "")` then `api.updatePreferences({ name })` and sets
  `drive.info` from the response.
- Keep the `+` glyph if we want a separate "fast new" affordance;
  recommend collapsing into the single hamburger to keep the
  header narrow on mobile.

## 3. Move/rename: autocomplete + create-folder hint + path warnings

Current rename uses `uiPrompt` (generic single-line modal,
`PromptModal.svelte`). Replace with a new `PathPromptModal.svelte`
used for both move and rename (and reused for "new file" /
"new folder" so we get autocomplete there too).

Behavior:
- As the user types, derive the directory portion (everything up
  to the last `/`) and filter `tree.entries` for `is_dir === true`
  whose paths start with that prefix. Show as a dropdown below
  the input.
- Tab / ↓ + Enter to accept a suggestion; Enter on the raw input
  submits.
- Live status row underneath the input:
  - "→ moves to existing folder `foo/bar/`" when the parent
    exists.
  - "→ creates folder `foo/new/`" with a warning glyph when the
    parent doesn't exist (server will mkdir-as-needed; if the
    server doesn't, that's a separate fix surfaced by this UI).
  - "✗ invalid path: <reason>" when client-side validation
    rejects (see below); submit button disabled.

Client-side path validation (matches what chan-drive will reject
or sanitize, so we can warn before the round-trip):
- Empty after trim.
- Absolute path (`/...`).
- `..` segments or `.` segments.
- NUL bytes, control chars (0x00-0x1F).
- Trailing/leading whitespace in any segment.
- Reserved names on Windows when running cross-platform: CON,
  PRN, AUX, NUL, COM1-9, LPT1-9 (drive may run on a Windows
  host; cheap to check).
- Segments ending in `.` or ` ` (also Windows-hostile).
- Length: any segment > 255, total > 4096.

Single source of truth: extract these checks into
`web/src/state/pathValidate.ts` and call from both the modal and
any future drag-drop / paste handler. Mirror the checks that
chan-drive enforces (see `chan-core/crates/chan-drive/src/drive.rs`
and the cap-std layer); keep the client list a strict subset so
"client says ok" never lies.

Overwrite handling: live status row shows "→ overwrites existing
file `foo/bar.md`" while typing. On submit when target exists,
intercept with a confirm dialog ("Overwrite `foo/bar.md`?") before
firing the API call. Cancel returns to the prompt with the input
intact so the user can adjust without retyping.

Implementation: reuse `PromptModal`'s pattern for the confirm
(or a new `ConfirmModal.svelte` if we don't have one already);
call it from the PathPromptModal submit handler when the target
path matches an existing `tree.entries` entry.

## 4. Search: prefill from selection + tags + images + inspector

### 4a. Prefill from selection on open

When `searchPanel.open` flips true (`SearchPanel.svelte:33`), if
`window.getSelection().toString()` is non-empty and passes the
gate below, set `query` to it and run `scheduleSearch()`
immediately.

Gate (cap to keep the palette useful):
- After `.trim()`, must be non-empty.
- Word count (split by whitespace) must be ≤ 8. Recommend 8 over
  10: BM25 starts losing ranking signal past that, and a long
  selection is almost always an accidental paragraph copy.
- Total length ≤ 200 chars.
- No newlines (single-line selection only).
- Drop selection if the focus is inside an `<input>` /
  `<textarea>` (likely a separate search-like field, not editor
  content).

If the gate fails, fall back to the current empty-input behavior.
Always select-all the prefilled value so the user can overtype
without manual select-all.

Wire-up in both entry points:
- Cmd/Ctrl+K shortcut handler (find in `store.svelte.ts` or
  wherever the global keybindings live; the Cmd+P comment in
  SearchPanel may be stale, verify).
- The toolbar button click handler.

### 4b. Search tags + images

Server work (smallest viable):
- Add an opt-in `kinds` query param to `/api/search/content`:
  default `kinds=content`, accept comma-separated
  `content,tag,image`.
- Extend `ContentHit` (route file at
  `crates/chan-server/src/routes/search.rs:107`) with a `kind`
  discriminant: `"chunk" | "tag" | "image"`. Existing fields
  stay; new optional `tag_name`, `image_path`, etc.
- Tag search: substring match against the tag list maintained by
  `chan-drive::Drive::search` / the graph. Cheap path: walk the
  graph snapshot (already loaded by the frontend for the inspector
  refs section) on the client. Server path is cleaner long-term
  but graph-on-client is enough for v1.
- Image search: filename match against `tree.entries` filtered to
  `isImage(path)`. Same client-side option; server-side later.

Recommend: keep server route unchanged for v1, do tag/image
filtering client-side from `tree.entries` + `graphData.view`.
Server change lands when we want OCR'd image text or tag-co-
occurrence ranking. This avoids a contract churn while we settle
the inspector UX.

Result list rendering changes:
- One row per hit, with a small kind chip (`doc`, `tag`,
  `image`) on the left. Reuse `NODE_COLORS` from
  `GraphPanel.svelte:285` so chips match the graph palette.
- Image hits show a thumbnail (use the existing image inspector's
  thumb URL pattern from `FileInfoBody.svelte`).
- Tag hits show the tag name and a count of referencing files.

### 4c. Open inspector on result click

Today: clicking a row calls `openInActivePane(h.path)` and closes
the search overlay (`SearchPanel.svelte:75-83`).

New behavior:
- Click on a desktop row opens the inspector pane on the right
  side of the search overlay (mirror the file-browser layout:
  `FileBrowserOverlay.svelte:184-201`). The inspector body
  dispatches by hit kind:
  - `chunk` / file → `FileInfoBody`
  - `image` → `ImageInfoBody` (split-out from FileInfoBody's
    image branch; same thumb + backlinks layout used by graph at
    `GraphPanel.svelte:1080-1088`)
  - `tag` → `TagInfoBody` (new; lists files referencing the tag,
    similar to graph's tag-node panel at
    `GraphPanel.svelte:1135-1158`)
- Double-click (or Enter on keyboard nav) keeps current behavior:
  open file in active pane and close.
- The "open" affordance (button in the inspector body) is the
  primary path-to-editor action; closes the overlay.

This needs the inspector pane wired into `SearchPanel.svelte`'s
`OverlayShell`. Pattern is identical to file browser; same
`paneWidths.search` slot (new) persisted via
`persistPaneWidths`.

## 5. Inspector reuse across overlays

Current state:
- `Inspector.svelte` is the chrome (resize handle, aside, title).
- `FileInfoBody.svelte` is the body for files + folders with a
  branch for images.
- Graph has its own inline body (`GraphPanel.svelte:1064-1160`)
  that mostly duplicates FileInfoBody's logic, except for tag
  nodes which only the graph renders today.

Refactor:
1. Split `FileInfoBody.svelte` into:
   - `FileInfoBody.svelte` (regular files + folders).
   - `ImageInfoBody.svelte` (image preview + backlinks, no Open).
   - `TagInfoBody.svelte` (new; lifted from
     `GraphPanel.svelte:1135-1158`).
2. Add an `InspectorBodyDispatcher` that takes a `{ kind, path |
   tagName }` and renders the right body. Used by search +
   browser + graph.
3. `GraphPanel.svelte` switches its right pane to use
   `Inspector.svelte` + the dispatcher instead of the inline
   markup. Keep the kind-chip color logic; the chip lives in
   each body component since it's body-shaped, not chrome-
   shaped.

Net: one chrome, one dispatcher, three body components, three
overlays consume them. `Inspector.svelte` stays as is.

## 6. Mobile: deferred

Desktop pattern (lock in for v1, all overlays):
- Click an inspectable element (graph node / tag / image, file
  browser file or image row, search result) opens the inspector
  pane in place. Double-click (or the inspector body's "Open"
  button) routes to the editor / focus action.
- This is what file browser does today; we extend it to search
  (section 4c) and align graph (section 5).

Mobile is out of scope for v1. Tap-to-open-inspector vs
tap-to-open-editor is the same single gesture and we don't have
a clean way to disambiguate without a long-press / force-touch
gesture, which we're explicitly skipping. Revisit once the
inspector flows are settled on desktop and we have a clearer
picture of how mobile usage actually shakes out.

Drop the planned `web/src/state/forceTouch.ts` module from the
file list at the top of this doc.

---

## Suggested order of merge

1. Settings → file-browser hamburger move (sections 1 + 2).
   Self-contained, no API changes, smallest blast radius.
2. PathPromptModal + path validation (section 3). Replaces the
   generic prompt for file ops; keep `PromptModal` around for
   the new drive-rename flow (single-line, no autocomplete).
3. Inspector body split + dispatcher (section 5). Pure refactor,
   no UX change yet.
4. Search prefill from selection (section 4a). One-file change.
5. Search inspector pane + tag/image hits (sections 4b + 4c).
   Largest piece; depends on 3.

Tests: pathValidate gets a unit test (hand-rolled vector of
inputs). The selection-prefill gate gets one. UI behaviors stay
manual for now; we don't have a frontend test harness in the
repo today.
