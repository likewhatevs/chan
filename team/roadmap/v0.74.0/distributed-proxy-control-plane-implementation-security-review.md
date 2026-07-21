# chan-gateway v0.74.0 control-plane implementation security review

Status: complete. Decision: accept the hardening proposal with mandatory amendments; refuse the current implementation as release-ready. Reviewed `v074/ctl` at `6ada0579b5dd710a645aa3f12049cdd069224230` on 2026-07-21. The review changed no implementation code.

## Executive decision

Accept [distributed-proxy-control-plane-hardening.md](distributed-proxy-control-plane-hardening.md) as the security direction, not as an unchanged implementation contract. The current branch has multiple independently exploitable authorization failures and several supported-deployment gaps that violate the stated credential-poor proxy model. It should not be merged or shipped as a remotely exposed, shared, or Desktop-enabled control plane until the blocking set below is closed.

The acceptance is conditional in four ways:

1. A1 through A7, A9, A10, A18 through A23, and the new A24 through A26 are required before the affected public, shared, Desktop, or multi-host paths are called production-ready. Some are code merge gates and some are deployment gates; the disposition table separates them.
2. A4, A6, A13, A14, A21, and A22 need factual or mechanical corrections before implementation. In particular, preserve the existing PAT-derived per-tunnel assertion key instead of replacing it with a node-wide key.
3. The existing implementation is not evidence that the proposal is unnecessary. Several green tests pin behavior that the proposal must deliberately replace, including lexicographic restart winners, removal on `CommandResult`, cookie-authenticated WebSockets with no `Origin`, continuously refreshed bridge lifetime, and control-loss grace cancellation at `SnapshotAccepted`.
4. The reliable-tunnel goal stays in scope. The proposal does not require controller HA, proxy-to-proxy traffic, durable controller state, mixed-version support, or automatic DNS and certificate management.

If the decision has to be reduced to one line: **accept the proposal with the corrections in this review, and refuse the current branch as release-ready until the blocker acceptance tests pass.**

## Scope and method

The review followed the live path end to end:

`chan devserver -> tunnel client -> devserver-proxy tunnel listener -> identity PAT validation -> proxy control stream -> devserver-control aggregate -> identity/profile entry decision -> browser or Desktop -> devserver-proxy HTTP/WebSocket gate -> per-tunnel gateway assertion -> chan-server routes`

It also reviewed the controller admin API, grant and account revocation, Desktop exact-origin capability minting, systemd and Kubernetes packaging, the supported multi-user sdme image, and failure behavior during controller and proxy outages.

Three independent lanes covered control protocol/state, browser/API authorization, and tunnel/Desktop/deployment boundaries. Every critical and high conclusion below was rechecked against the pinned source by the lead review. The Fabler, webdev, syseng, and rustacean disciplines were used. No dedicated network-security skill was available in this environment, so the network portion used an explicit attacker model, hop-by-hop credential inventory, transport review, deployment manifest review, and adversarial protocol tracing instead.

Only local verification was authorized. No production edge, DNS, certificate, firewall, service mesh, or cluster was inspected. A complete dependency advisory scan was not run because `cargo audit` was not installed; this review is about the control-plane and data-path design, implementation, and supported packaging rather than a claim that every transitive dependency is vulnerability-free.

## Threat model used for the decision

- An external attacker able to reach a mistakenly exposed internal listener.
- An authenticated Chan user with a valid PAT and arbitrary content on their own tenant origin.
- A grantee whose access should be confined to one explicit devserver and should end on revoke.
- A holder of one proxy control credential.
- One compromised devserver-proxy process, pod, or host service.
- A compromised controller that cannot forge an identity signature.
- Malicious JavaScript on one tenant origin running in a browser that also has a valid cookie for another tenant.
- A stale username, tunnel, node record, browser session, or Desktop origin grant surviving an ownership or placement change.
- A passive or active observer on any non-loopback service hop.

The security objective is stronger than transport availability: no actor should gain another immutable owner's devserver, shell, workspace data, browser session, or Desktop native authority; revocation must have a bounded and observable completion time; and one proxy compromise must not become fleet, database, OAuth, or operator compromise.

