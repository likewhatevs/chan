# backend-3: Layout standard/compact config support

Owner: @@Backend as Backend+Rustacean.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [frontend-1.md](./frontend-1.md)
- [rustacean-1.md](./rustacean-1.md)
- [syseng-1.md](./syseng-1.md)

## Goal

Support the Settings / Layout change from `[tight] [standard]` to
`[standard] [compact]`, with `standard` as the default and compatibility for
existing saved `tight` values.

## Context

[frontend-1.md](./frontend-1.md) reports that the frontend label/CSS change
needs backend config support because `LineSpacing` currently serializes as
`tight | standard` and CLI parsing rejects `compact`.

## Acceptance criteria

- Server preferences accept and emit `standard | compact` for layout/line
  spacing.
- Default is `standard`.
- Existing persisted `tight` values deserialize as `compact` or otherwise keep
  working with a documented compatibility path.
- `chan config get/set` accepts the new value and preserves compatibility for
  old `tight` if needed.
- External compatibility decisions are documented in this task before commit.

## Test expectations

- Add focused Rust tests for preference/default/deserialize behavior.
- Add CLI config tests for `standard`, `compact`, and old `tight`
  compatibility if supported.
- Run focused `cargo test` packages first, then fmt/clippy as appropriate.

## Review expectations

- Backend slot is now Backend+Rustacean for this phase; enum/serde/CLI quality
  review can happen in the same slot, with @@Architect escalating only if a
  second Rust review is needed.
- @@Frontend confirmation that frontend-1 can wire the new values cleanly.

## Progress notes

### 2026-05-16 @@Backend: landed.

### Compatibility decision

- Wire / on-disk canonical tokens become `standard` and `compact`.
- Default flips from `Tight` to `Standard`.
- Legacy `tight` token from pre-phase-3 `preferences.toml` files
  deserializes as `Compact` via `#[serde(alias = "tight")]`. On any
  subsequent save the canonical `compact` token replaces it, so the
  compatibility shim self-erodes — no manual migration required.
- CLI (`chan config set editor.line_spacing tight`) also accepts the
  legacy token and stores it as `Compact`; `chan config get
  editor.line_spacing` echoes the canonical `compact`, nudging
  scripts toward the new spelling without breaking the old one.

This keeps the journal's "preserve compatibility where it's an
external schema name; rename user-visible surfaces" rule honored on
both the TOML schema (read-side alias, write-side canonical) and the
CLI surface.

### Files changed

- `crates/chan-server/src/preferences.rs` — `LineSpacing::Tight` →
  `LineSpacing::Compact`, default flipped to `Standard`,
  `#[serde(alias = "tight")]` on `Compact`, doc comment refreshed
  for the new tokens + legacy alias. Five new tests (default flip,
  canonical serialization, legacy-`tight` deserialize as compact +
  next-save flushes canonical, compact round-trip).
- `crates/chan/src/main.rs` — `parse_line_spacing` accepts
  `standard | compact | tight` (tight → Compact); `line_spacing_label`
  emits canonical lowercase. Four new tests
  (`config_line_spacing_accepts_canonical_tokens`,
  `..._legacy_tight_alias`, `..._rejects_unknown_value`,
  `..._label_round_trips`).

### Tests run

```
cargo test -p chan-server          # 107 passed
cargo test -p chan                 # 50+4 = 54 passed
cargo fmt --check                  # clean
cargo clippy --all-targets -- -D warnings   # clean
```

### Frontend-1 wiring guidance

Once @@Frontend (or @@WebtestB, per the recent reassignment) picks
this up, the changes on the SPA side are:

- `web/src/api/types.ts:327` — change
  `export type LineSpacing = "tight" | "standard";` to
  `"standard" | "compact"`. The server never writes `"tight"` again,
  but a tolerant union (`"tight" | "standard" | "compact"`) is also
  fine if you want to keep parsing old persisted in-memory blobs
  cleanly.
- `web/src/components/SettingsPanel.svelte:511-515` — swap the
  radio set from `[tight, standard]` to `[standard, compact]`;
  flip the labels accordingly.
- `web/src/editor/{Source,Wysiwyg}.svelte` — change the
  `data-density` fallback from `"tight"` to `"standard"` (match the
  new default) and rename the CSS selectors
  `[data-density="tight"]` → `[data-density="compact"]`. Adjust the
  compact line-height values to land between the old tight/standard
  pair: suggested `Wysiwyg` 1.65 (was 1.5 tight / 1.8 standard) and
  `Source` 1.55 (was 1.4 / 1.7), per the read-only review in
  [frontend-b-1.md](./frontend-b-1.md) "Layout setting:
  standard / compact".

The wire is now stable for that work; no further backend coordination
needed.

### 2026-05-16 @@Rustacean review

Reviewed the enum/serde/CLI changes. No blocking Rust issues found.

- `LineSpacing::Compact` with `#[serde(alias = "tight")]` is the right
  compatibility shape: old TOML reads successfully, new writes are canonical,
  and the alias does not leak back onto the wire.
- Defaulting `LineSpacing` to `Standard` via the enum default is explicit and
  covered by a regression test.
- CLI parsing accepts `standard | compact | tight`, normalizes `tight` to
  `Compact`, and emits only canonical `standard | compact` from
  `line_spacing_label`.
- The compatibility decision is documented clearly enough for commit review.
- No dependency or public route shape changes were introduced.

Verification:

```
cargo test -p chan-server line_spacing
cargo test -p chan config_line_spacing
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test -p chan-server -p chan
```

All passed. Also checked for remaining Rust `LineSpacing::Tight` references;
none remain. Remaining `tight` references are frontend wiring/fallback strings
already called out above for frontend follow-through.

## Commit readiness notes

Ready for @@Rustacean review of the serde alias + the CLI alias.

@@Rustacean review: passed.

Files changed:

- `crates/chan-server/src/preferences.rs`
- `crates/chan/src/main.rs`

Tests run (all green): `cargo test -p chan-server`, `cargo test -p
chan`, `cargo fmt --check`, `cargo clippy --all-targets -- -D
warnings`.

Known risks: none operational. Behavior change is observable on
existing drives — the default density flips from tight to standard on
first read after upgrade, because the in-memory default is consulted
only when `preferences.toml` is absent OR the `line_spacing` field is
missing from the TOML. Existing drives that have `line_spacing =
"tight"` written explicitly load as `Compact` (same density as before
under the new name).

Proposed commit message:

```
chan: rename LineSpacing::Tight -> Compact, default to Standard

Phase-3 Settings/Layout change: editor density now exposes
`standard | compact` instead of `tight | standard`, with
standard as the default. Existing `preferences.toml` files
with `line_spacing = "tight"` deserialize as Compact via a
serde alias and the next save flushes the canonical `compact`
token. `chan config set editor.line_spacing tight` is
accepted for muscle-memory compatibility and stored as
Compact; `chan config get` echoes the canonical spelling.
```
