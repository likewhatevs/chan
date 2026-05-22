# fullstack-a-77 — Screensaver with PIN unlock (Round-2 item 3)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: 2 wave-3

## Goal

Implement screensaver with PIN unlock per Round-2
item 3. Local-only screensaver protecting the drive
contents from over-the-shoulder viewing when the
user steps away.

## Reference

[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
+ §"Backlog item 3 — screensaver" (referenced at
line 53 + 76).

## Scope

### Behavior

* After N minutes of inactivity (configurable per
  drive), screensaver overlay appears.
* Drive contents hidden behind a blank or themed
  overlay.
* User unlocks via PIN (numeric or any user-chosen
  string).
* PIN stored hashed in chan-drive metadata.
* PIN can be set / changed in Settings.
* On unlock, drive view restores.

### Storage

* PIN hash (e.g. argon2 or scrypt) stored in drive
  metadata via chan-drive config.
* No external crypto / no over-the-network — local
  hash only per round-2-plan: "isn't needed for a
  local-only screensaver PIN".

### Triggers

* Inactivity timeout (default 5 min; configurable).
* Manual "Lock now" affordance (chord OR menu
  entry).
* On window blur / tab background? Implementer's
  call — most conservative is inactivity-only.

## Acceptance

1. Settings shows screensaver enable/disable +
   timeout + PIN setup.
2. After timeout, screensaver overlay covers drive
   contents.
3. PIN entry unlocks.
4. Wrong PIN: shake + error feedback; no rate
   limit needed (local-only).
5. Manual "Lock now" works.

### Tests

Vitest pins for the timeout logic + overlay state
+ PIN verification.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA SPA primary.
* If chan-drive needs config schema additions for
  PIN hash + screensaver settings, scope-poke to
  @@Systacean OR bundle if minimal.
* Atomic-audit-commit.

## Authorization

Yes for SPA screensaver + Settings + chord
handlers + tests + task tail + outbound. If
chan-drive PIN storage needs new config field,
scope-poke first.

## Numbering

This is `-a-77`.

## Out of scope

* Network-based auth.
* Multi-user / per-user PINs.
* Drive encryption (separate concern).

## 2026-05-23 — audit findings + scope-poke (chan-drive PIN storage gap)

Audit-only round. Same shape as `-a-70` /
`-a-76`. Architecture documented, chan-drive
piece routed.

### Audit summary

**Existing primitives (SPA)**:
* `web/src/state/idle.svelte.ts` —
  short-window idle tracker (5s default / 2.5s
  read-mode) wired to floating-pill fade.
  Watches mousedown / click / touchstart /
  selectionchange (deliberately NOT keydown or
  scroll). NOT suitable for screensaver
  re-use: the timing window is too short, the
  ignored-event set is wrong for "user
  stepped away" semantics (keystroke activity
  should reset, scroll should reset).
* `pinAccessory()` helper for short-window
  pause; same shape needed for screensaver
  pause when a modal / dialog is open.

**Existing primitives (chan-drive)**:
* No `pin_hash` / `screensaver_*` config
  field. `Drive::reports_enabled` /
  `set_reports_enabled` is the closest
  per-drive boolean toggle pattern.
* No crypto utilities. chan-drive doesn't
  ship argon2 / scrypt / bcrypt.

### Architecture decisions to make

**1. Storage location**: chan-drive
metadata (per-drive PIN) vs chan-server
config (per-machine PIN) vs SPA
localStorage (per-window PIN).

* Per-drive matches the task body's framing
  ("PIN stored hashed in chan-drive metadata")
  + matches the "drive contents hidden
  behind overlay" semantic. A user with
  multiple drives can have different PINs
  per drive.
* localStorage would lose the PIN on cache
  clear + doesn't cross devices (relevant
  for chan-desktop where a drive may move
  between machines via tunnel).
* chan-server config is per-machine — wrong
  granularity for the task body.

**Recommend chan-drive metadata** per task
body.

**2. Hash algorithm**: argon2id vs scrypt vs
PBKDF2.

* argon2id is the OWASP recommendation for
  password hashing. Memory-hard; resistant to
  GPU attacks.