## What is already sound

The review tried to invalidate these properties and did not:

- Control frames are length-delimited and reject a frame over 1 MiB before allocating its declared body. Existing mpsc and broadcast queues are bounded and overflow fails the session instead of silently dropping state.
- Snapshot timing, heartbeat timing, generation contiguity, stale-incarnation fencing, UUID-targeted kills, and delayed-successor protection are implemented and directly tested.
- Routine joining snapshots are now live-first and do not evict a current live row. The older hardening report's claim that routine joins use the lexicographic restart rule is stale; only initial restart reconciliation still does.
- Identity now independently validates a controller-provided proxy base as a canonical origin exactly one label under the configured proxy apex with matching scheme and effective port. The A13 namespace-escape exploit is not present on this branch.
- Desktop's move-node lifecycle tears down the old connection and re-enters through a fresh validated entry. The A14 statement that pin reset is unspecified is stale.
- Gate JWTs enforce algorithm, type, expiry, audience, and devserver. A cookie copied to a different host or devserver is rejected.
- Unsafe HTTP methods require the CSRF mirror. Public wildcard routing does not expose the management API.
- Proxy-to-chan-server assertions already use a key derived independently from the tunnel PAT on both endpoints. This is narrower than a node-wide assertion key and should be retained.
- The Debian single-host example binds internal services to loopback and has substantial systemd sandboxing. The transport and exposure finding applies to non-loopback deployments, especially the shipped Kubernetes shape, not to a correctly preserved loopback hop.

These properties reduce accidental corruption and some replay classes. They do not repair the authority, revocation, browser-origin, or deployment-compartment failures below.

## Release-blocking findings

### 1. Proxy membership and tunnel ownership are self-asserted

`AdmissionRequest` contains only request and registration UUIDs, username, and devserver id (`gateway/crates/devserver-control-proto/src/lib.rs:156-161`). The controller trusts those strings, records a pending claim, accepts a matching `TunnelUp`, replaces an existing owner of the same key, and commands the honest proxy to kill the old registration (`gateway/crates/devserver-control/src/state.rs:318-493`). One shared bearer authenticates every proxy before `ClientHello`; the peer then chooses its proxy id and an origin derived from that same id (`gateway/crates/devserver-control/src/server.rs:170-289`).

A holder of the fleet token can therefore join as a new or existing node, fabricate an admission for a known user and devserver, move the authoritative row to itself, and have the controller evict the real tunnel. It can also fill a victim's admission capacity with fabricated pending keys. Duplicate proxy ids immediately retract the incumbent's rows, and restart snapshots still accept self-authored rows with no liveness or identity proof.

This confirms A1 through A4. The required root fix is an identity-signed, short-lived admission lease bound to immutable owner id, devserver id, registration id, proxy id, purpose, protocol version, issuance, and expiry, plus provisioned per-proxy authentication and duplicate-id anti-displacement. A1 also needs an explicit client-to-proxy lease-refresh protocol: the live client may use its retained PAT to obtain refreshes, but the proxy must not retain the raw PAT or receive a proxy-renewable bearer.

### 2. Controller work is attacker-amplified and production disables the intended user cap

The 1 MiB frame limit and 100,000-row snapshot limit are multiplicative. `TunnelRow` strings are unbounded, live deltas and pending claims have no session or fleet cap, and the singleton actor rebuilds, sorts, clones, and compares complete tunnel and proxy views after every command and every tick (`gateway/crates/devserver-control/src/actor.rs:167-183,500-527`). A single authenticated session can consume disproportionate memory and actor time until honest heartbeat deadlines expire.

The supported systemd and Kubernetes configuration sets `MAX_DEVSERVERS_PER_USER=0`, removing the one user-level bound that could limit normal clients (`packaging/gateway/scripts/configure.sh:176-184`, `packaging/kube/config.yaml:46-48`).

This confirms A5 as a merge gate. Bound strings before insertion, cumulative snapshot bytes, rows including deltas, pending claims, sessions, aggregate bytes, watchers, and per-user registrations. Publish dirty snapshots on a coalesced cadence rather than cloning the world on every read, heartbeat, and mutation. Production must use a finite nonzero per-user cap.

