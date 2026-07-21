# chan-gateway v0.72.0 distributed proxy control plane

Status: accepted scope for v0.74.0, deferred out of both v0.72.0 and v0.73.0.
**Steps 1 to 9 and the accepted security hardening are implemented,
own-gated, adversarially reviewed, live-verified, and final-gated on the
unmerged branch `v074/ctl`. The branch is ready to merge when the v0.74.0
integration window opens.** Steps 1 to 4 landed on
`v073/ctl`; steps 5 to 9 plus the joining ruling and hardening landed on
`v074/ctl`, which is `v073/ctl` rebased onto main. The size of the complete
cutover is why v0.73.0 closed without it rather than merging a half-cut-over
proxy. See [release-v0.73.0](../../release/release-v0.73.0.md).

What is already done on that branch: the controller state machine with session incarnations, generation contiguity, pending-claim expiry, deterministic restart reconciliation and convergence; the runnable service with separately bound admin/health and raw h2c proxy listeners; the proxy-side cutover that deletes `admin.rs`; and a live three-proxy vertical slice proving each node serves only its own tunnel and returns 404 for the others. An integrator review found and the lane fixed four defects a green gate could not see: a cancellation-unsafe framed read inside a `select!`, a connection and semaphore-permit leak on heartbeat death, an unbounded command wedge, and a terminal convergence latch.

What the `v074/ctl` round added: the live-first joining ruling (live rows are immutable winners; pending claims reserve their key and capacity; the lexicographic rule stays confined to initial restart reconciliation); the aggregate admin kill tree with registration-UUID routing, pending-claim cancellation, concurrent fan-out, and 502 partial-kill reporting; the `DevserverControlClient` rename with identity, profile, and admin migration; identity entry origins minted from controller-validated node bases with roster `proxy_origin`; the desktop move-node lifecycle; the extended three-proxy zone E2E (node matrix, shared-ingress distribution, controller restart, proxy control-stream drop, controller outage, and ownership move); fifth-service packaging, kube and sdme examples, ADR 0002, and the 0.74.0 version pins. Review of this round found and the lane fixed three more defects a green gate could not see: a forced resync after every confirmed kill that retracted uninvolved rows from the aggregate, a joining plan that ignored pending claims and could exceed the per-user cap, and an Active-with-no-generation zombie when a force-resync interrupted joining reconciliation. The full pre-push gate and the full zone suite (139 assertions) are green on the branch tip.

The hardening plan in
[distributed-proxy-control-plane-hardening.md](distributed-proxy-control-plane-hardening.md)
is accepted and implemented. Its implementation-closure section supersedes the
pre-hardening protocol, credential, browser-session, revocation, and deployment
details in this original plan. The original text below remains as a historical
record of the initial distributed-control design rather than the current
security contract.

The plan below is unchanged from its original v0.72.0 target, so the version it names throughout is the version it ships in, now v0.74.0. Grounded against `46722392` on 2026-07-19.

The branch contains buildable candidate groundwork, not a runnable control plane. A completing implementation may revise or replace the candidate protocol and controller abstractions where the live transport, failure semantics, or tests require it. The behavior and acceptance criteria in this plan are authoritative.

This work depends on the v0.71.0 exact-origin Chan Desktop capability design. It is an exact-version v0.72.0 cutover. Supporting an old singleton proxy beside the distributed design is out of scope.

## Step 4 barrier: met on 2026-07-20, evidence recorded here

The vertical slice was proven live on `e6e866db`, the build carrying the four review fixes. This transcript is reproduced from the round's working notes because those are not tracked and will not survive; it is the acceptance evidence for step 4 and the next round should not have to re-run it to know where things stand.

One controller plus p1, p2 and p3 on `127.0.0.2` through `127.0.0.4` with identical public port `17402` and tunnel port `17410`, and three real `chan devserver --service=none` clients with distinct ports, homes and runtime directories. The identity validator derived each `devserver_id` exactly as production does, `derivation=sha256-lowercase`.

After client admission the controller reported all three proxies `active` at `package_version 0.73.0` with `tunnel_count: 1`, and exactly three tunnel rows for `alice`, one per proxy, each carrying its own `proxy_base_url`.

The credentialed, host-bound 3x3 `/api/health` matrix:

```text
target=p1 node=p1 status=200  target=p1 node=p2 status=404  target=p1 node=p3 status=404
target=p2 node=p1 status=404  target=p2 node=p2 status=200  target=p2 node=p3 status=404
target=p3 node=p1 status=404  target=p3 node=p2 status=404  target=p3 node=p3 status=200
```

Each 200 returned a distinct workspace instance id. Post-matrix polls of all three client streams returned zero bytes, so no gateway assertion was rejected during the run. All three clients, all three proxies and the controller exited 0.

Two earlier attempts were discarded rather than counted, and the reasons are worth keeping. The first used an identity stub returning synthetic devserver ids where production derives them as lowercase SHA-256 of the PAT; the real clients correctly rejected the resulting gateway assertion, which the matrix alone would not have revealed. The second was withdrawn by the lane itself when a review showed the four transport and liveness defects were still present in the build under test.

**Barrier lessons that are not in the spec.** The identity stub must use the production derivation or a green matrix can hide `WrongDevserver` in the client logs, so always poll every client stream after the matrix. Controller readiness intentionally waits the full convergence window: wait for three active zero-row proxy snapshots, then start clients, then require three active proxy rows with `tunnel_count: 1` and exactly three tunnel rows before probing traffic. Proxy-first teardown produces controller peer-`BrokenPipe` warnings after active work is over; do not confuse those with active-session warnings.

## Accepted ruling, implemented on `v074/ctl`: joining must not evict a live row

