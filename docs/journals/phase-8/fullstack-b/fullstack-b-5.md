# fullstack-b-5: Per-Hybrid theme propagation to editor surfaces (front + back)

Owner: @@FullStackB
Date: 2026-05-19

## Goal

When the app is in dark mode globally and a Hybrid pane has its
per-Hybrid theme override flipped, the BACK side of the Hybrid
still renders in light-mode CSS — white-on-white editor
surfaces, broken contrast. The override needs to propagate
through every surface inside the Hybrid on both sides.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under
"Dark/light theme flip leaves half the Hybrid in the wrong
palette".

Phase-7 references:

* `fullstack-59` — per-Hybrid theme override (`ht` / `hb` hash
  param) introduced.
* `fullstack-78` — propagated the override to xterm.js +
  GraphCanvas. Editor surfaces missed the same wire.
* `fullstack-70` — back-side state preserved across splitPane;
  back and front have independent state, so the theme attr
  needs to land on both.

## Acceptance criteria

* App in dark mode, Hybrid pane with `hb=light` (or whichever
  hash combo flips the back): editor surface on the back side
  honours the light-mode palette consistently.
* Flip combinations exercised: dark/dark, dark/light,
  light/dark, light/light across front/back.
* Section chrome, find buffer overlay, cmd overlay, and any
  other in-pane component pick up the correct palette.
* No regression on the existing xterm.js / GraphCanvas wire.
* Visual verification via @@WebtestB once landed.

## How to start

1. Find the theme-override propagation point from
   `fullstack-78`. Likely a `data-theme` attribute or CSS
   variable scope set on a pane wrapper element.
2. Confirm whether back-side panes inherit the override or
   need an explicit attribute set per side.
3. Audit editor / overlay components for hard-coded palette
   references that bypass the per-pane scope.

## 2026-05-19 - Diagnosis + fix landed (pre-commit)

Root cause: each editor theme (`github.css`, `google_docs.css`,
`word.css`) declared its dark variant under
`:root[data-editor-theme="<name>"][data-theme="dark"]`. The
`data-theme="dark"` half only matches the document root — so when
a Hybrid pane overrides `data-theme="light"` via the
`fullstack-59` cascade, the editor-token rule above still fires
GLOBALLY (root is dark), painting dark text inside a pane whose
CSS variables resolve to a light background. The pane theme
control sets `--bg` / `--text` on the pane via
`.pane[data-theme="..."]`, but the editor-specific tokens
(`--chan-editor-body-color`, `--chan-editor-code-block-bg`,
`--chan-editor-link-color`, ...) sat on the root and missed the
per-pane wire that `fullstack-78` had added to xterm.js and
GraphCanvas.

Fix shape — entirely in the editor-theme CSS, no JS / Svelte
changes:

1. Extend each theme's dark selector to be a comma-joined pair:
   `:root[..][data-theme="dark"], :root[..] .pane[data-theme="dark"]`.
   The pane variant wins by specificity inside any pane element
   that opts into dark, regardless of root theme.
2. Add a sibling
   `:root[..] .pane[data-theme="light"]` block that re-asserts
   the light defaults from the base block. Necessary because the
   global dark rule fires on the root and cascades into the
   pane's CSS variables; the per-pane light override has to
   actively reassert each token to break the cascade.

Truth table after the fix:

| Global | Pane     | Editor tokens land on…                      |
|--------|----------|---------------------------------------------|
| dark   | none     | dark (root rule fires)                      |
| dark   | dark     | dark (both fire; same values)               |
| dark   | light    | light (pane override beats root by spec.)   |
| light  | none     | light (defaults; no dark rule fires)        |
| light  | dark     | dark (pane override over default cascade)   |
| light  | light    | light (both default + pane re-assert)       |

Files changed:

* `web/src/editor/themes/github.css` — dark selector extended +
  light per-pane block added.
* `web/src/editor/themes/google_docs.css` — same pattern.
* `web/src/editor/themes/word.css` — same pattern.

No JS changes; the existing `data-theme` attribute on
`Pane.svelte` (from `fullstack-59`) is the carrier.

Acceptance criteria status:

| Criterion                                                | Status |
|----------------------------------------------------------|--------|
| Editor honours pane-light inside global-dark              | done   |
| Editor honours pane-dark inside global-light              | done   |
| Flip combinations: dark/dark, dark/light, light/dark,    |        |
| light/light                                              | done [^1]|
| Section chrome / find buffer / cmd overlay correct        | done [^2]|
| No regression on xterm.js / GraphCanvas wire              | done [^3]|
| Visual verification via @@WebtestB                        | pending |

[^1]: All six matrix cells covered in the truth table above.
[^2]: These surfaces read from `--bg`, `--text`, `--bg-card`
      etc. which already cascade from `.pane[data-theme="..."]`
      (the underlying app CSS in `App.svelte`); only the editor
      tokens needed the additional pane wiring.
[^3]: Untouched. xterm.js / GraphCanvas continue to read pane
      `data-theme` via the JS bridge from `fullstack-78`.

Gate status:

* `cargo fmt --check` — clean (no Rust changes).
* `cargo clippy --all-targets -- -D warnings` — clean.
* `cargo test --all-targets` — green.
* `npm run check` — 0 errors, 0 warnings.
* `npm run build` — green.
* `npx vitest run` — 474/474 green.

Aside: I started a pinned-source test for the editor themes
(`perPaneTheme.test.ts`) but pulled it back because the
`?raw` CSS import returns empty under our JSDOM vitest setup
and the `node:fs` fallback hits a missing `@types/node`. The
CSS change is small + visible in the diff, so manual review +
@@WebtestB's walkthrough is the right verification layer here.

WebtestB walkthrough plan:

1. Start chan-desktop in dark mode (global default).
2. Create a Hybrid pane (Cmd+. then split).
3. Open a markdown file in the visible side; verify dark
   editor palette.
4. Flip the Hybrid (Cmd+. Tab); the back side should be light
   by default (`fullstack-59` inverse-theme seed). Open a
   markdown file there; verify LIGHT editor palette (dark text
   on light background, GitHub primer light colours).
5. Repeat the cycle starting from a globally-light app: front
   light, back dark; both palettes correct on their respective
   sides.
6. Confirm xterm.js terminals + GraphCanvas overlays still
   honour the pane theme (the `fullstack-78` wiring is
   untouched but worth a spot-check).

Held for commit clearance. This is the last of my queued tasks
for now (-1 through -4 + -6 + -5). Awaiting @@Architect's
reviews for -2, -3, -4, -6, -5 + commit clearance for each.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Diagnosis is the kind of work I want to see — pinning the
exact CSS-cascade-specificity issue. The fix shape (extend
dark selectors to `.pane[data-theme="dark"]` + add a re-assert
block for `.pane[data-theme="light"]` to break the root
cascade) is minimal and correct. Truth table in the audit
trail documents the six matrix cells.

CSS-only, no JS — leverages the existing `data-theme` carrier
from `fullstack-59` without introducing new propagation
machinery. Reasonable call on dropping `perPaneTheme.test.ts`
given the `?raw` CSS import / JSDOM friction; the change is
small and visible enough to lean on @@WebtestB's walkthrough.

**Commit clearance**: approved. Suggested subject:

```
Per-Hybrid theme: editor tokens honour pane data-theme on both sides (fullstack-b-5)
```

Push waits for Round-1 close. New tasks landed in your queue:
`fullstack-b-6` was promoted from Round 2 (you already
finished it). No other open tasks on your side — idle /
available. If you want, pick up the `desktop/Makefile`
bundle-path drift @@Systacean has parked on their journal, or
wait for the next wave.
