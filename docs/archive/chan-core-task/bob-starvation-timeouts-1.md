# Bob Task 1: Starvation And Timeout Attack Map

## Owner

Bob, syseng.

## Goal

Find ways a public client, tunnel client, or slow upstream can pin workers, queues, file descriptors, or bandwidth long enough to degrade other tunnel traffic.

## Code To Inspect

- `crates/chan-tunnel-server/src/public.rs`
- `crates/chan-tunnel-server/src/registry.rs`
- `crates/chan-tunnel-server/src/driver.rs`
- `crates/chan-tunnel-server/src/tunnel.rs`
- `crates/chan-tunnel-client/src/lib.rs`

## Attack Cases

- Fill the per-tunnel `mpsc` open queue by issuing many public requests while the client is slow or not polling yamux.
- Hold `TunnelHandle::open().await` waiters indefinitely and measure task growth.
- Open many inbound yamux substreams against the client and observe unbounded spawned `serve_one_substream` tasks.
- Start response bodies that send headers quickly and then drip bytes forever below the response cap.
- Start request uploads that are accepted by the body cap but drip slowly enough to hold substream resources.
- Exercise upgrade paths that move just enough bytes to reset the idle watchdog forever.

## Expected Output

- A short findings section added to `task/journal.md`.
- Reproduction commands or tests for confirmed issues.
- Recommended caps/deadlines, with defaults and rationale.

## Candidate Fixes

- Add public-router timeout around `handle.open()`.
- Use `try_send` or a bounded timeout for open requests when queue pressure is high.
- Add client-side concurrent substream limit or semaphore.
- Add idle/read-rate watchdog for non-upgrade proxied bodies if needed.

## Verification

- Prefer targeted async tests in `chan-tunnel-server/tests/public_e2e.rs` and client unit/integration tests.
- Run the smallest relevant cargo command first, for example:
  - `cargo test -p chan-tunnel-server public_e2e`
  - `cargo test -p chan-tunnel-client`
