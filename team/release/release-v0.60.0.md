# v0.60.0

The v0.60.0 release report, consolidated from the round's lane journals and task files as the streams landed on `main`. This is the axum 0.8 migration era: both Cargo workspaces moved from axum 0.7 to 0.8.9 in one three-lane round (a lead plus two parallel worker lanes on disjoint trees), cut as `v0.60.0-rc1` for a host smoke on chan-desktop and the devserver. The smoke found one CLI bug (`chan upgrade` vs prerelease versions), fixed in-round after an adversarial verification pass caught two ordering defects in the first fix; GA `v0.60.0` is being cut with it.

## Work streams (from `dev/v0.60.0/request.md`, "Axum 0.8 bump")

- [x] Root workspace on axum 0.8 (chan-server, chan-library, tunnel crates, desktop consumers), branch `axum08-root` (lane: Root)
- [x] Gateway workspace on axum 0.8, coupled with tower-sessions 0.14 + tower-sessions-sqlx-store 0.15 + tokio-tungstenite 0.29, branch `axum08-gateway` (lane: Gateway)
- [x] Integration: merge order, CHANGELOG reconcile, gateway lock prune, multi-agent seam review (lane: Lead)
- [x] Full pre-push gate on integrated main (green at b8c1e092 after one integrated-gate catch, below)
- [x] `v0.60.0-rc1` cut + host smoke (one finding, below)
- [x] `chan upgrade` prerelease fix, branch `upgrade-prerelease-fix` (lane: Root, from the rc smoke)

## Coordination scheme

A three-member team (`dev/v0.60.0/team/`): Lead sequencing and integrating from the repo root, Root owning the root workspace in the shared main checkout, Gateway owning `gateway/` from a scratch git worktree so the shared directory only ever held one worker branch. Both worker lanes ran /fabler + /rustacean; the lead ran /fabler with the architect, gate, and release skills. Pokes were one-line pointers into task files; every cross-lane claim was re-verified by the lead against primary sources (crates.io metadata, git objects) before redirecting a lane. Worker-to-host communication routed through the lead; the host kicked off the round and closes it after the rc smoke.

## Root workspace stream

**Branch:** `axum08-root` (main checkout), 5 commits, final sha 983821d8. Merged onto `main` as b2ec60c7. **Status:** complete, own-gate green, live WS smoke passed.