Reproduced here for the same reason as the barrier transcript. The integrator accepted this plan and the v0.73.0 lane deliberately did not implement it; the `v074/ctl` round implemented it as specified, including the red-before-green proof list, and this section is the record of what was asked.

Today `reconcile_joining` merges the controller's own live rows with the joining session's snapshot into one candidate list and applies the **restart** tie-break, lexicographically smallest `(proxy_id, registration_id)`. The spec confines that rule to controller restart, where recency is genuinely unavailable. Applied to a live fleet it lets a stale snapshot win: a proxy whose stream flapped can rejoin inside its grace window, present the registration it still holds, sort lower, and have the controller kill the live tunnel it had itself admitted on another node. The per-user capacity trim has the same shape, choosing victims by `devserver_id` order rather than by liveness.

1. Split the shared reconciliation plan in `gateway/crates/devserver-control/src/state.rs`. Keep the lexicographic duplicate and capacity logic for `begin_initial_reconciliation` only. Give `reconcile_joining` a live-first plan that starts from the current `self.tunnels` as immutable winners.
2. Classify any joining key already present in the live map as a joining loser. Reserve each user's existing live count first and admit only novel joining keys that fit the remaining slots. If several novel rows compete, use a stable ordering local to that one snapshot, but never re-rank or evict an existing live row and never treat proxy id as recency on a routine join.
3. Apply the same live-first recomputation in `finish_reconciliation_if_complete` for the joining kind. That second site matters: it recomputes from current actor state after command results so concurrent deltas are not overwritten. Keep joining rows invisible and the session `Joining` until every loser kill succeeds, and retain the fail-closed retirement of the joining session on failure.
4. Prove red before green, at minimum: live p2 owns `alice/one`, a lower-sorting p0 rejoins with a stale duplicate, and only p0 is killed; a capacity-full live user rejects p0's joining row without moving the live owner; nonconflicting rows in one snapshot become visible atomically; concurrent live deltas survive completion; and initial restart reconciliation still uses the deterministic lexicographic rule. Preserve and extend the existing joining-atomicity, delta-during-joining and duplicate-restart-rows coverage.

## Problem

`chan-tunnel-server::Registry` stores each live yamux `TunnelHandle` in the accepting proxy process. The current devserver-proxy admin tree reads and kills only that process's registrations. Identity uses that tree for live devserver selection, desktop roster liveness, PAT revocation, and account deletion. Profile uses it for block eviction and as the complete mark source for its stale-devserver sweeper.

Adding uncoordinated proxy processes breaks both planes:

- A tunnel accepted by proxy A can be followed by tenant traffic sent to proxy B. Proxy B has no usable `TunnelHandle` and returns 404.
- A process-local snapshot is incomplete. Treating it as complete can make identity report a live devserver offline and can let the profile sweeper age out a row still live on another proxy.
- Per-process admission cannot enforce duplicate ownership or `MAX_DEVSERVERS_PER_USER` across the fleet.
- Revoke, block, delete, and operator kill requests cannot know which process owns the registration.

Persisting registry rows in PostgreSQL does not solve the data path. The yamux connection, open-substream channel, shutdown signal, and assertion key stay process-local. Querying every proxy on every management request also fails the complete-snapshot requirement whenever one process is slow or unreachable.

## Decision

Add one database-free `devserver-control` service that owns the authoritative dynamic proxy directory, aggregate live-tunnel view, fleet-wide admission decisions, and management command routing.

Each devserver-proxy opens one authenticated outbound HTTP/2 control stream on startup. It publishes an atomic full registry snapshot, then ordered registry deltas and heartbeats. The controller pushes admission decisions and kill commands over the same stream. Proxies remain the sole owners of yamux handles and the sole data path for their registrations.

Tunnel ingress stays at one stable apex and may land on any ready proxy. Tenant traffic uses the owning proxy's node-specific wildcard host, so browser traffic reaches that proxy directly without a controller or inter-proxy data hop. Identity obtains the owning node from the aggregate tunnel row and mints the exact origin already carried by its entry response.

The controller is in-memory. Proxies reconstruct it after restart. A database would add stale durable membership without making a tunnel transferable.

## Grounded implementation state

Commit `46722392` establishes and independently passes focused tests for:

- explicit identity, tunnel-ingress, proxy-namespace, and node-base origins;
- desktop discovery host-depth validation;
- registry registration UUIDs, monotonic generations, ordered events, atomic snapshot/subscription, precise eviction, and an async admission hook;
- bounded control frame types, proxy id/origin validation, and an in-memory controller state-machine skeleton.

The committed branch does not contain a runnable `devserver-control` binary, a proxy-to-controller transport, proxy control configuration, controller-backed admission, aggregate admin HTTP handlers, consumer migration, failure/grace behavior, controller packaging, or a multi-proxy deployment harness. `devserver-proxy` still owns its process-local admin tree and cap enforcement. The controller actor currently accepts only session start, full snapshot, disconnect, and read operations; it does not implement live deltas, heartbeats, convergence, command routing, admission expiry, or watch publication.

Current committed runtime:

```text
                  protocol types + controller state skeleton
                                   (not served)

    proxy p1                    proxy p2                    proxy p3
  local Registry p1          local Registry p2          local Registry p3
  local admission            local admission            local admission
  local admin tree           local admin tree           local admin tree
       |                          |                          |
     yamux                      yamux                      yamux

There are no live proxy sessions, aggregate reads, or fleet-wide decisions.
```

Planning validation on the grounded commit:

- `devserver-control` passed 5 actor/state tests and `devserver-control-proto` passed 3 protocol tests;
- `chan-tunnel-server` passed 16 registry/listener unit tests and 6 loopback listener E2E tests;
- strict Clippy passed for the controller/protocol packages and `chan-tunnel-server`;
- gateway formatting and `git diff --check` passed.

