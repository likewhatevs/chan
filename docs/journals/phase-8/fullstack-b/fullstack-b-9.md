# fullstack-b-9: Cmd+T new terminal blocked on web (Chrome reserves Cmd+T) — pick alternate or document native-only

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Decide and implement the resolution for "Cmd+T does not open
a new terminal when chan runs in a regular web browser
(Chrome reserves Cmd+T as new-tab)". Native chan-desktop
already honours Cmd+T per `fullstack-b-2`. The question is
what to do on the web side.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) terminal
cluster: original "Cmd+T new terminal" requirement.
@@WebtestA's Round-1 sweep verdict: "partial — web blocked
by Chrome's Cmd+T reservation; native side feasible".

Chrome (and Edge / Brave / most browsers) reserves Cmd+T at
the user-agent layer; the SPA cannot override it.
Firefox is similar. Safari likewise.

## Options

1. **Native-only, document the limitation.**
   * Keep Cmd+T as the chan-desktop chord. Add a one-line
     note to `PaneModeHelp.svelte` (or wherever the
     terminal cluster's help text lives): "Cmd+T (native
     only — browser reserves this chord; use
     `<alternate>` on the web)".
   * Requires picking an alternate chord (see option 2).

2. **Pick a web-compatible alternate chord.**
   * Add a second binding that works in both web and
     native, e.g. Cmd+Shift+T (still reserved by Chrome
     as "reopen closed tab", actually — bad pick),
     Cmd+Option+T (free), or `t` in Hybrid NAV mode (free,
     already a mode for chord-heavy actions).
   * The Hybrid NAV `t` option fits the existing pattern
     (`s` was moved to `f` for Search; new chord = new
     letter mnemonic).
   * Update `PaneModeHelp.svelte` accordingly.

3. **Both: native keeps Cmd+T, all platforms also get the
   alternate chord.**
   * Simplest from the user's POV: one consistent chord
     works everywhere, plus the native fast-path stays.

Recommendation: **option 3** with Hybrid NAV `t` as the
universal chord (or a Cmd+Option+T global; @@FullStackB
picks based on what fits the existing chord taxonomy
cleanly).

## Acceptance criteria

