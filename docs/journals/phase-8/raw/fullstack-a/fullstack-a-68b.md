# fullstack-a-68b — Hybrid Nav rename: sweep missed shortcuts.ts label (slice 1 PARTIAL closure)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Catch the `shortcuts.ts` label miss in the
Hybrid NAV → Hybrid Nav rename from `-a-68 slice 1`.
@@WebtestA's walk flagged PARTIAL.

## Reference

@@WebtestA's walk (`3328d57`) verdict: `-a-68
slice 1` PARTIAL — `shortcuts.ts` still has
"NAV" / "NaV" labels somewhere the audit-grep
missed.

## Scope

Tiny sweep:

1. `grep -in "NAV\|NaV" web/src/state/shortcuts.ts`
   (case-insensitive; expected: zero matches for
   pre-rename forms).
2. For any hit: rename to "Nav".
3. Update any test pin that asserts the old form.

## Acceptance

1. **No "NAV" / "NaV" label remnants** in
   `shortcuts.ts` (case-exact sweep).
2. **Rename doesn't break chord bindings** — the
   accelerator / handler wiring stays; only
   label text changes.
3. **Any related test pins** for the new label.

### Tests

Vitest pin asserting the new "Nav" label literal
+ absence of pre-rename forms.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Tiny ~3-5 LOC + test.

## Authorization

Yes for `web/src/state/shortcuts.ts` + test + task
tail + outbound.

## Numbering

This is `-a-68 slice 1b` (follow-up under the
`-a-68` umbrella; slice 2 transactional mode
unchanged).

## Out of scope

* `-a-68 slice 2` transactional T/O/P/G/E
  staging (separate slice).

## 2026-05-22 — ready for review (slice 1b — shortcuts.ts label sweep)

Three-file change. SPA-only.

### What landed

`web/src/state/shortcuts.ts`:
* Bulk perl swap `Hybrid NAV` → `Hybrid
  Nav` across the entire file (note
  strings + doc-comment references).
* 5 user-visible note strings updated:
  rich-prompt, files-toggle,
  graph-toggle, terminal-toggle, +
  Enter Hybrid Nav label.
* Doc-comment references in the
  registry-block header demoted too
  (lines 96, 106, 130, 145, etc.) for
  internal-consistency on a future
  audit-grep.

`web/src/state/shortcuts.test.ts`:
* Pre-existing test
  `"advertises Hybrid NAV (Cmd+.) as the
  canonical spawn surface"` updated to
  assert title-case `Enter Hybrid Nav`
  in both `web` + `native` table
  rendering.

`web/src/state/hybridNavShortcutsLabel.test.ts`
(new): 4 raw-source pins:
* No `Hybrid NAV` literal remains.
* No intermediate `Hybrid NaV` literal.
* Spot-checks on each `Mod+. <letter>
  (Hybrid Nav)` note string.
* `Enter Hybrid Nav` label literal.

### Acceptance

1. **No "NAV" / "NaV" label remnants**
   in shortcuts.ts ✓ — both case-exact
   sweep + raw-source pins.
2. **Rename doesn't break chord
   bindings** ✓ — only label/note
   strings changed; handler wiring
   untouched.
3. **Related test pins updated** ✓ —
   the existing `shortcuts.test.ts`
   table-render assertion now expects
   title-case.

### Out of scope (deferred)

App.svelte's 6 internal comment-only
`Hybrid NAV` references aren't user-
facing (they're inside `///` JSDoc /
`//` line comments documenting keymap
intent). They're harmless drift; slice
1b's task body framed this as a
shortcuts.ts label sweep specifically.
A future cleanup pass can demote them
for internal consistency.

### Gate

