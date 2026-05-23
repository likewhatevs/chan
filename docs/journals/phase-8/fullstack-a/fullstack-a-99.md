# fullstack-a-99 — Screensaver themes: Matrix rain + Castaway (mid-fidelity) + theme picker

Owner: @@FullStackA
Phase: 8, Round 3
Date cut: 2026-05-23
Priority: P1 — release-class for v0.13.0 (bundles with `-a-98` per @@Alex 2026-05-23)

## Goal

Ship the two screensaver visual themes the original `phase-7/next-phase-backlog.md` § 3 backlog item specified: **Matrix rain (default) + Castaway (alternate)**, with a Settings-side picker. What landed in `fullstack-a-77` is the PIN / state-machine / overlay scaffolding — the user-visible payload (the actual moving picture you see when locked) was never built. Close that.

## Background

`phase-7/next-phase-backlog.md:321-339` reads:

> **One-line**: add an inactivity-triggered screensaver overlay with an optional PIN unlock, plus two pickable visual themes (**Matrix rain by default, Castaway as an alternate**).
>
> ### Settings surface
> [...]
> * **Theme picker** — Matrix (default) | Castaway.

The dispatched `fullstack-a-77` task body (cut by the prior @@Architect session) dropped the theme specs — only mentioned "blank or themed overlay" without identifying which themes. Result: PIN infrastructure shipped under `-a-77 / -a-77c`, themes did not. @@Alex flagged the gap 2026-05-23; cutting `-a-99` to close it.

Binary-size budget (per @@Alex 2026-05-23 size estimate): **mid-fidelity Castaway**, target binary delta ~200-400 KB. Faithful Windows-3.1-Johnny-Castaway recreation is out of scope for v0.13.0.

## Scope

### 1. Matrix rain theme

Pure canvas + JS animation. ~200 LOC TS.

* Full-canvas backdrop sized to the overlay (covers the entire window behind the PIN input from `ScreensaverOverlay.svelte`).
* Columns of falling glyphs. Each column has:
  * Head glyph: bright (default green `#0f0`-ish).
  * Trailing glyphs: progressively faded toward black.
  * Glyph turnover per column at a randomized rate.
* Character set: half-width Katakana + ASCII digits + a few symbols (the canonical mix).
* Frame-rate target: 30 FPS is enough; 60 FPS if cheap. Use `requestAnimationFrame`.
* `prefers-reduced-motion: reduce` honored — drop to a static gradient or a once-per-second refresh, not the full animation.
* No external assets. Reference algorithm: `https://github.com/dcragusa/MatrixScreensaver` (Python; reimplement in TS). Many MIT-licensed JS reference implementations exist — feel free to consult.

Component file suggestion: `web/src/components/screensaver/MatrixRain.svelte` (or `.ts` if you prefer the pure-canvas-controller shape).

### 2. Castaway theme (mid-fidelity)

Pixel-art scene with simple animation states. Visual reference: the Windows 3.1 "Johnny Castaway" screensaver — a stranded character on a small desert island with the sea, palm tree, occasional events (ship passing, fish jumping, etc.). **Mid-fidelity target — not a full recreation.**

Acceptable scope:

* **One static base scene** (island + palm tree + sea horizon) as a sprite or hand-drawn pixel-art PNG (transparency where needed for sprite overlay).
* **A character sprite** (Johnny / Castaway figure) with 3-5 animation frames cycling at low frame rate.
* **5-10 distinct animation states**, selected randomly with weighted timing:
  * Idle (watching sea).
  * Waving at horizon.
  * Sitting under palm.
  * Sleeping (Zzz overlay).
  * Drinking from coconut.
  * Walking (a few steps left/right then turn).
  * Optional: fish jumping in the background; ship passing in the distance once in a while.
* **Day / night cycle** (slow gradient transition on the sky, ~5-min cycle when locked) — optional polish; drop if scope tightens.
* **No audio.**
* Asset bundle target: **~150-400 KB total PNG** sprite art. Power-of-two sprite sheet (e.g. 512×512 or 1024×512) loaded once + drawn via canvas. Avoid SVG for the character/scene art (PNG sprite sheet is more efficient at this resolution).

Component file suggestion: `web/src/components/screensaver/Castaway.svelte` + `web/src/components/screensaver/castaway-sprites.png` (single sprite sheet) + animation-state TS helpers.

Art sourcing: free / CC-licensed pixel art OK; @@FullStackA's judgment. License + attribution in a sidecar `castaway-LICENSE.md` if needed. AVOID copyrighted Windows-3.1 sprites — make the art "inspired by", not "ripped from".

### 3. Theme picker (Settings UI)

In `SettingsPanel.svelte` § screensaver:

* New row: **Theme** with `<select>` dropdown.
* Options: **Matrix** (default) | **Castaway**.
* Stored in per-drive preferences via `chan-server`'s screensaver API surface (extend `routes/screensaver.rs` schema — add `theme: ScreensaverTheme` field).
* On change: persists immediately; affects next lock.

If backend schema extension needed, scope-poke @@Systacean or bundle if minimal. Per the `-a-77` pattern, this is likely a small chan-drive config field addition.

### 4. Integration in ScreensaverOverlay

`web/src/components/ScreensaverOverlay.svelte` (existing):

* Reads the per-drive theme preference at mount.
* Renders `<MatrixRain />` or `<Castaway />` as the backdrop behind the PIN input.
* Default to Matrix if preference is unset / missing.
* Theme renders ONLY when `screensaver.locked` is true (no idle-render cost).

