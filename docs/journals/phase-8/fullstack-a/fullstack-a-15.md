# fullstack-a-15: "New file" dialog double-appends .md extension

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Stop the "New file" dialog from appending `.md` when the
typed filename already ends in `.md` (or any markdown-
recognised extension). Typing `foo.md` should create
`foo.md` on disk, not `foo.md.md`.

## Background

Side observation from @@WebtestA's Round-1 sweep on
2026-05-20:

> "New file" dialog appends `.md` even if the typed name
> already ends in `.md` (`foo.md` → `foo.md.md` on disk).

Filed in [`../phase-8-bugs.md`](../phase-8-bugs.md). Small
UX papercut; user-visible because the file shows up in the
file browser with the doubled extension.

## Acceptance criteria

* Typing `foo` → creates `foo.md` (today's behaviour, no
  change).
* Typing `foo.md` → creates `foo.md` (NOT `foo.md.md`).
* Typing `foo.markdown` → creates `foo.markdown` (also a
  markdown-recognised extension; or `foo.markdown.md` if
  that's the safer convention — propose a decision in the
  first append if it isn't obvious from the existing
  filename-recognition utilities).
* Typing a non-markdown extension (e.g. `foo.txt`) — current
  behavior preserved, whatever that is. Document the
  decision in the task tail if it surfaces a separate
  question.

## How to start

1. Find the "New file" dialog handler. Likely in
   `web/src/components/FileTree.svelte` or a sibling
   dialog component.
2. The append happens somewhere along the input → submit
   path. Look for a string concat with `.md` or a regex
   replace.
3. Guard the append: `if (!name.endsWith(".md")) name += ".md"`
   or the equivalent in whatever shape the codebase prefers.
   Re-use the existing markdown-extension recognizer if
   chan-drive exposes one (don't roll a fresh `endsWith`
   chain).
4. Pin with a small test in the dialog's test file if one
   exists; otherwise visual verification on @@WebtestA's
   lane-A test server is sufficient.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.

## 2026-05-20 — implementation note

Root cause was NOT in `appendDefaultMd`. That helper is correctly
idempotent (`appendDefaultMd("foo.md")` returns `"foo.md"`, both
at the modal layer and the store layer). The doubling happened
one layer up, in `PathPromptModal.svelte`'s open-time selection
rule.

When the new-file modal opens with default `untitled.md`, it
focuses the input and calls `setSelectionRange(stemStart,
stemStart + DEFAULT_NEW_FILENAME_STEM.length)` — i.e. it selects
ONLY the stem `untitled`, leaving the `.md` suffix unselected.
The intent (per the existing comment) was "type to replace the
stem, hit Enter to accept as-is". But the unselected `.md`
remains in the field, so if the user types a name that
includes the extension (`foo.md`), the typed text replaces
the stem and the field ends up as `foo.md.md`. `appendDefaultMd`
sees a real extension at position 6 (the last dot) and returns
the value unchanged — there's nothing in the chain to collapse
the duplicated suffix.

Fix: extend the selection to `pathPromptState.defaultValue.length`
so it covers the whole filename including the `.md`. Typing now
replaces stem+extension together. Behaviours:

* Type `foo` → field becomes `foo` (no extension typed) →
  `appendDefaultMd` adds `.md` → `foo.md` on disk. Same as before.
* Type `foo.md` → field becomes `foo.md` → `appendDefaultMd`
  idempotent → `foo.md` on disk. Bug fixed.
* Type `foo.txt` → field becomes `foo.txt` → modal's
  `isEditableText` validator runs against the resolved path
  (already validated `.txt` as editable today) → `foo.txt` on
  disk. Unchanged.
* Hit Enter without typing → field still `untitled.md`
  (the whole filename is selected but unchanged) →
  submits `untitled.md`. Unchanged.

The directory prefix (everything before the trailing `/`)
stays outside the selection, so Tab-completed parents survive
a single-keystroke replace.

Files touched:

* `web/src/components/PathPromptModal.svelte` — selection now
  covers stem + extension instead of just stem; comment refreshed
  to record the rationale.

Pre-push gate (SPA portion): vitest 480/480 green
(FullStackB's `fullstack-b-8` + others added 5 new tests since
my last gate run; all pass alongside mine);
`npm run check` 0 errors / 0 warnings; `npm run build` clean.

To verify on the lane-A server (post-restart): open the new
file dialog (Ctrl+Alt+N or the file-browser plus button), type
`foo.md` instead of `foo`, hit Enter. File appears in the tree
as `foo.md` (not `foo.md.md`). Then test the non-extension
case: open dialog, type `bar`, Enter → `bar.md` on disk.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Right fix in the right layer. The bug was a selection-range
issue, not an extension-appender issue — extending the
selection to cover the full `untitled.md` default value
means a typed `foo.md` cleanly replaces the entire filename
in one keystroke instead of leaving the stale `.md` suffix
behind. The directory-prefix-stays-out-of-selection
discipline preserves Tab-completed parent dirs. All four
behaviour cases enumerated and verified consistent.

Single-file change, pre-push gate green.

**Commit clearance**: approved. Suggested commit subject:

```
New file dialog: select stem + .md so typed names with extension don't double-append (fullstack-a-15)
```

Push waits for Round-1 close.