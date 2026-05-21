# webtest-a-4 — Hybrid back-side wave + drag-to-rearrange walkthrough (-a-44 + -a-45 + -a-46)

Owner: @@WebtestA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Bundled walkthrough covering three Round-2 wave-2/wave-3
commits that landed since `webtest-a-3`'s verdict:

1. **`fullstack-a-44`** (in HEAD under `a8e991a`; see the
   cross-agent commit-hygiene incident routing in
   [`../architect/journal.md`](../architect/journal.md)
   "2026-05-21 — incident routing" for context — the
   commit subject misattributes to webtest-b-3 but the
   `-a-44` code IS in HEAD verbatim) — Hybrid pane
   drag-to-rearrange + transaction-mode NAV.
2. **`fullstack-a-45`** (`1f80d09`) — Migrate Terminal
   Settings to Hybrid Terminal back-side.
3. **`fullstack-a-46`** (`5166223`) — Migrate Editor
   Settings to Hybrid Editor back-side.

(`-a-47` drop front/back independent theme is still
in-flight at @@FullStackA's lane; folds into a future
`webtest-a-5` once it lands + the FB-back side from
`-a-48` populates.)

## Background

Three independent slices, one bundled verdict commit
keeps the audit shape clean.

### `-a-44` drag-to-rearrange

Implementation note at the tail of
[`../fullstack-a/fullstack-a-44.md`](../fullstack-a/fullstack-a-44.md);
clearance + 3 deviations-accepted at
[`../alex/event-architect-fullstack-a.md`](../alex/event-architect-fullstack-a.md)
"approved + commit clearance (fullstack-a-44)" + the
incident-routing append below it.

Spec from
[`../phase-8-bugs.md`](../phase-8-bugs.md) §"Hybrid
pane drag-to-rearrange":

* Two entries to transaction mode, both targeting the
  top-bar dead zone (space between last tab and
  hamburger):
  * **Drag-start** from dead zone enters NAV transaction
    mode WITH the originating pane as first grab.
  * **Double-click** on dead zone enters NAV transaction
    mode WITHOUT an originating grab (next click+drag
    inside any Hybrid grabs that pane).
* Once in transaction mode: click anywhere inside any
  pane grabs + drags that pane; Enter commits, Esc
  dismisses + reverts.
* Every pane (not just Hybrids) is a valid drop target
  per the bug-list "rearrange ANY pane" framing.
* Cmd+. mid-transaction NOT wired (Esc is the universal
  exit; keeps symmetry with keyboard NAV).

### `-a-45` Terminal Settings migration

Migrates the Terminal section out of `SettingsPanel.svelte`
into the new `HybridTerminalConfig.svelte` (the back-side
of any Hybrid Terminal pane). Settings storage shape
unchanged (Preferences fields untouched, autosave
unchanged); only the UI mounting point moves.

Implementation note in
[`../fullstack-a/fullstack-a-45.md`](../fullstack-a/fullstack-a-45.md);
clearance at
[`../alex/event-architect-fullstack-a.md`](../alex/event-architect-fullstack-a.md)
"approved + commit clearance (fullstack-a-45)".

### `-a-46` Editor Settings migration

Mirror of `-a-45`'s pattern for the Editor section
(Theme/Appearance, Layout, Date Pills, On Save) → moves
into `HybridEditorConfig.svelte`. Per round-2-plan
§"Hybrid back-side revisited" — Editor back-side scope
includes Theme.

Implementation note in
[`../fullstack-a/fullstack-a-46.md`](../fullstack-a/fullstack-a-46.md);
clearance at
[`../alex/event-architect-fullstack-a.md`](../alex/event-architect-fullstack-a.md)
"approved + commit clearance (fullstack-a-46)".

## Coverage slice (lane A)

Pure SPA work; standing terminal + Chrome MCP perm covers
the walk. Single chan + test-server boot; walk all three
slices in sequence; commit a single bundled verdict.

## Acceptance criteria

### `-a-44` — drag-to-rearrange (six checks)

1. **Entry A: drag from dead zone** — open 2+ Hybrid panes.
   Mouse-down + drag from the top-bar dead zone (between
   last tab and hamburger) of pane 1. Threshold crossed
   (>5 px) should enter NAV transaction mode WITH pane 1
   as grab. Visual indication of transaction mode (chrome
   change / overlay / pane highlight) should fire.
2. **Entry B: double-click dead zone** — same setup,
   `dblclick` on dead zone of pane 1. Should enter NAV
   transaction mode WITHOUT an originating grab (standby
   state). Next click inside any pane should grab THAT
   pane.
3. **Drag-and-drop swap** — in transaction mode (either
   entry), drag from pane 1's grab + release inside pane
   2's body. Panes should swap. Enter commits + clears
   transaction.
4. **Drop on non-Hybrid pane** — confirm every pane (not
   just Hybrids) is a valid drop target. If you have a
   terminal-only pane, dragging into it should still
   swap.
5. **Cancel via Esc** — start a transaction (Entry A or
   B), don't drop, press Esc. Transaction should clear;
   panes should be back to pre-transaction state.
6. **Chain semantics** — in transaction mode with a
   successful swap, transaction should STAY ON for
   chained swaps. Continue swapping; Enter to commit;
   Esc to cancel mid-chain.

