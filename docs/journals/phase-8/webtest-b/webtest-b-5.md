# webtest-b-5 — chan-desktop runtime walk: -b-26 (Reload + Inspector in tab menus) + -b-27 (Cmd+Shift+N accelerator)

Owner: @@WebtestB
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Walk two chan-desktop changes in one session:

1. **`-b-26`** (`77c0129`) — Reload + Open Inspector
   entries in editor tab + terminal tab right-click
   menus, reusing existing `reload_window` +
   `open_devtools` IPCs.
2. **`-b-27`** (`74bd746`) — "New Window" menu item
   moved from Cmd+N to Cmd+Shift+N (frees Cmd+N for
   future SPA New Draft handler from `-a-66`).

## Reference

* `-b-26`: [`../fullstack-b/fullstack-b-26.md`](../fullstack-b/fullstack-b-26.md)
  + scope correction in the tail.
* `-b-27`: [`../fullstack-b/fullstack-b-27.md`](../fullstack-b/fullstack-b-27.md).
* Addendum A: [`../alex/addendun-a.md`](../alex/addendum-a.md)
  for the menu-revamp context.

## Acceptance

### -b-26: Reload + Inspector in tab menus

1. **Editor tab right-click → Reload**: opens the tab
   menu; click "Reload" → window reloads via
   `reload_window` IPC.
2. **Editor tab right-click → Open Inspector**: opens
   the menu; click → DevTools open via `open_devtools`
   IPC.
3. **Terminal tab right-click → Reload**: same shape.
4. **Terminal tab right-click → Open Inspector**: same
   shape.
5. **No regression on existing entries**: "Reload from
   Disk" (editor) + "Restart" (terminal) still work
   per their existing semantics; the new entries don't
   replace them.

### -b-27: Cmd+Shift+N accelerator

6. **Cmd+Shift+N opens new window**: with chan-desktop
   focused, press Cmd+Shift+N → new chan-desktop
   window spawns.
7. **Cmd+N does NOT open a new window**: with chan-desktop
   focused, press Cmd+N → nothing happens (or whatever
   the SPA does today; `-a-66` will later wire it to
   New Draft).

### Walkthrough audit trail

Append to [`webtest-b-1.md`](webtest-b-1.md):
`## 2026-05-22 — fullstack-b-26 + fullstack-b-27 runtime walk`.
Capture verdicts + screenshots + tear-down.

## How to start

1. Confirm `77c0129` + `74bd746` in HEAD.
2. Rebuild chan-desktop (Cargo + bundle).
3. Spawn chan-desktop against a THROWAWAY DRIVE
   (per the standing safety constraint — don't disrupt
   @@Alex's running chan.app session).
4. Walk -b-26 checks (1-5) + -b-27 checks (6-7).
5. Append verdict; tear down (by-PID SIGTERM only).

## Coordination

* @@WebtestB lane.
* Standing chan-desktop runtime permission covers
  throwaway-drive spawn.
* By-PID SIGTERM only at teardown; no `pkill -f`.
* Light walk; ~20 min.

## Numbering

Highest committed `webtest-b-N` is `-4`. This is `-5`.

## Out of scope

* Re-walking `-b-25` (already 9/9 HOLD).
* The SPA New Draft handler from `-a-66` — Cmd+N's
  ultimate use case is a future task on @@FullStackA's
  lane.
* Right-click menu entries from `-a-67` (separate
  lane + task).