## Acceptance criteria

1. **Matrix rain** renders correctly when screensaver is enabled + theme = Matrix. 30+ FPS on common hardware; `prefers-reduced-motion` honored.
2. **Castaway** renders mid-fidelity scene + character + at least 5 animation states. Sprite sheet under 400 KB.
3. **Theme picker** in Settings UI; persists per-drive; takes effect on next lock.
4. **PIN entry** still functional on top of either theme; theme animates behind, PIN input is visually on top.
5. **Binary-size delta** within the mid-fidelity budget: ≤ 500 KB increase in embedded `web/dist`.
6. **Tests**:
   * Vitest pin for the theme-picker Settings UI (selection + persistence).
   * Vitest pin for the ScreensaverOverlay switching on theme preference.
   * Smoke test for the canvas renderers (existence + mount; the actual animation is visual and won't pin cleanly in vitest).
7. **Gate**: `npm run check` + `npm test -- --run` + `npm run build` green. `cargo build -p chan` clean (rust-embed picks up the new asset).

## How to start

1. Build a minimal Matrix-rain `MatrixRain.svelte` against a static `ScreensaverOverlay` test page (or via a debug query-string toggle). Get the rain rendering first — that's the visual proof. ~half-day-of-work scope.
2. Extend `chan-server`'s screensaver endpoint schema with `theme` (scope-poke @@Systacean if you want their hand on the chan-drive config field; or include it inline since it's small).
3. Wire Settings UI dropdown + persistence.
4. Castaway scene: source / draw the sprite sheet first (this is the longer pole). Mid-fidelity target — don't over-invest in art. Build the canvas controller around a simple state machine (current animation + frame index + tick).
5. Integrate both into `ScreensaverOverlay.svelte` behind the theme preference.
6. Walk locally against a throwaway drive: enable screensaver, set 30s timeout, watch each theme in sequence.

## Coordination

* Time-boxed: 1-2 sessions. Mid-fidelity Castaway, not faithful recreation.
* Safety guardrail: do NOT touch @@Alex's running chan.app session. Throwaway drives + dev builds.
* @@FullStackA queue: `-97` SHIPPED (awaiting @@WebtestA walk), `-96` sub-passes 1/2/3 polish (cleared, non-blocking), `-98` menu gaps (this wave), `-99` themes (this task). Sequence as you see fit; `-98` is probably faster (text-only menu work); `-99` is the longer build because of the art.
* @@Systacean: tiny chan-drive config field addition (`theme: ScreensaverTheme`) — happy to fold in or scope-poke them; either works.

## Authorization

Yes for:

* SPA edits (`web/src/components/screensaver/*`, `ScreensaverOverlay.svelte`, `SettingsPanel.svelte`, `web/src/state/screensaver.svelte.ts`).
* New asset files under `web/src/components/screensaver/` (PNG sprite sheets).
* New vitest pins.
* `chan-server` schema extension (`routes/screensaver.rs` + chan-drive config field) if you fold it in; or scope-poke @@Systacean.

## Out of scope

* Audio.
* Custom user-uploaded themes ("bring your own screensaver"). Future v1.x territory.
* Faithful Windows-3.1 Johnny Castaway recreation. Mid-fidelity only.
* Additional themes beyond Matrix + Castaway. Future feature requests.
* Performance optimization beyond hitting 30 FPS and honoring `prefers-reduced-motion`.

## Reference

* Original spec: [`../../phase-7/next-phase-backlog.md`](../../phase-7/next-phase-backlog.md) § "3. Screensaver with PIN unlock" (lines 321-407).
* PIN infrastructure already shipped: `fullstack-a-77` slices 1/2/3 + `-77c`.
* Storage: chan-server `routes/screensaver.rs` + chan-drive screensaver primitives from `systacean-40`.
* Matrix-rain algorithm reference: `https://github.com/dcragusa/MatrixScreensaver` (Python; algorithm only).
* Castaway visual reference (DO NOT copy art): the Windows 3.1 "Johnny Castaway" screensaver. Aesthetic, not assets.

## 2026-05-23 — scope amendment by @@Architect: screensaver timeout bounds

@@Alex 2026-05-23: "about the screensaver / screen lock: minimum time must be 10s, maximum 3600s".

### Add to -99 scope

Clamp the screensaver inactivity-timeout input to **[10s, 3600s]** (1 hour). Defense in depth:

1. **SPA-side** (`SettingsPanel.svelte` screensaver row): input element `min={10} max={3600}`; on-submit clamp + user-visible validation message if out of range. The existing timeout-input field needs the bounds adding.
2. **chan-server-side** (`crates/chan-server/src/routes/screensaver.rs` — the PATCH endpoint that stores timeout): reject `timeout_secs < 10 || timeout_secs > 3600` with `400 Bad Request` + a structured error message. Pin with a route-level test.
3. **(Optional) chan-drive-side**: chan-drive's screensaver storage doesn't need to validate (the boundary is at the API surface), but if there's a deserialization path that could ingest out-of-bounds data from disk (e.g., a manually-edited config file), consider clamping on read. Implementer's call.

### Acceptance addition

8. Screensaver inactivity timeout cannot be set below 10s or above 3600s via the UI; the chan-server PATCH endpoint rejects out-of-bounds writes with 400.

### Coordination

If chan-server validation is fastest as a fold-in alongside the `theme: ScreensaverTheme` extension you'll already be doing, bundle. If you'd prefer scope-poke @@Systacean for the chan-server piece, fine — small either way.
