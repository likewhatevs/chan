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

## 2026-05-22 — ready for review

Seven-file change. SPA-only.

### What landed

`web/src/App.svelte`:
* Removed the `altKey + Space` keymap branch
  (line ~618) that called
  `spawnRichPromptFromContext()`.
* Removed the `Alt+Space -> Rich Prompt (legacy
  alias)` line from the keymap doc header
  (~line 350).
* Replacement comment cross-references
  `-a-90` + the still-live entry points.

`web/src/components/TerminalTab.svelte`:
* Removed the secondary `altKey + Space`
  handler (line ~996) that called
  `openRichPrompt()`.
* Updated the hamburger-menu comment (~line
  1175) — no longer claims Alt+Space "still
  works".

`web/src/state/shortcuts.ts`:
* Registry block comment swapped from
  "Alt+Space chord stays bound ... muscle
  memory" to "`fullstack-a-90` retired the
  legacy Alt+Space alias the migration kept".
* Registry note string dropped the trailing
  `; legacy Alt+Space alias still bound`.

`web/src/components/TerminalRichPrompt.svelte`:
* Auto-focus rationale comment swapped
  `Cmd+K p / Alt+Space` → `Cmd+K p / Cmd+P`
  in the focus-race explanation.

`web/src/state/tabs.svelte.ts`:
* Focus-race comment swapped `immediately
  after Alt+Space` → `immediately after the
  rich-prompt chord (Cmd+P / Cmd+Alt+P /
  Hybrid NAV \`p\`)`.

`web/src/state/tabs.test.ts`:
* Mirror update to the test comment so the
  race description matches the live chord
  set.

`web/src/components/richPromptAutoFocus.test.ts`:
* Mirror update to the doc-comment.

`web/src/state/altSpaceRichPromptRemoved.test.ts`
(new): 8 raw-source pins:
* App.svelte `altKey + Space` branch gone.
* App.svelte rationale comment present.
* App.svelte keymap doc-header entry gone.
* TerminalTab.svelte secondary handler gone.
* TerminalTab.svelte rationale comment.
* TerminalTab.svelte hamburger-menu comment
  no longer claims Alt+Space live.
* shortcuts.ts registry note no longer
  mentions Alt+Space.
* shortcuts.ts retire comment present.

### Acceptance

1. **Alt+Space does nothing** (or browser
   no-op) ✓ — both keymap handlers removed
   (pinned by tests).
2. **Cmd+P opens rich prompt** ✓ — untouched
   spawn-chord family entry.
3. **Cmd+Alt+P opens rich prompt (web Mac)**
   ✓ — untouched.
4. **`Mod+. p` Hybrid NAV** ✓ — untouched.
5. **No stale Alt+Space comments as if the
   chord were live** ✓ — sweep covered all
   call sites + supporting comments.

### Gate

* vitest **968 / 968** (+8 net from `-a-66`
  slice c follow-up's 960).
* svelte-check 0 errors / 0 warnings across
  4032 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Replacement comments cite `-a-90`** at
  both call sites so a future audit can find
  the retire without git blame.
* **Test comments updated** rather than left
  as historical — the race description
  matches the live chord set; otherwise the
  comment would read as if Alt+Space were
  the trigger.
* **Did NOT change Cmd+P / Cmd+Alt+P semantics**
  per the task body's out-of-scope clause.

### Suggested commit subject

```
Rich prompt: remove legacy Alt+Space chord (fullstack-a-90)
```

Single commit. Keymap handler removal +
comment sweep + test pins.

### Files for `git add` (per-path discipline)

* `web/src/App.svelte`
* `web/src/components/TerminalTab.svelte`
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/state/shortcuts.ts`
* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`
* `web/src/components/richPromptAutoFocus.test.ts`
* `web/src/state/altSpaceRichPromptRemoved.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-90.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
