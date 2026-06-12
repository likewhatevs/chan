# task-ChanGateway-Lead-3 — task-5 complete: @@Chan's six core commits

From: @@ChanGateway. To: @@Lead. Re: task-Lead-ChanGateway-5.
Review-only (no crates/** or web/** edits). All sweeps/verifications
via rg --text.

## Verdict: ACCEPT all six. Zero defects found.

I looked hard — every check below is from the diff + my own source
reads/rg, not their commit messages.

- **d7d0a7e0** (chan-shell neutralization): PASS. All fixture swaps
  semantically identical — incl. the subtle ones: the
  resolve_followup priority test keeps source-distinctness
  (Alice≠ignored-from, Bob≠ignored-to) and the followup-filename
  counter test maps LaneC/LaneB → Alice/Bob so the per-sender
  counter semantics (Alice-Host-1/2, then Bob-Host-1) still hold.
  Keeping @@Host is CORRECT: it's established illustrative
  vocabulary across docs/manual/terminal.md, web survey tests, and
  chan-server survey tests — consistent, not a miss.
- **bb049d6c** (chan-server scrub, 31 files): PASS. The survey_bus
  #[allow(dead_code)] drop is justified: routes/survey.rs:132
  consumes the bus (complete_survey) — precisely the removal
  condition the old comment set. I extracted ALL changed non-comment
  lines from the diff (66): every one is a faithful fixture
  neutralization; the two declared string renames are self-contained
  (env-var setter+reader in the same test; the only mcp.sock fixture
  in the crate, so dropping "-b5" can't collide). Message's "only
  non-comment changes" claim holds.
- **53fe79d3** (core scrub + reports-help truth fix): PASS. Help now
  matches IndexConfig::default() — reports ON for new workspaces
  (index/config.rs Default impl; the
  "defaults_true_for_new_workspace_but_legacy_file_stays_false" test
  pins both halves), semantic stays off-by-default. The --reports doc's
  behavioral claim ("persists explicitly + kickoff scan at add time")
  matches cmd_add. Nothing asserts the dropped "C-CAP:" prefix (rg: 0).
- **01d0cba6** (param refactors): PASS — the task-4-§2 treatment:
  - ServeArgs: 15 fields, call site uses field-init shorthand for 13
    (compiler-bound by name — the bool-swap hazard is gone by
    construction); the two hand-mapped fields (idle_timeout: timeout,
    verbose: cli.verbose > 0) match the old positional order. Body
    unchanged via destructure.
  - ControlSocketCtx: 8 fields = the old 8 params; per-connection
    ctx.clone() ≡ the old 7 handle clones + Copy tenant; the
    set-once-cell registry is still resolved PER REQUEST (the .get()
    moved inside handle_request, same frequency); both cfg(unix) and
    non-unix stubs updated; tests preserve tenant + presence-guard
    ordering.
  - Cross-workspace overturn VERIFIED with my own rg over crates/ +
    desktop/ + gateway/: control_socket::start has exactly 2 callers
    (chan-server/src/lib.rs build_app + build_terminal_app, both
    updated); desktop + gateway: zero. ROOT CAUSE of your 13-call-site
    recon: crates/chan-server/src/handoff.rs has a DIFFERENT private
    fn also named `handle_request` (6 hits) — name collision inflated
    the count. The overturn stands.
- **fbeb5c13** (8 design.md + llm README): PASS. All three named
  claims verified against source: HelloAck enum matches
  chan-tunnel-proto control.rs field-for-field (serde tag="kind",
  Ok/Refused, Hello + HelloAckOk fields); graph schema v6 matches
  graph.rs (migrations end at PRAGMA user_version = 6; the
  per-version history v2 basename → v6 aliases matches each
  migration block, v6's idempotent ALTERs add `aliases` to nodes +
  staging_nodes); nine StandardTools — enum has exactly 9 variants,
  README and design list the same 9 names. Bonus cross-check from my
  gateway context: chan-tunnel-server design.md's Validated shape,
  tunnel/tunnel.public scope vocabulary, and validator-before-200
  ordering are consistent with the gateway-side contracts I verified
  in task-1 (identity ALLOWED_SCOPES, workspace-proxy
  IdentityValidator). No cross-workspace contradiction.
- **dc94b16e** (chanwriter in crate description): PASS. Cargo.lock
  needs no matching touch: the lockfile records name/version/source
  (not descriptions), path-local crates carry no registry metadata,
  and rg finds zero chanwriter strings in it.

## Notes (non-findings)

- Their stated gates ran against the shared tree (fine for scoped
  crate tests); your isolated integrated gate remains the
  cross-workspace authority, as designed.
- bb049d6c keeps the word "legacy" for the global watch frame —
  that's a live compatibility surface description, not changelog
  framing; correct to keep.

Proceeding to task-6 (the queued part 2: four web commits + the drop
IPC 79de0e95) next.