These checks validate the candidate primitives only. They do not validate a distributed service, because the live controller/proxy path described below does not exist yet.

### Consequences

- The controller is a singleton in v0.72.0. Controller HA and replicated control state are out of scope.
- Loss of the controller stops new tunnel admission immediately. Existing data paths get a bounded 30-second reconnect grace, then proxies evict them and fail closed.
- A node-specific tenant origin is part of the public entry contract. Discovery remains stable, but Chan Desktop must validate exactly two child labels below the advertised proxy apex.
- The existing `/admin/v1/*` tunnel tree moves from devserver-proxy to devserver-control. The proxy no longer exposes a partial management view.
- All gateway services and proxies must run the exact same package version. The control handshake rejects a mismatch.
- Dynamic membership covers provisioned proxy ids joining and leaving. It does not provision machines, DNS, certificates, or overlay networking.

## Rejected alternatives

- DNS round-robin across unchanged proxies is invalid because tenant traffic is not guaranteed to reach the process holding the yamux handle.
- A shared PostgreSQL registry cannot serialize a usable `TunnelHandle` and would retain stale ownership after a process failure.
- Query-time fan-out from identity, profile, or the admin CLI has no safe answer when one proxy is unreachable. It also makes the profile sweeper depend on a distributed partial read.
- Embedding fleet ownership in identity or profile couples proxy churn and command routing to database-facing request services. A dedicated database-free controller keeps those responsibilities separate.
- A central tenant reverse proxy could preserve one wildcard host, but it adds another streaming HTTP/WebSocket hop and makes the central machine the data-plane bottleneck that proxy scaling is intended to remove.

## Target topology

```text
                              central services
                          +-----------------------+
                          | profile <-> Postgres  |
                          | identity              |
                          | devserver-control     |
                          +-----+------------+----+
                                |            |
                 PAT validate   |            | h2 control over private network
                                |            |
              +-------------+--+  +--+-------------+  +--+-------------+
              | proxy p1       |  | proxy p2       |  | proxy p3       |
devserver --> | tunnel/yamux   |  | tunnel/yamux   |  | tunnel/yamux   | <-- devserver
usr.chan.app   | tenant data    |  | tenant data    |  | tenant data    |     usr.chan.app
              +----------------+  +----------------+  +----------------+
                       ^                  ^                  ^
                       |                  |                  |
          *.p1.usr.chan.app  *.p2.usr.chan.app  *.p3.usr.chan.app
                              browser data path
```

There is no proxy-to-proxy path. The controller carries metadata and commands only. PostgreSQL is not reachable from proxy nodes.

## Public host and routing contract

Public hosts are explicit deployment configuration, not names derived from a compiled-in domain or fixed `gw`, `usr`, or `rc` labels. The examples below use the chan.app deployment. A self-hosted deployment may use any valid origins with the same structural relationship.

- Keep `https://usr.chan.app/v1/tunnel` as the example discovery-advertised tunnel URL. DNS or a layer-4 load balancer may distribute it across ready proxy nodes. A tunnel remains attached to the node that accepted it.
- Give every proxy a stable DNS-label id such as `p1`. Its example configured base origin is `https://p1.usr.chan.app`.
- Serve tenant traffic at the example origin `https://{owner}--{disc}.p1.usr.chan.app`, where `p1` is the owning proxy and `disc` retains the existing 12-character devserver discriminator.
- Publish `*.p1.usr.chan.app` to node `p1` and issue a certificate covering the node wildcard. Repeat for each provisioned node. A parent wildcard such as `*.usr.chan.app` does not cover `tenant.p1.usr.chan.app`.
- Use `https://gw.chan.app` as the example identity and `/.well-known/chan-gateway` origin. Keep `/chan-mark.png` and all `chan devserver`, `chan_pat_*`, `CHAN_ADMIN_*`, and `CHAN_TUNNEL_*` compatibility names.
- Keep discovery's `devserver_proxy_origin` and `tunnel_url` on the explicitly configured stable proxy namespace and tunnel ingress origins. In the chan.app example both use `usr.chan.app`. Add `devserver_proxy_host_depth: 2`; Chan Desktop uses it to validate an entry origin with one tenant label and one proxy label below the proxy namespace apex.
- The isolated RC example uses `rc.chan.app` for its website, `gw.rc.chan.app` for identity and discovery, `usr.rc.chan.app` for tunnel ingress and the proxy namespace, `p1.usr.rc.chan.app` for a proxy node, and `*.p1.usr.rc.chan.app` for that node's tenant origins.
- Identity must never construct or accept an arbitrary node URL. It uses `proxy_base_url` from an aggregate controller row that was validated against the controller's configured template.

## Trust and network boundaries

- Run proxy-to-control and proxy-to-identity traffic on a private deployment network such as WireGuard. Do not expose either internal listener on the public interface.
- Authenticate `POST /v1/proxies/connect` with a dedicated `DEVSERVER_PROXY_TOKEN`. Do not reuse `DEVSERVER_ADMIN_TOKEN`, `IDENTITY_INTERNAL_TOKEN`, `PROFILE_AUTH_TOKEN`, or `DEVSERVER_GATE_SECRET`.
- Compare both proxy and admin bearer tokens in constant time. Preserve the repository's acknowledged length-only leak comment.
- Validate `DEVSERVER_PROXY_ID` as one lowercase DNS label. Validate `DEVSERVER_PROXY_BASE_URL` as an origin only, with no credentials, path, query, or fragment.
- Configure the controller with `DEVSERVER_PROXY_BASE_URL_TEMPLATE`, for example `https://{proxy_id}.usr.chan.app`. Expand the template with the validated id and require exact canonical-origin equality with the proxy's claim. A proxy cannot register an arbitrary redirect target.
- Keep `DEVSERVER_GATE_SECRET` on identity and every proxy because entry and session JWTs remain end-to-end between those services.
- Keep `IDENTITY_INTERNAL_TOKEN` on identity and every proxy because PAT validation remains a direct proxy-to-identity request during tunnel handshake.
- Give proxy nodes no database, profile-service, OAuth, session, or operator-admin credential.

