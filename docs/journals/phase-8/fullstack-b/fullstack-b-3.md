# fullstack-b-3: Watcher dialog cluster (out-of-root paths + create-dir UX)

Owner: @@FullStackB
Date: 2026-05-19

## Goal

Two related watcher-dialog UX fixes:

1. **Accept paths outside the drive root.** Today the watcher
   dialog reuses the drive-scoped path picker and rejects
   anything outside the drive (e.g. `/tmp/...`). Event files are
   infra traffic, not user content — per phase-7 architecture
   they go through `tokio::fs` in the event-reply endpoint, not
   `chan_drive::Drive::write_text`. The picker should accept
   arbitrary filesystem paths.
2. **Fix the create-dir flow.** Today:
   * Non-existent path → error rather than creating it.
   * Existing path → warns about "overwrite", which is incorrect
     (attaching a watcher is read-only).
   The correct behaviour: if missing → create silently (or with
   a single, focused confirm); if existing → just attach without
   any overwrite warning.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under
"Watcher dir picker is over-restricted" and "Watcher dialog
'create dir' flow is wrong".

Diagnosis from the phase-8 smoke test (2026-05-19): Alex tried
`/tmp/chan-watcher-test/`, dialog rejected it; switched to
`./tmp`, dialog erred on missing, then warned about "overwrite"
when the dir existed.

## Acceptance criteria

* Watcher dialog accepts absolute paths anywhere on the
  filesystem (subject to OS permissions). Optional UI hint when
  the user picks a path outside the drive root, but not a hard
  reject.
* Missing path → dialog creates it silently or with a single
  "create folder?" confirm; no error state.
* Existing path → dialog attaches the watcher without any
  "overwrite" wording. Attaching is read-only.

## How to start

* Path-picker code under `web/src/` — find the watcher-set
  dialog component.
* The dialog likely reuses the "new file" picker that's gated by
  the drive sandbox. Split the two flows OR pass a flag to the
  picker that disables the sandbox for the watcher case.
* chan-server side: confirm the watcher attach endpoint already
  accepts any path (it should, per
  `crates/chan-server/src/event_watcher.rs::start`).

## 2026-05-19 - Implementation landed (pre-commit)

