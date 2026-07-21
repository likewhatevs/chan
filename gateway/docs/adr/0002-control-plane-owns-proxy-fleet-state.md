# The control plane owns proxy fleet state

Status: accepted (2026-07-21).

A singleton, database-free controller (`devserver-control`) owns the dynamic proxy directory, the aggregate tunnel view, fleet admission, and command routing for the devserver proxy fleet. Every `devserver-proxy` node holds one authenticated h2 control session to the controller, publishes its full registry snapshot plus deltas, asks the controller for an admission decision before acknowledging any tunnel, and executes the controller's kill commands. identity, profile, and the admin CLI read one coherent aggregate view from the controller's `/admin/v1/*` tree; the proxy no longer exposes a process-local management view. Fleet state lives in one owner task fed by bounded messages, never in PostgreSQL: the state is a liveness view that is worthless when stale, and the owning proxy process is the only authority that can refresh it.

## Considered options

- **DNS round-robin across unchanged proxies** rejected: tenant traffic is not guaranteed to reach the process holding the yamux handle, so no load-balancer shape avoids an ownership directory.
- **A shared PostgreSQL registry** rejected: a database cannot serialize a usable `TunnelHandle`, and rows retained after a process failure would report stale ownership as truth.
- **Query-time fan-out from identity, profile, or the admin CLI** rejected: it has no safe answer when one proxy is unreachable (partial read presented as fleet truth), and it would make the profile sweeper depend on a distributed partial read.
- **Fleet ownership embedded in identity or profile** rejected: that couples proxy churn and command routing to database-facing request services; a dedicated database-free controller keeps those responsibilities separate.
- **A central tenant reverse proxy keeping one wildcard host** rejected: it adds another streaming HTTP/WebSocket hop and makes the central machine the data-plane bottleneck that proxy scaling exists to remove.
- **Singleton controller (chosen).** The controller carries metadata and commands only; tenant data never crosses it, so its cost is one small stateless process per deployment.

## Failure semantics

- Loss of the controller stops new tunnel admission immediately: a proxy admits nothing while its control session is down, and there is no singleton or local-admission fallback.
- Existing data paths get a bounded 30-second reconnect grace (`grace_seconds: 30`, heartbeats at 5 seconds, dead at 15). When grace expires, the proxy evicts every tunnel it holds and fails closed; recovery requires a fresh snapshot and `FleetReady`.
- After a controller restart, `/readyz`, aggregate reads, management writes, and admissions hold 503 until at least one complete proxy snapshot has arrived and a 30-second convergence window has elapsed. Duplicate rows left inside the proxies' grace period reconcile deterministically (lexicographically smallest `(proxy_id, registration_id)` wins; losers are commanded down before readiness), so a controller outage cannot create indefinite untracked or conflicting tunnels.

## Consequences

- The controller is a singleton. Losing it stops admission fleet-wide until it returns and convergence completes; controller HA, durable control state, leader election, and cross-region replication are out of scope.
- The aggregate `/admin/v1/*` tunnel tree moves from devserver-proxy to devserver-control; consumers point `DEVSERVER_ADMIN_URL` at the controller and always see either one coherent fleet view or an explicit upstream failure, never a partial process snapshot.
- Every management kill routes by registration UUID to the owning proxy only; proxy nodes hold no database, profile-service, OAuth, session, or operator-admin credential.
- All gateway services and proxies run the exact same package version; the control handshake rejects a mismatch.