* Web users can open a new terminal without leaving the
  page (no chord that's pre-empted by the browser).
* Native users still have Cmd+T as today.
* Chosen chord is documented in `PaneModeHelp.svelte`.
* No regression on Cmd+T native open path
  (`fullstack-b-2`).
* If option 1 (native-only): the help text clearly states
  Cmd+T is native-only and points at the alternate chord
  if one exists.

## How to start

1. Read `web/src/state/shortcuts.ts` for the chord
   declarations. Find the Cmd+T entry from `fullstack-b-2`.
2. Add the chosen alternate binding (Hybrid NAV `t` is
   suggested; otherwise Cmd+Option+T).
3. Update `PaneModeHelp.svelte` to surface the new chord.
4. Test on both lane-B web (`127.0.0.1:8820`) and on
   `Chan.app` (if standing permission covers the runtime
   launch; otherwise self-verify on the chan-desktop build
   path you own from `fullstack-b-1` /
   `fullstack-b-7`).

## Coordination

* @@WebtestB verifies on lane-B drive once landed.

## 2026-05-20 - decision + implementation

Picked the task's recommendation: **option 3 (both) with Hybrid
NAV `t` as the universal chord**. The chord stack now is:

| Surface             | Chord                  |
|---------------------|------------------------|
| chan-desktop native | `Cmd+T` (KEY_BRIDGE_JS)|
| Web Mac             | `Cmd+Alt+T` (existing) |
| Every platform      | `Mod+. t` (new alias)  |

The `Mod+. t` path closes the Win/Linux web gap (where neither
`Cmd+T` exists nor `Ctrl+Alt+T` is free — `app.tab.reopenClosed`
already owns the latter) without introducing any new top-level
chord that could collide with browser reservations down the
road. `t` is a natural mnemonic that slots into the existing
Hybrid NAV spawn cluster alongside the numeric `1`/`2`/`3`/`4`
without crowding it; the cap renders as a sibling of `1` on the
same row in the cheatsheet so the discovery path is one glance.

Implementation:

* `web/src/App.svelte::handlePaneModeKey` — added
  `case "t":` / `case "T":` as fall-through labels above the
  existing `case "1":` block. The fall-through means a single
  source of truth for the spawn body; refactors to the
  terminal-spawn path automatically apply to the new chord.
* `web/src/components/PaneModeHelp.svelte` — extended the
  terminal-spawn row's caps array from `[{ "1" }]` to
  `[{ "1" }, { "t" }]`. The clickable-cap path is unchanged
  (PaneModeHelp's `dispatchKey` already routes each cap's `key`
  through the document-level keydown listener), so the new
  `t` cap is fully mouse-driveable too.
* `web/src/state/shortcuts.ts` — updated the `app.terminal.toggle`
  `note` from "macOS only on web; native everywhere" to
  "macOS web + native everywhere; all platforms via Mod+. t
  (Hybrid NAV)". The note feeds the auto-generated chord table
  in `chan serve --help`.
* `crates/chan/src/main.rs::SERVE_LONG_ABOUT` — re-synced the
  auto-generated App section to pick up the new note, and added
  one line to the Hybrid NAV section describing the `t` alias.
  Left the rest of the Hybrid NAV section drift (still says
  "Pane Mode (Cmd+K)", lists `s` for Search and `k` for
  kill-pane both of which moved in earlier tasks) untouched
  to stay in scope; that's a separate cleanup pass and
  belongs alongside whoever owns the next chord update.

Tests:

* `web/src/components/paneModeKeymap.test.ts` — new test pins
  the `case "t": case "T": case "1":` fall-through so the
  alias can't be silently dropped during a refactor.
* `web/src/components/paneModeHelpClickable.test.ts` — new
  test asserts the terminal-spawn row carries both caps and
  the action label still reads "Stage: Terminal" (the "Stage:"
  copy itself is stale per `fullstack-a-16`, but that's its own
  task; matching the live label here keeps the test honest
  about what the cheatsheet currently says).

Pre-push gate green:
* `cargo fmt --all -- --check` — clean.
* `cargo clippy --workspace --all-targets -- -D warnings` — clean.
* `cargo test --workspace` — every suite passes.
* `cargo build --workspace --no-default-features` — clean.
* `npm run check` (svelte-check) — 0 errors, 0 warnings.
* `npx vitest run` — 479/479 (was 477 baseline from -8; +2
  new tests).
* `npm run build` — clean.

## 2026-05-20 - commit readiness

Files changed (proposed single commit):

* `web/src/App.svelte` — `t`/`T` fall-through into the `1`
  terminal-spawn case in `handlePaneModeKey`.
* `web/src/components/PaneModeHelp.svelte` — terminal-spawn
  row carries two caps (`1`, `t`).
* `web/src/state/shortcuts.ts` — updated note on
  `app.terminal.toggle`.
* `crates/chan/src/main.rs` — SERVE_LONG_ABOUT re-sync (the
  auto-generated portion) + `t` line in the Hybrid NAV section.
* `web/src/components/paneModeKeymap.test.ts` — new test
  pinning the fall-through.
* `web/src/components/paneModeHelpClickable.test.ts` — new
  test pinning the cap pair.

Tests run: full pre-push gate green (see implementation note).

Known risks: the `t` alias is a behaviour add, not a removal,
so existing flows are unaffected. The chord doesn't collide
with anything inside Hybrid NAV (lowercase `t` was previously
unbound there). Outside Hybrid NAV, `t` has no special
meaning, so the chord is genuinely additive.

Push waits for Round-1 close per the standing rule.

Proposed commit subject:
`Hybrid NAV: add 't' as a universal mnemonic alias for terminal spawn (fullstack-b-9)`

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Right pick on option 3. The chord stack you ended up with
(native `Cmd+T` + Mac-web `Cmd+Alt+T` + universal
`Mod+. t`) covers every platform without a single new
top-level chord that could collide with browser
reservations. `t` slotting in next to `1`/`2`/`3`/`4` as
a fall-through in `handlePaneModeKey` is the right
shape — single source of truth for the spawn body, so
future refactors don't drift between the two paths.

PaneModeHelp two-cap rendering for the same row gives
the user a clean discoverability path; the click-cap
routes through the existing `dispatchKey` so the new
cap is mouse-driveable.

`SERVE_LONG_ABOUT` re-sync + `app.terminal.toggle` note
update keeps the auto-generated `chan serve --help`
table honest. Two new pinned tests (`paneModeKeymap.test.ts`
+ `paneModeHelpClickable.test.ts`) lock the alias and
cap-pair in place.

Solid call leaving the rest of the Hybrid NAV section
drift (the stale "Pane Mode (Cmd+K)" header + `s`/`k`
references that moved in earlier tasks) untouched. Mixing
that cleanup into a chord-add commit would have crossed
two scopes; the journal flag is the right place to record
the deferral. I'm filing the drift as a bug entry in
`phase-8-bugs.md` for someone to absorb on the next chord
update.

Pre-push gate green (vitest 479/479, +2 from baseline; full
cargo + web stack clean).

**Commit clearance**: approved. Use your proposed commit
subject as-is. Push waits for Round-1 close.

Queue update: `fullstack-b-10` (b-3 partial-fix call-site
flip) is your next one. After that you're at queue
empty for the wave; pick up the `desktop/Makefile` follow-
up if you want a fill-in, or idle until I cut more.