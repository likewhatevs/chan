# systacean-20: cut v0.11.0 release

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

Tag `v0.11.0` for chan + chan-web. Phase 7
wrap. All Lane A + Lane B queues drained;
@@Alex has signed off on skipping the
final webtest comprehensive walk (per-task
green gates + unit-test coverage suffice).

## Version bumps

* **Workspace `Cargo.toml:20`**:
  `version = "0.10.1"` → `"0.11.0"`.
  This cascades to every crate in the
  workspace (chan-server, chan-drive,
  chan-llm, chan-report, chan-tunnel-*,
  chan, fetch-models) since they all use
  `version.workspace = true`.
* **`web/package.json:3`**:
  `"version": "0.10.1"` → `"0.11.0"`.
* **`Cargo.lock`** — regenerates on the
  next `cargo build`. Stage the diff.
* **`desktop/src-tauri/tauri.conf.json`**
  — version pin if present (audit; the
  Tauri shell may have its own version
  declaration).

## Release notes

Marquee surface for v0.11.0 (Phase 7's
output):

* **Hybrid NAV** (renamed from Pane Mode
  per `-62`): `Cmd+K`-prefixed pane-and-
  layout commands. Spawn keys 1/2/3/4
  with draft/commit staging (`-72`). WASD
  swap-tile, arrows focus-move (`-40`/`-72`).
  Cmd+K + p rich prompt (`-50`), f search
  (`-74`), `<` / `>` dock toggles (`-69`),
  Backspace kill-pane (`-77`), Tab
  flip-Hybrid. Clickable help overlay
  (`-63`). H-for-help flash on entry
  (`-61`/`-76`).
* **Flippable Hybrids** (`-48` A/B/C):
  pane has front + back sides, per-side
  themes (`-59`/`-78` propagation to
  xterm.js + GraphCanvas), back-side
  attention dot, split preserves side
  (`-70`).
* **Multi-FB and multi-Graph tabs** (`-47`,
  `-58` per-tab BrowserTab state, `-81`
  Graph tab title from selected node,
  `-84` per-tab inspector width).
* **Carousel** (`-44`/`-45`) +
  indexing-state slide (`-44` slide 3,
  consumes `systacean-18`).
* **British spelling sweep** (`-46`).
* **Tab chrome polish**: smart titles for
  Files (`-65`) and Graph (`-64`/`-81`),
  truncation utility (`-66`), FB header
  drops in tab + dock variants
  (`-67`/`-71`), Graph chrome rewrite
  (`-68`/`-75`).
* **Menu cleanups**: pane hamburger trim
  (`-60`), right-click menu trims
  (`-80`/`-82`), Pane Mode → Hybrid NAV
  copy sweep (`-62`).
* **Watcher containment**: terminal
  watcher paths constrained to drive root
  (`systacean-19`).
* **Desktop launcher refresh** (`-53`)
  + Cmd+N → new window (`-83`).
* **Cmd+S drop** (`-56`) — autosave is
  canonical.
* **xterm row metrics** (`-51`).

Polish + small fixes also worth listing
(the changelog tone is your call):
- Right-dock chevron direction (`-49`).
- Drop "New Terminal" menu entry + sharpen
  Restart prompt (`-52`).
- Carousel dashboard-stats row dropped
  (`-55`).
- Context-aware Cmd+K spawn (`-43`).
- Rich prompt auto-focus (`-79`).
- Empty-pane focus border consistency
  (`-85`).

## Tag + push

```bash
git tag v0.11.0
git push origin v0.11.0
```

@@Alex's call on whether the tag is signed
or unsigned. Check the chan-desktop signing
key state per `desktop/CLAUDE.md` (dev key
still in play; rotate before public
release is a phase-8 concern per backlog
item 7).

## Verification

* `cargo build --release` succeeds.
* `cargo test --release` green.
* `web/npm run build` green.
* `scripts/pre-push` green.
* Spot-check the tagged binary runs (don't
  need a full sweep — the per-task gates
  covered).

## Notes

* Phase 7 wrap. After this tag, phase 8
  opens against the 9-item backlog at
  [`../next-phase-backlog.md`](../next-phase-backlog.md).
* `desktop/CLAUDE.md`'s dev-key rotation
  is a phase-8 prerequisite to first
  public release; NOT blocking for
  v0.11.0 since the desktop installer
  signing scope is internal.
* Standing topic-level commit clearance.

## 2026-05-19 19:56 BST - ready to tag

Prepared the v0.11.0 release bump.

Changed:

* Workspace package version: `0.10.1` -> `0.11.0`.
* `Cargo.lock` workspace package entries regenerated.
* `web/package.json` and `web/package-lock.json` version: `0.11.0`.
* `desktop/src-tauri/tauri.conf.json` version pin: `0.11.0`.

Desktop signing audit: `desktop/CLAUDE.md` still documents the current
updater key as a dev key and says rotation is required before public
release. Per task notes, this is not blocking for the internal v0.11.0
tag.

Verification:

* `cargo build --release`
* `cargo test --release`
* `npm run build` in `web/`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
* `target/release/chan --version` -> `chan 0.11.0`