## Crates and packages

Add these members to the nested `gateway/` workspace:

- `gateway/crates/devserver-control-proto`: library containing control protocol frames, validated ids/origins, bounded framing, version constants, and shared tunnel/proxy view types. Keep its public API narrow and independent of axum.
- `gateway/crates/devserver-control`: binary and library containing the controller actor, proxy-session server, admin HTTP tree, health/readiness state, and command fan-out.

Package the binary as:

- executable `/usr/bin/chan-gateway-devserver-control`;
- systemd unit `chan-gateway-devserver-control.service`;
- env file `/etc/chan-gateway/devserver-control.env`;
- Debian package `chan-gateway-devserver-control`;
- public OCI image `fiorix/chan-gateway-devserver-control`.

Use `anyhow` with operation/resource context in binary startup paths and local `thiserror` enums for protocol, controller, and request errors. Keep session and fleet state in one owner task driven by bounded `mpsc` messages plus `oneshot` replies. Do not hold a lock guard across `.await`; use locks only for the chan-tunnel registry's synchronous map mutation.

## Configuration

### devserver-control

- `BIND_ADDR`, default `127.0.0.1:7003`: health, readiness, and `/admin/v1/*` HTTP.
- `PROXY_BIND_ADDR`, default `127.0.0.1:7101`: h2c proxy control listener.
- `DEVSERVER_ADMIN_TOKEN`, required: bearer for `/admin/v1/*`.
- `DEVSERVER_PROXY_TOKEN`, required: bearer for proxy control connections.
- `DEVSERVER_PROXY_BASE_URL_TEMPLATE`, required outside tests: canonical origin template containing exactly one `{proxy_id}` placeholder.
- `MAX_DEVSERVERS_PER_USER`, default `100`: fleet-wide cap over active and pending distinct devserver ids; `0` disables the cap.

### devserver-proxy

Retain the existing public, tunnel, identity, gate, forwarded-protocol, and wildcard settings. Add:

- `DEVSERVER_CONTROL_URL`, required: controller proxy-listener base URL.
- `DEVSERVER_PROXY_TOKEN`, required: control bearer.
- `DEVSERVER_PROXY_ID`, required: stable provisioned node id.
- `DEVSERVER_PROXY_BASE_URL`, required: exact node base origin.

Remove `DEVSERVER_ADMIN_TOKEN` and local `MAX_DEVSERVERS_PER_USER` from proxy configuration. The proxy must fail startup if any new control setting is absent or invalid. There is no singleton or local-admission fallback.

### existing consumers

- Identity and profile retain `DEVSERVER_ADMIN_URL` and `DEVSERVER_ADMIN_TOKEN`; deployments point the URL at devserver-control port 7003.
- Admin retains `CHAN_ADMIN_WORKSPACE_URL` and `CHAN_ADMIN_TOKEN`; change the default workspace URL port from 7002 to 7003.
- Rename `gateway_common::workspace_admin_client` to `devserver_control_client` and `WorkspaceAdminClient` to `DevserverControlClient`. Rust import compatibility is unnecessary, but the environment-variable compatibility names above remain unchanged.

### public origins

- `BASE_URL`, required outside tests: identity's canonical public origin, for example `https://gw.chan.app`.
- `DEVSERVER_PROXY_ORIGIN`, required outside tests: the canonical proxy namespace origin advertised by discovery, for example `https://usr.chan.app`.
- `DEVSERVER_TUNNEL_ORIGIN`, required outside tests: the canonical tunnel ingress origin. Discovery appends the stable `/v1/tunnel` protocol path.
- `DEVSERVER_PROXY_BASE_URL_TEMPLATE`, required by the controller as described above.
- `DEVSERVER_PROXY_BASE_URL`, required by each proxy as described above.

Remove runtime hostname derivation from `CHAN_DOMAIN`, `PUBLIC_SCHEME`, and fixed `id`, `devserver`, `gw`, or `usr` prefixes. Explicit local test origins remain supported. JWT issuers use the configured canonical service origin instead of a compiled-in hostname.

`chan devserver` has no compiled-in tunnel endpoint. Tunnel mode requires `--tunnel-url` or `CHAN_TUNNEL_URL`; supervised restarts retain the explicitly persisted URL.

## Control transport

The proxy opens an h2c connection and a single full-duplex stream:

```text
POST /v1/proxies/connect
Authorization: Bearer <DEVSERVER_PROXY_TOKEN>
Content-Type: application/x-chan-devserver-control+json; version=1
```

Use the existing `h2` and `H2Duplex` style primitives, but keep this as a separate protocol from `chan-tunnel-proto`. The two protocols have different trust, lifecycle, and compatibility boundaries.

Each direction is a sequence of `u32` big-endian length followed by one JSON frame. Encode frame enums with an explicit `type` tag and stable `snake_case` names. Reject a zero-length frame, a frame larger than 1 MiB, malformed JSON, a frame illegal in the current session state, and an unknown protocol version. Bound every internal queue; queue overflow or broadcast lag closes the session and forces a fresh snapshot instead of silently dropping state. Require the initial snapshot to finish within 30 seconds and reject more than 100,000 rows from one proxy so an authenticated peer cannot grow staging memory without bound.

