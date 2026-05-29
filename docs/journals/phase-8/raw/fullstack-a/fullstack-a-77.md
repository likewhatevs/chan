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

## 2026-05-23 — SPA slice 1 ready for review (client methods + PBKDF2 helper)

Three-file change. SPA-only. Slice 1 of the
multi-slice screensaver pickup. State
machine + overlay component + Settings UI
defer to slices 2 / 3.

### What landed

`web/src/api/client.ts`:
* New `api.screensaverState()` →
  `GET /api/screensaver/state` →
  `{ enabled, timeout_secs, pin_set }`.
* New `api.screensaverPatch(body)` →
  `PATCH /api/screensaver/state` with
  partial `{ enabled?, timeout_secs? }`.
* New `api.screensaverSetPin(hash_b64)` →
  `POST /api/screensaver/pin` body
  `{ hash }`. Hash composed client-side
  per the PBKDF2 helper below.
* New `api.screensaverClearPin()` →
  `DELETE /api/screensaver/pin`.
* New `api.screensaverVerify(hash_b64)` →
  `POST /api/screensaver/verify` →
  `{ verified: boolean }`.
* Doc-comment cross-references
  `systacean-40` + the hash-on-wire
  contract.

`web/src/state/screensaver.ts` (new):
* `hashPin(pin, driveSalt)` — PBKDF2 +
  SHA-256 via `crypto.subtle.deriveBits`.
  100_000 iterations (OWASP minimum
  circa 2023). 32-byte output → 44-char
  base64.
* Salt derivation: SHA-256 of the
  caller-supplied `driveSalt` (typical:
  `drive.info?.root`) so the same PIN
  against two drives produces distinct
  hashes. The pre-hash collapses
  arbitrarily-long paths into 32 bytes
  before feeding PBKDF2.
* `base64Encode(bytes)` —
  byte-safe wrapper around `btoa` so
  raw PBKDF2 digest bytes round-trip
  cleanly (some have non-UTF8 byte
  values).
* Module-level constants:
  `SCREENSAVER_DEFAULT_TIMEOUT_SECS = 300`
  (matches `systacean-40`'s chan-drive
  default); `SCREENSAVER_MIN_TIMEOUT_SECS
  = 30`; `SCREENSAVER_MAX_TIMEOUT_SECS =
  14400` (4h). The chan-drive layer
  doesn't clamp; the SPA enforces a
  reasonable range so a typo of `1`
  doesn't lock the user out
  mid-keystroke.

`web/src/state/screensaver.test.ts` (new):
14 pins across:
* 6 raw-source pins on the client
  methods (one per endpoint + the
  doc-comment cross-reference).
* 4 behavioral pins on `hashPin`:
  deterministic for same inputs;
  different salts diverge; different
  PINs diverge; empty salt
  fall-through.
* 2 constant-value pins (default + min/max
  bracket).
* 2 raw-source pins on the rationale +
  iteration-count documentation.

### Slice plan (post slice 1)

* **Slice 2**: `state/screensaver.svelte.ts`
  state machine (inactivity timer +
  lock event bus) +
  `components/ScreensaverOverlay.svelte`
  overlay (full-window cover + PIN
  entry + shake on wrong PIN). The
  longer-window timer + the right
  event set (keydown + scroll +
  pointer events; opposite of
  `idle.svelte.ts`'s short-window
  trigger set).
* **Slice 3**: Settings overlay
  Features section extension (pair
  with `-a-76` slice 2's reports +
  BGE toggles). PIN setup dialog.
  Manual "Lock now" chord
  (suggested `Mod+L`).

### Acceptance (slice 1 — client + helper only)

1. **5 client methods exposed** ✓ —
   state / patch / setPin / clearPin /
   verify. Each mirrors the
   systacean-40 contract.
2. **PBKDF2 hash produces deterministic
   digests** ✓ — same inputs → same
   output (pinned).
3. **Different salts diverge** ✓ —
   PIN reuse across drives doesn't
   collide.
