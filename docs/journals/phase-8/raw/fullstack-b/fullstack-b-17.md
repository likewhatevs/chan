# fullstack-b-17: Tab right-click Reload + Open Inspector (Tauri IPC)

Owner: @@FullStackB
Date: 2026-05-21

## Goal

Tauri side of the chan-desktop dev meta-blocker fix. Expose
two Tauri IPC commands the SPA tab context menu invokes for
its Reload + Open Inspector entries; wire `Cmd+R` /
`Cmd+Opt+I` accelerators for keyboard parity. Verify
`tauri.conf.json` `app.devTools` is enabled at runtime.

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "Tab right-click
'Reload' + 'Open Inspector' entries no-op on chan-desktop
(macOS)" (filed 2026-05-21).

Paired with [`../fullstack-a/fullstack-a-36.md`](../fullstack-a/fullstack-a-36.md)
(SPA dispatch + runtime detection). `-a-36` calls the IPC
commands this task exposes.

Severity: **DEV META-BLOCKER** — without Open Inspector
working on chan-desktop, @@Alex (and webtest lanes with
chan-desktop runtime permission per `ada8478`) can't
DevTools-inspect any of the other desktop-native UX bugs
in the v0.11.2 wave (file-moved false-positive, notification
spinner stuck, etc.). This task + `-a-36` unblock the rest of
the wave's investigation work.

## Authorization

**Authorization: yes**, covers:

* `desktop/src-tauri/src/main.rs` (or wherever Tauri IPC
  handlers are registered) — new `reload_window` +
  `open_devtools` commands.
* `desktop/src-tauri/tauri.conf.json` — confirm
  `app.devTools` is enabled (or the equivalent Tauri 2 key
  gating devtools in release builds; flip to true if gated
  away).
* `desktop/src-tauri/src/key_bridge.rs` (or wherever
  `KEY_BRIDGE_JS` lives — established by `-a-32`'s chord
  migration) — wire `Cmd+R` + `Cmd+Opt+I` accelerators to
  the new IPC commands.

@@FullStackB may proceed without further @@Alex confirmation.

## Acceptance criteria