The first client frame is:

```text
ClientHello {
  protocol_version: 1,
  package_version,
  proxy_id,
  proxy_base_url,
  boot_id
}
```

`boot_id` is a random UUID generated once per proxy process and retained across control reconnects. The controller returns `ServerHello` only after authentication, id/origin validation, and exact package-version validation. A new connection for an already-active proxy id closes and retires the prior session immediately, including its aggregate rows and pending claims. The replacement remains `joining` and invisible until its snapshot completes. Repeated replacement with different boot ids is a configuration fault and must be logged.

### Proxy-to-controller frames

- `ClientHello` as above.
- `SnapshotStart { base_generation }`.
- `SnapshotChunk { rows }`, at most 128 rows per frame.
- `SnapshotEnd { base_generation }`.
- `TunnelUp { generation, row }`.
- `TunnelDown { generation, registration_id }`.
- `AdmissionRequest { request_id, registration_id, user, devserver_id }`.
- `AdmissionCancel { request_id, registration_id }`.
- `CommandResult { command_id, killed, missing, failed }`.
- `Pong { nonce }`.

### Controller-to-proxy frames

- `ServerHello { protocol_version, package_version, heartbeat_seconds: 5, dead_seconds: 15, grace_seconds: 30 }`.
- `SnapshotAccepted { base_generation }`.
- `FleetReady`, sent after the controller convergence barrier; only then may the proxy pass readiness and request admissions.
- `AdmissionDecision { request_id, registration_id, decision }`, where decision is `admit`, `at_capacity`, `control_warming`, or `stale`.
- `KillRegistrations { command_id, registration_ids }`. Commands target registration UUIDs, never only `(user, devserver_id)`, so a delayed command cannot kill a successor.
- `ResyncRequired { expected_generation }`.
- `Ping { nonce }`.
- `Shutdown { reason }` for a version, identity, or administrative rejection.

The control protocol version and gateway package version are distinct checks. v0.72.0 uses control protocol 1 and rejects a package-version mismatch even if the protocol number matches.

## Registry event contract

Change `crates/chan-tunnel-server::Registry` so each inserted registration receives a UUID and every successful mutation advances one monotonic `u64` generation.

Expose:

- the registration UUID on `TunnelHandle` and `TunnelInfo`;
- `snapshot_and_subscribe() -> (generation, Vec<TunnelInfo>, Receiver<RegistryEvent>)`;
- ordered `TunnelUp` and `TunnelDown` events carrying the post-mutation generation;
- `evict_registration(registration_id)` for precise control commands;
- `evict_all()` for fail-closed control loss.

Create the event receiver and copy the snapshot while holding the same registry mutex used by insert/remove. Send each event without awaiting while still under that mutex. This ordering guarantees that a mutation is either present in the snapshot or appears after it on the receiver, never missed between the two. Broadcast lag is not recoverable by guessing; the proxy control task requests a fresh snapshot.

Replace channel-identity stale-owner checks with registration UUID equality. A predecessor driver's late teardown must not remove or emit `TunnelDown` for its successor. Snapshot rows contain registration id, user, devserver id, peer address, and connection time. The controller does not receive `TunnelHandle`, gateway assertion keys, or any bearer secret.

Keep the in-process lookup path and yamux ownership unchanged. Public HTTP and WebSocket requests still resolve a local `TunnelHandle` and open a local yamux substream.

## Synchronous fleet admission

Add an async `RegistrationAdmission` hook to `chan-tunnel-server`. Invoke it after PAT validation and `Hello` validation but before writing `HelloAck::Ok`. The hook returns a permit containing a proxy-generated registration UUID. A refusal writes a normal `HelloAck::Refused` with a stable `control_unavailable` or existing `too_many_workspaces` code.

The proxy implementation sends `AdmissionRequest` and waits at most 10 seconds for the matching decision. It sends `AdmissionCancel` if the handshake, HelloAck write, or local registration fails. A pending claim expires in the controller after 15 seconds even if cancel is lost. Local registry insertion uses the permitted UUID; the resulting `TunnelUp` activates the claim.

The controller serializes these rules in its fleet actor:

1. Reject every admission until the controller is globally ready and the requesting proxy session is active.
2. Count active plus pending distinct `(user, devserver_id)` keys against `MAX_DEVSERVERS_PER_USER`.
3. Treat a reconnect of an existing key as count-neutral, including when the old owner is another proxy.
4. Allow only the newest pending claim for a key. Resolve superseded request ids as `stale`.
5. On `TunnelUp`, atomically make that registration authoritative, then command the prior owning proxy to kill the replaced registration UUID.
6. On `TunnelDown`, remove the key only when the registration UUID is still authoritative.
7. User-wide kill cancels matching pending claims before commanding active registrations.

The controller is the only cap authority. Do not repeat a per-process cap that can disagree with it.

After a controller restart, snapshots can reveal duplicate keys left within the proxies' grace period. Do not compare wall clocks across machines. Choose the lexicographically smallest `(proxy_id, registration_id)` row, publish only that winner, and issue precise kill commands for losers before declaring the fleet ready. If reconstructed rows exceed the per-user cap, keep the first `MAX_DEVSERVERS_PER_USER` distinct keys sorted by `(devserver_id, proxy_id, registration_id)` and command the excess down before readiness. Wait for successful command results; failure keeps initial convergence unready or rejects a later joining snapshot rather than admitting conflicting state.

## Controller state and readiness

The controller actor owns:

- proxy sessions keyed by validated proxy id, including a controller-issued session incarnation, boot id, package version, base URL, registry generation, last heartbeat, command sender, and current registration ids;
- authoritative registrations keyed by `(user, devserver_id)` and by registration UUID;
- pending admission claims with expiry;
- readiness and the initial convergence deadline.

Session state transitions are `joining -> active -> dead`. `begin_session` allocates an opaque controller-side incarnation and returns it to the connection task. Every actor command from that task carries the incarnation. A late frame, heartbeat timeout, or stream close from a retired connection is rejected as stale and cannot mutate or disconnect its replacement with the same proxy id. The incarnation is internal and never appears on the control wire or admin API.

A snapshot is staged outside the authoritative maps and becomes visible only on a matching `SnapshotEnd`. Require deltas to start at `base_generation + 1` and remain contiguous. Any gap, duplicate out of state, mismatched snapshot terminator, or illegal registration id sends `ResyncRequired` and discards the joining state.

Send `Ping` every 5 seconds. A valid `Pong` or other valid client frame updates liveness. Treat a clean stream close as dead immediately and a silent session as dead after 15 seconds. Removing a session atomically removes its aggregate rows, cancels its pending claims, and emits tunnel/proxy watch updates.

On process startup, `/healthz` returns 200 but `/readyz`, aggregate reads, management writes, and admissions return 503 until at least one complete proxy snapshot has arrived and a 30-second convergence window has elapsed. Broadcast `FleetReady` at that point. If the active fleet reaches zero, leave readiness and start a new convergence window after the next complete snapshot. If the controller is already globally ready when an additional proxy snapshot is accepted, send `FleetReady` to that proxy immediately.

A routine additional proxy may join an already-ready fleet without resetting global readiness. Its rows become visible atomically when its snapshot and conflict reconciliation complete.

## Proxy failure semantics

The proxy control client reconnects with bounded exponential backoff from 500 ms to 10 seconds, with jitter. It sends a fresh full snapshot on every connection; there is no delta-resume protocol in v0.72.0.

Proxy endpoints behave as follows:

- `/healthz` reports process liveness and remains 200 while the service can run.
- `/readyz` is 200 only after `FleetReady` on the current control connection.
- New tunnel admissions stop immediately when control disconnects or readiness is lost.
- Existing public HTTP, WebSocket, and yamux traffic continues for a 30-second reconnect grace.
- An authenticated reconnect plus `SnapshotAccepted` cancels the eviction timer because authoritative control is restored. The proxy remains unready and rejects admissions until `FleetReady`.
- Grace expiry calls `Registry::evict_all()`, clears the username cache, and leaves the proxy unready until control is restored.

Own spawned tasks with one cancellation tree. If the public listener, tunnel listener, or control client terminates permanently, trigger service shutdown rather than leaving a partially functional process. Log state transitions, proxy id, boot id, generation, row counts, admission decisions, resyncs, and command outcomes without logging tokens or JWTs.

## Aggregate admin API

Move these existing paths unchanged from devserver-proxy to devserver-control:

- `GET /admin/v1/tunnels`.
- `GET /admin/v1/users/{user}/tunnels`.
- `POST /admin/v1/tunnels/{user}/{devserver_id}/kill`.
- `POST /admin/v1/users/{user}/tunnels/kill`.
- `GET /admin/v1/tunnels/watch`.

Add:

- `GET /admin/v1/proxies`.
- `GET /admin/v1/proxies/watch`.
- `GET /healthz`.
- `GET /readyz`.

Remove `gateway/crates/devserver-proxy/src/admin.rs`, its router merge, and `DEVSERVER_ADMIN_TOKEN` from the proxy. The proxy apex keeps health/readiness and tunnel ingress only; wildcard hosts keep tenant data only.

Extend each public `TunnelView` with `proxy_id` and `proxy_base_url`. Keep `user`, `devserver_id`, `peer_addr`, and `connected_at` unchanged. Registration UUID stays internal and is never returned by the admin tree.

Define `ProxyView` with `proxy_id`, `proxy_base_url`, `package_version`, `boot_id`, `connected_at`, `last_seen_at`, `tunnel_count`, and `status`. Sort tunnel rows by `(user, devserver_id)` and proxy rows by `proxy_id` for stable JSON and ETags.

Tunnel and proxy watch endpoints send an initial `event: snapshot` and a new complete snapshot after each visible state change. Keep SSE keepalives. Do not poll every second inside the controller.

Exact kill returns 404 if no aggregate row matches. User kill groups registration UUIDs by current proxy and sends commands concurrently. A command timeout is 5 seconds. If every target responds, return the total killed count. If any proxy that owned a target fails or disconnects before confirming, return 502 with `{ "error": "partial kill", "killed": N }`; the operation remains idempotent and a retry kills any surviving rows.

## Identity, profile, and admin changes

Use `DevserverControlClient` everywhere that currently uses `WorkspaceAdminClient`:

- Identity `/api/me` live lists, share landing selection, desktop roster, desktop entry, PAT revoke, and account deletion.
- Profile block eviction and the stale-devserver sweeper.
- The operator CLI's tunnel commands.

The client's list methods consume aggregate controller snapshots. A controller 503 or transport error remains an upstream failure, never an empty list. This preserves desktop roster 502 degraded behavior and makes the profile sweeper skip the whole tick rather than marking live rows offline.

Identity uses `TunnelView.proxy_base_url` when it selects a live `(owner, devserver_id)`. Build the tenant origin by prefixing `{owner}--{disc}.` to the validated node base host while preserving its scheme and effective port. Use that exact origin for entry `aud`, `proxy_origin`, and `entry_url`. A missing or invalid node base is an upstream error, not a fallback to the shared apex.