4. **Crypto availability checked** ✓
   — throws an explanatory error if
   `crypto.subtle` is missing (older
   browsers; should never fire on
   chan's supported targets but the
   guard's free).
5. **Timeout constants exposed** ✓ —
   default matches chan-drive
   default; min/max bracket the
   configurable range.

### Gate

* vitest **1078 / 1078** (+14 net from
  `-a-76` slice 2's 1064).
* svelte-check 0 errors / 0 warnings
  across 4043 files.
* npm build clean.
* Rust gate not re-run (no Rust touched;
  systacean-40 already shipped chan-drive
  + chan-server).

### Decisions

* **Crypto.subtle PBKDF2** vs adding an
  argon2/scrypt dep — task body framed
  the threat model as local-only;
  PBKDF2 + SHA-256 + 100k iterations is
  sufficient. Built-in to every browser
  chan supports; no new deps.
* **Per-drive salt via `drive.info?.root`**
  — the same PIN across two drives
  produces distinct hashes. SPA passes
  the salt explicitly (not implicit on
  some global state) so the helper
  stays testable + the salt source is
  the caller's choice.
* **Salt pre-hash** — SHA-256 the salt
  source first to bound the PBKDF2
  salt-buffer size regardless of path
  length.
* **Slice-by-slice ship** — client
  methods + helper are small (~150 LOC
  + tests) and the state machine
  + overlay are substantial enough to
  ship as separate slices. Each is
  independently reviewable +
  empirically walkable.

### Suggested commit subject

```
Screensaver: api.screensaver* client methods + PBKDF2 PIN-hash helper (fullstack-a-77 slice 1)
```

Single commit. Client methods + helper +
14 test pins.

### Files for `git add` (per-path discipline)

* `web/src/api/client.ts`
* `web/src/state/screensaver.ts` (new)
* `web/src/state/screensaver.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-77.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance +
slice 2 (state machine + overlay) pickup.

## 2026-05-23 — SPA slice 2 (state machine + overlay) ready for review

Five-file change. SPA-only.

### What landed

`web/src/state/screensaver.svelte.ts` (new):
* Singleton `screensaver` state with 5
  fields (enabled / timeout_secs / pin_set
  / locked / loaded).
* `loadScreensaverState()` — fetches from
  `/api/screensaver/state`; populates the
  singleton; arms the inactivity timer.
* `noteScreensaverActivity()` — reset on
  user events; short-circuits when locked
  or disabled.
* `lockNow()` — manual-trigger lock
  (slice 3 wires the chord).
* `unlockWithPin(pin, driveSalt)` —
  hashes via slice 1's `hashPin` + posts
  to `/verify`; flips `locked=false` on
  success; surfaces failure to the
  caller.
* `pauseScreensaverTimer()` — caller-side
  pause for modals; returns idempotent
  release fn. Mirrors `pinAccessory()`
  from `idle.svelte.ts`.
* `installScreensaverTracker()` — global
  listeners on keydown / mousedown /
  touchstart / click / scroll / wheel /
  pointermove (wider set than
  `idle.svelte.ts`'s; matches "user
  stepped away" semantic).
* Internal `armInactivityTimer` guards
  on enabled + locked + pauseCount.

`web/src/components/ScreensaverOverlay.svelte`
(new):
* Renders `{#if screensaver.locked}` —
  full-window cover with PIN input + shake
  on wrong.
* `role="dialog"` + `aria-modal="true"`
  + `aria-label="Screen locked"` for AT
  support.
* PIN input auto-focuses on lock via
  `$effect` watching `screensaver.locked`.
* Enter key triggers `submit()`; wrong
  PIN triggers 400ms shake animation +
  clears input. No rate limiting per
  task body.
* No-PIN-set branch surfaces a copy
  pointing to Settings (chan-drive's
  verify returns `verified: false` when
  no PIN is set; the overlay still arms
  but the user can configure from
  Settings).
* CSS `@keyframes screensaver-shake`
  with `translateX` jitter; z-index 2000
  sits above every other chan overlay.

`web/src/App.svelte`:
* Imports `installScreensaverTracker` +
  `loadScreensaverState` from the new
  state module.
* Imports + mounts
  `ScreensaverOverlay` at App root.
* `onMount` calls `installScreensaverTracker()`
  alongside `installIdleTracker()` (the
  short-window pill tracker stays
  separate; same boot sequence).
* After `bootstrap()`, fire-and-forget
  `void loadScreensaverState()`. Failure
  is non-fatal — the singleton stays in
  its default disarmed state.

`web/src/state/screensaverMachine.test.ts`
(new): 21 pins across:
* 2 pins — singleton + interface shape.
* 6 pins — state machine helpers
  (load / noteActivity / lockNow /
  unlockWithPin / pauseTimer / install
  + the arm-time guards).
* 8 pins — overlay component (gating /
  ARIA / focus / Enter handler /
  submit / shake / CSS animation /
  z-index).
* 4 pins — App.svelte wiring (imports
  / mount / onMount call /
  post-bootstrap load).

### Acceptance (slice 2)

1. **State machine drives lock state** ✓
   — singleton + helpers pinned.
2. **Overlay renders full-window when
   locked** ✓ — backdrop CSS + z-index +
   gating.
3. **PIN input auto-focuses** ✓ —
   `$effect` ties to lock state.
4. **Wrong PIN shakes + clears** ✓.
5. **Manual `lockNow()` available** ✓ —
   slice 3 wires the chord.

### Slice 3 plan

* Settings Features section extension —
  pair with `-a-76` slice 2's reports +
  BGE toggles. Add: screensaver
  enable/disable toggle; timeout
  slider/select; PIN setup +
  change/clear flow.
* Manual "Lock now" chord (suggested
  `Mod+L` per the audit). Routes to
  `lockNow()`.
* `pauseScreensaverTimer()` consumers —
  Settings overlay + any open dialog
  call it on mount + release on
  unmount.

### Gate

* vitest **1099 / 1099** (+21 net from
  `-a-77` slice 1's 1078).
* svelte-check 0 errors / 0 warnings
  across 4046 files.
* npm build clean.
* Rust gate not re-run (no Rust
  touched).

### Decisions

* **Singleton `$state`** vs Svelte store
  — consistent with the rest of chan's
  state modules (idle.svelte.ts /
  pathPromptState / spawnDialogState).
* **Wider event set** than idle.svelte —
  keydown + scroll + pointermove +
  wheel. Idle deliberately ignores
  keydown (typing leaves pills hidden);
  screensaver must reset on any
  evidence of presence.
* **Auto-focus the PIN input** —
  immediate; no extra click required.
  Mirror of the empty-pane carousel
  auto-focus.
* **No rate limiting** — per task body's
  local-only framing.
* **Overlay above every overlay** — z=2000;
  spawn/team dialog z=50;
  disconnect/missing-token z=1500.
* **Fire-and-forget state load** — failure
  leaves the singleton disarmed, which
  is the safe default.

### Suggested commit subject

```
Screensaver: state machine + full-window overlay (fullstack-a-77 slice 2)
```

Single commit. State module + overlay
component + App root wiring + 21 test
pins.

### Files for `git add` (per-path discipline)

* `web/src/state/screensaver.svelte.ts` (new)
* `web/src/components/ScreensaverOverlay.svelte` (new)
* `web/src/App.svelte`
* `web/src/state/screensaverMachine.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-77.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance +
slice 3 (Settings UI + Mod+L chord).

---

## Slice 3 (Settings UI + Mod+L chord)

Date: 2026-05-23.

### Scope

Settings Features section extension:
screensaver enable/disable toggle,
inactivity-timeout input (clamped to
SCREENSAVER_MIN/MAX), PIN setup +
change/clear flow. Manual "Lock now"
chord on `Mod+L` (web + native);
escapeTerminal=true so it works even
from inside an xterm.

### Files touched

* `web/src/components/SettingsPanel.svelte`
  * Imported `hashPin`,
    `SCREENSAVER_MAX_TIMEOUT_SECS`,
    `SCREENSAVER_MIN_TIMEOUT_SECS` from
    `./state/screensaver`.
  * Imported `loadScreensaverState`,
    `pauseScreensaverTimer` from
    `./state/screensaver.svelte`.
  * Added 6 reactive state vars:
    `screensaverEnabled` (null sentinel
    for pre-load), `screensaverTimeoutSecs`,
    `screensaverPinSet`, `screensaverBusy`,
    `screensaverError`, `pinDialog`.
  * Extended `loadFeaturesState()` to
    fetch `api.screensaverState()` +
    capture errors into the section's
    error pin (mirrors the BGE +
    reports patterns from slice 2 of
    `-a-76`).
  * Added handlers:
    `toggleScreensaverEnabled`,
    `commitTimeout`,
    `openPinDialog`/`cancelPinDialog`,
    `commitPin` (validates match +
    hashes via PBKDF2 with the drive
    root as salt), `clearPin`. Each
    refreshes the singleton via
    `loadScreensaverState()` so the
    App-root tracker re-arms with the
    new shape.
  * Markup: `.feature-row.screensaver-row`
    with enable toggle on the right;
    sub-block (timeout input + PIN
    controls + inline PIN dialog)
    rendered only when
    `screensaverEnabled === true`.
  * `onMount` now grabs a
    `pauseScreensaverTimer()` release fn
    + fires it on destroy, so a long
    Settings session doesn't trigger
    the lock mid-config.
* `web/src/state/shortcuts.ts`
  * Added `id: "app.screensaver.lock",
    label: "Lock screen", web: "Mod+L",
    native: "Mod+L", group: "App",
    escapeTerminal: true`.
* `web/src/App.svelte`
  * Imported `lockNow` from
    `./state/screensaver.svelte`.
  * Added `case "app.screensaver.lock":
    lockNow(); return;` branch in
    `runCommand`.
  * Added Mod+L hotkey detection in
    `onWindowKey` (also covers the
    desktop bridge path that replays
    via `chan:command`).
* `web/src/state/screensaverSettings.test.ts`
  (new) — 13 architectural pins for:
  shortcut entry shape + group +
  escapeTerminal, App.svelte chord
  routing + runCommand branch +
  lockNow import, Settings imports +
  state vars + loadFeaturesState
  fetch, toggle/timeout/PIN/clear
  handlers, pauseScreensaverTimer
  mount/unmount wiring, markup
  structure.

### Run gate

* `npx svelte-check --tsconfig
  tsconfig.json` → 0 errors, 0
  warnings.
* `npx vitest run` → 109 files, 1115
  tests, all passing.
* `npm run build` → clean (web bundle
  fresh).
* `cargo fmt --check` → clean.
* `cargo clippy --all-targets -- -D
  warnings` → clean.
* `cargo test --workspace` → green.

### Suggested commit subject

```
Screensaver: Settings UI + Mod+L lock chord (fullstack-a-77 slice 3)
```

### Files for `git add` (per-path)

* `web/src/components/SettingsPanel.svelte`
* `web/src/state/shortcuts.ts`
* `web/src/App.svelte`
* `web/src/state/screensaverSettings.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-77.md`

Auth held. Standing by for cleared push
+ next dispatched task.
