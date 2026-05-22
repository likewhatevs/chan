# fullstack-b-28 — chan-desktop launcher pre-flight UX (surfaces BGE + chan-reports toggles)

Owner: @@FullStackB
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3
Dependency: `systacean-27`

## Goal

Extend chan-desktop's drive launcher / pre-flight
screen to surface the BGE-small + chan-reports
feature toggles. Both off by default; user can
enable at pre-flight OR via Settings (separate
`-a-76` task).

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Pre-flight feature toggles" (line 193+) +
§"UI surface (item 2 pre-flight report + Settings)"
(line 222+).

`fullstack-b-13` (chan-desktop launcher pre-flight UX
foundation) is the surface to extend.

## Scope

* Add per-drive toggle UI for `bge` + `reports` to
  the pre-flight screen.
* Both default OFF.
* Persist the user's choice via the chan-drive
  config API (`systacean-27` provides).
* Tooltip / info-button explaining what each toggle
  enables (BGE = semantic search; reports = file
  classification + stats).

## Acceptance

1. **Pre-flight screen** shows BGE + reports toggle
   rows.
2. **Default OFF**: clean install opens drive lean.
3. **Toggle persistence**: user enables BGE →
   reflected in chan-drive config → BOOT picks up
   on next launch.
4. **Tooltip / info** describes each feature.

### Tests

Vitest pins + chan-desktop runtime test under
standing perm.

### Gate

`cargo` + `npm` gates green.

## Coordination

* @@FullStackB lane.
* Depends on `systacean-27` API. Can stub-shell the
  toggle persistence + wire when `-27` lands.

## Authorization

Yes for chan-desktop launcher + SPA pre-flight UI +
tests + task tail + outbound.

## Numbering

This is `-b-28`.

## 2026-05-22 — scope question for @@Architect (pre-flight foundation + -27 dep + scope footprint)

At pickup, two premises in the task body don't match the
current code state:

### 1. `fullstack-b-13` is NOT the pre-flight UX foundation

The task body says "Extend `fullstack-b-13`'s pre-flight
UX with two toggles" and "the existing pre-flight screen".
`-b-13` was actually the **shell/agent submit-mode toggle**
(rich-prompt header toolbar; see `dce2373` "Rich prompt:
shell/agent submit-mode toolbar toggle + SerTab roundtrip"
+ `e24b931` "chan-server: per-session shell/agent submit-
mode toggle"). No drive pre-flight surface in `-b-13`.

`round-2-plan.md:228` actually frames `-b-13` as the
*predicted* implementer of the pre-flight SPA copy ("likely
`fullstack-b-13` SPA copy + `systacean-10` server-side
pre-flight report schema"). That prediction was made
pre-rich-prompt-mini-wave; `-b-13` got repurposed and the
prediction never came true.

The two "pre-flight" code surfaces that DO exist today are
both unrelated to the user-facing drive pre-flight described
in `-b-28`:

* `desktop/src-tauri/src/main.rs:800` (`compute_bin_status`
  + `frontend-visible verdict from boot-time chan preflight`)
  — runs once at chan-desktop boot to verify the bundled
  chan binary; no per-drive surface.
* `web/src/state/watcherEvents.test.ts` — `pre-flight`
  event type in the rich-prompt watcher protocol; irrelevant
  here.

The actual chan-desktop launcher (`desktop/src/main.js`
`render(drives)` at line 236+, `pickAndAdd()` at line 220+)
goes directly from the directory picker
(`window.__TAURI__.dialog.open({directory: true, ...})`) to
`invoke('add_drive', { path })` — no pre-flight intervention.

### 2. systacean-27 dependency is dispatched but not landed

`systacean-27` (the chan-drive config persistence + BOOT
process integration) is dispatched (`docs/journals/phase-8/systacean/systacean-27.md`)
but no commit yet (`git log` shows no `systacean-27` SHA).
The task body explicitly allows shell-and-stubbing the
toggle UI now + wiring later. The dependency is real but
parallelisable.

### 3. Resulting scope footprint

Building a per-drive pre-flight screen from scratch is
materially bigger than the task body suggests:

* New screen / step in the chan-desktop launcher flow
  between `pickAndAdd` and `add_drive` (modal? dedicated
  pre-flight page?).
* The full pre-flight report the round-2-plan describes
  (permissions, size class, media class, SCM, conflict
  check, file count, etc.) plus the two toggles plus the
  load-bearing explanatory copy (the verbatim ~70-word
  paragraph in §"UI surface").
* Wiring through both `pickAndAdd` (new-drive flow) AND
  per-drive Settings (`-a-76` covers Settings, but enable
  semantics need a shared service surface).
* Tooltip / info copy + accessibility + i18n-ready
  strings.

Conservatively this is 3-5x larger than the "small
extension" framing in the task body. Probably 200-400
LOC SPA + Rust IPC + several tests.

### Routing options

* **(A) Hold pickup until `systacean-27` lands**, then
  build with the API in hand. Lower risk of wasted
  stub-wiring churn; serialises with `-27` so total wall-
  clock is slower.
* **(B) Build a minimum-viable pre-flight surface now**
  using the existing add-drive flow as the foundation:
  insert a modal between picker + `add_drive` carrying
  the two toggles + the load-bearing copy + a stub
  "Show pre-flight report" placeholder. Persist via a
  new chan-desktop config field (`per_drive_features:
  HashMap<String, FeatureFlags>`) keyed by drive path
  until `-27` ships, then swap to chan-drive API.
  Bigger now, smaller later when `-27` lands and the
  stub gets replaced with the real API call.
* **(C) Split `-b-28` into `-b-28a` (toggles in
  launcher row UI, no full pre-flight screen) and
  `-b-28b` (full pre-flight screen with report + toggles
  + copy)**. `-28a` ships immediately as a narrow
  extension; `-28b` waits for `systacean-27` + a more
  thorough design pass.
* **(D) Defer entirely** until `systacean-27` lands +
  the launcher pre-flight foundation is built as a
  separate `-b-N` task. `-b-28`'s scope as stated
  isn't directly buildable today.

Recommendation: **(C)** — narrow the immediate landing
to the toggle UI (e.g. an expandable per-drive row in
the launcher table showing two checkboxes + the brief
copy), stub the persistence until `-27`, surface a
larger `-b-28b` for the full pre-flight report screen
when `-27` is in HEAD. Preserves momentum without
guessing the full pre-flight UX in advance.

Holding on implementation. No code edits yet.