Extend each desktop roster row with `proxy_origin: Option<String>`. It is `None` while offline and the controller-derived exact origin while online. Chan Desktop uses the field only to detect that an online registration moved nodes: invalidate the row lifecycle, close the old managed connection and windows, then perform a fresh authenticated entry flow before reconnecting. It must never mint native authority from the roster field. The entry response remains the sole exact-origin authorization source, and the old exact Tauri grant remains dormant under the v0.71 restart-only purge limitation.

Retain the existing best-effort semantics for revoke, block, and delete after their primary database action. Log a failed control kill and rely on PAT validation plus the proxy's bounded control-loss behavior to close the gap. Reads that require fleet truth continue to fail closed as documented above.

Admin `tunnel ps` and JSON output add proxy id and base URL. Add `proxy ps` and `proxy watch` commands against the new endpoints. Preserve `CHAN_ADMIN_WORKSPACE_URL` and `CHAN_ADMIN_TOKEN`.

## Chan Desktop discovery and exact-origin validation

Build on the v0.71.0 exact-origin runtime capability implementation. Do not restore a gateway wildcard capability.

Extend the discovery response and parser with required `devserver_proxy_host_depth: 2`. The entry validator must require:

- exact owner and full devserver id match;
- exact scheme and effective-port match with discovery's `devserver_proxy_origin`;
- exactly two non-empty DNS labels below the discovery apex;
- an entry URL whose canonical origin equals `proxy_origin` exactly;
- no credentials, query, fragment, or non-root path in `proxy_origin`;
- HTTPS outside the existing loopback development exception;
- the same pinned exact origin on refresh and subsequent window entry.

The expected shape is `{owner}--{disc}.{proxy_id}.<configured-proxy-apex>`, for example `{owner}--{disc}.{proxy_id}.usr.chan.app`. Reject the apex, one-label legacy child, three-level nested child, unrelated suffix, suffix lookalike, scheme mismatch, and port mismatch. Mint Tauri authority only for the validated exact origin and retain the v0.71 shared-devserver consent lifecycle.

## Packaging, release, and documentation surface

Update every construction site for the fifth gateway service:

- `gateway/Cargo.toml` members, shared dependencies, version, and lockfile.
- Root `Makefile` gateway release crate list and `packaging/gateway/scripts/build-debs.sh`.
- `packaging/docker/gateway.Dockerfile`, `packaging/docker/build.sh`, and Docker documentation.
- `.github/workflows/gateway-ci.yml` and `.github/workflows/release.yml`, including amd64/arm64 Debian assets, OCI build targets, digest merge, manifest inspection, dry-run behavior, and immutable release tags.
- Marketing release-asset service lists, metadata generators, fixtures, install page, and smoke tests.
- `packaging/kube/` controller workload/config/secret examples, proxy config, service routing, and sdme pod example.
- `packaging/gateway/scripts/dev/` local stack and `scripts/e2e/gateway-zone.sh`.
- Gateway top-level README, CONTEXT, dev setup, root design, per-crate README/design files, admin docs, and `CHANGELOG.md`.
- Add accepted ADR `gateway/docs/adr/0002-control-plane-owns-proxy-fleet-state.md` containing the decision, rejected alternatives, failure semantics, and singleton-controller consequence.

Release all five Debian packages and four service OCI images at the same immutable v0.72.0 version. Release validation must fail if any asset or manifest entry is missing or version-skewed.

## Implementation order

Keep each step buildable and tested before crossing crate boundaries. The first four steps form one vertical-slice barrier. Do not begin broad consumer or release migration until the barrier is proven with live processes.

1. Audit the candidate `devserver-control-proto`, registry, admission, and controller code against this plan. Retain passing code where its interfaces fit; revise it where live ownership requires a different shape. Add controller-side session incarnations before attaching network tasks. Complete pure state transitions for snapshots, deltas, admissions, pending expiry, command results, heartbeats, disconnects, convergence, and watches.
2. Add the `devserver-control` binary with separate axum admin/health and raw h2c proxy listeners. Make the session adapters enforce authentication, content type, control and package versions, origin template, legal frame order, snapshot limits/deadline, contiguous generations, heartbeat deadlines, and bounded queues. Keep snapshot staging and socket I/O outside the actor.
3. Add a `devserver-proxy` control supervisor that owns one boot UUID, the current session, snapshot/event publication, admission correlation, command execution, readiness, reconnect backoff, and grace eviction. Invoke `serve_tunnel_listener_with_admission` with controller admission and a disabled local registry cap. Remove proxy-local admin and cap authority only after controller replacements pass their focused tests.
4. Add the aggregate proxy/tunnel reads needed to run one controller with p1, p2, and p3. Connect three real `chan devserver` clients, one owned by each proxy, and prove all three proxy rows and tunnel rows through the controller. Prove each node host serves its own tunnel and the other two nodes return 404 for it. This live milestone is required before the foundation is called an implemented control plane.
5. Complete reconnect grace, deterministic restart reconciliation, fleet-wide capacity, precise and user-wide command routing, partial-kill reporting, SSE watches, and additional-proxy joining behavior. Exercise all time-dependent behavior under paused Tokio time before relying on process-level smokes.
6. Rename the aggregate admin client and update profile, identity, and admin consumers. Preserve explicit upstream errors and best-effort write semantics at their existing call sites.
7. Build identity entry origins from controller-validated node origins, extend roster node state, and complete the desktop move-node lifecycle on top of the existing discovery depth validation.
8. Extend the local and deployment E2E stacks to the full three-proxy matrix, including shared-ingress distribution and controller/proxy failure scenarios.
9. Complete packages, images, release automation, documentation, ADR, version bumps, and changelog in the same release change.