### 3. One proxy can impersonate browser users across the fleet

Identity and every proxy share `DEVSERVER_GATE_SECRET`; any holder can mint a valid 24-hour session with attacker-chosen subject, role, audience, and devserver. An honest destination proxy accepts it and then signs the upstream request with that destination tunnel's legitimate PAT-derived assertion key. This turns one compromised node into cross-node owner impersonation.

This confirms A6 but rejects one proposed mechanism. Use a node-scoped entry-verification key and A20 proxy-local opaque sessions. Preserve the existing per-tunnel, PAT-derived proxy-to-chan-server assertion key; replacing it with `HMAC(gate_master, proxy_id)` would broaden authority and create avoidable key-delivery and node-move problems.

### 4. Mutable username is treated as ownership

Controller rows and keys identify ownership by mutable username and devserver id, with no immutable owner user id. Username rename updates profile state but does not retire the old tunnel. Profile's owner shortcut returns `owner` when caller id equals the requested owner id without first proving that the requested devserver row exists for that owner (`gateway/crates/profile/src/http.rs:1189-1222`). Identity then joins the current username owner to the stale username-indexed controller row.

The concrete crossover is: Alice renames while her old `alice` tunnel remains live; Bob claims `alice`; identity resolves Bob as the current owner, selects Alice's stale row, profile's unconditional owner shortcut approves Alice's devserver id under Bob's id, and Bob receives owner and shell-equivalent access to Alice's devserver.

This confirms A18 as critical. The authoritative key must be `(owner_user_id, devserver_id)` from PAT validation through admission lease, controller state, profile lookup, entry, session, assertion, roster, and Desktop validation. Username is routing and display metadata only. Rename must retire stale rows and sessions before reuse.

### 5. The grant model promises roles that do not exist, and assertion validation fails open

The UI and APIs advertise `viewer` and `editor`, but both reach workspace mutation and terminal routes. Role enforcement covers only selected launcher mutations. A terminal runs as the devserver owner's Unix uid, so any granted data-plane access is shell-equivalent.

Worse, missing or invalid gateway assertions become `TunnelOrigin { caller: None }` and continue (`crates/chan-server/src/devserver.rs:1099-1144`). Authentication admits any request carrying `TunnelOrigin`, regardless of verified caller (`crates/chan-server/src/auth.rs:119-127`). Proxy assertion-signing failure similarly degrades to forwarding without an assertion (`gateway/crates/devserver-proxy/src/proxy.rs:1051-1068`).

This confirms A19. Remove devserver-share roles and present one explicit binary, shell-equivalent grant. Require a valid assertion bound to subject, immutable owner, exact devserver, and exact audience before every tunnel route. Missing, malformed, expired, wrong-owner, wrong-devserver, and wrong-audience assertions must fail before route execution.

### 6. Grant and account revocation do not terminate browser access

Grant deletion removes only the database row. Browser sessions are stateless 24-hour JWTs and are never checked against the current grant after minting. Block and delete paths perform best-effort tunnel kills and log failures. WebSocket bridges have a resettable idle timeout, no absolute authorization deadline, and no cancellation handle, so normal traffic can keep a revoked terminal or document socket alive indefinitely.

This confirms A10 and A20 as a single release dependency. Proxy-local opaque sessions need bounded records, absolute expiry, and cancellation handles. Grant delete must revoke the exact subject/owner/devserver tuple on every connected proxy; account block and delete must revoke the subject fleet-wide. HTTP streams and WebSockets must close before acknowledgement, retries must be durable enough to survive partial node failure for the stated window, and APIs must distinguish pending revocation from completed revocation.

### 7. Cookie-authenticated WebSockets do not authorize browser origin

The proxy authenticates a WebSocket with the target host's cookie but never reads or validates `Origin` (`gateway/crates/devserver-proxy/src/proxy.rs:245-354`). Sibling tenant JavaScript can therefore open the victim tenant's terminal, document, scene, event, or watch socket in a victim browser. The browser supplies the victim's correctly scoped cookie; the proxy ignores the attacker's origin and relays a bidirectional shell or mutation channel.

