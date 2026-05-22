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