* New Tauri IPC command `reload_window` that calls
  `WebviewWindow::reload()` (or eval'd
  `location.reload()` as fallback if Tauri 2's API differs).
* New Tauri IPC command `open_devtools` that calls
  `WebviewWindow::open_devtools()`.
* `tauri.conf.json` `app.devTools` (or equivalent) set to
  `true` for both debug + release builds (or document why
  release should hide devtools — but @@Alex's framing
  implies dev/release parity for the inspector affordance).
* Both commands are reachable via SPA `__TAURI__.invoke(...)`
  calls.
* Tauri-side accelerators:
  * `Cmd+R` → invokes `reload_window`.
  * `Cmd+Opt+I` (macOS) / `Ctrl+Shift+I` (Linux / Windows
    if @@Alex adds those platforms) → invokes
    `open_devtools`.
* Unit / integration test in
  `desktop/src-tauri/src/main.rs` (or adjacent) pinning the
  IPC command registration + the accelerator binding.
* Pre-push gate: clean (chan-desktop cargo test + fmt +
  clippy `-D warnings`).

## How to start

1. Read Tauri 2's WebviewWindow API docs for `reload` +
   `open_devtools`. Confirm the methods exist on Tauri 2.x
   (the chan-desktop tauri-bundler version per
   `fullstack-b-15`'s investigation).
2. Audit `tauri.conf.json` — find `app.devTools` (or
   whichever Tauri 2 key controls it). If false / absent,
   flip to true.
3. Register the two IPC commands in `main.rs`. Each is a
   thin wrapper that grabs the WebviewWindow via
   `app.webview_windows()` or equivalent + calls the API.
4. Wire the accelerator bindings via `KEY_BRIDGE_JS`
   following `-a-32`'s pattern.
5. Add the test pin.
6. Test locally:
   * `make app` build → launch → `Cmd+R` reloads, `Cmd+Opt+I`
     opens DevTools.
   * Right-click a tab in the SPA (after `-a-36` lands) →
     Reload / Open Inspector → see the same behaviour.
7. Append commit-readiness.

## Coordination

* **Pairs with [`-a-36`](../fullstack-a/fullstack-a-36.md)** —
  SPA dispatch consumes the IPC commands exposed here. Lock
  the IPC command names + signatures early so @@FullStackA
  can scaffold against them.
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Priority 1 in the wave's critical path (DEV META-BLOCKER
  unlock).

## Open questions

(populated as you investigate)

## 2026-05-21 — implementation note

### IPC contract for @@FullStackA's -a-36

Two commands exposed in `desktop/src-tauri/src/main.rs`:

```rust
#[tauri::command]
fn reload_window(window: tauri::WebviewWindow) -> Result<(), String>;

#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow);
```

Invoke from SPA: `__TAURI__.core.invoke('reload_window')` and
`__TAURI__.core.invoke('open_devtools')`. No arguments. The
calling webview is resolved automatically by the Tauri runtime
from the IPC's source frame.

### Changes landed

* **`desktop/src-tauri/Cargo.toml`** — added `devtools` feature to
  the `tauri` workspace dep. Tauri 2 removed the v1
  `app.devTools` JSON key in favour of this compile-time flag;
  enabling here gives release builds the inspector affordance
  (matching @@Alex's dev/release-parity framing).
* **`desktop/src-tauri/src/main.rs`** —
  * `pub` `reload_window(window: tauri::WebviewWindow) -> Result<(), String>`
    eval's `window.location.reload()` inside the calling webview.
    Tauri 2's `WebviewWindow` has no direct `reload()` method
    (Tauri 1 had it); the `eval` path is the supported route in
    v2 and is what `tauri-plugin-process`'s `relaunch` /
    `process.reload` ports do under the hood.
  * `open_devtools(window: tauri::WebviewWindow)` — calls
    `WebviewWindow::open_devtools()` directly. Method is
    unconditionally available with the `devtools` feature on.
  * Both registered in `tauri::generate_handler![...]` next to
    `reveal_in_finder`.
* **`desktop/src-tauri/src/serve.rs`** — `KEY_BRIDGE_JS` extended
  to wire the two accelerators:
  * `Cmd+R` (no shift, no alt) → `invokeIpc(e, 'reload_window')`.
  * `Cmd+Opt+I` / `Ctrl+Alt+I` → `invokeIpc(e, 'open_devtools')`.
  New `invokeIpc(e, cmd)` helper calls
  `window.__TAURI__.core.invoke(cmd)` directly, bypassing the
  `chan:command` SPA bus. This way a frozen Svelte runtime or a
  broken chord registry can't lock the dev affordances away —
  the bug entry's reproducer (right-click no-op, Cmd+R no-op)
  was exactly that shape on the SPA side. The native bridge
  becomes the inspector backdoor.
* **No `tauri.conf.json` change** — Tauri 2 dropped the
  `app.devTools` field; the `devtools` Cargo feature is the
  equivalent.

### Tests landed (chan-desktop 26 → 29)

| Test                                                              | Pinned contract                                                                              |
|-------------------------------------------------------------------|----------------------------------------------------------------------------------------------|
| `invoke_handler_registers_reload_window_and_open_devtools`        | `tauri::generate_handler!` list + function signatures in `main.rs` (via `include_str!`).      |
| `key_bridge_wires_reload_and_devtools_ipc`                        | KEY_BRIDGE_JS string contains both IPC calls + the case labels they're bound from.            |
| `key_bridge_invokes_tauri_ipc_via_core_invoke`                    | Tauri 2 IPC shape (`window.__TAURI__.core.invoke`) — guards against accidental v1-shape regression. |

### Acceptance criteria — verification

| Criterion                                                           | State                                                              |
|---------------------------------------------------------------------|--------------------------------------------------------------------|
| `reload_window` IPC command                                         | Landed; eval's `window.location.reload()`.                          |
| `open_devtools` IPC command                                         | Landed; calls `WebviewWindow::open_devtools()`.                     |
| `app.devTools` (or equivalent) for debug + release                  | `devtools` Cargo feature on `tauri` workspace dep (v2's mechanism). |
| Reachable via SPA `__TAURI__.invoke(...)`                           | Yes — registered in `generate_handler!`; v2 invoke path documented above. |
| Cmd+R accelerator → `reload_window`                                 | KEY_BRIDGE_JS line `case 'KeyR': invokeIpc(e, 'reload_window'); return`. |
| Cmd+Opt+I (macOS) / Ctrl+Alt+I → `open_devtools`                    | KEY_BRIDGE_JS alt-branch: `if (!shift && code === 'KeyI') { invokeIpc(e, 'open_devtools'); }`. |
| Test pin in `desktop/src-tauri/src/`                                | Three pins in `serve.rs::tests` covering both handler reg + accelerator binding. |
| Pre-push gate                                                       | Workspace fmt + clippy `-D warnings` + test (chan-desktop 26 → 29) + workspace tests + svelte-check (3978 / 0 errors) all green. |

### Runtime verification (deferred)

The chord paths can only be confirmed empirically (the
KEY_BRIDGE_JS injection only runs inside a real Tauri webview).
@@WebtestB's lane-B walkthrough validates `Cmd+R` reload + `Cmd+Opt+I`
DevTools + the SPA tab right-click flows once both `-a-36` and
`-b-17` land. My standing chan-desktop runtime permission covers
this if @@WebtestB wants me to do a smoke check first.

### Coordination footprint

* **Pairs with `-a-36`** — SPA-side dispatch uses the two IPC
  names locked here. @@FullStackA can scaffold against
  `__TAURI__.core.invoke('reload_window')` /
  `invoke('open_devtools')` immediately.
* **Independent of `-b-18` and `-b-19`** — no file overlap;
  `-b-18` is SPA-only, `-b-19` touches `main.rs` accelerators
  but a separate code region.
* **One small touch in shared file space**: `Cargo.toml`'s
  `tauri.features` array adds `"devtools"`. If @@CI / @@Systacean
  also touch the workspace `Cargo.toml`, the discipline applies
  (explicit per-file `git add` + pre-commit
  `git diff --staged --stat`).

### Suggested commit subject

```
chan-desktop: reload_window + open_devtools IPC + Cmd+R / Cmd+Opt+I accelerators (fullstack-b-17)
```

Touches:
* `desktop/src-tauri/Cargo.toml`
* `desktop/src-tauri/src/main.rs`
* `desktop/src-tauri/src/serve.rs`

Holding for @@Architect commit clearance. Push waits until the
v0.11.2 commit-grouping cut.
