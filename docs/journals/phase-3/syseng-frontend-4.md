# syseng-frontend-4: Settings layout standard/compact frontend wiring

Owner: @@Syseng.

Status: REVIEW.

Related:

- [journal.md](./journal.md)
- [frontend-1.md](./frontend-1.md)
- [backend-3.md](./backend-3.md)
- [webtest-1.md](./webtest-1.md)
- [webtest-2.md](./webtest-2.md)

## Role

Frontend implementation/support lane. Load the frontend/webdev skill before
editing.

## Goal

Wire the frontend Settings / Layout UI to the backend-3 `standard | compact`
LineSpacing contract.

## Context

[backend-3.md](./backend-3.md) is REVIEW and changed the backend canonical
values from `tight | standard` to `standard | compact`, with legacy `tight`
accepted as `compact`. [frontend-1.md](./frontend-1.md) says this is the last
remaining Settings/Layout item.

## Acceptance criteria

- Settings / Layout shows `[standard] [compact]`, not `[tight] [standard]`.
- Standard is the frontend fallback/default.
- Compact line-height lands between old tight and old standard:
  - Wysiwyg suggested value: `1.65`.
  - Source suggested value: `1.55`.
- API/types accept the new backend values. A tolerant read union may keep
  legacy `tight` if needed, but writes should use canonical `standard |
  compact`.
- Existing preference loading does not break if an old in-memory or persisted
  value still says `tight`.

## Test expectations

- `cd web && npm run check`.
- `cd web && npm test -- --run` or focused tests if a helper is changed.
- Browser smoke through @@Webtest / @@WebtestB: Settings layout radio, default
  selection, compact density, and reload persistence.

## Boundaries

- Keep this scoped to frontend layout/density wiring.
- Do not change backend enum/CLI behavior; backend-3 owns that and is REVIEW.
- Do not restart webtest services.

## Progress notes

- 2026-05-16 @@Syseng: Started. Loaded webdev skill. Inspecting Settings layout
  controls, API preference type, and editor density CSS before editing.
- 2026-05-16 @@Syseng: Wired frontend Settings / Layout to the backend-3
  canonical `standard | compact` values. Settings now shows Standard /
  Compact and writes those values only. Preference normalization maps legacy
  `tight` reads to `compact` and falls back to `standard` for unknown/missing
  values.
- 2026-05-16 @@Syseng: Updated editor density rules: Wysiwyg standard remains
  `1.8`, compact is `1.65`; Source standard remains `1.7`, compact is `1.55`.
  Both editors treat legacy `tight` as compact and missing prefs as standard.

## Files changed

- `web/src/api/types.ts`
- `web/src/components/SettingsPanel.svelte`
- `web/src/editor/Wysiwyg.svelte`
- `web/src/editor/Source.svelte`

## Tests run

- `cd web && npm run check` — pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run` — pass, 14 files / 168 tests.

## Browser validation needed

- Settings / Layout shows Standard and Compact, not Tight.
- Fresh/default state selects Standard.
- Selecting Compact saves/reloads as compact and visibly tightens editor
  line-height without reverting to Tight.
- Existing legacy `tight` config reads as Compact and the UI remains usable.

## Commit readiness notes

- Ready for review; browser validation still owed by @@Webtest / @@WebtestB.
