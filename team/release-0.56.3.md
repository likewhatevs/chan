# v0.56.3: markdown list alignment and pane shortcut hints

Cut from `main` after `v0.56.2`. This is a focused patch for the Markdown editor list rendering follow-up and one shortcut-discovery correction in the pane menu.

## Theme

Make list markers predictable across Markdown syntax and editor themes, and make shortcut hints reflect the platform that is actually running.

## Editor lists

- Bullet, hyphen, ordered, and task-list markers now share the same marker column, so item text starts consistently across marker types.
- `*` and `+` list markers render as the depth glyphs used in the editor (`●`, `○`, `■`) without changing the underlying Markdown source.
- Hyphen and ordered-list markers stay literal text, but sit in the same marker column as bullet glyphs and task checkboxes.
- Task-list rows hide the raw `- ` prefix before the checkbox marker, keep the checkbox clickable, and still toggle the source `[ ]` / `[x]` text.
- Nested-list indentation now uses the reduced 2x visual offset after the 4x pass proved too wide.
- Google Docs and Microsoft Word themes inherit the same list marker tokens as the default editor theme, avoiding font-driven marker drift.

## Pane shortcut hints

- The pane hamburger now renders pane navigation and split-row hints through the shared shortcut registry instead of hardcoded `Mod` chord strings.
- In the web build, split rows show no direct `Cmd+/` / `Ctrl+/` hint because split is reached through Hybrid Nav (`Mod+.` then `/` or `?`) and CodeMirror owns `Mod+/` for comments when the editor is focused.
- Pane next/previous hints now show the web `Alt+]` / `Alt+[` bindings, while native keeps the direct `Cmd/Ctrl+]` / `Cmd/Ctrl+[` labels.
- The split-bottom row uses the registry command id `app.pane.splitDown`, matching the App command bridge.

## Validation

- `npm run test -- components/Pane.test.ts`
- `npm run test -- state/shortcuts.test.ts`
- `npm run test -- decorations/blocks.test.ts`
- `npm run build`
- `cargo build -p chan`

## Release

- GA bumps all release pins to `0.56.3`, updates the changelog and this release report, then dry-runs the release workflow with `publish=false` before tagging `v0.56.3`.
