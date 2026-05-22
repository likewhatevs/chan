# fullstack-a-90 — Remove Alt+Space legacy rich-prompt chord

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Remove the Alt+Space chord that's been bound to the
rich prompt as a legacy alias since `-a-32`. Rich
prompt now lives on Cmd+P (native) + Cmd+Alt+P (web
Mac fallback); Alt+Space is no longer needed.

## Reference

@@Alex 2026-05-22: "let's remove the alt+space
shortcut for the rich prompt".

`-a-32` promoted rich prompt to the spawn-chord
family but kept Alt+Space "for muscle memory."
That window's closed.

## Audit (call sites to remove)

* `web/src/App.svelte:618-627` — primary
  `altKey + Space` keymap branch calling
  `spawnRichPromptFromContext()`.
* `web/src/App.svelte:350` — comment line in the
  keymap documentation header.
* `web/src/components/TerminalTab.svelte:996+` —
  secondary `altKey + Space` handler in terminal
  context.
* `web/src/components/TerminalTab.svelte:1177` —
  comment reference.
* `web/src/state/shortcuts.ts:114+126` — shortcut
  registry entry mentioning Alt+Space.
* `web/src/state/tabs.svelte.ts:912` — comment
  about focus race after Alt+Space.
* `web/src/components/TerminalRichPrompt.svelte:73`
  — comment reference.

## Fix shape

1. **Delete** the two keymap handler branches
   (App.svelte + TerminalTab.svelte).
2. **Update** the registry in `shortcuts.ts` to
   drop the Alt+Space mention from the note +
   any explicit registry entry if one exists.
3. **Sweep comments** that reference Alt+Space
   as a live binding. Where the comment still
   makes sense without Alt+Space (e.g. focus-race
   pattern at tabs.svelte.ts:912 is generic to
   keymap-driven rich-prompt opens), keep the
   comment + rephrase. Where the comment was
   specifically about Alt+Space, remove.

## Acceptance

1. **Alt+Space does nothing** (or falls through
   to whatever the browser does — likely no-op).
2. **Cmd+P still opens rich prompt** (native).
3. **Cmd+Alt+P still opens rich prompt** (web Mac).
4. **Hybrid NAV `Mod+. p` still works**.
5. **No stale Alt+Space comments** as if the
   chord were live.

### Tests

Vitest pin asserting the Alt+Space branch is GONE
in App.svelte + TerminalTab.svelte + the shortcuts
registry note.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* Tiny — ~20-30 LOC removal + comment sweep.

## Authorization

Yes for the 6 listed files + tests + task tail +
outbound.

## Numbering

This is `-a-90`.

## Out of scope

* Re-architecting the other spawn chords.
* Adding new chords.
* Changing Cmd+P / Cmd+Alt+P semantics.
