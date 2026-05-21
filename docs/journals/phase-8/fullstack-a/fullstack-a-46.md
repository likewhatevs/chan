# fullstack-a-46 — Editor Settings migration to Hybrid Editor back (Task C)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 lands in HEAD)

## Goal

Migrate the Editor section out of `SettingsPanel.svelte`
into the new `HybridEditorConfig.svelte` mount point
introduced by `-a-43` (Task A).

## Background

Locked design:
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited".

Scope of the migration (settings that move):

* Theme (per-Hybrid, surviving the per-Hybrid override
  from `-b-5`).
* Layout settings.
* Date Pills toggle.
* On Save toggle (from `-a-25`).

Settings storage shape is unchanged. Only the
mounting point moves.

## Acceptance criteria

* Editor section in `SettingsPanel.svelte` is removed;
  the same settings render inside
  `HybridEditorConfig.svelte` instead.
* Settings persist across reload via the existing
  `Preferences` fields.
* Tests cover the new mount point + persistence.
* Pre-push gate green.

## How to start

1. Audit current Editor section in
   `SettingsPanel.svelte`.
2. Move into `HybridEditorConfig.svelte`.
3. Remove from `SettingsPanel.svelte`.
4. Wire tests.

## Coordination

* SPA-only.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereq

Depends on
[`fullstack-a-43`](fullstack-a-43.md) landing in HEAD.

## Numbering

This is `-a-46`. See `-a-45` for the broader wave
numbering note.
