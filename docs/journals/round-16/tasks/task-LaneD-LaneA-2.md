# task-LaneD-LaneA-2: Verify gate GREEN + app staged + smoke ready

From: @@LaneD  To: @@LaneA  Type: completion (verify)
Re: task-LaneA-LaneD-2

## Result: GATE GREEN. App built + staged. Smoke checklist ready for @@Alex.

I re-ran the full gate myself (did not trust the lane reports). All green.

## Gate (ran against the 7-file working tree, NOT committed)

  step                                   result
  -------------------------------------  --------------------------------
  cargo fmt --check                      PASS
  cargo clippy --all-targets -D warnings PASS
  cargo build --workspace                PASS
  cargo test --workspace                 PASS 1274 passed / 0 failed
                                         (34 test binaries; 0 FAILED)
  cd desktop && make build (app bundle)  PASS (exit 0)

(My workspace test total 1274/0 supersedes the per-crate 79/0 @@LaneC
cited; same green, wider scope.)

## Built app (staged, unsigned, local)

  target/release/bundle/macos/Chan.app
  Info.plist CFBundleShortVersionString = 0.24.0 (matches workspace).
  binary: Contents/MacOS/chan-desktop, mtime 2026-06-02 20:24:37.

Launch: `open target/release/bundle/macos/Chan.app`
(first launch: right-click -> Open; unsigned local build).

IGNORE the stale `rw.25836.Chan_0.17.0_aarch64.dmg` sitting in the same
bundle/macos dir; it is a May-29 v0.17.0 leftover, NOT this build. Hand
@@Alex the .app above.

## Provenance (no stale-binary risk)

- Binary linked 20:24:37, ~25 min AFTER the last desktop/src edit
  (19:59:29). make build re-embeds frontendDist every run.
- Launcher assets are brotli-compressed in the binary (9 brotli/
  EmbeddedAssets markers), so `strings` can't see the JS; that is why a
  literal grep for the modal marker is empty. Not a stale build.
- Working-tree source verified to carry the redesign before the build
  (see landed-code checks below).

## Landed-code verification vs LOCKED design (verify-don't-trust)

- MODAL path (D2): showNewWorkspaceDialog() with .nw-choices segmented
  switch {local,outbound,inbound}; wired to [New] (main.js:1193),
  empty-state #empty-pick (909), first-run (172). No new.html/new.js/
  window command (correctly rejected path).
- Per-row gear REMOVED: 0 JS fns (renderFeaturesToggle/Panel/
  bindFeaturesToggle), 0 Rust cmds (get/set_workspace_features), 0
  app.toml feature perms, 0 serve.rs feature canaries.
- Add-time toggles KEPT (D1): data-feat bge/reports live ONLY inside the
  modal's Local choice; default off. Distinct from the removed gear.
- Rows (D3): thead On|Where; renderWhere(); conn-dot ('on' when hasUrl,
  grey otherwise); no url/tunnel text tag. Row On-toggle perm
  (allow-set-workspace-on) correctly RETAINED (different command).
- Header (D4): single [New] (#new-workspace); no tagline; theme toggle.

## Smoke checklist for @@Alex

new-team-1/smoke-checklist-LaneD.md  (MODAL-adapted, 9 sections, ASCII).
WKWebView is not Chrome-automatable (Blink), so @@Alex (or a Mac lane)
must hand-drive the click-through. The checklist is the last gate before
the redesign is called done.

## Not done (per task: "Do not commit")

Left the 7-file change uncommitted in the working tree. Your call on the
commit, gated on @@Alex's smoke.

## On completion

This file + poke to @@LaneA. Journal updated (journal-LaneD.md).
