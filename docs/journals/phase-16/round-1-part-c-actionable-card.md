# Round-1 wave-3 follow-up DESIGN: reports-on-by-default + actionable P2 card

@@LaneC. DESIGN-FIRST per the wave-3 brief: posting WHAT the card options are,
HOW each wires, and WHERE the reports-default change lives, for @@Lead/@@Host
review BEFORE implementing. Pivots the P2 nudge (35aa69e8) from "Open Dashboard"
to inline actionable controls; keeps per-workspace dismiss. Files at impl:
`crates/chan-workspace/src/index/config.rs` (reports default) +
`web/src/components/PreflightOverlay.svelte` (card). Maybe one field on the
server summary (`preflight.rs`) - see Q1.

--------------------------------------------------------------------------------

## 1. Reports ON by default

WHERE: `crates/chan-workspace/src/index/config.rs:247`, the `Default for
IndexConfig` impl, `reports_enabled: false`.

CHANGE: flip that one line to `true`. Nothing else.

WHY that is exactly "new-on, existing-unchanged, no migration":
- `config::load()` (:300) returns `IndexConfig::default()` ONLY when the
  workspace has no `config.toml` yet (:302-303) - i.e. a brand-new workspace.
  So new workspaces boot with reports on.
- An EXISTING `config.toml` is deserialized as-is: a persisted
  `reports_enabled` (true or false) wins, and a LEGACY file that omits the
  field hits `#[serde(default)]` on the field (:164) = `bool::default()` =
  `false`, so it STAYS off. We deliberately do NOT touch the serde field
  default, only the struct Default, so existing workspaces never flip.

TESTS to update (same file):
- `reports_enabled_defaults_false_and_round_trips_true` (:369): the
  `assert!(!cfg.reports_enabled, "default must be false")` (:378) flips to
  assert true; rename to `..._defaults_true_...`.
- `missing field defaults to false` (:398): STAYS as-is. It is the
  no-migration guarantee for legacy configs and must keep asserting false.
- facade.rs fixtures at :1620/:1660/:1698 set `reports_enabled: false`
  EXPLICITLY (not via Default), so they are unaffected.

BEHAVIOR NOTE (call out to @@Host): a new workspace now runs the chan-report
scan at boot (tokei language detection + SLOC + COCOMO over every file) and
maintains it from watcher events. That is the intended cost of on-by-default;
it is incremental after the first scan. It does NOT add a locking pre-flight
step (reports has no readiness gate; only the embedding model does).

--------------------------------------------------------------------------------

## 2. Card = inline actionable controls (replace "Open Dashboard")

The card already reads `summary.{indexed_docs, scm, semantic_enabled,
reports_enabled}` from the pre-flight snapshot. Today it shows those + an
"Open Dashboard" button + "Dismiss". The pivot: turn the two optional layers
into inline controls that toggle via the existing routes; KEEP dismiss; DROP
the dashboard button. The routes already exist (used by the dashboard
SearchSlotConfig / WorkspaceSlotConfig), so this is wiring, not new backend.

### Reports control (simple)

A toggle reflecting `summary.reports_enabled` (now usually ON by default):
- turn off -> `api.reportsDisable()`; turn on -> `api.reportsEnable()`.
- Both return the fresh `ReportsState`; the card updates its toggle from the
  returned `enabled`. No model, no async fetch dance.

### Semantic search control (model-guarded - the one real decision)

`POST /api/semantic/enable` GUARDS on the embedding model being on disk
(index.rs:268 `resolve_model`); it errors if the BGE-small model is absent.
The dashboard's `SearchSlotConfig.semanticToggle` handles this with a
download-then-enable flow (download the ~63 MB model, poll, then enable).

Two ways to wire the card's "Enable Semantic search":

- OPTION A (RECOMMENDED, lean + no server change): optimistic enable, handle
  the guard. Click -> `api.semanticEnable()`. On success, on. On the
  model-missing error, swap to a "Semantic search needs the embedding model
  (~63 MB)" line + a single "Download & enable" button -> `api.semanticDownload()`
  (brief "downloading model..." state) then `api.semanticEnable()`. Disable ->
  `api.semanticDisable()`. The common case (model already downloaded from a
  prior workspace - the model is shared machine-wide) is one click; the
  first-ever download is two. No model picker in the card (that stays in the
  Dashboard Search panel).

- OPTION B (no download in-card): enable directly when the model is present,
  else fall back to a one-line "set up in the Dashboard Search panel" pointer
  for the download. Lighter card, but reintroduces a partial dashboard pointer
  for the uncommon model-missing case - against the spirit of "actual
  clickable options", so I lean A.

Either way the card does NOT need the model picker or per-model UI; it only
ever drives the default BGE-small via download+enable.

### State source (Q1 for review)

Option A needs no extra state (it reacts to the enable error). If we prefer to
show the model-present status UP FRONT (so the button reads "Download & enable"
vs "Enable" before the click), add one boolean `semantic_model_present` to the
server `summary` block (preflight.rs already resolves the model in `model_step`,
so it is a 1-line addition) rather than a second client fetch. RECOMMENDATION:
ship Option A without the extra field first (simplest); add `semantic_model_present`
only if @@Host wants the pre-click label to differ.

--------------------------------------------------------------------------------

## Open questions for @@Lead / @@Host

1. Semantic model-missing handling: OPTION A (in-card download+enable) vs
   OPTION B (pointer to Dashboard for download). I recommend A.
2. Card shape now that it is actionable: keep the read-only summary line
   (indexed/SCM) ABOVE the two toggles, or drop it for a pure controls card?
   I lean keep the one-line summary (cheap context) + the two controls +
   dismiss. "Don't restyle beyond what the options need" - so just turn the
   two layer rows into toggle rows, add the semantic download sub-state, keep
   everything else.
3. With reports on by default, the card's Reports control mostly shows "on"
   on first open - fine (it is still the place to turn it off). Confirm @@Host
   wants Reports shown as an OFF-able toggle in the card (vs hidden once on).

## Verify (at impl)

- chan-workspace: `cargo test -p chan-workspace` (the config default tests) +
  the full gate. Reports-default is a chan-workspace change, task-authorized
  here; commit pathspec.
- Card: svelte-check + npm build; then BROWSER-SMOKE the toggle actions
  (enable/disable reports, enable semantic incl. the model-missing path) -
  static gates miss Svelte-5 runtime reactivity, and this card now mutates
  real workspace state on click.
