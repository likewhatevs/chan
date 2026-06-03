# new-team-1 - RE-BOOTSTRAP (resume the launcher-redesign round)

This round paused for @@Alex to hand-smoke the chan-desktop launcher
redesign (he was on chan.app and could not drive the local WKWebView app).
Work is COMMITTED and gate-green; only the manual smoke + a few small
follow-ups remain.

## WAKE PROMPT (send to each agent; they self-identify from $CHAN_TAB_NAME)

    Team new-team-1 is resuming. You are $CHAN_TAB_NAME. Read
    new-team-1/bootstrap.md (team process), then this file
    (new-team-1/re-bootstrap.md) for the round state + your resume task.
    @@LaneA is lead and coordinates; workers HOLD for @@LaneA's poke unless
    your section below says otherwise.

## STATE AS OF PAUSE (2026-06-02)

- Goal: build the chan-desktop launcher redesign left over from phase-16
  (merge [Open workspace] + [Attach] into one [New] modal; On|Where rows;
  remove the per-row gear; INBOUND/OUTBOUND indication). @@Alex-approved
  design, design-first.
- Locked decisions: D2 = in-launcher MODAL, D3 = connection dot, D4 = drop
  the header tagline, D1 = keep the add-time feature toggles. See the
  ">>> DECISIONS LOCKED" block at the top of
  new-team-1/desktop-redesign-design.md.
- BUILT + COMMITTED: commit fd27d29d (on main, NOT pushed). 7 files:
  desktop/src/{index.html,main.js,styles.css} +
  desktop/src-tauri/{permissions/app.toml,src/main.rs,src/serve.rs,
  src/embedded.rs}.
- GATE GREEN at commit: cargo fmt/clippy/build + cargo test --workspace
  1274/0; cd desktop && make build (Chan.app, 0.24.0).
- NOT YET DONE: the WKWebView hand-smoke. App staged at
  target/release/bundle/macos/Chan.app. Checklist:
  new-team-1/smoke-checklist-LaneD.md (9 sections). Agents CANNOT drive
  WKWebView (Chrome MCP is Blink); @@Alex hand-drives.

## RESUME SEQUENCE (driven by @@Alex's smoke result)

1. @@Alex reports the smoke result to @@LaneA.
   - PASS -> go to step 2.
   - FAIL (by checklist section #) -> @@LaneA routes the fix: a frontend
     bug (sections 1-8 launcher behavior) -> @@LaneB (desktop/src); a
     Rust/wiring issue -> @@LaneC (desktop/src-tauri). After the fix:
     @@LaneD re-verifies (full gate + rebuild) -> re-smoke -> then step 2.
2. flag-1 coverage canary (APPROVED, post-smoke): @@LaneA pokes @@LaneC to
   add ONE serve.rs canary pinning the modal's outbound/inbound JS wiring
   (#new-workspace -> showNewWorkspaceDialog + invoke('add_outbound_
   workspace') + invoke('tunnel_start')); anchor on fn name + invoke calls,
   not copy text. Gate cargo test green. Spec in task-LaneA-LaneC-3.md.
3. @@LaneA amends the fd27d29d work as needed (any smoke fixes + the
   canary) and confirms the tree is green + clean.
4. Round close (when @@Alex says): commit the new-team-1/ coordination
   bus to main as a docs(...) commit (it is gitignored live state now),
   with a retrospective. Do NOT push without @@Alex's explicit ask.

## CARRYOVER / FOLLOW-UPS (not blocking the smoke)

- crates/chan/src/main.rs:1660-1666: the comment says the CLI `--json`
  reports_enabled field exists FOR chan-desktop's get_workspace_features
  IPC, which we DELETED. Follow-up: assess whether the --json
  reports_enabled field is now vestigial (other consumers?) and either
  drop it or update the comment. Different crate; deferred deliberately,
  not a comment-only tweak. Owner TBD by @@LaneA.
- flag-2 (DEFERRED): the serve.rs include_str! frontend canaries are
  brittle (go red on benign renames). Optional hardening task if @@Alex
  wants it; not this round.

## UNRELATED WORKING-TREE ITEMS (left untouched; @@Alex's call)

- ` D .codex/config.toml` (a deletion not from this round)
- `?? docs/journals/phase-16/alex-new-draft/` (@@Alex's new draft)
- `?? tmp/` (stray untracked dir)
@@LaneA did NOT touch/commit these. Handle separately.

## PER-LANE RESUME

- @@LaneA (lead): read this file; get @@Alex's smoke result; drive the
  resume sequence above. Journals: journal-LaneA.md. The full event trail
  is in the task-* files (all completions accepted through task-3/verify).
- @@LaneB: HOLD. Resume ONLY if @@LaneA routes a frontend smoke fix
  (desktop/src). Your build is committed in fd27d29d. journal-LaneB.md.
- @@LaneC: HOLD. Resume for the flag-1 canary (step 2, post-smoke) per
  task-LaneA-LaneC-3.md, or a Rust smoke fix if routed. Your task-2 +
  task-3 are committed in fd27d29d. journal-LaneC.md.
- @@LaneD: HOLD. Resume to re-verify (full gate + rebuild + re-stage) after
  any smoke fix or the canary lands. journal-LaneD.md.
