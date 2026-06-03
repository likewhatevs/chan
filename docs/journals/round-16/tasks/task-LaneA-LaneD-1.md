# task-LaneA-LaneD-1: SPA settings-gap check (gear removal)

From: @@LaneA (lead)  To: @@LaneD  Type: recon (read-only, no code)

## Context

The chan-desktop launcher redesign removes the per-row Settings gear. The
draft's rationale: "the removal of the SETTINGS button from here because
from now on these will only exist inside chan's SPA, not configurable from
here at all." Before we remove the gear, we must confirm the settings it
exposes are actually reachable/configurable from INSIDE the chan SPA
(web/). If any are NOT, that is a GAP the redesign must address (otherwise
we strand a setting). This is the one open design question; your finding
gates @@LaneB's design.

Draft (context): docs/journals/phase-16/desktop-redesign-draft/draft.md

## Scope (read-only)

1. Identify exactly what the launcher's per-row gear toggles. Read the
   features-panel handler in desktop/src/main.js (search for the gear /
   features-panel / the toggles it renders). Likely candidates: semantic
   search on/off, reports on/off. Confirm the real list + the Tauri
   commands/routes they call. (@@LaneC is inventorying the same handler in
   parallel; you may both read main.js read-only. If C's
   new-team-1/launcher-inventory-LaneC.md already lists the gear toggles,
   reuse it; otherwise read the source directly, do not block.)
2. For EACH setting the gear exposes, find whether the SAME setting is
   reachable/configurable from inside the chan SPA (web/). Look in web/src
   for the preferences / workspace-settings / file-browser-settings UI and
   the routes they hit (chan-server preferences/storage routes). Report, per
   setting: "in SPA at <component:line> via <route>" OR "GAP: not in SPA".

## Deliverable

new-team-1/spa-settings-gap-LaneD.md - a per-setting table: setting name |
launcher route | SPA location-or-GAP | note. No em dashes, 80-col ASCII.
If you find a GAP, state plainly what the redesign must add (or whether the
setting should simply be dropped, given chan is pre-release: no back-compat).

## Constraints

- Read-only. No code edits.

## On completion

Cut a completion task back to @@LaneA at tasks/task-LaneD-LaneA-1.md
(append-only) pointing at the finding + calling out any GAP, then poke @@LaneA.
