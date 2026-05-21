# fullstack-a-45 — Terminal Settings migration to Hybrid Terminal back (Task B)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 lands in HEAD)

## Goal

Migrate the Terminal section out of `SettingsPanel.svelte`
into the new `HybridTerminalConfig.svelte` mount point
introduced by `-a-43` (Task A).

## Background

Locked design:
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited". Task A
(`fullstack-a-43`) introduced the four
`Hybrid{Terminal,Editor,Graph,FileBrowser}Config.svelte`
components with empty body placeholders. Task B
populates Terminal.

Scope of the migration (settings that move):

* Scrollback buffer (MB) — from `-b-11`.
* Default TERM value — from `-b-11`.
* Any future font controls (parked).

Settings storage shape is unchanged. Only the
mounting point of the UI moves.

## Acceptance criteria

* Terminal section in `SettingsPanel.svelte` is
  removed; the same settings render inside
  `HybridTerminalConfig.svelte` instead.
* Warning copy added: "These settings apply to ALL
  terminals, not just this one." (Or similar; aligns
  with the round-2-plan Hybrid back-side scope note
  that per-type settings apply per-type, not per-tab.)
* Tests cover: settings persist across reload, the
  underlying `Preferences` shape is unchanged, the
  values bind to the new mount point correctly.
* Pre-push gate green: fmt + clippy + cargo test +
  svelte-check + npm build + vitest.

## How to start

1. Audit current Terminal section in
   `SettingsPanel.svelte` to inventory what moves.
2. Move the section into `HybridTerminalConfig.svelte`
   (the empty body placeholder introduced by `-a-43`).
3. Remove the corresponding section from
   `SettingsPanel.svelte`.
4. Add the "applies to all terminals" warning copy.
5. Wire tests + verify gate.

## Coordination

* SPA-only.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereq

Depends on
[`fullstack-a-43`](fullstack-a-43.md) landing in HEAD.
Task A introduces `HybridTerminalConfig.svelte` as an
empty body; Task B populates it.

## Numbering

Highest committed `-a-N` is `-a-41`; `-a-42` is About,
`-a-43` is Task A, `-a-44` is drag-to-rearrange; this
is `-a-45`. Task C (`-a-46`), Task E (`-a-47`), Task F
(`-a-48`) fan out alongside.