Diagnosed both issues to a single layered cause: the path prompt
modal reused the `move` mode (which renders the "overwrite
existing dir" warning), and the chan-server resolver enforced
`drive_root` containment + required the dir to already exist on
disk.

Files changed:

* `crates/chan-server/src/routes/terminal.rs::resolve_watcher_dir` —
  dropped the `path_canon.starts_with(root_canon)` gate for
  absolute inputs; event files are infra traffic (per the
  phase-7 event protocol they go through `tokio::fs` directly,
  not `chan_drive::Drive::write_text`), so the drive sandbox
  doesn't apply. Added `std::fs::create_dir_all(&abs)` so a
  missing watcher dir is created silently on attach. Drive-
  relative inputs still go through `resolve_safe_strict` so the
  in-drive symlink-escape protection is preserved on that path.
* Test deltas in the same file:
  - new `resolve_watcher_dir_allows_absolute_outside_drive_root`
    proves a `/tmp/...` path is accepted.
  - new `resolve_watcher_dir_creates_missing_path` proves the
    silent create works for both drive-relative and absolute
    missing paths.
  - the previously-named
    `resolve_watcher_dir_rejects_absolute_symlink_escape` is now
    `resolve_watcher_dir_absolute_symlink_accepts_target`
    (intentional behaviour change; symlink targets outside the
    drive are first-class now).
  - the old "rejects_empty_escape_and_files" test trimmed: empty
    + drive-relative `..` escape + file-not-directory all still
    rejected, but the "absolute outside the drive" assertion was
    removed since the new behaviour is the opposite.
* `web/src/state/store.svelte.ts` — added `"attach"` to
  `PathPromptMode`. Documented why this differs from create /
  move.
* `web/src/components/PathPromptModal.svelte`:
  - `status` derivation: existing directory in `attach` mode →
    treat as a normal attach (no overwrite warning, no ancestor
    chain).
  - Absolute paths in `attach` mode → suppress the ancestor
    preamble (the SPA can't see the OS filesystem; the backend
    creates if missing). Status row reads "attach watcher to
    /tmp/foo/" without claiming "creates directories tmp/".
  - `pathSegments` final-segment colouring: in attach mode, a
    final segment that's already in the tree no longer renders
    in the mint-green "new" colour.
  - Template: new `attach watcher to` branch in the status row.
* `web/src/components/TerminalRichPrompt.svelte` — watcher
  dialog now passes `mode: "attach"` (was `mode: "move"`,
  `allowAbsolute: true` already there).
* `web/src/components/PathPromptModal.test.ts` (new) — pins the
  attach-mode branches in source so a future refactor that
  reverts the watcher path can't sneak through.

Acceptance criteria status:

| Criterion                                          | Status |
|----------------------------------------------------|--------|
| Accept absolute paths anywhere on filesystem       | done   |
| Optional "outside drive root" hint                 | n/a [^1]|
| Missing path → create silently                     | done   |
| Existing path → attach without "overwrite" wording | done   |

[^1]: The modal no longer manufactures a "creates a/, b/, c/"
      preamble for absolute paths (suppressed in attach mode).
      A literal "outside drive root" hint felt redundant once
      the path is rendered verbatim in the status row — the
      user already sees that they typed an absolute path. Easy
      to add later if @@Alex wants it explicit.

Gate status:

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` (RUSTFLAGS=-D
  warnings) — clean.
* `cargo test --all-targets` — 5 new tests pass; one pre-existing
  flake in `routes::graph::tests::link_to_non_markdown_disk_file_resolves_to_real_file`
  surfaced on a parallel run but passes in isolation (unrelated
  to this task).
* `cargo build --no-default-features` (RUSTFLAGS=-D warnings) —
  green.
* `npm run check` — 0 errors, 0 warnings.
* `npm run build` (vite) — green.
* `npx vitest run` — 456/456 green (clean run).

Test plan @@WebtestB can pick up:

1. Open a terminal in chan-desktop; press Alt+Space to open the
   rich prompt; pick "watch directory".
2. Type `/tmp/chan-watcher-test/` (a path that doesn't exist
   yet). Expect: status row says "attach watcher to
   /tmp/chan-watcher-test/" — no "creates ancestors" warning,
   no "overwrites" warning. Submit succeeds; check the path
   exists on disk afterwards.
3. Repeat with an existing directory inside the drive (e.g.
   `notes/`). Expect: status row says "attach watcher to
   notes/" with the trailing-slash segment in muted grey (not
   mint-green); no "overwrites existing directory" warning.
4. Drop an event JSON in `/tmp/chan-watcher-test/`; verify the
   rich-prompt bubble overlay reflects it.

Held for commit clearance from @@Architect. Moving on to
`fullstack-b-4` (indexing-chart pan/zoom parity).

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Diagnosis is clean: drive-root containment + missing-path
rejection were the two layers, both shed. The new `"attach"`
PathPromptMode is the right split — the modal stays sandboxed
for `create` / `move` (where the drive-text gate matters) and
relaxes only for the watcher case (infra traffic, per phase-7
event-protocol architecture). The `silent create_dir_all`
matches the spec.

Symlink-target test rename (rejects → accepts) is intentional
behaviour change — flagged in the task body, audit trail
preserved. `PathPromptModal.test.ts` pin protects against a
future refactor reverting the watcher mode.

The pre-existing `routes::graph::link_to_non_markdown_disk_file_resolves_to_real_file`
flake under parallel run is unrelated; @@Systacean owns the
fix for that test elsewhere — note that `systacean-2` actually
adds that exact test, so the flake should disappear once `-2`
lands.

**Commit clearance**: approved. Suggested subject:

```
Watcher dialog: accept any path + create-if-missing + drop overwrite warning (fullstack-b-3)
```

Push waits for Round-1 close. Pick up `fullstack-b-4` next
(indexing-chart pan/zoom).
