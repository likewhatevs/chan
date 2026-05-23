# systacean-40 — chan-drive screensaver PIN/timeout storage + chan-server endpoints (unblocks -a-77)

Owner: @@Systacean
Cut: 2026-05-23 by @@Architect
Status: dispatched

## Goal

Add chan-drive primitives + chan-server endpoints
for the screensaver feature (enable, timeout, PIN
hash). Unblocks @@FullStackA's `-a-77` (Screensaver
+ PIN unlock overlay).

## Reference

@@FullStackA's `-a-77` audit (`5810d4f`):
SPA-side overlay + state machine straightforward;
needs persistent server-side state for enable +
timeout + PIN hash. PIN hash NEVER leaves the
server in plaintext readable form — verify path
compares hashes server-side.

## Scope

### chan-drive (`crates/chan-drive/src/drive.rs`)

New methods on `Drive`:

* `screensaver_enabled() -> Result<bool>`
* `set_screensaver_enabled(bool) -> Result<()>`
* `screensaver_timeout_secs() -> Result<u32>`
  (default 300 = 5 min)
* `set_screensaver_timeout_secs(u32) -> Result<()>`
* `screensaver_pin_hash() -> Result<Option<Vec<u8>>>`
  (None when no PIN set)
* `set_screensaver_pin_hash(Option<Vec<u8>>) -> Result<()>`
  (None clears the PIN)

All four read/write fields live in `IndexConfig`
next to `reports_enabled` / `semantic_enabled`.

### chan-server (new `crates/chan-server/src/routes/screensaver.rs`)

* `GET /api/screensaver/state` → JSON
  `{ enabled: bool, timeout_secs: u32, pin_set: bool }`.
  `pin_set` indicates whether a PIN exists; the
  hash itself NEVER leaves the server.
* `PATCH /api/screensaver/state` body
  `{ enabled?: bool, timeout_secs?: u32 }` for
  enable + timeout updates.
* `POST /api/screensaver/pin` body `{ hash: base64 }`
  to set the PIN (server stores the base64-decoded
  bytes).
* `DELETE /api/screensaver/pin` clears the PIN.
* `POST /api/screensaver/verify` body `{ hash: base64 }`
  returning `{ verified: bool }`. Server compares
  candidate hash bytes against stored.

Wire in `lib.rs::router()` + re-export from
`routes/mod.rs`.

## Acceptance

1. `Drive::screensaver_*` methods round-trip
   (set then get returns the same value).
2. `IndexConfig` schema includes the new fields;
   migration safe for existing drives (defaults
   apply).
3. `/api/screensaver/state` returns current state
   including `pin_set` boolean (not the hash).
4. `/api/screensaver/verify` returns true for
   matching hash, false otherwise.
5. PIN clear via DELETE flips `pin_set` to false.
6. No regression on other config fields.

### Tests

* Rust pin per method: round-trip get/set.
* Rust pin per endpoint via fixture drive.
* Backward-compat: existing drive opens with
  defaults if fields not present.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* After this lands @@FullStackA wires SPA-side
  state machine + overlay component + PBKDF2
  hashing via `crypto.subtle` + Settings UI.

## Authorization

Yes for `crates/chan-drive/src/drive.rs` +
`IndexConfig` + `crates/chan-server/src/routes/screensaver.rs`
+ lib.rs + routes/mod.rs + tests + task tail +
outbound.

## Numbering

This is `-40`.

## Out of scope

* SPA-side overlay / state machine / PBKDF2
  hashing (`-a-77` lane).
