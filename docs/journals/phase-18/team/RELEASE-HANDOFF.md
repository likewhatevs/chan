# Release handoff -> @@LaneE (recycled as the RELEASE lane)

@@Alex is winding the team down. You (@@LaneE) are recycled as the single
RELEASE lane. The other lanes (A/B/C/D/F) + @@Lead are being cleared. You have
TWO jobs, then you own the release + coordinate DIRECTLY with @@Alex.

## Job 1: the new desktop bug (your domain)
Read ./desktop-bug-report/draft.md + image.png / image-2.png (workspace root).
Summary: turning a workspace OFF flips the chan-desktop toggle to OFF in the UI
BEFORE the underlying chan-server actually shuts down. A subsequent click then
fails and leaves the UI broken (workspace shows ON but the OPEN button is gone).
It's a toggle/lifecycle RACE: the UI state changes ahead of the server-shutdown
completion. Your domain: desktop/src/main.js (workspace on/off lifecycle) + any
src-tauri shutdown IPC. Fix so the toggle reflects ACTUAL server state (await
the shutdown / disable the control until the transition completes), smoke it in
chan-desktop (WKWebView - only you/@@Alex can drive that), commit as an append
`fix(desktop): ...`.

## Job 2: take over + finish the release
### Current committed state (main, base d5f7dd38, NOT pushed, 12 commits)
  688955c5 refactor(editor): real glyph-widget bullet markers, delete snap
  948faed1 docs(agents): em-dash sweep + phase-8 roster
  d5886380 docs(coordination): rewrite for docs/phases layout
  0408db30 fix(graph): persist selected node across reload
  2e372a93 docs: repoint stale journal refs; delink agent skills
  74909e64 docs(phases): consolidate journals into docs/phases + playbook
  3a6623a0 chore(state): merge cross-lane state + resync serve help
  9fcf0187 fix(file-browser): tab-menu root actions + hints, debounce hash
  ae22d5a1 fix(graph): select-on-from-here, dir-edges, binary node, no reload; copy link
  2e429f27 fix(terminal): UTF-8 locale, copy/paste chords, rich-prompt focus; drop desktop pre-flight
  296f6495 feat(inspector): pill + dropdown per item category
  c9ea3c56 fix(editor): list cursor parity, hyphen lists, free-scroll, [[ paths
Working tree is CLEAN except docs/journals/phase-18 (the live team bus, untracked).
Full round narrative: docs/journals/phase-18/team/journals/journal-Lead.md.

### @@LaneF's PENDING Wave-3 (you inherit this - it is fully documented + staged)
@@LaneF did the SAFE Wave-3 (phases 1-17 + playbook + scrubs + coordination.md,
all committed above). It is HOLDING on the FINAL go, which is now yours:
  1. Fold phase-18 -> docs/phases/phase-18.md (from the DISTILLED essence of this
     round, NOT the raw bus) + add the README index entry. Do this AFTER the
     round is otherwise final (it captures the close-out incl your desktop fix).
  2. Deletions, IN ORDER, docs/journals LAST:
     rm -rf .claude .codex (untracked); git rm docs/archive; git rm the 8
     redirect cards + docs/agents/bootstrap.md + the skills/ subdirs (skills/
     cut is CONFIRMED by @@Alex); git rm docs/journals LAST (this also removes
     the team bus + journal-Lead.md + this handoff - so capture anything you
     still need FIRST).
  Source of truth for the exact lists: task-LaneF-Lead-2.md (keep/cut + scrub),
  task-Lead-LaneF-3.md (ratified plan + guardrails), task-Lead-LaneF-5/6.md
  (the go + greenlights). @@Alex PRE-AUTHORIZED the deletions (round-close survey
  option 1). If you'd rather @@LaneF executes its own documented deletions, that
  is fine too - your call as release lane; coordinate via @@Alex.

### Release mechanics + caveats (load-bearing - confirm before any tag)
- Version bump 0.25.0 -> 0.26.0 in ALL pins TOGETHER: Cargo.toml
  [workspace.package], desktop/src-tauri/tauri.conf.json, Cargo.lock, AND
  web/package.json (web/ has drifted before - do not miss it).
- FULL gate before tag: run `make pre-push` from an ISOLATED gate.sh worktree
  (gates the COMMITTED state, immune to WIP). It must build EVERY workspace CI
  ships - the core workspace AND gateway/ (separate Cargo workspace) - plus
  `cargo build --no-default-features` AND `cd desktop && make build` (the DMG;
  @@LaneE deferred that to the release lane - that's now you). I have already run
  core fmt/clippy/test + web-check GREEN on the integrated tree; you still owe
  the --no-default-features build, gateway build, and the desktop DMG build.
- Tag vX.Y.Z fires release.yml; macOS sign/notarize runs ONLY on Actions.
  DRY-RUN first via workflow_dispatch publish=false before the real tag.
- Gated-push SIGPIPE trap: the pre-push hook gates EVERY push incl tags (~3min);
  a backgrounded push SIGPIPEs + silently fails to update the remote. Push in
  FOREGROUND, redirect to a file, verify with `git ls-remote` before tagging.
- KNOWN external gap (NOT a CI failure): chan.app/dl/*/latest.json 404s - the
  chan.app -> Pages routing for /dl is the unfixed bit; desktop self-upgrade
  404s because of it, not because the release build failed. See
  docs/journals/phase-18 ... project memory "/dl Pages routing gap".
- NO push and NO tag without @@Alex's EXPLICIT go. Pre-release: no back-compat.

## Outstanding hand-smoke (it is @@Alex's to run; you document + coordinate)
See docs/journals/phase-18/team/smoke-checklist.md - the CHECKED vs YET-TO-CHECK
list. @@Alex runs the YET-TO-CHECK items (WKWebView/real-trackpad + your new
desktop fix) after your patch. I rebuilt the Chrome smoke server at :8787 (PID
in /tmp/chan_serve2.log) with all fixes baked; reuse it or tear it down. @@Alex
also has his OWN server at :8791 (lists.md drive) - that one needs ITS OWN
rebuild (npm run build -> cargo build -p chan -> restart) to show the fixes.

## Handoff protocol
Confirm you have this (poke @@Lead: "release handoff accepted"). On your confirm,
@@Lead clears out and you own the release end-to-end, coordinating with @@Alex
directly. Welcome to the hot seat.
