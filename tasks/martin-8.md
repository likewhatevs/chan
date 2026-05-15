# martin-8: Move model picker into the assistant inspector + persist

Owner: Martin. Depends on: martin-7 (removes the Settings model
picker so this becomes the only model UI).

## Why

User feedback: model selection doesn't belong in Settings. Move it
to the assistant overlay inspector so the user can pick a model
inline. The pick should persist per backend, similar to how pane
widths persist via the preferences store.

## Backing store (already exists)

Server-side persistence is already wired:
- `chan_llm::ModelsConfig` has `claude_cli`, `gemini_cli`,
  `codex_cli` fields (each `Option<String>`), one per CLI backend.
- The preferences PUT path round-trips these via `CliPrefsView.model`
  on `routes/preferences.rs`.

No Bob work is needed. Martin reads/writes through the existing
prefs surface.

## Files to touch

- `web/src/components/AssistantInspectorBody.svelte` (the inspector
  view — see imports in `Inspector.svelte` / `InspectorBody.svelte`).
- Possibly `web/src/components/InlineAssist.svelte` if the picker
  needs to be reachable from the live overlay rather than only the
  inspector. Use judgement: if the inspector is where the user
  already lives when chatting, that's enough. Don't duplicate the
  control in two places.

## Required UI

A compact selector near the model attribution line. Pre-populated
options come from `chan_llm`'s known model list per backend if the
frontend has access to it; otherwise show a free-text input with
the current model as the placeholder. Either is acceptable in v1;
free-text is fine because the CLI itself validates.

Behaviour:
- Read the current value from
  `preferences.assistant.<active_backend>.model` (already exposed
  in martin-7's data model).
- On change, PUT the new value to the preferences endpoint.
  Optimistic update locally; revert on PUT failure.
- Selecting a different backend later should show that backend's
  last-used model, not the previous backend's.

## Acceptance criteria

1. `npm run check` and `npm test -- --run` are green.
2. Switching the model in the inspector persists across reload.
3. Switching the active backend (via martin-7's dropdown) restores
   that backend's last-used model in the inspector.
4. Empty / null model means "let the CLI choose its default" (the
   server already handles `None` correctly per chan-llm).
5. No model picker remains in Settings.

## Hints

- Pane-width persistence in this app lives on `editor_prefs.pane_widths`
  and saves via the same PUT endpoint as everything else. Mirror
  that pattern: optimistic-local update, debounced PUT, no manual
  reload required.
- If the inspector also needs to display the model when the
  current backend has none set (the `None` case), show a
  placeholder like "default" or the inferred fallback.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-8, plus
a one-line log entry).
