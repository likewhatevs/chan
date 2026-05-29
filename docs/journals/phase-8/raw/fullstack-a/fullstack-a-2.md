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

## 2026-05-19 — implementation note

The file is `web/src/components/AppStatusBar.svelte`. It
already had a `collapse` button (the "notification expand /
collapse" the spec calls out) plus three section `<button>`s
that wrapped the index / import / status pills:

* `onClickIndex` → `openSettings()` (the bug)
* `onClickImport` → `openBrowser()`
* `onClickStatus` → `ui.status = null`

Removed all three buttons, the corresponding handlers, and the
now-unused `openSettings` / `openBrowser` imports. The sections
are plain `<span>` ambient labels now; the collapse handle is
the only interactive surface left on the bar. Dropped the
`.section.btn` / hover / focus CSS that no longer has any
targets.

Watcher-dot colour swap: `.dirty.watcher` in `Pane.svelte`
previously rendered as `var(--success-text, var(--link))` —
`--success-text` isn't defined so it fell back to `--link`
(blue, the "info / idle" tone). Switched to
`var(--warn-text, #d29922)` — same token the back-attention
pulse and pane-chrome activity dot use, so attention cues share
one yellow language. Also corrected `.dirty.activity` which
referenced the misspelt `--warning-text` (no such token)
falling back to a hard-coded hex.

Acceptance criteria pass:

* Clicking the status bar outside the collapse button is now
  inert — no Settings overlay, no Files overlay, no message
  dismiss.
* Collapse / expand still toggles via the `<button>`.
* Watcher unread dot blinks yellow (warn-text) instead of blue
  (link).

Files touched:

* `web/src/components/AppStatusBar.svelte`
* `web/src/components/Pane.svelte` — `.dirty.watcher` /
  `.dirty.activity` token swap.

Pre-push gate (SPA portion): vitest 446/446 green;
`npm run check` reports 0 errors / 0 warnings; `npm run build`
clean.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Shape matches the spec: status-bar click handlers removed
(except the collapse), `--warn-text` swap for the watcher dot
unifies the yellow attention language across the chrome. The
`--success-text` / `--warning-text` correction is a nice
incidental cleanup — both tokens were fall-through dead
references.

Acceptance criteria all met. Gate green.

**Commit clearance**: approved. Commit `fullstack-a-2` as a
standalone change. Suggested subject:

```
Status bar: drop ambient click handlers; watcher dot blinks yellow (fullstack-a-2)
```

Push waits for Round-1 close commit-grouping plan.

Pick up `fullstack-a-3` next (Cmd+K cluster: status-bar label,
flashing-H removal, immediate-commit on 1/2/3).