This confirms A21 as critical. Require exactly one canonical origin equal to the externally visible request origin, with forwarded host and scheme trusted only from configured edge peers. Reject missing, multiple, opaque, malformed, sibling, wrong-scheme, and wrong-port origins before upgrade. Desktop's native tungstenite watch requests currently add `Cookie` but no `Origin` (`desktop/src-tauri/src/window_watcher_wiring.rs:403-440`); the implementation must add the exact canonical `Origin` there or the correct server hardening will break Desktop feeds.

### 8. Same-origin active content can inherit web and Desktop authority

The file API serves extension-classified SVG bytes inline as `image/svg+xml` with no attachment or sandbox (`crates/chan-server/src/routes/files.rs:304-320,560-650`). Neither chan-server nor devserver-proxy adds a credentialed-page response policy that supplies CSP framing denial, `nosniff`, no-referrer, or private no-store behavior. A malicious repository or grantee can plant SVG and induce navigation at the credentialed exact tenant origin.

Desktop amplifies this browser bug: after the entry flow trusts that exact origin, its runtime capability grants workspace-window actions, file picking, clipboard-related commands, opener access, zoom, and fullscreen (`desktop/src-tauri/src/runtime_capability.rs:68-100`). Exact-origin ACL does not contain code that is executing inside that origin, and the runtime capability cannot be removed until app restart.

This confirms A22 and makes its minimum a Desktop and public rollout gate. Force demonstrated active formats such as SVG to attachment with a restrictive sandbox, or serve user-controlled active content from a cookieless untrusted origin. Add framing denial, MIME allowlisting, `nosniff`, no-referrer, and appropriate no-store policy. Direct inline HTML execution was not demonstrated on this branch; keep HTML in the defensive policy without citing it as the proven route.

### 9. Entry handoff is a replayable URL bearer containing PII

Entry JWTs contain display name and email and are placed in `?t=` for browsers and Desktop. They have no single-use identifier or replay store. A token copied from history, request logging, or diagnostics during its 30-second life can mint another 24-hour shell-equivalent session, while decoded PII persists in logs after authorization expiry. Session-issuing redirects do not set explicit no-store or no-referrer policy.

This confirms A23 as high-impact browser hardening and a public rollout gate. Exchange a short-lived, single-use code through POST, remove PII from authorization material, reject replay by `jti`, and prove that browser-visible URLs, history fixtures, access logs, audit events, and errors contain neither bearer nor decoded PII.

## New findings outside A1-A23

### A24. Isolate service credentials and operating-system identities

The Kubernetes example defines one Secret containing the database URL and password, OAuth client secret, profile service and admin tokens, identity internal token, fleet gate secret, and controller admin and proxy tokens (`packaging/kube/secret.example.yaml:27-41`). Every workload, including devserver-proxy, receives the whole Secret through `envFrom` (`packaging/kube/devserver-proxy.yaml:33-37`). There is no NetworkPolicy in the shipped manifests. Proxy compromise therefore becomes database, OAuth, identity, controller, and operator compromise.

The systemd packages have the same credential-collapse property despite otherwise good sandboxing. Identity, profile, controller, and proxy all run as `chan-gateway`; generated env files are all `root:chan-gateway 0640`, so any compromised service can read every sibling's credentials. The admin package can additionally reset `/etc/chan-gateway` from `root:chan-gateway` to `root:root`, making service restarts fail depending on package installation order (`gateway/crates/admin/packaging/postinst:4-8`).

Required outcome: separate Unix service identities and per-service readable env files; per-workload Kubernetes Secrets using explicit `secretKeyRef`; least-privilege database roles and service credentials; default-deny ingress and egress NetworkPolicies; and a package-order test that preserves directory traversal for each intended service while keeping operator credentials root-only. Resource limits, read-only root filesystems, seccomp, capability drops, and disabled service-account token automount are follow-up defense in depth.

### A25. Make the devserver host a real tenant boundary or declare it one trust domain