Do not combine controller state ownership with HTTP handlers through shared mutable locks. Keep the actor state machine independently testable, then make h2 and axum thin adapters around it. Own every spawned task through one cancellation tree; no listener or control task may fail while the process continues partially functional.

## Verification

### Protocol and framing

- Round-trip every frame and reject zero, oversized, truncated, malformed, unknown-version, and illegal-state frames.
- Prove snapshot chunks cap at 128 rows, cumulative rows and snapshot duration are bounded, and queues cannot silently drop a delta or command.
- Reject wrong bearer, proxy id, base URL, control version, and package version.
- Verify Ping/Pong deadlines and reconnect backoff under paused Tokio time where practical.

### Registry

- Atomic snapshot plus subscription cannot miss a concurrent insert or remove.
- Generations are contiguous and monotonic across insert, replacement, explicit eviction, and driver teardown.
- A predecessor's late down event cannot remove its successor.
- Broadcast lag requests resync.
- `evict_registration` targets only its UUID; `evict_all` closes every driver and clears the wrapper cache.

### Controller actor

- Joining snapshots remain invisible until `SnapshotEnd`; a duplicate proxy id retires the prior session before the replacement joins.
- A retired session's late delta, heartbeat timeout, command result, and disconnect cannot affect the replacement session for the same proxy id.
- Missing, duplicate, or out-of-order generations force resync without corrupting active state.
- Clean close and heartbeat death remove proxy rows and watch state.
- Controller startup and zero-fleet recovery hold 503/readiness for the full convergence window.
- Additional proxies join an already-ready fleet without a global outage.
- Duplicate snapshot rows choose one deterministic winner and command the losers down.
- Fleet-wide cap counts active plus pending distinct ids, exempts reconnect, expires abandoned claims, and rejects while warming.
- Exact and user-wide kills route by registration UUID, fan out concurrently, report partial failure, and are safe to retry.

### Proxy

- A control admission completes before `HelloAck::Ok` and a denial returns the stable refusal code.
- No new tunnel is admitted while unready or disconnected.
- Existing HTTP, WebSocket, and yamux traffic survives a reconnect inside 30 seconds.
- Grace expiry evicts all tunnels and later recovery requires a fresh snapshot plus `FleetReady`.
- `/healthz` and `/readyz` reflect their distinct contracts.
- A command delayed past a reconnect cannot kill the replacement registration.

### Consumers and origin security

- Identity selects the controller-reported owner node and mints matching `aud`, `proxy_origin`, and same-origin `entry_url`.
- Identity never falls back to the shared apex on missing fleet state.
- Desktop roster preserves 502 and last-known-state behavior on controller failure.
- A roster `proxy_origin` change tears down the old managed connection and obtains a fresh entry before opening the new node origin.
- Profile sweeper consumes the three-proxy aggregate and skips the tick on any controller error.
- Desktop accepts the exact two-label production origin and rejects wrong depth, proxy node, suffix, scheme, port, owner, devserver id, and refresh origin.
- Revoke, block, delete, exact kill, and user kill evict the owning proxy registration.

### Three-proxy end to end

Run profile, identity, controller, proxies p1-p3, three node wildcard edges, and at least three real tunnel clients:

1. Start p1-p3 against one controller and wait for all three complete snapshots and `FleetReady` before admitting clients.
2. Register one real tunnel directly through each proxy's tunnel listener. Confirm `/admin/v1/proxies` contains three active rows and `/admin/v1/tunnels` contains all three registrations with the correct owning proxy.
3. Open each identity entry URL and prove its node-specific host serves the owning tunnel. Send the same request to both non-owning nodes and prove each returns 404.
4. Register additional tunnels through the shared apex and prove the edge can distribute independent h2 connections across p1-p3 without changing ownership after acceptance.
5. Confirm aggregate admin and desktop roster responses remain complete across all three proxies.
6. Reconnect a devserver through a different proxy and prove ownership, entry origin, aggregate row, and data path move together. Deliver a late disconnect from the retired proxy session and prove it cannot remove the replacement.
7. Exercise the global per-user cap with concurrent admissions on all three proxies.
8. Block or revoke users with registrations on different proxies and prove commands reach the owning processes only.
9. Disconnect p2's control stream and prove p2 rejects admission immediately, p1 and p3 remain ready, p2 traffic survives inside grace, and only p2 registrations disappear after grace.
10. Restart the controller and prove reads stay 503 during convergence, accepted snapshots cancel proxy eviction before grace expires, the fleet reconstructs all three owners, duplicate reconciliation is deterministic, and existing tunnels survive.

Run the gateway workspace format, clippy, test, build, package, container, and release dry-run gates, plus the root workspace and Chan Desktop gates required by the discovery change.

## Acceptance criteria

- Adding or removing a provisioned proxy requires no controller restart or static membership edit.
- Tenant traffic never depends on controller or inter-proxy forwarding after identity returns the entry URL.
- Every management read is one coherent aggregate view or an explicit upstream failure, never a partial process snapshot.
- Duplicate ownership and per-user capacity are decided synchronously before `HelloAck::Ok` across the fleet.
- A controller outage cannot create indefinite untracked tunnels.
- The profile sweeper cannot delete a row because it queried only one healthy proxy.
- All retained chan protocol, env, package, image, route, and branding compatibility names remain intact.
- v0.72.0 packages, images, controller, proxies, identity, profile, admin, and desktop agree on one immutable version and origin contract.

## Explicit non-goals

- Controller HA, durable controller state, leader election, or cross-region replication.
- Moving a live yamux connection between proxies.
- Proxy-to-proxy request forwarding.
- Automatic machine, WireGuard, DNS, or certificate provisioning.
- Mixed-version or old singleton-proxy compatibility.
- PostgreSQL schema changes.
