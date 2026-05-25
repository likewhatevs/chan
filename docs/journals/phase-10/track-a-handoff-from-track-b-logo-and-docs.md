# Phase 10 Track A Handoff From Track B

Date: 2026-05-25.

Track B is cutting two desktop-facing follow-ups to Track A/syseng. This note
is separate from Track A's roadmap so it can be sequenced without Track B
editing active Track A files.

## Tauri App Icon Regeneration

Ownership reason:

- The visible desktop icon belongs to the Tauri shell and packaged desktop
  artifacts, not the public site generator.
- Track B only confirmed the site mark/tokens that should feed the icon.

Task:

- Regenerate Tauri app icons so Cmd+Tab and Dock show a dark background
  `#101112` with the orange enso `#ef8f58`.
- Base the colors on the current dark site tokens.
- Update `desktop/src-tauri/icons/*`, `desktop/src-tauri/icon.icns`, and
  `desktop/src-tauri/icon.ico` where applicable.
- Verify the generated macOS app icon in Cmd+Tab and the Dock, not only the
  source PNGs.

## Desktop Docs And Config Audit

Ownership reason:

- The stale copy is in desktop packaging docs/config/comments and should be
  audited with Track A's desktop/syseng context.
- Track B is keeping public docs fresh-state only and avoiding active Track A
  files.

Task:

- Audit stale desktop docs, config, and comments for old release/install
  contracts.
- Include `desktop/README.md` Linux/Windows wording.
- Include `desktop/design.md` old `/dl/latest`, MSI, and updater language.
- Include `desktop/src-tauri/tauri.conf.json` updater URL shape.
- Keep any corrected wording fresh-state only: no migration, old-contract, or
  backward-compatibility framing in public-facing docs.
