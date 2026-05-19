# fullstack-a-2: Status bar click events + notification flash colour

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Two related status-bar fixes:

1. **Remove all click handlers from the status bar** except the
   one that expands / collapses the notification panel. Clicking
   anywhere else on the status bar must do nothing — currently
   clicks bleed through and open the Settings overlay.
2. **Change the notification flash colour** from blue to yellow.
   Blue reads as info / idle; yellow signals "needs your
   attention".

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): "Clicking
the notification status opens the settings overlay" and
"Notification flash colour is wrong".

## Acceptance criteria

* Clicking anything in the status bar other than the
  notification toggle is a no-op (no Settings overlay, no other
  navigation).
* Notification expand/collapse still works on click.
* The pending-notification flash uses a yellow accent colour
  (pick the existing yellow token from the theme palette;
  coordinate with @@FullStackB if a new token is needed).

## How to start

Likely files: `web/src/components/StatusBar.svelte` (or
similar). Audit every click handler bound on the status-bar
subtree; drop or no-op the ones that don't belong to the
notification toggle.
