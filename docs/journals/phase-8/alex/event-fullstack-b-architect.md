# event-fullstack-b-architect.md

From: @@FullStackB
To: @@Architect
Date: 2026-05-19

## 2026-05-19 — poke

`fullstack-b-1` ready for review: chan-desktop now keeps a 20-deep
LRU stack of closed window configs in its sidecar config and pops
the most-recent matching entry on next open. Reuses the `?w=<label>`
so `session.json` rehydrates panes / tabs; mirrors the URL hash so
overlay state round-trips. `cargo test -p chan-desktop --bin
chan-desktop` green (17 tests, 6 new), clippy + fmt clean.

See [../fullstack-b/fullstack-b-1.md](../fullstack-b/fullstack-b-1.md)
for the full implementation note and acceptance-criteria table.
Holding for commit clearance; moving on to `fullstack-b-2`.

## 2026-05-19 — poke

`fullstack-b-2` ready for review: terminal cluster shipped as one
patch. Cmd+T native + Cmd+Alt+T web (Mac) for new terminal,
scrollback-loss-on-Hybrid-NAV root-caused and fixed (Pane.svelte
no longer unmounts TerminalTabs across `paneMode.active` toggles),
`lineHeight: 1.0 -> 1.2` to match iTerm's row spacing for
multi-row ASCII glyphs. Picked up a stale `SERVE_LONG_ABOUT`
resync as a bonus. Full pre-push gate clean.

See [../fullstack-b/fullstack-b-2.md](../fullstack-b/fullstack-b-2.md)
for the file-by-file breakdown + WebtestB walkthrough plan.
Holding for commit clearance; moving on to `fullstack-b-3`.

## 2026-05-19 — poke

`fullstack-b-3` ready for review: watcher dialog cluster. Backend
`resolve_watcher_dir` now accepts absolute paths anywhere on the
filesystem (drive sandbox dropped for absolute inputs — event
files are infra traffic, not user content) and creates missing
dirs silently. Frontend gained a `PathPromptMode = "attach"`
mode so existing dirs don't trip the "overwrite" warning and
absolute paths don't manufacture a fake ancestor preamble. The
TerminalRichPrompt watcher dialog now uses it. Five new backend
tests + four new SPA tests; pre-push gate green.

Heads-up flag: chan-server will now `create_dir_all` arbitrary
user-typed paths on watcher attach. Appropriate per the bug ask,
but I called it out in the journal in case you want tighter
guards. See
[../fullstack-b/fullstack-b-3.md](../fullstack-b/fullstack-b-3.md)
for the full breakdown. Moving on to `fullstack-b-4`.

## 2026-05-19 — poke

`fullstack-b-4` ready for review: indexing-chart pan/zoom parity
with the main Graph view. Self-contained pan/zoom layer on the
existing SVG hierarchy in `EmptyPaneCarousel.svelte`
(`chartTransform` + pointer capture + wheel-zoom + Locate
recenter button). Same `exp(-delta * 0.0015)` smoothing as
`GraphCanvas` so the two views feel identical under the wheel.
Behaviour sits on the `<svg>` element directly so the Round-2
Infographics-tabs refactor inherits it. Eight new pinned-source
tests.

Heads-up: while in `shortcuts.test.ts` I picked up @@FullStackA's
in-flight `fullstack-a-7` chord swap (Hybrid NAV: Cmd+K → Cmd+.)
that had stranded the
`"advertises Hybrid NAV (Cmd+K)"` test. Trivial one-line label
update + `SERVE_LONG_ABOUT` resync. Called out in the task file
so the audit trail attributes both to the right initiating
change.

See [../fullstack-b/fullstack-b-4.md](../fullstack-b/fullstack-b-4.md)
for the full breakdown + WebtestB walkthrough plan. Moving on to
`fullstack-b-6` (FB watcher scope, the higher-priority of the
two new tasks per your event note).

## 2026-05-19 — poke

`fullstack-b-6` ready for review: FB watcher scoped on the SPA
side. Each FB instance contributes a scope from its selection
(drive root / dir / parent-of-file); `onWatchEvent` only
refreshes when an event lands in an active scope, and only
refreshes the affected parent dir via the new
`refreshTreeForPath`. No chan-server changes (the
subscribe-by-prefix direction the task floated turned out
unnecessary). Limitation flagged in the journal: shared
`tree.entries` means two open FBs on different scopes both see
refreshes for events in either scope. Matches the "ship strict
first" note in the task. Ten new tests, gate green (474/474
SPA).

See [../fullstack-b/fullstack-b-6.md](../fullstack-b/fullstack-b-6.md)
for the full breakdown + WebtestB walkthrough plan. Moving on to
`fullstack-b-5` (per-Hybrid theme propagation).

## 2026-05-19 — poke

`fullstack-b-5` ready for review: per-Hybrid theme propagation
into editor surfaces. Root cause was each editor theme
(github / google_docs / word) gating its dark variant on
`:root[data-editor-theme=..][data-theme="dark"]` — only the
root match. A Hybrid pane that overrode to light inside a
globally-dark app still picked up dark editor tokens (visible
as "back of Hybrid white-on-white"). Fix: extended each
theme's dark selector to also fire for
`.pane[data-theme="dark"]` and added a sibling
`.pane[data-theme="light"]` block that re-asserts the base
light defaults. No JS changes — pure CSS, contained in three
files. Truth table covered for all six combos. Gate green.

That closes out my queue. Holding all four (-2, -3, -4, -5,
-6) for commit clearance. See
[../fullstack-b/fullstack-b-5.md](../fullstack-b/fullstack-b-5.md)
for the design write-up + WebtestB walkthrough plan.

## 2026-05-19 — poke

All five cleared tasks committed (push waits for Round-1 close):

* `315fcc1` fullstack-b-2 — Cmd+T new terminal + scrollback +
  lineHeight.
* `a9579f0` fullstack-b-3 — watcher dialog: any path +
  create-if-missing + drop overwrite warning.
* `ca8a441` fullstack-b-4 — indexing-chart pan/zoom +
  SERVE_LONG_ABOUT resync (single landing for the chord drift,
  per your -4 clearance).
* `f3ec455` fullstack-b-6 — scope FB watcher.
* `28b168a` fullstack-b-5 — per-Hybrid theme propagation to
  editor surfaces.

Queue empty. Idle / available — happy to pick up the
`desktop/Makefile` bundle-path drift you flagged in the -5
clearance, or to wait for the next wave.
