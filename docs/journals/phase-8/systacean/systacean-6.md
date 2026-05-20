# systacean-6: Gate BGE-small embedding behind cargo feature + runtime model resolution

Owner: @@Systacean
Date: 2026-05-20

## Goal

Make the BGE-small semantic embedding model (~63 MB,
`crates/chan-server/resources/models.tar.zst`) opt-in at
build time via a new cargo feature, and add the runtime
lookup path so chan finds the model in
`<user-config>/chan/models/` when the feature is off.
Default build no longer ships the model; binary drops from
~89 MB → ~26 MB.

## Background

@@Alex requested the detour 2026-05-20 to shrink the first
release. The semantic-search model bloats every default
binary even though most users either don't need Hybrid
search or would prefer to fetch the model on demand.

Three-task split:

* **This task (`systacean-6`)** — build-side gating +
  runtime resolution.
* `systacean-7` — CLI subcommands + chan-server API
  endpoints (download, enable, disable, status).
* `fullstack-a-21` — Settings page UI.

This task is the foundation; the other two depend on it.

## Authorization

**Authorization: yes**, this task covers edits to
`crates/chan-server/Cargo.toml`,
`crates/chan-drive/Cargo.toml`, the workspace `Cargo.toml`,
`crates/chan-server/src/embed_seed.rs`, the `fetch-models`
crate, and the `Makefile` / `desktop/Makefile` rules that
invoke `make models`. @@Systacean may proceed without
further in-chat confirmation from @@Alex.

## Acceptance criteria

* New cargo feature `embed-model` on `chan-server` (or
  workspace-level if cleaner — your call). Default-off.
* `cargo build` (default) produces a chan binary that does
  NOT include `models.tar.zst`. Binary size drops to
  ~26 MB on release.
* `cargo build --features embed-model` (or
  `cargo build -p chan --features chan-server/embed-model`,
  whichever shape lands cleanly) produces the current
  ~89 MB binary with the model bundled. `make models`
  remains the prefetch helper for this path.
* `crates/chan-server/src/embed_seed.rs` conditionally
  compiles the `include_bytes!` and the unpack logic
  behind `#[cfg(feature = "embed-model")]`. The non-
  feature path stubs the function to "not embedded; see
  runtime resolver".
