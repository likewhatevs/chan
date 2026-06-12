# journal — @@ChanGateway (round 1, tidy-up: gateway)

Append-only. Task: new-team-1/tasks/task-Lead-ChanGateway-1.md.

## 2026-06-12 — session open

- Read task + round-1-plan. Scope: gateway/** + docs/manual/gateway.md.
- Archaeology sweep done (file-anchored to dodge output truncation):
  - phase mentions: **0** in gateway/ outside node_modules (confirms
    @@Lead's recon; checked all file types incl. design.md/READMEs).
  - handle pattern: only false positives ("## Architecture" headings,
    `SystemCallArchitectures=native` in systemd units) **except**
    `gateway/docs/dev-setup.md:237` — "@@Host uses Cloudflare" → real
    team-speak in user-facing docs, will neutralize.
- Next: baseline gates (fmt/clippy/test inside gateway/), then source
  read for design-doc rewrites + param-count/duplication hunt.

## 2026-06-12 — recon complete

- Baseline gate GREEN before any edit: fmt clean, clippy 0 warnings,
  cargo test --workspace 153 passed (Postgres via chan-psql sdme
  container + ssh -L 5432 bridge; lima's forwarder can't see sdme's
  DNAT publish, see lessons).
- Read all 5 crates end-to-end + all design.md/READMEs/manual.
- Work list:
  1. REFACTOR: ApiTokenService::create 7 params -> NewToken struct +
     RequestMeta{ip,user_agent} (pair recurs in create/revoke/validate/
     write_audit + 5 call sites in http.rs/desktop_authorize + tests).
  2. DEDUP: throttle default constants (4.0/16.0/4096) duplicated in
     identity/token_throttle.rs + workspace-proxy/throttle_validator.rs
     -> single-source in gateway_common::token_bucket.
     SKIPPED dedup: error enums (documented design: consumers map
     locally), tracing-init (gateway-common has no tracing-subscriber
     dep; bad trade for 6 lines x4).
  3. ~26 changelog-style comment sites (no longer/used to/v0/follow-up/
     "item #2"/"#18"). Two comments factually WRONG: admin/main.rs:16
     ("tunnel ps/kill/watch land in a follow-up" — they exist),
     identity/http.rs:130 ("/internal/* uses PROFILE_AUTH_TOKEN" — it's
     IDENTITY_INTERNAL_TOKEN).
  4. Design docs: gateway-common (4 modules claimed, 8 exist; decode
     signature stale; phantom InvalidSignature variant), identity
     (desktop_authorize missing; "per-PAT scopes not wired" FALSE),
     workspace-proxy (claims tower_governor on admin tree — code
     deliberately has none; "tower::timeout" wrong), profile (schema
     missing api_tokens.scopes; audit actions missing
     created_via_desktop), admin ("now lives server-side" framing;
     per-service env vars that don't exist).
  5. READMEs: root gateway (phantom crates/workspace-proxy/web SPA,
     chan-writer org ref, "six debs" vs 8, "flags workspace the
     rollout" typo, v0 (done) status), gateway-common (2/8 modules),
     identity (internal route says PROFILE_AUTH_TOKEN; desktop routes
     missing), workspace-proxy (tower_governor claim), profile
     (by-username + admin email routes missing; workspace-proxy
     sessions claim), admin (user update --email doesn't exist).
  6. docs/manual/gateway.md: claims verified against source
     (--tunnel-url + CHAN_TUNNEL_TOKEN real); only the "Verification
     status" audit-process section is questionable — flag to @@Lead
     (desktop.md in their lane has the same pattern).
  7. gateway/Cargo.toml header comment: stale narrative (3 crates
     listed of 5, GitHub-only v0, "reverse-proxy ships once...").
  8. gateway/docs/dev-setup.md:237 "@@Host uses Cloudflare" handle ref.

## 2026-06-12 — lane complete

- All work items done; commit 5c44bf00 (pathspec-atomic, 38 files,
  verified zero non-lane paths via `git show --name-only | grep -v`).
- Final gate AFTER last edit, in order: cargo fmt --check clean (one
  rustfmt fix applied post-refactor), clippy 0 warnings re-run after
  the fmt fix, cargo test --workspace 153/153.
- Completion task: new-team-1/tasks/task-ChanGateway-Lead-1.md.
- Infra left UP for @@Lead's integrated gate: chan-psql sdme container
  + ssh -L 5432 bridge (teardown commands in the completion task).
- Lessons this session:
  - sdme `-p 5432:5432` publishes via nftables DNAT; lima's
    socket-watching forwarder never sees it → bridge with
    `ssh -F ~/.lima/default/ssh.config -N -L 5432:<container-ip>:5432`.
  - Bash tool truncated/suppressed output repeatedly on compound
    commands (echo after grep swallowed, exit-code display vs real $?
    mismatches). Anchored every load-bearing check to a file +
    separate cat, per the confabulation lesson. No conclusions were
    drawn from unverified output.

## 2026-06-12 — addendum (task-Lead-ChanGateway-2)

- Wider archaeology sweep over gateway/** + manual: ONE real hit
  (build-gateway.sh "round-4 de-risk"), fixed in commit 26f72350;
  proxy.rs "slice" matches are Rust slices (false positives);
  manual clean. Re-sweep zero.
- Folded into task-ChanGateway-Lead-1.md addendum section; poked.
- Near-misses logged honestly:
  - First draft of the addendum cited a confabulated commit hash
    ("8f1aef62") instead of the real 26f72350 — caught on the
    post-append git log read-back, corrected before the poke.
    Lesson reinforced: copy hashes from anchored git output, never
    from memory.
  - Bundled verify+poke in one command again; the poke ran but its
    confirmation was swallowed, so I re-poked and @@Lead gets a
    harmless duplicate. Keep pokes strictly standalone.
- Noted in passing: eb668de7 shows @@Lead already applied the
  matching manual cleanup to desktop.md (flag #1 resolved).

## 2026-06-12 — rider (task-Lead-ChanGateway-3, chanwriter purge)

- @@Lead's known hit (README.md:62) was already fixed in 5c44bf00.
- New pattern caught 3 real hits round-1 missed: all three packaged
  systemd units pointed Documentation= at the deleted
  chan-writer/chan-gateway org → now fiorix/chan/tree/main/gateway.
  Commit 7d79259c (lane-purity verified via name-only grep).
- Re-sweep chanwriter|chan-writer|chan_writer over gateway/** +
  manual: zero hits. Poked @@Lead.

## 2026-06-12 — trail correction (per task-Lead-ChanGateway-4)

- CORRECTION for the audit trail: the build-gateway.sh round-4-citation
  fix is commit 26f72350 (NOT 8f1aef62 — that string was a hallucinated
  hash in the first draft of the completion-file addendum; the file was
  corrected pre-poke, this line resolves any copy @@Lead read mid-window).
- task-4 item 1 (.service Documentation URLs) crossed in flight: fixed
  in 7d79259c before task-4 arrived. URL chosen:
  github.com/fiorix/chan/tree/main/gateway (deep link; task text allows).
- rg --text re-sweep of ALL prior shim sweeps (round-plan addendum):
  surface confirmed clean except ONE miss — workspace-proxy
  tests/api.rs "no longer holds sessions" (my marker sweep covered
  src/ not tests/) → fixed in 2d13684a. Load-bearing claims re-proved
  under rg: profile_client only consumed by identity; chanwriter 0;
  phase 0.

## 2026-06-12 — task-4 complete (desktop second-pass review)

- Fixup item crossed in flight (7d79259c predates task-4); deep-link
  URL choice flagged for @@Lead's veto.
- Desktop review (ad6d5c2c + e8b4356a): ACCEPT. WindowSpec mapping
  faithful at all 5 sites; unbury_or_restore order/defaults exact;
  spawn paths diverge only where intended; 4/4 sampled design.md
  claims verify; the rewrite even fixed a dangling fn reference.
- Two findings routed via completion file: F1 chord-policy comment
  claims Cmd+[/] unbound while serve.rs:1066-1067 binds pane
  prev/next (inherited staleness); F2 desktop/README.md still
  documents the deleted start_file_browser_drag_out command (only
  other repo reference is web's negative-pin test).
- Completion: new-team-1/tasks/task-ChanGateway-Lead-2.md.

## 2026-06-12 — acceptance + one more crossing

- @@Lead ACCEPTED riders 26f72350 + 7d79259c (independent rg confirms
  gateway zero-hit).
- @@Lead's "§2 is your live assignment" poke crossed my task-4
  completion in flight (third crossing this round — the queue delays
  pokes while the target is mid-generation, so file-then-poke can
  lag a fast dispatcher). Resolved peer-to-peer: re-pointed at
  task-ChanGateway-Lead-2.md; no work redone.
- Lane state: all assigned tasks complete (1, 2, 3, 4§1 crossed-out,
  4§2 reviewed). Standing by for routed fixes or round close.
  Postgres container + ssh bridge still up for @@Lead's gate.

## 2026-06-12 — task-5 complete (six core commits reviewed)

- ACCEPT all six, zero defects. Highlights: survey_bus allow-drop
  condition verified satisfied (routes/survey.rs:132); ServeArgs
  field-init shorthand kills the bool-swap hazard by construction;
  cross-workspace overturn confirmed by my own rg AND root-caused
  (@@Lead's 13-call-site recon counted handoff.rs's same-named
  private handle_request); fbeb5c13's 3 named claims all verify
  (HelloAck/user_version=6/nine StandardTools); Cargo.lock carries
  no descriptions → dc94b16e needed no lock touch.
- Completion: task-ChanGateway-Lead-3.md. Repeated the verify+poke
  bundling mistake once more before catching it — poke re-sent
  standalone (possible harmless dup).
- task-6 queued behind this; starting now.

## 2026-06-12 — tasks 6+7 complete (batched)

- Web commits (4): ACCEPT. 5 pin samples equal/stronger (positional
  anchors verified unique + ordered); FA57 identifier rename complete;
  3 corrected-FALSE comments all true against code; closeEmpty/
  richPrompt dispatch predicates match the registry exactly;
  suppressions narrow (ceiling kept, one onwarn code, element-scoped
  ignore); stragglers comment-only + residue clean.
- Drop IPC 79de0e95: ACCEPT. Both amendments in; negative ACL pins
  cover the capability AND both broad surfaces; remote.urls loopback
  restriction is extra defense; dropped_paths.rs read whole — clean.
  Notes: Result-vs-Vec signature benign; default.json negative pin
  absent (outside threat model).
- Web guard a19d7d40: ACCEPT. Discriminator pinned both ways; no
  stopPropagation; preventDefault-before-await ordering correct;
  file-browser allowlist deviation justified AND pinned; newline
  filename probe safe. Findings: F-W1 (Rich Prompt help row misleads
  Linux under the Cmd=Ctrl header — needs note: field), F-W2
  (codemod scar test name "can workspace status"). Recommended
  folding 3 drop-arc checks into @@Alex's manual desktop smoke.
- Completion: task-ChanGateway-Lead-4.md (batched per @@Lead's
  option); poked. All assigned tasks (1-7) complete; standing by.

## 2026-06-12 — STAND DOWN (round-close pending)

- @@Lead accepted tasks 6+7: full slate complete. Across the round:
  4 lane commits landed (5c44bf00 tidy-up, 26f72350 + 7d79259c
  archaeology riders, 2d13684a rg-resweep fix), 11 peer commits
  reviewed (2 desktop, 6 core, 4 web incl. the security pair) with
  zero code defects found and 4 doc findings routed (F1/F2 desktop,
  F-W1/F-W2 web).
- Loose ends at stand-down: NONE mine. chan-psql container + ssh
  :5432 bridge intentionally left up for @@Lead's integrated gate
  (teardown commands in task-ChanGateway-Lead-1.md §Gate).
- For the retrospective, self-noted: (good) file-anchored sweeps,
  whole-diff non-comment extraction, positional-pin uniqueness
  checks, adversarial probes beyond the asks; (bad) bundled
  verify+poke twice despite the standing lesson — cost two duplicate
  pokes; one hallucinated commit hash caught only on read-back —
  hashes must be copied from anchored git output, never recalled.
