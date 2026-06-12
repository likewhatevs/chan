# task-ChanDesktop-Lead-1 — desktop tidy-up: DONE

From: @@ChanDesktop. To: @@Lead. Re: task-Lead-ChanDesktop-1.
Commits on main: `ad6d5c2c` (code scrub + hygiene), `e8b4356a`
(design.md rewrite). Pathspec-atomic, pre/post-stat verified, not
pushed. Journal: new-team-1/journals/journal-ChanDesktop.md.

## Gate

- BEFORE: clippy --all-targets = 0 warnings already (macOS cfg).
- AFTER (run after the last edit): `cargo fmt --check` clean,
  `cargo clippy --all-targets` 0 warnings, `cargo test` all green
  (81 unit + 7 tunnel_e2e + the rest), all in desktop/src-tauri.
- Linux/x86_64 cfg branches: compile-unverified locally (per task; no
  CI dry-run triggered). Eyeballed — my edits in cfg(not(macos)) code
  are comment-only; the two refactors live in cfg-neutral code.
- web/** untouched; web/dist already present, no npm build needed.

## 1. Archaeology scrub

All phase-N / agent-handle / ticket-id / round-marker references in
desktop/** are gone (verified by re-grep; design.md rewritten, see
below). Constraint-bearing comments were rewritten, not deleted, e.g.:
Cmd+I reserved for editor italic, Cmd+\ shadowed by 1Password, Linux
Ctrl+W left to readline, bury-vs-really-close exceptions, SPA-owns-
preflight. Also caught markers the recon patterns missed:
`systacean-27`, `Round-2-plan §`, `B10`, `(B-slice N)`, a "bug
report" citation, and one stray `new-team-1` marker in main.js.

Genuinely STALE docs fixed along the way (worth knowing they existed):
- add_workspace doc claimed features flow "through to `chan add`"
  CLI flags — in-process for a while now.
- a main.rs comment referenced `get_workspace_features`, which no
  longer exists.
- registry.rs claimed "mutation goes through the `chan` binary".
- capabilities/default.json description claimed File ▸ New Window
  spawns main-N launchers (launcher is a singleton, test-pinned).
  Description fixed; `main-*` glob KEPT (capability shape = behavior).
- main.js [New]-modal doc described the removed in-modal preflight
  scan + add-time feature toggles.

## 2. Hygiene

- Warnings: 0 → 0 (was already clean; kept clean).
- `build_workspace_window`: 9 positional params → `WindowSpec` struct;
  the `#[allow(clippy::too_many_arguments)]` and its justification
  comment are gone. 5 call sites converted. No other >5-param
  functions in desktop/** (scripted scan).
- Dedupe: the identical 10-line spawn preamble (unbury check, window
  cap, WindowConfig pop) in the local/tunnel/outbound spawn fns →
  one `unbury_or_restore` helper.
- Renamed ticket-named test locals (`pre_b19`→`missing_zoom`,
  `pre_b28`→`missing_features`).

## 3. design.md — current-snapshot rewrite (e8b4356a)

Read the source first (serve.rs, main.rs, embedded.rs, config.rs,
registry.rs, watcher.rs, auth.rs, pdf.rs, download.rs, cs_install.rs,
tunnel/mod.rs, main.js, connecting.js, release.yml). New structure:
purpose / mental model / lifecycle / validation / runtime / WINDOW
MODEL (new: label scheme + capabilities, chord bridge + menu routing,
bury-on-close + restore LRU, connecting screen, standalone terminals
+ shared /terminal tenant + cs control socket, remote-window reopen)
/ CLI + AppImage cs wrapper / distribution / GUI stack / self-upgrade
/ SIGN-IN (new) / settings / remote workspaces (updated UI/lifecycle)
/ NATIVE FILE INTEGRATIONS (new). Deleted outright: File Browser
drag-out section (command no longer exists anywhere), "in-app webview
for tunneled workspaces" deferred item (they ARE in-app), v0.26.1
updater aside, stale open-questions section.

## Flags for @@Lead (decisions, none shipped)

1. **Vestigial feature plumbing**: `WorkspaceSettings.features` (+ the
   `cfg.workspaces` map) is WRITE-ONLY — nothing reads it back; and no
   caller passes `features` to `add_workspace` (launcher sends path
   only; SPA onboarding owns layers post-boot). Pre-release posture
   says drop the param + field + mirror write outright. Wire/config
   shape change → your call; I only corrected the docs.
2. **desktop/release-review.md**: 797-line point-in-time review of an
   architecture that no longer exists (BinStatus preflight,
   `chan serve` subprocess supervisor). Zero inbound references
   anywhere. Deletion candidate — routed per round-plan rule instead
   of deleting mid-round.
3. **updater-bridge.md**: kept + scrubbed (cited by design.md and
   .agents/desktop.md). The one-time DEV→prod key bridge it documents
   may be fully in the past; if so it could shrink to the
   key-identification + failure-mode halves. Your call.
4. **Serde legacy defaults**: `zoom_level` / `features` missing-field
   defaults (and their tests) are back-compat paths for old
   config.json files; pre-release no-backcompat would delete them.
   Cheap to keep, trivial to drop — flagging, not shipping.
5. Out of my lane: `.agents/desktop.md` carries agent-handle content
   (it's the agent docs home, likely exempt-ish — your surface).

---

# Addendum results (task-Lead-ChanDesktop-2)

## Workspace correction — acknowledged, re-gated

Re-ran the own-gate from the repo root as a root-workspace member:
`cargo fmt --check -p chan-desktop` clean,
`cargo clippy -p chan-desktop --all-targets` 0 warnings,
`cargo test -p chan-desktop` green (81 + 7). Note: my original gate ran
from desktop/src-tauri, which cargo resolved into the same root
workspace, so the earlier green stands — this is a re-verification
under the prescribed form, not a different result.

## Extended patterns — already clean

The wider grep (systacean/desktacean/desktest/desktect, @@Host/CI/
Architect/Lane/FullStack/Webtest, round-N, wave-N, "slice x",
"track a/b") finds ZERO archaeology in desktop/. The only matches are
legitimate technical usage of "slice" (pdf.rs run-loop time slices,
main.rs Vec-slice doc). The systacean-27 and @@Architect hits the
recon expected were caught and removed in pass 1 (commit ad6d5c2c).
No new commit from this addendum — nothing to change.

## Bundle identifier — REPORT (unchanged, as instructed)

- `desktop/src-tauri/tauri.conf.json:5` →
  `"identifier": "com.chanwriter.desktop"`. This is the macOS
  CFBundleIdentifier (notarization history, keychain/app identity)
  and the Linux .desktop/AppImage app id. NOT renamed.
- Full repo blast radius of `chanwriter`: exactly two files — the
  conf above, plus `docs/release/macos-signing.md:151`, which
  *documents* the current value (your surface; accurate as-is, only
  needs touching if the id ever changes).
- Everything else identifier-shaped in desktop/ is clean: updater
  endpoint is `https://chan.app/dl/desktop/latest.json`, deep-link
  scheme is `chan`, keychain service is `chan-desktop` (auth.rs),
  signing identity is Alex's personal Developer ID (not org-named).
  No Info.plist files exist in-repo (bundle id is baked from the conf
  at bundle time).
- One more chan-writer leftover, already flagged in item 2 above:
  `desktop/release-review.md:642-643` cites
  `github.com/chan-writer/chan-core` git URLs (dead org) — folds into
  the existing deletion-candidate flag.
