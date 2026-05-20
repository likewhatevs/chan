# fullstack-b-10: Watcher dialog "overwrites existing" warning still shows — call site not switched to attach mode

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Complete the `fullstack-b-3` watcher-dialog fix. The
backend resolver + the new `PathPromptMode = "attach"`
branches in `PathPromptModal.svelte` are live, but the
only caller — `TerminalRichPrompt.svelte:197` — still
passes `mode: "move"` to `uiPathPrompt`. Result: the
misleading `⚠ overwrites existing directory <name>/`
warning is still shown to the user even though the new
attach-mode branches exist behind it. Flip the call-site
to `mode: "attach"` and update the hint copy.

## Background

@@WebtestB's wave-1 verification on 2026-05-20 flagged
the partial fix:

> `fullstack-b-3` (watcher dialog) — **partial fix**.
> Backend `resolve_watcher_dir` works correctly: outside-
> drive absolute paths accepted, missing dirs silently
> created (validated on disk both inside and outside the
> drive root). But the dialog still shows
> `⚠ overwrites existing directory docs/` for an existing
> in-drive dir because the call site was not switched:
> `TerminalRichPrompt.svelte:197` still passes
> `mode: "move"` to `uiPathPrompt`. The new
> `PathPromptMode = "attach"` and the `mode === "attach"`
> branches in `PathPromptModal.svelte` (lines 250 / 264 /
> 290 / 337 / 517) are live but never reached for the
> only caller that needed them.

Source-side fix is genuinely one-line plus a hint-copy
update. @@WebtestB's recommended fix: flip the call site
+ switch the hint from "moves to X/" to "attach watcher
to X/".

This is the cleanup follow-up to `fullstack-b-3`
(committed at `a9579f0`). Land as a new commit, do not
amend the prior one.

## Acceptance criteria

* `TerminalRichPrompt.svelte:197` passes `mode: "attach"`
  to `uiPathPrompt`.
* Hint copy adjacent to the dialog reads "attach watcher
  to X/" (or whatever phrasing matches the new mode's
  intent — propose in the first append if the existing
  "move" hint can be repurposed cleanly).
* Selecting an existing in-drive directory in the watcher
  dialog no longer shows the
  `⚠ overwrites existing directory <name>/` warning.
* No regression on the backend resolver
  (`resolve_watcher_dir`) behavior — outside-drive paths
  still accepted, missing dirs still silently created.
* @@WebtestB's three repros now all pass cleanly: in-drive
  existing dir, in-drive missing dir, outside-drive
  absolute path.

## How to start

1. Open `web/src/components/TerminalRichPrompt.svelte`.
   Line 197 is the call site @@WebtestB identified.
2. Flip `mode: "move"` → `mode: "attach"`.
3. Update the hint copy (likely a string template
   adjacent in the same file).
4. Verify on @@WebtestB's lane-B server
   (`127.0.0.1:8820`, drive `/tmp/chan-test-phase8-wb`)
   that the three watcher-dialog cases all pass.

## Coordination

* @@WebtestB verifies on lane-B drive once landed.

## 2026-05-20 - call-site flip

Genuinely one line of substantive change in
`TerminalRichPrompt.svelte::watchDirectory`:
`mode: "move"` → `mode: "attach"`. The
`PathPromptModal::status` derivation already keys the visible
label on `status.mode`, so no separate hint-copy edit is needed
in the modal — line 518's `"attach watcher to"` branch picks
up the new mode automatically. The mode flip propagates through
every modal-internal branch landed in `fullstack-b-3`:

* `pathPromptState.mode === "attach"` skips the
  `overwrites existing directory` warning when the target
  already exists.
* Absolute paths bypass the fabricated ancestor preamble (the
  SPA cannot see the OS filesystem; the backend creates on
  demand).
* `tailIsExisting` demotes the final segment so the visible
  chunk reads as context, not as a fresh-create cue.

Hint copy in the modal already reads "attach watcher to X/"
in attach mode (per `fullstack-b-3`'s landed branch). No
additional copy update required.

I added a small block comment in `TerminalRichPrompt.svelte`
explaining why the call site uses `attach` instead of `move`,
so a future reader doesn't have to chase the `PathPromptMode`
union to understand why the watcher dialog differs from the
file-move dialog. The new comment is the only other line
change in the file.

Tests:

* `web/src/components/PathPromptModal.test.ts` — new
  describe block under `fullstack-b-10` adds a `?raw` import
  of TerminalRichPrompt and asserts:
  * `watchDirectory` passes `mode: "attach"` to
    `uiPathPrompt`.
  * The deprecated `mode: "move"` does NOT appear inside the
    `watchDirectory` block.
  This pins the call site to the new mode and trips any
  refactor that drops the contract.

Pre-push gate green:
* `cargo fmt --all -- --check` — clean.
* `cargo clippy --workspace --all-targets -- -D warnings` — clean.
* `cargo test --workspace` — every suite passes.
* `cargo build --workspace --no-default-features` — clean.
* `npm run check` (svelte-check) — 0 errors, 0 warnings.
* `npx vitest run` — 480/480 (was 479 baseline from -9; +1
  new test). Two pre-existing terminal-tab tests still flake
  under parallel load but pass standalone (called out in
  @@FullStackB earlier journal entries); not from this fix.
* `npm run build` — clean.

## 2026-05-20 - commit readiness

Files changed (proposed single commit):

* `web/src/components/TerminalRichPrompt.svelte` — call site
  flip from `mode: "move"` to `mode: "attach"` + a short
  explanatory comment.
* `web/src/components/PathPromptModal.test.ts` — new test
  block pinning the call site contract.

Tests run: full pre-push gate green (see implementation note).

Known risks: none beyond the call-site behaviour swap. The
backend resolver, the modal branches, and the hint-copy
mapping are all already in tree from `fullstack-b-3`. This
fix flips the last switch.

Push waits for Round-1 close per the standing rule.

Proposed commit subject:
`Watcher dialog: flip TerminalRichPrompt call site to mode "attach" (fullstack-b-10)`