The supported sdme image advertises multiple per-user devservers, but the provisioner grants every such Unix user unrestricted `NOPASSWD:ALL` (`packaging/sdme/chan-devserver-provision.sh:89-124`). A devserver grant reaches a terminal as the owner's uid. From there a grantee can become root, read sibling users' PAT-bearing unit files, and cross every devserver and workspace in the container.

The current A19 phrase "exact shared devserver" is false for this deployment. Required outcome: one externally shared trust domain per container or VM, or remove unrestricted sudo and prove OS isolation between users. If neither is intended, product and deployment documentation must state that one grant is container-root and all-devservers equivalent, and multi-user use must be prohibited.

### A26. Bound control-loss grace by restored authority, not snapshot receipt

The proxy cancels its eviction deadline when `SnapshotAccepted` arrives, but it becomes ready only at `FleetReady` (`gateway/crates/devserver-proxy/src/control.rs:282-286,570-572`). If reconciliation never completes, the controller remains unable to serve authoritative reads or kills while the proxy can retain existing tunnels indefinitely. The passing controller-restart E2E scenario explicitly depends on snapshots cancelling eviction before fleet authority returns.

Required outcome: snapshot acceptance may extend grace to a fixed convergence deadline, but only `FleetReady` cancels it. Connected-but-unready past the hard deadline must evict local tunnels and sessions. Immutable subject-wide revoke and block commands should fan out to every connected proxy even while aggregate reads are warming. Add paused-time coverage for reconciliation that retries forever.

## A1-A23 disposition

| Amendment | Decision | Gate | Current-branch correction or constraint |
|---|---|---|---|
| A1 | Accept | Merge | Specify lease refresh over the live tunnel protocol; never retain the raw PAT in the proxy. |
| A2 | Accept | Merge | Per-proxy allowlisted identity and revocation are required; template equality is not membership. |
| A3 | Accept | Merge | Land retry-safe A9 semantics first; hold incumbent rows until replacement proves authority. |
| A4 | Accept, revise | Merge | Routine join is already live-first. Signed restart rows and fail-closed duplicate reconciliation remain required. |
| A5 | Accept | Merge | Include global bytes/work bounds, coalesced watch publication, watcher caps, and a finite production user cap. |
| A6 | Accept, revise | Merge | Node-scope entry verification and use opaque sessions. Preserve PAT-derived per-tunnel assertions. |
| A7 | Accept, scope | Deployment | Plain HTTP/h2c is acceptable only on verified loopback. Every non-loopback hop requires authenticated encryption and exposure policy. |
| A8 | Accept | Pre-GA | Observed down and bounded tombstones improve detection; node isolation remains the response to a consistently lying proxy. |
| A9 | Accept | Merge | Protocol rejection must retry while unready, not exit the process and drop all tunnels. |
| A10 | Accept with A20 | Merge | Retry and completion semantics must cover tunnel and browser-session revocation. |
| A11 | Accept | Follow-up | `__Host-` cookies are useful defense in depth, not a replacement for A21 or A20. |
| A12 | Accept | Follow-up | Add transfer damping after immutable ownership and admission proof are authoritative. |
| A13 | Accept, revise | Merge via A1/A18 | Node-base re-anchoring is already implemented. Keep lease and immutable-owner cross-checks. |
| A14 | Accept, revise | Pre-GA | Move/pin lifecycle is already implemented. Add owner and first-label binding and fix the `127.*` DNS HTTPS waiver. |
| A15 | Accept | Pre-GA | Add ownership, admission, kill, revoke, peer, and initiator audit without credential or content logging. |
| A16 | Accept, split | Pre-GA/follow-up | Service credential isolation is blocking under A24. PII minimization and watcher limits remain required hardening. |
| A17 | Accept | Follow-up | Add admin response versioning and consumer skew diagnostics. |
| A18 | Accept | Merge | Critical immutable-owner binding and username-reclaim closure. |
| A19 | Accept | Merge | Binary shell-equivalent grant and fail-closed assertion middleware. Reconcile with A25. |
| A20 | Accept | Merge | Revocable opaque sessions, stream cancellation, and absolute deadlines. |
| A21 | Accept, revise | Merge | Exact-origin WebSockets plus explicit exact `Origin` on Desktop native clients. |
| A22 | Accept | Desktop/public gate | Demonstrated SVG active-content route plus Desktop native authority makes the minimum blocking. |
| A23 | Accept | Public gate | Single-use POST exchange, no URL bearer or PII, no-store/no-referrer. |

