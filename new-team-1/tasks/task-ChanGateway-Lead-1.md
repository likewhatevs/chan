# task-ChanGateway-Lead-1 — gateway tidy-up complete

From: @@ChanGateway. To: @@Lead. Re: task-Lead-ChanGateway-1.
Commit: **5c44bf00** on main (pathspec-atomic: gateway/** +
docs/manual/gateway.md; verified no peer WIP swept in). No push.

## Gate

- Baseline before any edit: fmt clean, clippy 0 warnings, cargo test
  --workspace 153 passed.
- After last edit (re-ran in order): fmt --check clean, clippy
  **0 warnings**, cargo test --workspace **153/153** green.
- Postgres: stood up the chan-psql sdme container (rootfs already
  existed) per gateway/docs/dev-setup.md. NOTE for your isolated
  gate: lima's port-forwarder can't see sdme's DNAT publish, so
  127.0.0.1:5432 needed an ssh bridge:
  `ssh -F ~/.lima/default/ssh.config -f -N -L 5432:<container-ip>:5432 lima-default`
  (container IP via `sdme ps`). I LEFT the container + bridge UP in
  case your integrated gate runs gateway tests; kill with
  `pkill -f 'ssh -F .*lima-default'` + `limactl shell default sudo
  sdme stop chan-psql` when done.

## Archaeology scrub (item 1)

- Phase mentions: **0** in gateway/ outside node_modules (file-anchored
  grep, all file types incl. design.md/READMEs) — confirms your recon.
- Handle artifacts: one real hit, gateway/docs/dev-setup.md
  ("@@Host uses Cloudflare") — neutralized.
- ~26 changelog-style comment sites rewritten current-snapshot
  (no longer / used to / v0 / follow-up / "item #2" / "#18" refs).
  Two were factually WRONG, not just stylistic:
  - admin/main.rs module doc said tunnel ps/kill/watch "land in a
    follow-up" — they are implemented.
  - identity/http.rs said /internal/* is gated by PROFILE_AUTH_TOKEN —
    it's IDENTITY_INTERNAL_TOKEN (README had the same error).

## Hygiene (item 2)

- Warnings: 0 before, 0 after (baseline was already clean).
- Param-struct refactor (the one >6-param fn in the workspace):
  `ApiTokenService::create` 7 args + allow(too_many_arguments) →
  `create(NewToken<'_>, &RequestMeta)`; revoke/validate/write_audit
  now take the same `RequestMeta` (the ip/user_agent pair that
  recurred at 5 call sites). allow attribute deleted. Note: the old
  code carried an inline comment arguing AGAINST a builder; I judged
  the named-fields struct strictly clearer than 7 positionals and
  your task spec mandated it — flagging since I overrode an inline
  decision note.
- Dedup into gateway-common: throttle default limits (4 rps/16
  burst/4096 map) were duplicated in identity/token_throttle.rs and
  workspace-proxy/throttle_validator.rs; now pub consts in
  gateway_common::token_bucket (they're documented defense-in-depth
  twins — single-sourcing prevents drift).
- Dedup considered and SKIPPED (judgment calls):
  - error.rs enums across the 3 services: variants + IntoResponse
    mappings genuinely differ, and gateway-common's design doc
    documents "consumers map errors locally" as a decision. Merging
    adds coupling for no drift risk.
  - tracing-init boilerplate (6 lines x4 binaries): gateway-common
    has no tracing-subscriber dep; adding one to a shared lib for
    this is a bad trade.
  - identity's profile_client/workspace_admin_client/static_files
    "duplicates" from your task hint: already thin documented
    re-export shims; nothing to do.
- Non-idiomatic: nothing clear-cut found (clippy-clean baseline;
  manual loops the recon agent flagged in proxy.rs are deliberate
  and clearer than the iterator alternative).

## Docs (item 3)

All 5 design.md rewritten as current-snapshot, grounded in a full
source read. Factual fixes worth knowing:
- gateway-common: claimed "four modules", there are eight; decode()
  signature was stale; error table listed a phantom InvalidSignature
  variant (signature failures collapse into Decode).
- identity: desktop_authorize (738-line module, 3 routes) was
  entirely undocumented; "per-PAT scopes not wired" was FALSE
  (scopes shipped: tunnel/tunnel.public, migration 0008).
- workspace-proxy: claimed a tower_governor rate limit on the admin
  tree — the code deliberately has none (documented rationale in
  admin.rs); tunnel listener port said 7003, default is 7100;
  "tower::timeout" — actual mechanism is tokio timeout + DeadlineBody.
- profile: schema block missing api_tokens.scopes; audit-action
  invariant missing created_via_desktop.
- admin: documented per-service token env vars that don't exist
  (CLI takes a single CHAN_ADMIN_TOKEN / --token).

READMEs (root + 5 crates): root listed a phantom
crates/workspace-proxy/web SPA (workspace-proxy ships none),
referenced the deleted chan-writer org, had a v0 "(done)" status
list, said "six debs" (it's 8: 4 pkgs x 2 archs), and its identity
dev-run env was missing the REQUIRED IDENTITY_INTERNAL_TOKEN +
WORKSPACE_GATE_SECRET (instructions as written would bail at boot).
identity README: internal route bearer fixed + desktop routes added;
profile README: 2 missing routes added (by-username, admin email),
sessions/ownership claims corrected; admin README: nonexistent
`user update --email` replaced with the real change-email subcommand.

docs/manual/gateway.md: verified every claim against source
(--tunnel-url flag + CHAN_TUNNEL_TOKEN are real; route/role table
corrected: profile is called by identity + CLI, not "the others").

## Flags / for your attention

1. I REMOVED the manual's "Verification status" section (audit-process
   meta, not user content). docs/manual/desktop.md has the same
   pattern and is YOUR lane — decide consistency there.
2. gateway/package.json version is "0.0.0" (private, never
   published). Memory says version pins bump in lockstep; if you
   want it pinned to 0.31.1, that's a release-mechanics call — left
   untouched.
3. Per task: ready for a second-pass review of another lane's diff
   if you want to route one.

Journal: new-team-1/journals/journal-ChanGateway.md (full trail).

## Addendum (task-Lead-ChanGateway-2 — extended patterns)

Ran the wider sweep (systacean/desktacean/desktest, extra @@handles,
round-/wave-/slice/track vocab) over gateway/** + docs/manual/
gateway.md, file-anchored:
- ONE real hit: scripts/dev/sdme/build-gateway.sh cited "the round-4
  de-risk"; rewritten as a timeless cross-reference to
  build-chan-desktop.sh. Commit **26f72350** (one file, 2 lines,
  pathspec-atomic).
- False positives only otherwise: proxy.rs "const slice" /
  "configuration slice" (Rust slices, matched `slice [a-z0-9]+`).
- docs/manual/gateway.md: zero hits; no illustrative handles present.
- Re-sweep after fix: zero hits.
No gate impact (shell comment); fmt/clippy/test state unchanged from
the task-1 report.

## Rider (task-Lead-ChanGateway-3 — chanwriter purge)

Commit **7d79259c** (3 files, pathspec-atomic, verified lane-pure).
- Your known hit (gateway/README.md:62 "chan-writer/chan/web/") was
  already gone — fixed in 5c44bf00's README rewrite.
- The wider net caught 3 hits my round-1 patterns missed: all three
  packaged systemd units carried
  `Documentation=https://github.com/chan-writer/chan-gateway` (dead
  org, user-visible via systemctl status). Now
  `https://github.com/fiorix/chan/tree/main/gateway`.
- Re-sweep `chanwriter|chan-writer|chan_writer` over gateway/** +
  docs/manual/gateway.md: **zero hits**.
- No gate impact (unit-file text consumed by cargo-deb as assets);
  fmt/clippy/test state unchanged from the task-1 report.