* vitest **1032 / 1032** (+4 net from
  `-a-81 slice 3`'s 1028).
* svelte-check 0 errors / 0 warnings
  across 4039 files.
* npm build clean.
* Rust gate not re-run (no Rust
  touched).

### Decisions

* **Bulk perl swap** — cheap + reliable
  for case-exact `Hybrid NAV` →
  `Hybrid Nav` everywhere.
* **Comment references demoted too** —
  not strictly required by the task
  body but cheap to do at the same
  time + reduces future audit-grep
  noise.
* **App.svelte comments left alone** —
  not in slice 1b's scope; flagged for
  follow-up.
* **Test pin update bundled** — pre-
  existing `shortcuts.test.ts`
  assertion was the test-suite signal
  of the rename; mandatory to keep
  green.

### Suggested commit subject

```
Hybrid NAV → Hybrid Nav: shortcuts.ts label sweep (fullstack-a-68 slice 1b)
```

Single commit. Perl sweep + test pin
update + 4 new pins on the no-remnants
guard.

### Files for `git add` (per-path discipline)

* `web/src/state/shortcuts.ts`
* `web/src/state/shortcuts.test.ts`
* `web/src/state/hybridNavShortcutsLabel.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-68b.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.

## 2026-05-22 — slice 1c (source-file comment sweep) ready for review

Ten-file change. Doc-comment + JSDoc sweep
across all non-test source files. Closes the
deferred follow-up from slice 1b.

### What landed

Bulk perl swap `Hybrid NAV` → `Hybrid Nav`
across:

* `web/src/App.svelte` (5 comment refs).
* `web/src/components/AppStatusBar.svelte`
  (2 refs).
* `web/src/components/Pane.svelte` (2
  refs).
* `web/src/components/PaneModeHelp.svelte`
  (1 ref in the component header
  comment).
* `web/src/components/TerminalTab.svelte`
  (2 refs).
* `web/src/editor/Wysiwyg.svelte` (1 ref).
* `web/src/state/spawnDialog.svelte.ts`
  (1 ref).
* `web/src/state/store.svelte.ts` (1 ref).
* `web/src/state/tabs.svelte.ts` (4 refs).
* `web/src/state/teamDialog.svelte.ts`
  (1 ref).

All edits comment-only. No runtime
behaviour changes.

### Test files preserved verbatim

Test files (`*.test.ts`) NOT touched. Two
reasons:

1. **Negative pins**: `hybridNavShortcutsLabel.test.ts`
   literally asserts the ABSENCE of "Hybrid
   NAV" in shortcuts.ts. The string must
   appear in the test file (as the
   pattern being negated) for the pin to
   make sense.
2. **Historical describe blocks**:
   `Pane.test.ts` line 484 + `tabs.test.ts`
   line 1104 use "Hybrid NAV" in describe-
   block labels that historicize `-a-44`'s
   work. Renaming would break test
   discovery + obscure the historical
   context.

### Acceptance (slice 1c)

1. **All non-test source files swept** ✓
   — 0 `Hybrid NAV` references in
   src/{components,editor,state,App.svelte}.
2. **No runtime behaviour change** ✓ —
   comment-only edits.
3. **Test pins preserved** ✓ — `*.test.ts`
   files untouched.

### Gate

* vitest **1043 / 1043** (unchanged from
  `-a-70`'s SPA wiring — no test files
  edited).
* svelte-check 0 errors / 0 warnings across
  4040 files.
* npm build clean.

### Decisions

* **Bulk perl swap** — case-exact
  consistent with slice 1 + slice 1b.
* **Test files preserved** — negative-pin
  + historical describe-block context
  needs the literal.
* **Single commit** — comment-only sweep
  across 10 files reads cleaner as one
  atomic change than 10 separate commits.

### Suggested commit subject

```
Hybrid NAV → Hybrid Nav: source-file comment sweep (fullstack-a-68 slice 1c)
```

Single commit. 10 source files + this
impl note + journal + outbound.

### Files for `git add` (per-path discipline)

* `web/src/App.svelte`
* `web/src/components/AppStatusBar.svelte`
* `web/src/components/Pane.svelte`
* `web/src/components/PaneModeHelp.svelte`
* `web/src/components/TerminalTab.svelte`
* `web/src/editor/Wysiwyg.svelte`
* `web/src/state/spawnDialog.svelte.ts`
* `web/src/state/store.svelte.ts`
* `web/src/state/tabs.svelte.ts`
* `web/src/state/teamDialog.svelte.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-68b.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