## Hardening implementation proposal

The order matters. Do not parallelize across a boundary whose authority model is still changing.

### Phase 0: amend the contract and contain current deployments

- Update the control-plane design and ADR with A24 through A26 and the corrections to A4, A6, A13, A14, A21, and A22.
- State that a devserver grant is binary and shell-equivalent. State the sdme trust-domain rule explicitly.
- Keep multi-host Kubernetes, sharing, and Desktop remote-origin rollout disabled until their named gates pass.
- Set a finite nonzero production devserver cap and verify every internal listener remains loopback-only until protected transport and network policy exist.

### Phase 1: establish immutable protocol authority

- Land A9 retry semantics first so expected rejection cannot kill a proxy process.
- Add `owner_user_id` end to end and make `(owner_user_id, devserver_id)` authoritative.
- Add the identity-signed admission lease and its client-driven renewal path.
- Add provisioned per-node control identity, rotation overlap, disablement, and duplicate-id anti-displacement.
- Require leases on snapshots and deltas; remove attacker-selected restart winners.
- Add A5 bounds and dirty/coalesced view publication.
- Move grace completion from `SnapshotAccepted` to `FleetReady` with the A26 absolute deadline.

### Phase 2: make authorization and revocation real

- Remove viewer/editor from the devserver-grant schema, APIs, UI, entry/session claims, assertions, and roster.
- Make assertion verification shared, mandatory, and fail closed for every tunnel route.
- Replace portable session JWTs with bounded proxy-local opaque sessions. Node-scope only entry verification; retain per-tunnel PAT-derived assertions.
- Add exact and subject-wide session revocation commands, active stream cancellation, absolute deadlines, fan-out while warming, retry, audit, and honest completion states.
- Add cross-node, rename/reclaim, stale-row, grant-delete, block, and partial-node-failure tests before consumer migration.

### Phase 3: close browser and Desktop boundaries

- Enforce exact canonical WebSocket `Origin` and trusted forwarded-header policy. Update Desktop native WebSocket clients to send the exact origin.
- Move entry exchange to single-use POST with no PII and add replay protection and no-store/no-referrer behavior.
- Contain SVG and other active user content, deny framing, pin MIME, and add the response policy required by A22.
- Bind Desktop entries and roster rows to immutable owner id and the exact `{username}--{devserver_id[..12]}` label. Fix loopback detection by parsing IP literals.
- Define hard Desktop revocation behavior for the unremovable runtime capability, including managed window closure and an app-restart path when authority must be purged.

### Phase 4: make supported deployment match the threat model

- Split systemd users, groups, env files, database roles, and operator credentials. Fix the admin package directory ownership regression.
- Split Kubernetes Secrets per workload, apply default-deny NetworkPolicies, and expose only intended public ports.
- Require TLS with peer authentication or an authenticated encrypted overlay for every non-loopback hop carrying PATs, internal bearers, assertions, terminal/file traffic, or control frames. Refuse non-loopback h2c without an explicit test-only override.
- Split identity's internal validation surface onto a protected listener or prove equivalent edge and network enforcement.
- Resolve A25 by one trust domain per container/VM or by removing unrestricted sudo and proving OS isolation.

### Phase 5: defense in depth and operations

- Land A8 tombstones and quarantine, A11 cookie prefixes, A12 transfer damping, A15 audit and alerts, A16 least privilege and watcher bounds, and A17 version headers.
- Add Kubernetes resource/security contexts and service-account minimization.
- Run a complete Rust advisory scan in the release gate and preserve its evidence with the release artifacts.

## Required acceptance tests

The branch becomes acceptable only when the tests prove the negative cases, not merely the happy path:

- A node without a valid per-node identity cannot claim any proxy id. A valid node cannot fabricate admission, snapshot, delta, owner id, or lease fields, and cannot cause the real registration to be killed.
- A healthy incumbent survives duplicate connects. A legitimate restart converges without process exit. Reconciliation that never reaches `FleetReady` hits a bounded fail-closed deadline.
- Every string, frame, row, pending claim, session, byte, fleet, user, and watcher limit has exact-boundary tests. An attacker cannot starve honest heartbeats through actor publication work.
- Username rename plus immediate reclaim cannot expose a stale tunnel. Unknown-devserver owner lookup returns 404. Controller/profile field mixing fails closed.
- No devserver-share role remains. A grant is shown as shell-equivalent. Missing or invalid assertions fail before HTTP or WebSocket route execution.
- Grant deletion closes matching HTTP streams and active WebSockets. Block and delete revoke subject-wide. Unreachable-node retries and absolute expiry have deterministic paused-time tests.
- Exact WebSocket origin succeeds; missing, multiple, opaque, malformed, sibling, wrong-scheme, and wrong-port origins return 403. Desktop watch clients send the exact origin and continue to work.
- Active SVG cannot execute with tenant cookies or Desktop IPC authority. Credentialed pages cannot be framed and dynamic authenticated responses are appropriately non-cacheable and `nosniff`.
- Entry exchange is single-use, absent from URLs and logs, contains no PII, and cannot mint a second session on replay.
- A compromised proxy systemd user or Kubernetes pod cannot read sibling service, database, OAuth, admin, or master credentials and cannot reach unauthorized internal services.
- A grantee on the supported devserver image cannot cross into another Unix user's devserver or PAT. If that is intentionally impossible to guarantee, the image refuses the multi-user/shared combination.
- A three-node matrix covers two owners, one grantee, one ungranted caller, same devserver ids under different owners, wrong-node cookies, rename/reclaim, transfer, revoke during live sockets, proxy restart, controller restart, and a partitioned node.

## Verification performed

- `cargo test -p devserver-control-proto -p devserver-control -p devserver-proxy` in `gateway`: 4 protocol, 55 controller, 34 proxy unit, and 42 proxy API tests passed.
- `cargo test -p chan-tunnel-proto -p chan-tunnel-server -p chan-tunnel-client -p chan-server`: 26 protocol, 17 server, 12 client, 9 listener E2E, and 745 chan-server tests passed.
- Focused identity namespace tests passed 4/4. Focused Desktop gateway-entry tests passed 4/4 and runtime-capability tests passed 6/6. Focused gateway-assertion tests passed 7/7.
- `TEST_DATABASE_URL=postgres://chan:chan@127.0.0.1:5432/chan_gateway_test cargo test --workspace` in `gateway` passed the complete unit, API, database, and doc-test workspace.
- `scripts/e2e/gateway-zone.sh` passed all 139 assertions against an isolated PostgreSQL schema and a local three-proxy stack, including controller restart and outage, one-node control partition, registration transfer, sharing, CSRF, routing, roster, and multi-node isolation scenarios.

Passing tests are evidence of implementation stability, not proof that the security findings are absent. In particular, the current suites accept missing WebSocket `Origin`, allow continuous activity to extend a bridge indefinitely, settle kills on proxy report, preserve lexicographic restart winners, and cancel control-loss eviction on snapshot acceptance. The hardening work must first replace those assertions with red adversarial tests.

## Residual risk after the proposal

Even after all blockers land, a fully compromised active data-path proxy can deny service and can observe or alter traffic for tunnels legitimately assigned to it. Signed leases prevent it from inventing other owners; node-scoped browser authority prevents fleet impersonation; tombstones and audit improve detection; transport and service isolation constrain lateral movement. They do not turn an assigned proxy into a trusted execution environment. Node isolation, credential rotation, tunnel reassignment, and deprovisioning remain the definitive response.

The singleton in-memory controller also remains an availability dependency. That is an explicit non-goal rather than an unreported security claim, provided fail-closed deadlines, bounded work, retry semantics, and existing-tunnel policy are implemented and tested as proposed.
