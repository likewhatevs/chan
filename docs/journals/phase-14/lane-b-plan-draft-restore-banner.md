# Lane B plan - false "unsaved changes from a previous session" banner

Phase 14, round 2 correctness item, pulled forward (live user-facing
bug). Primary owner: **Lane B** (frontend fix). **Lane A** owns the
backend half of the end-to-end stress test. Cross-referenced from
`lane-a-request.md` and `lane-b-request.md`.

## Symptom

The editor shows "Unsaved changes from a previous session were found.
[Restore] [Discard]" even when the user is the only editor, editing
their own draft, with no external edits. Flagged in phase-13 round 1;
still misfires.

This is the localStorage hang-recovery banner, NOT the watcher
"changed on disk" path. Gated by `{#if recoveredBuffer}`
(`web/src/components/FileEditorTab.svelte:~728`), where
`recoveredBuffer = divergentBufferOrNull(tab.path, tab.path, disk)` at
mount (`:~177-191`).

## Root cause (investigated)

`web/src/state/editorBuffer.ts` persists unsaved content to
localStorage (`chan:editor-buffer:<path>`, 500ms debounce) for
force-reload recovery. `divergentBufferOrNull` (`:272-285`) returns the
buffer (-> banner) whenever `buf.content !== diskContent`. It has:

- **no session identity** - the entry is `{content, updatedAt, path}`
  (`:52-65`), no marker of WHICH page-load wrote it, so a buffer from
  the CURRENT live session (your own edits past the 500ms debounce) is
  indistinguishable from a crashed prior session's and reads as "from a
  previous session".
- **no age / mtime guard** - a buffer older than the last on-disk save
  still counts as divergent.
- **sticky on save** - FileEditorTab's persistence effect only clears
  the buffer when `content === saved` AND the banner is not already
  showing (early `return` while `recoveredBuffer !== null`), so once the
  banner appears it persists across saves.

For drafts this is the common case (seed `"# Draft\n"` on disk; typing
writes a divergent buffer; remount surfaces the banner). Adjacent but
separate: "Broken draft ... missing draft.md" / "reindexing Drafts/..."
come from `chan-workspace` draft preflight
(`crates/chan-workspace/src/drafts.rs`); fix only if it also feeds the
banner.

## Lane B fix (frontend)

1. **Session identity.** Add a per-page-load `sessionId` (module const,
   generated once) to the persisted entry. `divergentBufferOrNull`
   treats a CURRENT-session buffer as live (no banner; content already
   loaded / being edited); only a DIFFERENT-session buffer surfaces the
   banner. File: `web/src/state/editorBuffer.ts`.
2. **Age / mtime guard.** Discard a buffer older than the tab's
   last-saved mtime. The server returns `mtime_ns` on read + write
   (`crates/chan-server/src/routes/files.rs WriteResponse`); thread it
   onto the tab (`savedMtimeNs` in `web/src/state/tabs.svelte.ts`) and
   pass to `divergentBufferOrNull`.
3. **Reliable clear on save.** Drop the "don't clear while the banner is
   showing" stickiness in `FileEditorTab.svelte` (`:~238-248`); on save
   success / discard / clean transition, cancel the pending write and
   `clearEditorBuffer(path)` unconditionally, then re-evaluate.
4. **Draft seed.** A pristine / freshly-seeded draft must not write a
   divergent buffer (seed == disk -> no divergence; verify drafts use
   the same clean path).

Net: own current-session edits never raise the banner; a genuine
crashed prior session still recovers; external edits stay the watcher
path (`self_writes.rs` + `bus.rs`, already correct - 1500ms own-write
suppression).

## End-to-end stress test

- **Lane B (vitest).** Extend `web/src/state/editorBuffer.test.ts` and
  add a FileEditorTab/store lifecycle test: loop create -> type ->
  autosave -> save -> remount; assert `recoveredBuffer` is null for
  current-session and saved content; fires ONLY for a different
  `sessionId` AND newer-than-saved-mtime; assert clear-on-save. Build on
  the `divergentBufferOrNull` test (`:129`) + watch-event tests
  (`store.test.ts:419-482`).
- **Lane A (Rust integration).** New chan-server/chan-workspace stress
  test (reuse `crates/chan-workspace/tests/` harness e.g. `smoke.rs`,
  the draft unit tests `routes/drafts.rs:228`): hammer create-draft ->
  write/autosave (`/api/files` with `expected_mtime_ns` CAS) -> re-read,
  many iterations; assert self-write suppression holds (own writes never
  broadcast as external via `bus.rs`/`self_writes.rs`), CAS mtime
  round-trips, no spurious `DraftBroken` / "missing draft.md".
- Gates: `cd web && npm test`; `cargo test` (chan-server, chan-workspace).
  A true in-browser cross-process e2e is beyond the current harness;
  flag if a Playwright-style driver is wanted (larger).

## Verification

- Stress tests reproduce the false banner BEFORE the fix, pass after;
  web vitest + cargo test green.
- Manual (run the app): open a draft, type, save -> NO banner; type
  across a save -> NO banner; force-reload mid-edit -> banner once
  (genuine recovery), Restore/Discard both work; normal reload after a
  clean save -> NO banner.
