# fullstack-a-41: Source-mode editor list intervention (strip list keymaps)

Owner: @@FullStackA
Date: 2026-05-21

## Goal

Source mode in the editor should be RAW — no list-continuation
smarts, no auto-renumber, no bullet behaviour. The wysiwyg
mode is where rendering intelligence lives; source mode is
where the user reads / edits the raw markdown verbatim.

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "Source-code
editor mode auto-intervenes with list typing (it shouldn't)"
(filed 2026-05-21).

Today: typing `1.` + space + Enter in source mode auto-inserts
`2.` on the next line. Same for `-` / `*` bullets. Should
not. Mode toggle from `-a-26` swaps the renderer but the
input keymap retains markdown-aware list-continuation.

## Authorization

**Authorization: yes**, covers `web/src/editor/Source.svelte`
(or wherever source-mode CM6 extensions are configured) +
any related extension-stack helpers in `web/src/editor/`.

@@FullStackA may proceed without further @@Alex confirmation.

## Acceptance criteria

* In source mode: typing `1.` + space + Enter inserts a
  newline only (no auto-continued `2.` on the next line).
* Same for `- ` + Enter (no auto-bullet on next line).
* Same for `* ` + Enter, `+ ` + Enter, `1) ` + Enter (any
  list marker).
* Wysiwyg mode behaviour UNCHANGED — list continuation
  keymap still fires there.
* Other source-mode affordances PRESERVED: undo / redo,
  multi-cursor, find-in-file, copy / paste, indentation.
  Just strip the markdown-aware list keymap.
* Vitest pin: mount source-mode editor, dispatch keystroke
  sequence, assert no auto-continuation.
* Pre-push gate: clean.

## How to start

1. Grep the editor source for CM6 extension stack
   composition. Find the markdown-language extension + any
   custom list keymaps.
2. Identify whether the wysiwyg + source modes share the
   extension stack today, OR if they have separate stacks
   that happen to load the same extensions.
3. Implementation options:
   * (a) Strip the markdown-language extension's list bits
     from the source-mode stack. Source mode loads a
     reduced extension set.
   * (b) Gate every list-continuation keymap on an
     `is-wysiwyg` flag at the keymap-handler level. Same
     stack but per-mode runtime gating.
   * Recommend (a) — cleaner separation; easier to reason
     about source mode as "minimal raw editor".
4. Update source-mode mount logic accordingly.
5. Write the vitest pin.
6. Test locally in source mode: list keystrokes don't
   intervene.
7. Append commit-readiness.

## Coordination

* Independent of other v0.11.2 tasks.
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Parallelisable.
* Composes with `-a-40`'s wysiwyg dotted-numbering work —
  both touch list rendering but at different layers (this
  is keymap-stripping for source mode; -a-40 is CSS-only
  for wysiwyg).

## Open questions

(populated as you investigate)
