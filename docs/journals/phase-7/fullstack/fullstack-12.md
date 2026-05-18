# fullstack-12: Cmd+` → Cmd+T (native) + Cmd+Alt+T (web) (B16)

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Rebind the "new terminal" shortcut off `Cmd+\``  (conflicts
with macOS window-cycle) to `Cmd+T` on Chan.app native, and
`Cmd+Alt+T` on the web variant (browsers reserve `Cmd+T`
for "new browser tab"). Bind both on native so muscle
memory transfers when @@Alex switches between native and web.

Same native-vs-web detection pattern as @@FullStack-5's
`Cmd+T` handling for tab spawning.

## Relevant links

* [../request.md](../request.md) Bugfixes — B16.

## Acceptance criteria

* On Chan.app native: `Cmd+T` and `Cmd+\`` both spawn a new
  terminal in the active pane.
* On the web variant: `Cmd+Alt+T` spawns a new terminal;
  `Cmd+T` is left to the browser (new browser tab).
* `Cmd+\`` is removed from the web variant's binding map
  (avoid the macOS conflict for users running chan in a
  browser inside macOS).
* Tooltip / menu hint reflects the active platform's
  binding.

## Out of scope

* Window switching (Cmd+\` itself is the OS shortcut; we're
  vacating it, not replacing OS behavior).
* Tab-switching shortcuts in general.

## How to start

1. Locate the current `Cmd+\`` handler in
   `web/src/state/shortcuts.ts`.
2. Use the same `import.meta.env` / `window.__TAURI__`
   detection pattern @@FullStack-5 used. Branch the binding
   registration.
3. Update the visible hint wherever it's shown (likely a
   tooltip on the new-terminal affordance).

## Hand-off

Implemented in `web/src/state/shortcuts.ts`,
`web/src/App.svelte`, `desktop/src-tauri/src/serve.rs`, and
`crates/chan/src/main.rs`.

* Web browser binding moved from `Cmd+\`` to literal `Cmd+Alt+T`;
  `Cmd+T` remains untouched for the browser.
* Native bridge now maps `Cmd+T` to `app.terminal.toggle` while keeping
  `Cmd+\`` as an accepted native bridge alias.
* Shortcut hints now show `Cmd+Alt+T` on web and `Cmd+T` in native.
  The `chan serve --help` table was regenerated from
  `web/src/state/shortcuts.ts`.
* Added focused coverage for the shortcut table and native bridge map.

Gate:

* `npm run test -- shortcuts`
* `npm run check`
* `npm run build`
* `cargo test -p chan-desktop serve::tests::key_bridge_maps_terminal_to_t_and_backquote`
* `scripts/pre-push`