* For LOCAL-ONLY screensaver per task body
  ("isn't needed for a local-only
  screensaver PIN"), the threat model is:
  someone with shell access to the local
  machine could read the hash but the
  screensaver itself isn't a serious
  security barrier.
* PBKDF2 with SHA-256 + 100k+ iterations is
  built into `crypto.subtle` (no extra deps);
  the hash isn't the bottleneck since the
  threat model is casual.

**Recommend PBKDF2 + SHA-256** for SPA-side
hash via `crypto.subtle.deriveBits`. Avoids
adding an argon2/scrypt dep + matches the
task body's "local-only" framing.

**3. Hashing layer**: SPA-side hashes
client-side + sends hash to chan-server vs
chan-drive does the hashing.

* SPA-side: client sends pre-hashed bytes.
  chan-drive just stores opaque
  `Vec<u8>`. Verification: SPA hashes
  candidate PIN + compares to stored.
  Server is hash-agnostic.
* chan-drive-side: client sends plaintext;
  chan-drive does the work. Heavier
  chan-drive dep set.

**Recommend SPA-side hashing**. Simpler
contract; matches the localStorage
fallback shape if the chan-drive piece
slips a milestone.

**4. Inactivity timer**: separate from
`idle.svelte.ts` (different cadence + event
set).

* New module `state/screensaver.svelte.ts`
  with a longer-window timer (5 min
  default; configurable).
* Event set: mousedown, click, keydown,
  touchstart, scroll — anything that
  indicates the user is at the keyboard.
* Pause for modals / dialogs (mirror
  `pinAccessory`).

### Scope-poke to @@Systacean (via architect)

`crates/chan-drive/src/drive.rs`:
* New `Drive::screensaver_pin_hash() ->
  Result<Option<Vec<u8>>>` reading the PIN
  hash from the index config (None when no
  PIN set).
* New `Drive::set_screensaver_pin_hash(hash:
  Option<Vec<u8>>) -> Result<()>`. None
  clears the PIN.
* New `Drive::screensaver_timeout_secs() ->
  Result<u32>` (default 300 = 5 min).
* New `Drive::set_screensaver_timeout_secs(secs:
  u32) -> Result<()>`.
* New `Drive::screensaver_enabled() ->
  Result<bool>`.
* New `Drive::set_screensaver_enabled(bool) -> Result<()>`.
* All four read/write fields go in
  `IndexConfig` next to
  `reports_enabled` / `semantic_enabled`.
* Rust pins for each round-trip.

`crates/chan-server/src/routes/`:
* New `screensaver.rs` (or extension to
  `preferences.rs`) with:
  * `GET /api/screensaver/state` → JSON
    `{ enabled, timeout_secs, pin_set }`
    (where `pin_set: bool` indicates
    whether a PIN exists — the hash itself
    never leaves the server).
  * `PATCH /api/screensaver/state` for
    enabled + timeout updates.
  * `POST /api/screensaver/pin` with body
    `{ hash: base64 }` to set the PIN.
  * `DELETE /api/screensaver/pin` to
    clear.
  * `POST /api/screensaver/verify` with
    body `{ hash: base64 }` returning
    `{ verified: bool }` — server compares
    against stored.

### SPA-side scope (post chan-drive landing)

* `state/screensaver.svelte.ts` — timeout
  state machine + lock event.
* `components/ScreensaverOverlay.svelte` —
  full-window overlay + PIN entry.
* `SettingsPanel.svelte` Features section
  extension (enable + timeout + PIN
  setup; pairs with the `-a-76` reports
  toggle).
* PBKDF2 hash via `crypto.subtle` for
  client-side hashing.
* Manual "Lock now" chord (suggested
  `Mod+L`; or a Hybrid Nav letter).

### No commit this round

Audit-only. Deliverable:
* This impl note documenting the
  architecture decisions.
* Outbound poke to architect for
  @@Systacean routing of the chan-drive +
  chan-server endpoints.

### Acceptance (pending chan-drive piece)

1. Settings shows screensaver section ✓
   (UI post-endpoint).
2. After timeout, overlay covers drive
   ✓.
3. PIN entry unlocks ✓ — SPA hashes
   candidate + posts to /verify.
4. Wrong PIN shake / error feedback ✓.
5. Manual "Lock now" works ✓.

### Suggested commit subject (when shipping)

```
docs(fullstack-a-77): audit + scope-poke for chan-drive screensaver PIN storage
```

### Files for `git add` (per-path discipline)

* `docs/journals/phase-8/fullstack-a/fullstack-a-77.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for the chan-drive +
chan-server endpoints + then the SPA-side
implementation.
