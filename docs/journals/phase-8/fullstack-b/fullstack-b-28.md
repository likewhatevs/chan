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

## 2026-05-22 — @@Alex: option (C) — proceed with narrow -b-28a

@@Alex picked **(C)** in chat ("go with C"). Splitting the
task: `-b-28a` ships the narrow expand-panel toggle UI now
against stub persistence; `-b-28b` covers the full pre-flight
screen post-`systacean-27`.

## 2026-05-22 — implementation note (b-28a ready for commit clearance)

### Scope landed

Narrow per the (C) routing. Every drive row in the
chan-desktop launcher gains a ⚙ expand button that flips
a sibling row with the BGE + reports checkboxes + brief
explanatory copy. Default OFF for both; persisted via
chan-desktop's sidecar config (stub) until `systacean-27`
lands the chan-drive config API.

### Changes

* **`desktop/src-tauri/src/config.rs`** — new
  `DriveFeatures { bge: bool, reports: bool }` struct +
  `DriveSidecar.features: DriveFeatures` field. Both
  serde-default to false so existing `config.json` files
  load cleanly (pre-`-b-28a` entries roll forward as
  off-off). Four new unit tests pin the contract:
  default off; missing-field deserialise; round-trip;
  partial-field deserialise.
* **`desktop/src-tauri/src/main.rs`** — two new IPCs
  `get_drive_features(path)` + `set_drive_features(path,
  features)` mirroring `reclaim_drive_lock`'s pattern.
  Both registered in `generate_handler!`. Body reads /
  writes the sidecar `HashMap<key, DriveSidecar>` with
  the existing atomic ConfigStore save semantics. `-b-28b`
  will swap the body to call chan-drive's
  `Drive::set_feature_bge` / `set_feature_reports` from
  `systacean-27` without changing the IPC contract.
* **`desktop/src/main.js`** — new `renderFeaturesToggle()`
  + `renderFeaturesPanel()` + `bindFeaturesToggle()` +
  `loadFeaturesInto()` + `collectFeaturesFromPanel()`.
  Click on the ⚙ button flips the sibling
  `tr.features-panel`'s `hidden` attribute; first open
  lazy-loads state via `get_drive_features` so the IPC
  cost is paid only on drives the user actually
  inspects. Checkbox changes fire `set_drive_features`
  with optimistic update + revert on failure. Small
  `cssEscape` helper for the legacy-webview path
  (Tauri's WKWebView / WebView2 both ship `CSS.escape`
  today but the polyfill is cheap insurance).
* **`desktop/src/styles.css`** — `.features-toggle` +
  `.features-panel` + `.features-content` + per-row
  hint copy styles. Toggle button mirrors the
  existing icon-button shape (`button.btn.icon`); panel
  uses a brand-tinted background + dashed top border so
  it reads as a sub-section of the parent row.
* **`desktop/src-tauri/src/serve.rs::tests`** — three new
  structural pins:
  * `invoke_handler_registers_drive_features_ipcs` —
    both `#[tauri::command]` handlers registered in
    `generate_handler!`.
  * `launcher_calls_drive_features_ipcs` — main.js
    invokes both IPCs by exact name.
  * `launcher_features_panel_carries_round2_plan_toggles`
    — panel HTML carries the "Semantic search" +
    "Reports" labels and the `data-feat="bge"` /
    `data-feat="reports"` checkbox bindings.

### Pre-push gate (local, macOS aarch64; -b-28a scope only)

| Surface                                              | State                                       |
|------------------------------------------------------|---------------------------------------------|
| `cargo fmt --check` (my files)                       | Clean.                                      |
| `cargo clippy -p chan-desktop --all-targets -- -D warnings` | Clean.                              |
| `cargo test -p chan-desktop`                         | 51 tests (was 44 in `-b-27`; +4 config tests + 3 structural pins = +7 net). |
| `cargo build -p chan-desktop --no-default-features`  | Clean.                                      |
| `web/` `npx svelte-check`                            | 4012 files / 0 errors / 0 warnings.         |
| `web/` `npx vitest run`                              | 829/829 (one earlier flake on three timer-driven tests cleared on rerun; pre-existing pattern not from `-b-28a`). |
| `web/` `npm run build`                               | Clean.                                      |

Workspace-wide `cargo clippy` + `cargo fmt --check` flag
unrelated diffs in @@Systacean's WIP on chan-drive +
chan-server (their `IndexConfig::reports_enabled` field
addition is mid-flight; not mine to fix). I use
`cargo clippy -p chan-desktop` + `git commit --only`
to isolate.

### Files to stage

```
desktop/src-tauri/src/config.rs
desktop/src-tauri/src/main.rs
desktop/src-tauri/src/serve.rs
desktop/src/main.js
desktop/src/styles.css
docs/journals/phase-8/fullstack-b/fullstack-b-28.md
```

### Suggested commit subject

```
chan-desktop: per-drive feature toggle expand panel + stub persistence (fullstack-b-28a)
```

### -b-28b prerequisites

Architect cuts `-b-28b` when:

1. `systacean-27` is in HEAD (chan-drive `Drive::set_feature_*`
   API + BOOT integration).
2. (Optional) Pre-flight screen design lock if the full
   report (perms / size / SCM / etc.) needs a separate
   product-design review.

When -28b lands, it:
1. Replaces the IPC body to call chan-drive helpers
   instead of writing to the chan-desktop sidecar.
2. Adds the full pre-flight screen between
   `pickAndAdd` + `add_drive` carrying the report +
   verbatim explanatory copy from round-2-plan.
3. Migrates any in-the-wild stub-persisted feature
   pairs into chan-drive on first read (forward-compat
   shim; drop after one release cycle).

### Runtime walkthrough

Standing chan-desktop runtime perm is available if you'd
prefer a quick visual smoke. Otherwise routing to
@@WebtestB per the established lane boundary; webtest
walks the expand UI + checkbox flips + persistence
across restart.