* Specific hash algorithm choice — server stores
  whatever bytes the client posts (client uses
  PBKDF2 per `-a-77`'s audit).

## 2026-05-23 — implementation complete

Picked up `-40` per the dispatch.

### chan-drive (5 commits' worth in one file change)

* **`IndexConfig` extended** with 3 fields (`crates/chan-drive/src/index/config.rs`):
  * `screensaver_enabled: bool` (default false).
  * `screensaver_timeout_secs: u32` (default 300 via `default_screensaver_timeout_secs`).
  * `screensaver_pin_hash: Option<Vec<u8>>` (default None; serialized via custom serde module to base64 so the TOML stays text-only).
  * All three use `#[serde(default ...)]` so existing drives' `index/config.toml` opens with defaults — backward-compat verified by existing test that opens a `SCHEMA_VERSION - 1` config file.
* **6 `Drive::screensaver_*` methods** added (`crates/chan-drive/src/drive.rs`):
  * `screensaver_enabled()` / `set_screensaver_enabled(bool)`.
  * `screensaver_timeout_secs()` / `set_screensaver_timeout_secs(u32)`.
  * `screensaver_pin_hash()` / `set_screensaver_pin_hash(Option<Vec<u8>>)`.
* **4 facade setters** added (`crates/chan-drive/src/index/facade.rs`): atomic write parallels `set_semantic_enabled` / `set_reports_enabled`. Idempotent on no-change.
* **`base64`** added as a chan-drive workspace dep (was already a workspace dep from `-33`).

### chan-server (new file)

* **`crates/chan-server/src/routes/screensaver.rs`** (new):
  * `GET /api/screensaver/state` → `{ enabled, timeout_secs, pin_set }`. `pin_set` derived from `screensaver_pin_hash().is_some()` — the hash bytes NEVER appear on the wire.
  * `PATCH /api/screensaver/state` body `{ enabled?, timeout_secs? }` — partial update; returns post-update state.
  * `POST /api/screensaver/pin` body `{ hash: base64 }` — sets the hash; returns post-update state. Rejects invalid base64 with 400.
  * `DELETE /api/screensaver/pin` — clears the hash; returns post-update state.
  * `POST /api/screensaver/verify` body `{ hash: base64 }` → `{ verified: bool }`. Server-side **constant-time byte-equality** compare (prevents PIN-length / prefix-match timing leaks). Returns `verified: false` when no PIN is set.

Constant-time compare implemented locally (~10 LOC) instead of pulling `subtle` — keeps deps minimal + the algorithm doc-comment notes that a future bcrypt-style migration must preserve this property.

### Routing

* `PATCH /state` + `POST /pin` + `DELETE /pin` land in the **settings-writes lane** (flipping these is a settings change).
* `GET /state` + `POST /verify` land in the **unrestricted lane** (verify must work on shared-machine scenarios where the unlocker is a non-owner).
* All routes still gated by the per-launch bearer token (the auth middleware applies before the settings-writes lane).

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `Drive::screensaver_*` methods round-trip | ✓ (chan-drive test `screensaver_primitives_round_trip_and_default_correctly`) |
| 2 | `IndexConfig` migration safe for existing drives | ✓ (all 3 new fields are `#[serde(default ...)]`; existing schema-version-bump test verifies the migration path) |
| 3 | `/api/screensaver/state` returns `pin_set` boolean (not hash) | ✓ (verified explicit in `screensaver_pin_set_verify_clear_round_trip` — body MUST NOT contain the hash bytes) |
| 4 | `/api/screensaver/verify` returns matching hash → true, otherwise false | ✓ |
| 5 | DELETE flips `pin_set` to false | ✓ |
| 6 | No regression on other config fields | ✓ (chan-drive 464/0 was 463; chan-server 238/0 was 233) |

### Tests (+6)

* `chan_drive::drive::tests::screensaver_primitives_round_trip_and_default_correctly` — round-trip + defaults + idempotency.
* `chan_server::routes::screensaver::tests::screensaver_state_default_is_off_300s_no_pin` — defaults exposed via API.
* `chan_server::routes::screensaver::tests::screensaver_patch_updates_enabled_and_timeout` — partial PATCH semantics.
* `chan_server::routes::screensaver::tests::screensaver_pin_set_verify_clear_round_trip` — full PIN lifecycle + asserts hash is NEVER in any response body.
* `chan_server::routes::screensaver::tests::screensaver_set_pin_rejects_invalid_base64` — 400 on bad input.
* `chan_server::routes::screensaver::tests::screensaver_endpoints_require_auth` — 401 anonymous.

All chan-server tests run via `crate::router(state)` + `oneshot` (full router + middleware coverage, not just handlers in isolation).

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-drive --lib`: **464 / 0 / 2-ignored** (was 463; +1).
* `cargo test -p chan-server --lib`: **238 / 0** (was 233; +5).
* workspace tests all green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                                | +    | -  |
|-----------------------------------------------------|------|----|
| `crates/chan-drive/Cargo.toml`                      | +1   | 0  |
| `crates/chan-drive/src/index/config.rs`             | +68  | 0  |
| `crates/chan-drive/src/index/facade.rs`             | +54  | 0  |
| `crates/chan-drive/src/drive.rs`                    | +85  | 0  |
| `crates/chan-server/src/routes/screensaver.rs` (new) | +393 | 0 |
| `crates/chan-server/src/routes/mod.rs`              | +5   | 0  |
| `crates/chan-server/src/lib.rs`                     | +20  | -2 |

Plus task tail + outbound poke. 9 paths.

### Suggested commit subject

```
chan-drive + chan-server: screensaver storage primitives + /api/screensaver/* endpoints (systacean-40; unblocks -a-77)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-40-smoke`. Expected ALL GREEN.

### What this unblocks

`fullstack-a-77` (Screensaver overlay + PIN unlock). SPA wires:
* `api.screensaver.state/patch()` for enable + timeout config.
* `api.screensaver.setPin/clearPin()` for PIN management.
* `api.screensaver.verify(hash)` for unlock.
* Overlay state machine + PBKDF2 client-side hashing.

Per architect's pre-authorization, proceeding to commit + push + smoke.
