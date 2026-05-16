# Tunnel Hardening Journal

## Session

- Worktree: `../chan-core-tunnel-review`
- Branch: `tunnel-hardening`
- Roles: Alice = architect/coordinator, Bob = syseng, Martin = rustacean
- Goal: break and harden the `chan-tunnel-*` layer before production use.

## Current Map

- `chan-tunnel-proto`: control frames, `H2Duplex`, validators, h2 byte-stream glue.
- `chan-tunnel-client`: dial/reconnect loop, yamux client, per-substream h1 server into the local router.
- `chan-tunnel-server`: h2c listener, validator seam, registry, yamux driver, public h1 proxy.

## Initial Observations

- Existing protections:
  - h2 handshake, first-stream, validator, and Hello read timeouts.
  - in-flight handshake semaphore on the listener.
  - drive and username validation.
  - public-drive scope gate.
  - request body cap and response body cap in the public router.
  - host suffix allowlist, opt-in X-Forwarded-For trust, and proxy header stripping.
  - reconnect jitter and nonblocking event channel on the client.
- Risk areas still worth attacking:
  - `TunnelHandle::open()` can wait behind a full per-tunnel open queue with no deadline.
  - client `serve_substreams` spawns one task per inbound yamux stream with no explicit concurrency bound.
  - yamux stream fairness is not tuned for mixed workloads like large image preview plus text edits.
  - public response streaming is capped by bytes, but not by idle rate or long-lived non-upgrade response behavior.
  - logs and error strings should be audited for user data and bearer-token echo paths.
  - malicious peers can exercise h2/yamux edge cases beyond the current happy-path e2e tests.

## Task Index

- `task/bob-starvation-timeouts-1.md`
- `task/martin-yamux-h2-fuzz-1.md`
- `task/bob-privacy-abuse-2.md`
- `task/martin-preview-fairness-2.md`

## Handoffs

- Bob started `task/bob-starvation-timeouts-1.md`, focused first on `TunnelHandle::open()` queue pressure and public-router deadlines.
- Martin started `task/martin-yamux-h2-fuzz-1.md`, focused first on `H2Duplex` zero-sized reads and h2 flow-control edge cases.

## Coordination Rules

- Commit after each coherent test or hardening change.
- Keep findings in this journal with file references and reproduction commands.
- Prefer regression tests before policy changes when a failure can be reproduced.
- Avoid broad refactors while the risk surface is still being mapped.

## Bob Task 1: Starvation And Timeout Map

- Finding: `crates/chan-tunnel-server/src/public.rs` used
  `PublicConfig::upstream_request_timeout` only after
  `TunnelHandle::open().await` completed. A full per-tunnel open
  queue or stalled yamux driver could therefore hold public request
  tasks outside the configured deadline. Fix: start the deadline
  before `handle.open()` and wrap the open in `timeout_at`; timeout
  returns 504 and logs `tunnel substream open timed out`.
- Finding: `crates/chan-tunnel-client/src/lib.rs` spawned one h1
  handler per inbound yamux stream with no concurrency bound. A
  hostile server-side peer could grow client tasks and local
  router work unbounded. Fix: add
  `DEFAULT_MAX_CONCURRENT_SUBSTREAMS = 128`, expose
  `serve_substreams_with_limit`, and have `run` honor
  `ClientConfig::max_concurrent_substreams`. When the cap is full,
  the client stops accepting new yamux substreams until a handler
  exits.
- Still open: non-upgrade request and response bodies are byte
  capped, but there is no idle/read-rate watchdog. A peer that
  drips below caps can still pin one active substream and one
  handler slot. Recommend an idle body timeout or minimum-rate
  policy as the next targeted change, with defaults separate from
  `upstream_request_timeout`.
- Verification:
  `cargo test -p chan-tunnel-server proxy_times_out_when_substream_open_waits_forever`,
  `cargo test -p chan-tunnel-server --test public_e2e`, and
  `cargo test -p chan-tunnel-client`.