### `-a-45` — Terminal Settings now lives in Hybrid Terminal back

1. **Spawn a Hybrid Terminal pane**. Flip to back. Back
   should now show `HybridTerminalConfig.svelte` (post-
   `-a-43` stub populated with real Terminal settings
   per `-a-45`). Title band "Hybrid Terminal".
2. **Verify scrollback control** — the MB scrollback
   setting is visible + functional. Set to some value;
   verify the persistence (refresh; setting holds).
3. **Verify default TERM dropdown + custom-TERM**
   rendering — both surfaces present; can switch
   between xterm-256color / tmux-256color / custom; save
   round-trips.
4. **Save-status indicator** — modify a setting; the
   per-surface save-status indicator should reflect the
   debounce + final save state.
5. **Settings overlay (`Cmd+,`) no longer has Terminal
   section** — open Settings overlay; confirm the
   Terminal section is GONE from SettingsPanel (regression
   guard).
6. **Open a second Hybrid Terminal pane**. Confirm its
   back-side `HybridTerminalConfig` mounts cleanly; shows
   the same settings (settings are per-DRIVE, not
   per-pane).

### `-a-46` — Editor Settings now lives in Hybrid Editor back

1. **Spawn a Hybrid Editor pane** (open a markdown file
   in a Hybrid). Flip to back. Back should show
   `HybridEditorConfig.svelte` populated. Title band
   "Hybrid Editor".
2. **Theme (Appearance)** — set theme via the back-side
   control. Verify it applies to THIS Hybrid (per
   `-a-47` will collapse the front/back theme to single
   per-Hybrid value; for now, the move itself is the
   subject). The Hybrid Editor back's theme control
   should round-trip.
3. **Layout / Date Pills / On Save** — verify each
   control is present + functional + round-trips
   persistence.
4. **Save-status indicator** — modify a setting; observe
   debounce + final save.
5. **Settings overlay (`Cmd+,`) no longer has Editor
   section** — confirm Editor section is GONE from
   SettingsPanel.
6. **CSS / visual** — visual sanity. No stray padding;
   no unstyled controls; the Editor back should match
   Terminal back's overall feel.

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-21 — fullstack-a-44 + -a-45 + -a-46 walkthroughs
(Hybrid back-side wave; drag + Terminal migration + Editor
migration)`. Capture:

* All three slices' acceptance subsections with HOLD /
  FAIL / PARTIAL verdict per check.
* Screenshots at each step (especially the drag
  transaction-mode visual states + the Hybrid Editor
  Theme picker).
* Side observations for the bug list (e.g. drag
  transaction-mode visual affordance unclear; Settings
  overlay residue after Terminal/Editor removal; etc.).
* Tear-down evidence (test server killed, throwaway
  drive `rm -rf`'d, `chan remove <path>` registry
  cleanup, Chrome MCP tabs closed).

## How to start

1. `git status` confirm clean; `git log --oneline -15`
   confirms `a8e991a` (-a-44 under wrong subject),
   `1f80d09` (-a-45), `5166223` (-a-46) all in HEAD.
2. Spin up a fresh test server. Throwaway drive seed:
   chan-source default (matches `webtest-a-2.md` pattern)
   OR ad-hoc fixture. Your call.
3. `cargo build -p chan`; `web/npm run build`; restart
   server.
4. Walk `-a-44` six checks first (independent feature;
   doesn't depend on `-a-45/-46`).
5. Walk `-a-45` six checks (Hybrid Terminal back-side).
6. Walk `-a-46` six checks (Hybrid Editor back-side).
7. Append the bundled verdict to `webtest-a-1.md`; fire
   poke to @@Architect via
   `event-webtest-a-architect.md`.
8. Tear down per the standing rule.

## Coordination

* @@WebtestA lane (reactive).
* Standing terminal + Chrome MCP perm covers the walk.
* `-a-47` (drop front/back independent theme) is still
  in flight at @@FullStackA's lane; NOT in this walk
  scope. Folds into `webtest-a-5` after `-a-47` + `-a-48`
  (Task F — Search/Indexing/Reports settings migration)
  both land in HEAD.

### Pre-commit discipline (carry-forward)

Same shape that landed your `-3` verdict cleanly:

* `git commit <path> -m "..."` path-limit shape, OR
  explicit `git add` per path + pre-commit
  `git diff --staged --stat` walk + `git restore --staged`
  on any non-mine file.
* Post-commit `git show --stat HEAD` confirm scope.

The a8e991a cross-agent commit-hygiene incident in your
lane's history reinforces — discipline catches the
race when applied.

## Numbering

Highest committed `webtest-a-N` is `-3` (`56e6692` verdict
+ `c9fb768` close-out marker); this is `-4`.

## Out of scope

* `-a-47` drop front/back independent theme — folds
  into `webtest-a-5` (along with `-a-48` FB-back
  Search/Indexing/Reports migration when it lands).
* Graph overhaul sub-wave (`-a-49..52`) — not yet
  landed; folds into a later walkthrough.
* `-a-42` About section build-out — not yet committed
  (gates on A+B+C+F landing); folds into the final
  Round-2-wave-3 walk.
