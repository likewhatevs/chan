# fullstack-a-61 — Cmd+N opens new untitled-N.md editor tab; move chan-desktop "New Window" to Cmd+Shift+N

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two parts to rebind Cmd+N:

1. **chan-desktop side**: move the existing "New Window"
   menu item accelerator from `CmdOrCtrl+N` to
   `CmdOrCtrl+Shift+N`.
2. **SPA side**: bind `Cmd+N` (Ctrl+N on Linux/Windows web)
   to open a new editor tab with file
   `untitled-N.md` at the drive root, where `N` is the
   smallest integer making the name unique among
   existing files + open drafts. First call lands
   `untitled.md`; second call `untitled-1.md` (or
   `untitled-2.md` if `untitled-1.md` exists); etc.

## Today's behaviour

* chan-desktop: `Cmd+N` → opens a new chan-desktop
  window via Tauri menu accelerator at
  `desktop/src-tauri/src/main.rs:1069`
  (`MenuItemBuilder::with_id("app-new-window", "New Window")
   .accelerator("CmdOrCtrl+N")`).
* SPA: no Cmd+N binding (grep returned no KeyN bindings
  in `web/src`).
* Web build: Cmd+N falls through to browser default
  (new window).

## Spec

### chan-desktop accelerator move (1-line change)

`desktop/src-tauri/src/main.rs:1070`: change
`.accelerator("CmdOrCtrl+N")` to
`.accelerator("CmdOrCtrl+Shift+N")`.

The menu item stays in the Window submenu; label stays
"New Window"; only the accelerator moves.

This frees Cmd+N for the SPA-side handler. In chan-desktop
the SPA's key-bridge captures keydown events the same
way it does for Cmd+T / Cmd+P / etc. once the menu's
accelerator no longer claims the chord.

### SPA new-untitled-tab handler

* Add `Cmd+N` (or `Ctrl+N` on non-Mac) to the SPA keymap
  in `App.svelte` (or wherever the cmd-bridge lives).
* On fire: compute the next `untitled[-N].md` filename
  at drive root using a new helper alongside the
  existing `proposeDefaultFilename` /
  `DEFAULT_NEW_FILENAME_STEM = "untitled"` at
  `web/src/state/pathValidate.ts:160`. Add a function
  like `nextUntitledFilename(driveRoot, existingFiles,
  openDrafts): string` that picks the smallest `N`
  (or empty if `untitled.md` is free) producing an
  unused name.
* Open a new editor tab with that file path. The file
  doesn't need to exist on disk; the tab is a fresh
  draft. Existing draft / new-file infrastructure
  (per `proposeDefaultFilename` + `PathPromptModal`
  patterns) should support this. Implementer audits +
  picks the right entry point — most likely
  `openFileInActivePane` or `createDraftTab` shape
  depending on what the tab system has.

### File creation semantics

Implementer's call between two reads:

* **(A) Lazy create**: tab opens with the unique
  `untitled-N.md` path; file isn't written to disk
  until the user types / saves. Matches typical
  editor "New File" semantics.
* **(B) Eager create**: file written as 0-byte
  `untitled-N.md` at drive root immediately + tab
  opens against it. Visible in FB instantly.

Recommend **(A) lazy** — matches the existing
PathPromptModal pattern. Empty draft until user types.
But (B) is acceptable if (A) needs significant new
infrastructure.

## Acceptance

1. **chan-desktop accelerator moved**: Cmd+Shift+N
   opens new chan-desktop window; Cmd+N does NOT open
   a new window.
2. **Cmd+N fires SPA handler**: in chan-desktop AND web
   builds, Cmd+N triggers the new-untitled-tab logic
   (not browser default in web build; not "New Window"
   in chan-desktop).
3. **First fire**: Cmd+N at empty drive → new tab
   `untitled.md` at drive root.
4. **Second fire**: Cmd+N when `untitled.md` exists →
   new tab `untitled-1.md`.
5. **Existence-aware**: Cmd+N when `untitled.md` +
   `untitled-1.md` exist → new tab `untitled-2.md`.
   The check considers BOTH disk files AND open
   drafts (so two consecutive Cmd+N presses don't
   collide on the same name).
6. **Save-flow**: typing in the new tab + saving
   persists to disk as `untitled-N.md`. Rename
   workflow (`PathPromptModal`) still works to rename
   the draft.

### Tests

* Vitest pin on `nextUntitledFilename` for the count-up
  cases (empty / one exists / multiple exist / drafts
  consume names).
* Vitest pin on the keymap chord → handler dispatch.
* Tauri-side: structural test that the menu accelerator
  string is `"CmdOrCtrl+Shift+N"` (catches future
  regressions; small).

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-desktop` green.
* `npm test`, `npm run check`, `npm run build` green.

## Coordination

* @@FullStackA primary. Touches one Tauri line +
  substantive SPA work.
* Standing chan-desktop runtime perm covers any
  throwaway-drive verification.
* Atomic-audit-commit discipline.

## Authorization

**Yes** for:

* `desktop/src-tauri/src/main.rs` (one-line accelerator change)
* `web/src/state/pathValidate.ts` (extend with `nextUntitledFilename`)
* `web/src/App.svelte` (or wherever the cmd-bridge lives — keymap binding)
* `web/src/state/tabs.svelte.ts` or `store.svelte.ts` (new-tab dispatch)
* Vitest + Rust test files
* Task tail + outbound

## Numbering

This is `-a-61`.

## Out of scope

* The deeper "draft files" infrastructure rework (if
  this surfaces, fire scope poke).
* Custom user-configurable default filename (Round-3
  polish).
* Multi-window state sync via Tauri (the New Window
  Cmd+Shift+N path stays as-is functionally).
* Browser-default Cmd+N suppression in web build is
  the SPA handler's job (preventDefault on keydown).
