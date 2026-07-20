# Command Launcher Flip Pane Is Dead

> Status: shipped in [v0.73.0](../../release/release-v0.73.0.md).

Implemented in `4e4dfeee`, merged as `5c748e8d`. Reported by the owner on 2026-07-20 and root-caused against `897f2bfc` the same day. The three behavioral tests were captured red against the unmodified tree before the fix landed; `make web-check` is green (svelte-check clean, launcher 294/294, workspace-app 2860/2860, both production builds). The end-to-end launcher-to-`flipHybrid` assertion was judged disproportionate and skipped, as the item permits: the production handler is private to `App.svelte` and reaching it needs an App mount harness.

**The live check exists.** `scripts/e2e/browser-smoke/checks/15-launcher-flip-pane.mjs` creates side B through the pane chrome plus a normal Dashboard spawn, drives the real Command Launcher row, and asserts the side glyph, the card label and the visible tab strip all switch sides. It was verified green against a rebuilt server, and proven able to fail: against the pre-fix parent commit it reports `pane pane-2 stayed on side A, expected B`.

That closes the one gap this item shipped with. The integrator had judged the check disproportionate, on the grounds that no command creates a B side, since `app.pane.splitRight` and `app.pane.splitDown` make new panes rather than a hybrid. That reasoning was wrong: the affordance exists in the pane chrome, and the lane found it. The check landed after the v0.73.0 tag and therefore rides in the following release, but the behavior it covers shipped in v0.73.0.

## Problem

Flipping the hybrid pane has three entry points. Two work and one does nothing.

- The ``Ctrl+` `` chord works. `web/packages/workspace-app/src/App.svelte:641-651` calls `flipHybrid(layout.activePaneId)` directly.
- The A/B pane control works. `web/packages/workspace-app/src/components/Pane.svelte:1351` calls `flipHybrid(pane.id)` directly.
- The Command Launcher row titled "Flip pane" does nothing at all.

The launcher row is registered, visible, selectable, and passes its `available` check. Selecting it dispatches, the dispatch is delivered, and the handler returns without flipping anything.

`app.pane.flip` is the only case in `runCommand` guarded by `paneChordBlocked()`:

```
      case "app.pane.flip":
        if (paneChordBlocked()) return;
        flipHybrid(layout.activePaneId);
        return;
```

`paneChordBlocked()` (`web/packages/workspace-app/src/App.svelte:525-546`) opens with `topOverlay() !== null`. The launcher is itself an overlay, and it is still on the overlay stack when it dispatches:

- `CommandLauncher.svelte:352-360` calls `closeCommandLauncher()` and then `entry.cmd.run(entry.arg)` on the next line, in the same synchronous task.
- `closeCommandLauncher()` (`web/packages/workspace-app/src/state/store.svelte.ts:2910-2912`) only writes `launcherPanel.open = false`.
- `overlayStack.ids`, which `topOverlay()` reads, is written solely by `syncOverlayStack()`, and the only non-test call site is a Svelte `$effect` (`App.svelte:314-319`). Under Svelte 5.56.4 that effect is batched and has not flushed. No `flushSync` or `tick` exists anywhere on this path.

So `topOverlay()` still returns `"launcher"`, the guard fires, and the command is swallowed. Because the flip is the only guarded case, every other launcher row is unaffected, which is why the defect reads as specific to flipping.

Two rows are dead, not one. The File Browser row titled "Flip" (`web/packages/workspace-app/src/state/commands/browser.ts:381-388`, id `app.browser.settings`) dispatches the same command and fails the same way.

This has never worked. The guard term landed in `14f2bd14` on 2026-05-29; the launcher row was added in `0d59d01f` on 2026-07-06, already blocked. There is no revision in which selecting the row flipped a pane.

The guard itself is correct and must stay. It fixes a real defect recorded at `web/packages/workspace-app/src/components/cmdCommaFlipGuard.test.ts:4-12`: with Search open, the flip silently reordered the panes behind the overlay, and the user discovered it only on dismiss.

## Why no gate caught it

The overlay stack lag is already known and worked around in the tests rather than fixed in the store. `web/packages/workspace-app/src/state/settingsOverlay.test.ts:45-50` has to call `syncOverlayStack()` by hand after closing an overlay before `topOverlay()` reads null.

Coverage of the flip is real but points at the wrong artifact. The A/B button has a behavioral test that clicks it and asserts the side changed (`components/Pane.test.ts:492-520`). The chord and the guard have only source-regex tests that read `App.svelte` with `?raw` and match literals (`cmdCommaFlipGuard.test.ts:16-54`). A regex over the guard can assert that it is broad enough; it can never assert that it is too broad. No test drives the launcher into `flipHybrid`, and `components/CommandLauncher.test.ts:8` mocks the real command catalog away, so its fixtures are `vi.fn()` spies that cannot observe the guard swallowing a dispatch.

No test mounts `App.svelte`. All 27 App-level test files read it as text.

## Desired contract

- Invoking "Flip pane" from the Command Launcher flips the pane, exactly as the chord and the A/B control do.
- The File Browser "Flip" row does the same.
- The guard keeps biting for real overlays. With Search or a modal open, a flip dispatched from any source stays a no-op.
- `topOverlay()` tells the truth to any caller that reads it immediately after an overlay closes, not one flush later.
- The regression check is behavioral. A source-regex assertion does not satisfy this item, because a source-regex assertion is what let the defect ship.

## Acceptance

- A test that invokes the launcher row and observes that the launcher is off the overlay stack by the time the command runs. It fails on the unmodified tree, and that red is recorded before the fix lands.
- A test pinning the store contract: closing the launcher leaves `topOverlay()` null with no manual `syncOverlayStack()` call in between.
- A test that the guard still blocks a launcher-dispatched flip while Search or a modal is open.
- Both dead rows reach `flipHybrid`.
- `make web-check` green, including the full vitest run, and the existing `?raw` pins in `cmdCommaFlipGuard.test.ts` and `cmdCommaFlipMatcher.test.ts` still green.
- The fix confirmed live in a rebuilt server. The frontend is embedded at build time, so a running server proves nothing about a tree change.

## Boundaries

- Do not weaken or delete `paneChordBlocked()`. It fixes a shipped defect.
- Do not add a dispatch-origin flag such as `viaLauncher` that lets one command opt out of the overlay term. That widens the command contract to hide a state-staleness bug, and the next caller of `topOverlay()` inherits the same lie.
- Do not change what `paneChordBlocked()` reads, in text or in meaning; the existing `?raw` pins depend on its literal shape.
- Do not restructure the command catalog, the launcher's ranking, or the overlay shell.
- The `app.settings.toggle` alias at `App.svelte:1157` is a separate question. It is pinned by `cmdCommaFlipMatcher.test.ts:15-17` and is not part of this item.
