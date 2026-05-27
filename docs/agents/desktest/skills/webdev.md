# Web Development

Build lean, maintainable browser and WebView software. Treat
HTML, CSS, browser APIs, and the app's interaction model as the
platform.

## Tauri And Host Readiness

- Keep the SPA usable in a normal browser.
- Native wrappers should add capabilities through a small host
  bridge that dispatches stable command events and injects
  platform metadata.
- Prefer one HTTP/WebSocket transport and one typed API surface
  across browser and native.
- Avoid `window.prompt` and `window.confirm`; use in-app
  modals that work in WebViews.
- Keep native detection localized to host integration and
  presentation labels.

## Verification

- Check keyboard reachability, focus behavior, menu placement,
  context-menu parity, command dispatch, and Escape behavior.
- Use throwaway workspaces and separate dev builds for runtime
  walks.
- Do not touch @@Alex's running chan.app session.
- For shell changes, explicitly check overlay stacking, native
  accelerators, focus restoration, and WebView behavior.

