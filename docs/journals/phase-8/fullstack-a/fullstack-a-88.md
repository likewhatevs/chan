# fullstack-a-88 — First-boot: remove "open FB tab" rule; always boot with docked FB on the left

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Replace the first-boot UX from "open a File Browser
tab when chan opens a drive for the first time" with
"always boot with the docked File Browser on the
LEFT-hand side."

## Reference

@@Alex 2026-05-22: "we had previously created this
rule that when chan boots a drive for the first
time, we open a file browser.. we no longer need
that, and we will always do the first boot with the
docked file browser on the left hand side."

## Scope

### 1. Remove first-boot FB-tab spawning

Audit + remove whatever logic spawns a File Browser
TAB on first drive boot. Likely lives in
`store.svelte.ts` or `App.svelte`'s drive-load /
SerTab-restore path. Look for FB-tab spawn calls
that are gated on "no prior layout state" / "first
launch" / "empty SerTab".

### 2. Default docked FB to LEFT on first-boot

On first-boot (when there's no prior
`browser_side_panes` preference in
`~/.chan/preferences.toml`):

* Set `browser_side_panes.left = true` (docked FB
  visible on left).
* Set `browser_side_panes.right = false` (right
  stays empty).
* Persist this as the user's preference on first
  write so subsequent boots respect their toggle.

### 3. Preserve existing user preferences

* If the user has already configured
  `browser_side_panes` (left/right docked or both
  hidden), the boot respects their setting.
* Only the FIRST-BOOT (empty preferences) path
  changes.

## Acceptance

1. **First-boot opens with docked FB on left**: a
   brand-new drive (no `~/.chan/preferences.toml`
   OR no `browser_side_panes` in it) opens with
   the FB docked on the left, NO separate FB tab
   spawned.
2. **No FB-tab spawn on first-boot**: previously
   the first-boot spawned an FB as a tab; that
   spawn path is REMOVED.
3. **Existing user preferences respected**: if
   user has `browser_side_panes.right = true`
   from prior session, boot keeps it that way.
4. **No regression** on drive switch / reopen
   flows beyond this first-boot defaulting.

### Tests

Vitest pin on:
* First-boot (empty preferences) → docked FB on left.
* First-boot does NOT spawn an FB tab.
* Existing preferences preserved.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.

## Authorization

Yes for `web/src/state/store.svelte.ts` +
`web/src/App.svelte` (or wherever the first-boot
logic lives) + preferences default + tests + task
tail + outbound.

## Numbering

This is `-a-88`.

## Out of scope

* Re-styling the docked FB.
* Changing the FB tab behavior (open via menu /
  Cmd+O still works).
* Re-doing the carousel for empty panes.
