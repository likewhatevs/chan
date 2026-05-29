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

## 2026-05-21 — ready for review

### Root cause

`@codemirror/lang-markdown@6.3.x`'s `markdown(config)` accepts
`addKeymap` (default `true`). When true the language support
prepends `markdownKeymap` at high precedence — namely
`{ key: "Enter", run: insertNewlineContinueMarkup }` plus
`Backspace: deleteMarkupBackward`. The Enter binding is what
auto-continues list markers on the next line.

The wysiwyg's grammar wrapper (`web/src/editor/markdown/grammar.ts`)
already sets `addKeymap: false` since it owns its own keymap
stack. Source mode (`web/src/editor/Source.svelte`) used the
built-in `markdown()` without that flag → default ON → list
keymap active.

### Fix

Picked spec option (a) — strip the markdown keymap from the
source-mode extension stack:

* `web/src/editor/Source.svelte` — both `markdown()` call sites
  (the synchronous `pickInitialLanguage` and the async
  `resolveLanguage`) now pass `{ addKeymap: false }`. Added a
  comment explaining why + cross-referencing the wysiwyg's
  symmetric setting.

This preserves every other CM6 affordance on source mode
(undo / redo, multi-cursor, find-in-file, copy / paste,
indentation via `indentWithTab`, default keymap actions); only
the markdown-aware Enter + Backspace handlers are gone.

The wysiwyg path is unaffected — it uses `chanMarkdown()`
which already sets `addKeymap: false` and wires its own list
behaviour via a different keymap (the `enterMarkdown` /
`tab` handlers in `editor/commands/`).

### Test pin

`web/src/editor/sourceModeListKeymap.test.ts` mounts a minimal
CM EditorView with the exact source-mode extension shape:
`keymap.of(defaultKeymap) + markdown({ addKeymap })`. Then
dispatches a synthetic Enter keystroke at end-of-line and
asserts the resulting doc.

5 pins:

1. **Default behaviour (addKeymap=true)**: `1. item` + Enter
   → `1. item\n2. ` — proves the bug exists without the fix.
   Sanity check against the lang-markdown contract; locks the
   default behaviour so a future package upgrade that changes
   semantics surfaces here.
2. **Fixed behaviour (addKeymap=false)**: `1. item` + Enter
   → `1. item\n` — just a newline.
3. **`- item` + Enter** → `- item\n` (no bullet auto-continue).
4. **`* item` + Enter** → `* item\n` (star bullet).
5. **`1) item` + Enter** → `1) item\n` (alternate ordered
   marker).

### Files touched

| File                                                    | Change                                                          |
|---------------------------------------------------------|-----------------------------------------------------------------|
| `web/src/editor/Source.svelte`                          | `markdown({ addKeymap: false })` both call sites + comment      |
| `web/src/editor/sourceModeListKeymap.test.ts`           | NEW — 5 pins on the keymap-strip behaviour                      |

### Suggested commit subject

```
Source mode: disable lang-markdown auto-list continuation keymap (fullstack-a-41)
```

### Gate

* vitest **586 / 586** (+5 in
  `sourceModeListKeymap.test.ts`).
* svelte-check 0 errors / 0 warnings across 3983 files.
* npm build clean.

### Composition

* `-a-40` (wysiwyg dotted markers) — separate component +
  pipeline; no interaction.
* `-a-26` (source-mode toggle) — provides the mode swap;
  this task just stops source mode from doing unwanted
  intervention. Pairs cleanly.

v0.11.2 mini-wave queue empty on my lane. All six tasks
(-a-36 through -a-41) ready for review.