* Runtime resolver: when search wants to enter Hybrid
  mode and `MODEL_BUNDLE` is absent, look for the model
  in `<user-config>/chan/models/<model-name>/` (per
  platform: Linux `~/.local/share/chan/models/`, macOS
  `~/Library/Application Support/chan/models/`, Windows
  `%LOCALAPPDATA%\chan\models\`). Use the
  `dirs`/`directories` crate or whatever chan already
  uses for user-config paths (audit; don't add a new
  dep).
* **Forward-compat for Round-2 model picker**: the
  resolver indexes by **model name**, not a single
  hardcoded path. Default is the existing
  `BAAI/bge-small-en-v1.5` (already in
  `crates/chan-drive/src/index/facade.rs:1336`). Round-2
  task adds a curated-list picker that swaps the model
  name; resolver just needs to look up the correct
  subdirectory under `<user-config>/chan/models/`.
  Keep the API shape `resolve_model(name: &str)` → path.
* If the model is present at the runtime path, search
  initialises in Hybrid mode normally.
* If the model is NOT present and the user requested
  Hybrid, return a structured error to the caller (the
  CLI / API will surface "model not downloaded; run
  `chan index download-model` or enable in Settings").
* Default search mode (no model present, no user request
  for Hybrid) is BM25-only — same as today's
  `SearchMode::default()`.
* `fetch-models` crate stays in the workspace as the
  build-time helper for `--features embed-model` builds.
  Audit it for any runtime-side reuse opportunities (the
  download URL + extraction logic; if cleanly factored
  out, the runtime download in `systacean-7` can reuse).
* Pre-push gate: fmt + clippy `-D warnings` + workspace
  test + `cargo build` (default) + `cargo build --features
  embed-model` (both paths must compile clean).
* Tests:
  * Existing tests in `crates/chan-drive/src/index/`
    that exercise BGE-small need a guard: skip if model
    isn't present in the resolver path (or compile-time
    gate them behind the feature).
  * New test pinning the default build does NOT ship the
    bundle (compile-time check or runtime: assert
    `MODEL_BUNDLE.len() == 0` when feature is off).

## How to start

1. Audit current state:
   * `crates/chan-server/src/embed_seed.rs` — `MODEL_BUNDLE`
     `include_bytes!` site (line 21 per current state) +
     downstream extract / cache logic.
   * `crates/chan-server/Cargo.toml` — current rust-embed +
     candle deps.
   * `crates/chan-drive/src/index/facade.rs` — where
     SearchMode::Hybrid initializes; finds where the
     embed_seed model gets consumed.
   * `fetch-models/Cargo.toml` + crate source — the
     build helper. Understand its URL + extraction shape
     for runtime reuse later.
2. Add the `embed-model` feature flag to `chan-server`.
   Wire it through any downstream crates that depend on
   the model being available (chan-drive likely).
3. `#[cfg(feature = "embed-model")]` the `include_bytes!`
   site. The non-feature path returns a zero-length
   slice or a sentinel.
4. Implement the runtime resolver:
   * New helper in `chan-server` (or `chan-drive`,
     wherever the search-mode init reads the model)
     that returns either the embedded bundle (feature
     on) OR the path to a runtime-resolved model file
     (feature off + model present in user-config dir) OR
     an error (feature off + no runtime model).
5. Update SearchMode::Hybrid init to consume the
   resolver. If error, return the structured error all
   the way up to the API / CLI surface.
6. Default mode stays BM25-only when no model is
   available; no behaviour change for users who don't
   opt in.
7. Adjust tests that assumed embedded model.
8. Run both `cargo build` (default) and `cargo build
   --features embed-model`; measure binary sizes.

## Coordination

* `systacean-7` (CLI + API) depends on this task's
  runtime resolver being in place + the structured
  error type being exported.
* `fullstack-a-21` (Settings UI) depends on
  `systacean-7`'s API endpoints landing.
* No webtest verification needed at this layer (build
  + unit-test gate is sufficient); end-to-end verifies
  through `systacean-7` + `fullstack-a-21`.
* Coordinate with @@CI on `ci-5` (BGE model dir cache).
  Once this lands, the cache key in `ci-5` only matters
  for `--features embed-model` builds; default-build
  CI paths don't need the cache. @@CI may want to
  re-shape `ci-5`'s cache scope after this lands;
  flag for them via journal note.

## 2026-05-20 — implementation + commit

### Feature graph

| Crate         | Feature       | Implies                              | Default |
|---------------|---------------|--------------------------------------|---------|
| `chan-drive`  | `embeddings`  | candle + tokenizers + hf-hub         | on      |
| `chan-server` | `embeddings`  | `chan-drive/embeddings`              | on      |
| `chan-server` | `embed-model` | `embeddings` + `dep:tar + dep:zstd`  | **off** |
| `chan`        | `embeddings`  | server/drive `embeddings`            | on      |
| `chan`        | `embed-model` | `embeddings` + `chan-server/embed-model` | **off** |

The `tar` + `zstd` deps moved from `chan-server`'s
`embeddings` feature into the new `embed-model` feature
(they're only needed for the bundle decode). `embeddings`
keeps the candle stack so the runtime resolver path still
has the embedder code to invoke once the model is in
place.

### Changes

* `crates/chan/Cargo.toml`, `crates/chan-server/Cargo.toml` —
  new `embed-model` feature, default-off; existing
  `embeddings` keeps its current scope minus `tar`/`zstd`.
* `crates/chan-server/src/embed_seed.rs` — module gate
  changed from `#![cfg(feature = "embeddings")]` to
  `#![cfg(feature = "embed-model")]`. Module doc updated
  to reflect the split.
* `crates/chan-server/src/lib.rs` — `seed_models_from_bundle`
  call site gated on `embed-model` instead of `embeddings`.
* `crates/chan-server/build.rs` — comment refresh; the
  stub file logic stays useful for `--features embed-model`
  builds without a prior `make models`.
* `crates/chan-drive/src/index/embeddings.rs` — new
  `EmbedError::ModelNotDownloaded { model_id, expected_dir }`
  variant; new public helpers `repo_dir_name(model_id)` +
  `resolve_model(model_id)`; private `resolve_model_in(
  model_id, cache_dir)` + `model_files_present(repo_dir)`
  for testability.
* `crates/chan-drive/src/index/facade.rs::embedder()` —
  calls `resolve_model` before `Embedder::open`, so a
  missing model surfaces `ModelNotDownloaded` instead of
  triggering hf-hub's silent network fetch. Doc comment
  updated.
* `Makefile` — header comments split `make build` (lean
  default, no model) from `make build-release` (now
  passes `--features embed-model`, ships the model). `make
  rpm` also passes `--features embed-model` so the .rpm
  artifact keeps its current shape. `make models` stays as
  the model prefetch helper.

### Path-discrepancy note (deferred decision for @@Architect)

The task spec mentions
`<user-config>/chan/models/<model-name>/` with platform
paths:
* Linux `~/.local/share/chan/models/` (`dirs::data_dir`)
* macOS `~/Library/Application Support/chan/models/` (`dirs::data_dir`)
* Windows `%LOCALAPPDATA%\chan\models\` (`dirs::data_local_dir`)

Today chan-drive's `global_models_dir()` uses
`dirs::cache_dir()`:
* Linux `~/.cache/chan/models/`
* macOS `~/Library/Caches/chan/models/`
* Windows `%LOCALAPPDATA%\chan\models\`

The macOS + Linux paths differ; Windows matches. I kept
`global_models_dir()` as-is for backward compatibility:
migrating existing installs (which have models in
`cache_dir`) to `data_dir` is a breaking change that
warrants its own task. Flagging for @@Architect: should
`systacean-N` cover a data-dir migration, or do we accept
the existing cache-dir location as the canonical path?
The task spec's "whatever chan already uses for
user-config paths" gives me the out to defer this, but
the spec's explicit paths suggest the migration is
intended downstream.

### Desktop sidecar consideration

`desktop/Makefile::chan-bin` builds `cargo build --release
--bin chan` with default features, so the desktop sidecar
inherits the new lean default (no embedded model). For
Hybrid search in the desktop app to work out-of-the-box,
either:

(a) Desktop opt into `--features embed-model` (binary
    grows back to ~89 MB plus the Tauri shell on top).
(b) Desktop leans on systacean-7's first-launch download
    UX through `fullstack-a-21`'s Settings page.

I left `desktop/Makefile` unchanged; (b) seems more in
line with the task's intent (download-on-demand UX).
Flagging for @@Architect: should desktop bundle the model
by default to avoid the first-launch download friction, or
trust the Settings flow to surface it?

### Gate

* `cargo fmt --check` — clean.
* `cargo clippy -p chan -p chan-server -p chan-drive --all-targets -- -D warnings` (default features) — clean.
* `cargo clippy -p chan -p chan-server -p chan-drive --all-targets --features embed-model -- -D warnings` — clean.
* `cargo build -p chan` (default) — clean.
* `cargo build -p chan --features embed-model` — clean.
* `cargo test --all` — 422 tests in `chan-drive` lib (was 417 pre-task; 5 new tests from this task).
* `cargo test -p chan-server --features embed-model embed_seed` — 6 tests green (previously gated on `embeddings`, now on `embed-model`; same suite, same coverage).

### Binary size measurement

`cargo build --release -p chan` (default features):

```
-rwxr-xr-x  25M  target/release/chan
```

`cargo build --release --features embed-model -p chan`:

```
-rwxr-xr-x  89M  target/release/chan
```

| Build       | Size  | Delta            |
|-------------|-------|------------------|
| Default     | 25 MB | baseline         |
| embed-model | 89 MB | +64 MB (bundle)  |

Default drops 64 MB (the BGE-small bundle). Matches the
task's "~89 MB → ~26 MB" target; the 1 MB delta from "26"
is target/host-specific noise (lto + strip + macOS
aarch64).

### Behavioural notes

* Default `cargo build` produces a chan binary that:
  * Compiles candle + tokenizers + hf-hub (search code).
  * Does NOT include the model bundle.
  * On Hybrid search request with no downloaded model:
    `EmbedError::ModelNotDownloaded` propagates up;
    `IndexError::Embed` carries it; the CLI / API
    (systacean-7) renders the user-friendly hint.
  * On Hybrid search request with the model downloaded
    via systacean-7's CLI (or any other means putting the
    model under `global_models_dir()`): `resolve_model`
    succeeds, `Embedder::open` hits hf-hub's cache (no
    network), search proceeds.
* `--features embed-model` build behaves like
  pre-systacean-6: bundle ships in the binary, seeder
  extracts on first launch, `resolve_model` succeeds
  because the seeder populated the cache.
* hf-hub network fetch is now blocked at the `resolve_model`
  gate; only explicit download flows (systacean-7,
  `fetch-models`) trigger the network path.

### Status

Committed as `8b35c03`:

```
Gate BGE-small model behind embed-model cargo feature + runtime resolver (systacean-6)
```

8 files (`Makefile`, `crates/chan/Cargo.toml`,
`crates/chan-server/Cargo.toml`,
`crates/chan-server/build.rs`,
`crates/chan-server/src/embed_seed.rs`,
`crates/chan-server/src/lib.rs`,
`crates/chan-drive/src/index/embeddings.rs`,
`crates/chan-drive/src/index/facade.rs`), +269 / -38.
Push held pending Round-1 close + systacean-7 / -fullstack-a-21
landing.

Pre-commit audit (`git diff --staged --stat` before
commit) clean per the systacean-4 lesson — no stowaway
files this time.

### Open questions for @@Architect

1. data_dir migration — switch `global_models_dir()` from
   `cache_dir` to `data_dir` for Linux/macOS consistency
   with the task spec's stated paths, or accept the
   existing cache_dir location? (Breaking change for
   existing installs.)
2. Desktop sidecar — should `desktop/Makefile::chan-bin`
   opt into `--features embed-model` for out-of-the-box
   Hybrid, or rely on the Settings-driven download UX?
3. systacean-7 prerequisites — the `ModelNotDownloaded`
   error carries `model_id` + `expected_dir`. Both fields
   are public on the variant so the CLI / API layer can
   render the user-facing message however it wants. Let
   me know if you want a different shape (e.g. include
   the candidate download URL on the variant for offline
   diagnostics).

## 2026-05-20 — @@Architect: approved + cleared (already committed)

Reviewer: @@Architect.

Excellent work. Three things stand out:

1. **The feature graph table** (chan-drive `embeddings` →
   chan-server `embeddings` → chan-server `embed-model`)
   is the right shape — moving `tar` + `zstd` out of
   `embeddings` and into `embed-model` keeps the
   embedder code compileable everywhere, with the
   bundle-decode path gated only where it's actually
   used.
2. **Binary size delta** (25 MB default vs 89 MB with
   `embed-model`) lands the task's headline savings.
   The 1 MB delta from the spec's "~26 MB" target is
   noise; we've hit the goal.
3. **Forward-compat resolver shape**
   (`resolve_model(model_id)` returning a path) is
   exactly what the Round-3 multi-model picker needs.
   The `EmbedError::ModelNotDownloaded { model_id,
   expected_dir }` variant is also forward-compat for
   the picker since it carries the model_id.

Pre-commit audit clean (per the systacean-4 lesson). 8
files, +269 / -38. Tests cover both feature paths.

**Cleared (already committed)**: `8b35c03`. Push waits
until end of Round 2 (no Round-1 binary cut).

### Answers to your three open questions

**Q1 — data_dir migration**: defer. Switch the resolver
to `dirs::data_dir()` in a Round-3 cleanup task (likely
falls under the "config currency audit" item 5 since the
config path family wants a consistency pass anyway). The
cache_dir status quo is acceptable for now because:
* chan hasn't shipped publicly yet; existing installs are
  dev machines, migration cost is essentially zero.
* Round 3's release-readiness pass is the right slot for
  one-shot migrations + path-naming hygiene before the
  public flip.
* The Round-3 task can include a one-shot move-on-first-
  access: if `cache_dir/chan/models/<name>/` exists and
  `data_dir/chan/models/<name>/` doesn't, rename. Two
  releases of grace, then drop the migration path.

For now, your `global_models_dir()` returning `cache_dir`
is fine. Don't re-commit.

**Q2 — Desktop sidecar bundling**: leave
`desktop/Makefile::chan-bin` on default features (small
binary). Option (b) from your note. Reasoning:
* The whole point of the detour was the small binary;
  Tauri-shelling around the small binary while quietly
  re-bundling the model defeats the user-visible win.
* The Settings UI (`fullstack-a-21`) is the canonical
  enable + download path. First-time chan-desktop users
  see a small download (rather than a large
  installer).
* Power users / offline installs / enterprise can build
  with `--features embed-model` themselves (the cargo
  feature stays available); we don't need to ship that
  shape as the default.

Don't change `desktop/Makefile`.

**Q3 — ModelNotDownloaded error shape**: current shape
is good. `model_id` + `expected_dir` is enough — the
download URL is derivable from `model_id` via the
HuggingFace convention, and surfacing it on the error
variant would duplicate the canonical source of truth in
`fetch-models`. Keep the shape.

If the Round-3 multi-model picker ever ships a model not
on HuggingFace (e.g. self-hosted), THEN the URL becomes
non-derivable and we add it to the variant. Cross that
bridge when we hit it.

### What's next for you

* `systacean-7` (CLI subcommands + chan-server API
  endpoints) — the task that consumes this resolver.
  Pick up next.
* Future task (post-`systacean-7` commit): I'll cut a
  CI follow-up for @@CI to gate `ci-5`'s cache step on
  the `--features embed-model` workflow path. Not
  blocking your work.