What shipped: axum 0.8.9 with the same feature set (ws, macros, multipart); 27 route strings rewritten to `{param}`/`{*rest}` syntax (two sites beyond the brief's inventory, found by the lane's own sweep); 10 WS construction sites moved to `Message::text`/`Message::binary` (two static control payloads dropped a per-send allocation via `from_static`); tokio-tungstenite 0.26 to 0.29 (aligning with axum 0.8.9's internal pin, so the root lock carries a single tungstenite tree); the unused tower_governor dependency dropped from chan-tunnel-server, clearing the entire axum 0.7 subtree from the root lock; about 16 comment sites updated.

The methodology highlight of the round: four routing behavior pins (launcher root fallback vs prefix nesting, wildcard capture shape) were written and run green on axum 0.7 FIRST, then re-run green on 0.8, so behavior preservation is proven by the same tests on both frameworks rather than asserted. The 0.8 nest-inherits-fallback change turned out structurally inert here: the launcher root fallback is manual oneshot dispatch in the chan-library host, not axum nest inheritance, and the two production nests wrap inner routers that own their fallbacks.

Verification: own-gate green after the last edit (fmt, clippy workspace, 40 test binaries, no-default-features chan-server build). Live smoke on the 0.8 build: a terminal WS session end to end (auth, JSON frames, binary PTY output, command echo verified), the library watch stream delivering live updates through the migrated `{*prefix}` route, and probes over the three `{*path}` wildcard APIs confirming slash-free captures.

Surprises: contrary to the request's analysis, one `Option<T>` extractor existed (`Option<Json<RestartTerminalBody>>` on the terminal restart route). Kept after auditing every shipped caller; the behavior shift (a bodyless request still restarts with defaults, but a request declaring a Content-Type now rejects 415/400/422 instead of silently defaulting) is documented in the CHANGELOG and was later independently confirmed exact by the integration review, which also verified it is the only optional extractor in either workspace. A process incident during smoke: the first `chan open` handed the scratch workspace to the host's production devserver via the uid-scoped handoff socket (`CHAN_HOME` does not prevent it); registration was removed immediately and the re-run used `CHAN_NO_DEVSERVER_HANDOFF=1`.

Follow-up candidates recorded, not done: the axum `macros` feature is unused workspace-wide (kept for the rc to preserve change attribution); tungstenite 0.25+ defaults to a 128 KiB per-connection read buffer on the server side, a watch item for terminal-heavy workspaces (the desktop client side was already on the same default pre-round; `WebSocketConfig` is the tuning knob if the rc smoke shows it matters).

## Gateway workspace stream

**Branch:** `axum08-gateway` (worktree `../chan-axum08-gateway`), single commit cc44ca97. Merged onto `main` as e1a22ecc. **Status:** complete, own-gate green, combined validation against merged root green (tunnel suite 31/31).

What shipped: axum 0.8 (resolving 0.8.9, features macros + ws); tower-sessions 0.13 to 0.14.0 with tower-sessions-sqlx-store 0.15.0; tokio-tungstenite 0.24 to 0.29 with explicit handshake + connect features; 33 route templates rewritten (three in devserver-proxy's admin.rs beyond the brief's inventory); the devserver-proxy ws bridge converted to translate text frames and close reasons between axum's and tungstenite's `Utf8Bytes` wrappers; gateway docs and comments updated.

The pairing deviation that defined the stream: the brief said tower-sessions 0.15, but tower-sessions 0.15.0 pins `tower-sessions-core =0.15.0` while the newest sqlx-store requires `core ^0.14`, so the 0.15 pairing cannot compile. The lane proved it from crates.io dependency metadata, the lead re-verified against the same primary source, and 0.14.0 + 0.15.0 shipped instead (0.14 is where the axum-core 0.5 support the migration needed landed). The lane also source-diffed the store release against its predecessor (byte-identical `src/`, md5-matched) to establish zero prod-migration risk for the existing sessions table.

A second premise correction: the brief expected `FromRequestParts` + `#[async_trait]` conversions in the two devserver-proxy validators; they actually implement chan-tunnel-server's own `Validator` trait, consumed as `Arc<dyn Validator>`, so no conversion existed to do and `async-trait` stays for dyn-compatibility.

Verification: own-gate green from `gateway/` after the last edit; the four DB-backed suites (identity auth, desktop_authorize, tokens, and profile api) ran against a user-space Postgres 16.6 built from the official source tarball (no docker socket in the sandbox; recipe preserved in the lane journal); the SSE admin endpoint verified at runtime (event-stream headers, initial snapshot, live ticks, auth probes), which also proves every rewritten route template constructs, since axum 0.8 panics at router build otherwise. The tunnel handshake integration suite type-checks against the ROOT workspace's axum via a path dep, so it was validated pending-on-root, then run on the combined state: 31/31, first possible run, exercising the ws `Utf8Bytes` bridge end to end.

## Integration

Merge order was locked root-first once Gateway's recon proved `chan_tunnel_client::serve_substreams` takes a concrete `axum::Router` typed against the root workspace's axum, with chan-tunnel-client a dev-dependency of devserver-proxy: exactly one gateway test binary blocks until the root migration sits underneath it. Both merges were clean apart from the two CHANGELOG `[Unreleased]` entries (kept both, root first).

The seam neither lane could close alone: Gateway's committed lock was resolved against pre-merge root manifests, so after both merges `gateway/Cargo.lock` recorded chan-tunnel edges (axum 0.7.9, tower_governor) that no merged manifest declares, and no gateway build path passes `--locked`, so the drift would re-resolve silently rather than fail. Gateway predicted the re-resolve delta package by package in their combined validation; the integrator's prune commit (d6943dfd) matched the prediction exactly: 23 removals across three subtrees (axum 0.7, old tungstenite, governor closure), zero additions, zero version movement.

The integration review ran as a 45-agent workflow: six seam lenses (residual route syntax over the combined tree, the tunnel ws wire seam, both-locks coherence, the desktop tungstenite consumer, CHANGELOG surface accuracy with an independent caller audit, fresh-eyes cross-cutting), three adversarial refuters per finding. 13 findings confirmed, zero refuted, zero open blockers at tip: the one blocker-class item was the stale gateway lock, independently discovered by three lenses and already closed by the prune commit before the refuters ran. The rest were positive verifications (the ws bridge conversions checked zero-copy and infallible against vendored crate sources, close code+reason round-trip exact, upgrade signaling intact) plus the process findings carried below.

The full `make pre-push` gate caught exactly one thing the own-gate model predicts it will: a cross-language source-pin test (`web/packages/workspace-app/src/terminal/protocol.test.ts` greps the chan-server terminal route's Rust source to pin the PTY-stays-binary invariant) still expected the pre-migration `Message::Binary(...)` variant syntax. The invariant holds in the migrated code; the two patterns were repinned to the constructor form (b8c1e092) and the gate is green across all seven stages: fmt, clippy, workspace tests, no-default-features build, gateway build, web-check (2152 tests), marketing-check.

## The rc window: one finding, fixed in-round

`v0.60.0-rc1` was tagged from the gated tree and published (prerelease-flagged release, docker `latest` withheld; `/dl` latest metadata serves whatever tag was pushed, so the rc rode the live update channel by v0.56.0-rc1 precedent). The host smoke on chan-desktop and the devservers came back clean except one finding: `chan upgrade` on a 0.59.x devserver hard-errored on the rc metadata ("release version patch component must be numeric"). The diagnosis found a worse silent half: `semver_newer` treats unparseable input as "not newer", and an rc binary cannot parse its own version, so rc installs would never have been offered the GA upgrade.

The fix (Root's lane) makes `validate_version`/`parse_semver` prerelease-aware with semver ordering at the triple level and a deliberate, documented deviation inside one identifier so `rc2 < rc10` (strict semver's ASCII compare misorders pipeline-minted `rcN` names). The first fix went through a four-lens adversarial verification that executed the extracted implementation in harnesses; three lenses passed (parser edges, ordering totality over a 283-version corpus, a differential harness proving plain `X.Y.Z` caller flows bit-identical), and one found two real defects: the identifier-level numeric-below-alphanumeric rule was never applied (`0.60.0-2` sorted above `0.60.0-1a`, contradicting the code's own comment, hidden behind a test pinning only the friendly case), and leading-zero identifiers were accepted without a pinning test. Both were fixed by classifying identifiers before comparing (`Identifier::Numeric` below `Alphanumeric`, digit-run comparison retained within alphanumerics), pinned in both directions. Field impact: 0.59.x clients error on `chan upgrade` only while an rc is the latest published tag and heal at GA; the two rc1 smoke devservers need one manual GA install (the rc1 binary cannot see the GA offer); every binary from GA onward handles prerelease windows natively.

## Request-analysis corrections (for the next pre-round analysis pass)

Three `dev/v0.60.0/request.md` facts were falsified during execution, all from the same pre-round analysis: axum 0.8.9's internal tokio-tungstenite is ^0.29, not 0.26 (true only through 0.8.4); the tower-sessions 0.15 pairing does not exist on crates.io; and one `Option<T>` extractor did exist in the tree. None changed the migration's shape, but all three redirected work mid-round. Verify version-pairing claims against crates.io dependency metadata at execution time, not analysis time.

## Retrospective (through rc1; GA pending)

What went well:

- Two lanes on physically disjoint trees (shared checkout + scratch worktree) ran genuinely in parallel with zero staging conflicts; the shared directory never held more than one worker branch.
- Claim discipline held end to end: every cross-lane redirect was re-verified against primary sources before acting; the lock prune was predicted package-for-package before it was executed; behavior pins ran on both framework versions.
- Both lanes corrected their own briefs' premises (validators, route inventories, pairing) instead of following them into the weeds, and reported the deviations rather than silently absorbing them.
- Verification that EXECUTES the code under review beat verification that reads it, twice: the rc-fix defects sat exactly where the author's reviewers had read the ordering code but not run it, and the lock-prune prediction was trusted only after the re-resolve reproduced it package for package.

What slowed us down / lessons:

- The shared checkout is a peer's in-flight branch, not `main`: a lead-side sweep read it as ground truth and nearly falsified a correct lane finding. Ground truth in a shared-tree round comes from git objects (`git show ref:path`, `git grep ref`).
- SSH is denied from the round's sandboxes (lead and workers alike); pushes go over HTTPS with the gh credential helper, and `git ls-remote`/`gh api` replace SSH-based remote verification. The pre-push hook is not installed in the round's clone, so the manual full gate is the only gate.
- Three stale request facts (above) cost three mid-round redirects.

## Carryover to v0.61.0

- CI path-filter seam, demonstrated live this round: `gateway-ci.yml` triggers only on `gateway/**`, so root-side changes to the tunnel path-dep crates never run gateway clippy/tests, and `make gateway-build` (also the release.yml path) passes no `--locked`, so gateway lock drift re-resolves silently. Fix options: add `crates/chan-tunnel-**` plus root manifests to the gateway CI paths, or add `--locked` to gateway-build.
- Root axum feature spec leaks into gateway binaries via chan-tunnel-client's workspace-inherited axum (multipart/multer compiled into gateway services that never use it); pairs with dropping the unused axum `macros` feature. Pre-existing shape, not a regression.
- The gateway's legacy http-0.2/hyper-0.14/rustls-0.21 duplicate tree hangs entirely off oauth2 4.4; oauth2 5.x (reqwest 0.12) would collapse it.
- Server-side WS read buffer (128 KiB per connection since tungstenite 0.25, arriving here via axum 0.8's internal 0.29): watch on terminal-heavy workspaces; `WebSocketConfig` is the knob.
- Unix-domain socket `--bind={path}` for the devserver (deferred in v0.59.0 because axum 0.7 was TcpListener-only): axum 0.8's generic `Listener` removes that blocker; re-evaluate.