## Martin Task 1: Yamux And H2 Edge Cases

- Finding: `crates/chan-tunnel-proto/src/h2_duplex.rs` violated
  the `AsyncRead` zero-capacity read contract. With an empty
  `ReadBuf` and no available h2 DATA, `poll_read` could return
  `Pending` instead of immediately completing with no bytes read.
  Fix: return `Poll::Ready(Ok(()))` before polling `RecvStream`
  when `buf.remaining() == 0`.
- Regression: `zero_capacity_read_completes_without_data`.
- Reviewed: h2 write capacity already loops through zero-capacity
  grants until nonzero capacity, error, closed stream, or Pending.
  No concrete wakeup bug reproduced there in this pass.
- Verification:
  `cargo test -p chan-tunnel-proto zero_capacity_read_completes_without_data -- --nocapture`,
  `cargo test -p chan-tunnel-proto h2_duplex -- --nocapture`, and
  `cargo test -p chan-tunnel-proto`.

## Bob Task 2: Privacy And Abuse Surface

- Finding: `crates/chan-tunnel-server/src/public.rs` stripped
  forwarded and proxy headers, but still forwarded public
  `Authorization`, `Cookie`, and `Set-Cookie` request headers into
  the local chan-serve process. That lets an internet visitor inject
  auth-shaped state into the private upstream surface. Fix: strip
  those headers in `build_forwarded` and add
  `public_credentials_are_stripped`.
- Finding: `crates/chan-tunnel-client/src/dial.rs` already avoided
  echoing CONNECT proxy credentials on normal non-2xx responses, but
  malformed proxy response diagnostics included attacker-controlled
  status-line text. Fix: use fixed malformed-response messages and
  add `connect_malformed_status_does_not_echo_proxy_text`; also
  avoid formatting the proxy URL in the "cannot infer proxy port"
  error.
- Reviewed: bearer tokens are not logged by the tunnel listener
  itself; the remaining leakage seam is a consumer `Validator`
  returning token material inside `ServerError::Identity`, already
  documented as forbidden in `crates/chan-tunnel-server/src/lib.rs`.
- Reviewed: public/private drive semantics are enforced at
  handshake by `check_public_scope`, and registry replacements carry
  the new `Hello.public` bit, so stale registrations do not preserve
  a prior public flag.
- Production `PublicConfig` recommendation: keep
  `trust_forwarded_for = false` unless nginx overwrites XFF instead
  of appending it; set `allowed_host_suffixes` explicitly to the
  wildcard suffix plus the bare host if needed; keep public-router
  rate limiting disabled when all traffic arrives from one reverse
  proxy and apply per-client rate limiting at nginx instead.
- Verification:
  `cargo test -p chan-tunnel-server public` and
  `cargo test -p chan-tunnel-client`.

## Martin Task 2: Image Preview Fairness

- Probe: added
  `small_request_completes_while_image_preview_stream_is_active` in
  `crates/chan-tunnel-server/tests/public_e2e.rs`. The test keeps a
  slow chunked `image/png` response open over one public request,
  then sends a small `/edit` request over the same registered tunnel.
- Result: the small request completes while the image stream is still
  active. This gives regression coverage for the basic "one preview
  tab plus one edit request" case with current yamux defaults and the
  existing per-client substream concurrency cap.
- Finding: no request-class policy is implemented yet. Current
  knobs bound total client substream concurrency, open/header
  latency, and response bytes, but they do not reserve capacity for
  editing traffic under saturated tunnel egress or many concurrent
  previews.
- Recommendation: do not add a brittle path/Accept classifier until
  a saturated-bandwidth benchmark reproduces edit latency. If needed,
  keep policy in `PublicConfig`: a small request classifier plus
  per-class active-request semaphores is narrower than changing the
  registry or registering separate tunnel classes.
- Verification:
  `cargo test -p chan-tunnel-server --test public_e2e small_request_completes_while_image_preview_stream_is_active -- --nocapture`
  and `cargo test -p chan-tunnel-server --test public_e2e`.
