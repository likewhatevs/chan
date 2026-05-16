# Martin Task 2: Image Preview Fairness

## Owner

Martin, rustacean.

## Goal

Design and, if small enough, implement plumbing so large preview traffic does not starve latency-sensitive editing traffic over the same tunnel.

## Context

Expected normal payloads are editor traffic plus occasional images. Opening an image preview in an editor tab should not hog the tunnel and make a text editing window feel broken.

## Code To Inspect

- `crates/chan-tunnel-server/src/public.rs`
- `crates/chan-tunnel-server/src/driver.rs`
- `crates/chan-tunnel-client/src/lib.rs`
- `crates/chan-tunnel-client/src/dial.rs`
- `crates/chan-tunnel-server/tests/public_e2e.rs`

## Questions

- Does yamux provide sufficient stream fairness with current defaults?
- Are large response bodies allowed to monopolize h2/yamux window credit?
- Can we classify preview/image requests by path, method, `Accept`, or response `Content-Type` without leaking policy into unrelated crates?
- Should image previews use a lower response cap, separate concurrency limit, or separate tunnel/queue class?
- Where should policy live: public router config, registry/open queue, or client router integration?

## Expected Output

- A short design note in `task/journal.md`.
- Tests or a minimal benchmark demonstrating starvation or acceptable fairness.
- If implementing, keep public API narrow and make knobs explicit in `PublicConfig`.

## Candidate Approaches

- Per-tunnel semaphore for active proxied requests, with a reserved lane for small/editing requests.
- Per-class response caps or idle deadlines.
- Optional request classifier callback in `PublicConfig`, if static heuristics are too brittle.
- Separate tunnel registration class only if single-tunnel fairness is not viable.

## Verification

- Add an e2e test with one slow/large image response and one small text response.
- `cargo test -p chan-tunnel-server public_e2e`
