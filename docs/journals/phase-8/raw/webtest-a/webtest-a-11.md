# webtest-a-11 — Bundled walk: -a-64 CRITICAL tab switch focus + -a-65 editor bug bundle

Owner: @@WebtestA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two recent landings, both editor-surface; one CRITICAL:

1. **`-a-64`** (`ba0c754`) — **CRITICAL** tab switch
   chord focus pulse. Cmd+Shift+[/] follows the
   focus; data damage closed.
2. **`-a-65`** — editor bug bundle (right-click
   whole-line + image-as-text on tab switch + new-dir
   cursor at end).

## Reference

* `-a-64` task body + commit `ba0c754`.
* `-a-65` task body + (commit imminent — see worktree).

## Acceptance

### -a-64 CRITICAL (data-damage closure)

1. **Cmd+Shift+] editor → terminal**: type
   immediately — keystrokes land in terminal PTY.
   No editor damage.
2. **Cmd+Shift+[ terminal → editor**: type
   immediately — keystrokes land in editor doc.
3. **Paste-buffer test**: copy text in editor;
   Cmd+Shift+] → terminal; Cmd+V → paste lands in
   terminal, NOT in editor.

### -a-65 — 3 editor bugs

4. **Right-click no-select**: right-click on editor
   doc — menu opens WITHOUT selecting a line.
5. **Image re-render after tab switch**: editor with
   image → switch to terminal → switch back; image
   renders correctly without needing a cursor poke.
6. **New Directory dialog cursor at end**: open
   New Directory dialog (FB selection menu → New
   Folder); cursor sits at END of pre-populated path,
   NOT select-all.

### Walkthrough audit trail

Append to [`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-22 — fullstack-a-64 (CRITICAL) + fullstack-a-65 bundled walk`.

## How to start

1. Confirm `ba0c754` (and `-a-65` commit when it
   lands) in HEAD.
2. Rebuild chan (web/dist stale); spin up test server.
3. Walk -a-64 checks (1-3) — CRITICAL; focus on
   paste-buffer test (#3) since that's the data-damage
   trigger.
4. Walk -a-65 checks (4-6).
5. Append verdict; tear down.

## Coordination

* @@WebtestA lane.
* Standing terminal + Chrome MCP perms.
* Light-medium walk; ~25 min.

## Numbering

This is `-11`.

## Out of scope

* `-a-66` (depends on `systacean-25` lifting; future
  walk).
* `-a-67` right-click menu revamp (substantial future
  walk).
* `-a-59` chan-desktop window-focus mechanic (lane-B
  scope).
