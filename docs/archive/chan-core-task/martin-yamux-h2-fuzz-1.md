# Martin Task 1: Yamux And H2 Edge-Case Breakage

## Owner

Martin, rustacean.

## Goal

Probe protocol edge cases around `H2Duplex`, framed handshakes, and yamux driver behavior. We want panics, hangs, broken wakeups, unexpected EOF behavior, and missing regression tests.

## Code To Inspect

- `crates/chan-tunnel-proto/src/h2_duplex.rs`
- `crates/chan-tunnel-proto/src/frame.rs`
- `crates/chan-tunnel-proto/src/io.rs`
- `crates/chan-tunnel-server/src/driver.rs`
- `crates/chan-tunnel-client/src/lib.rs`

## Attack Cases

- h2 DATA frames larger than the caller read buffer, including zero-sized caller buffers.
- h2 capacity returning repeated zero grants.
- half-close ordering: peer closes read/write sides during Hello, HelloAck, and yamux startup.
- oversized control-frame length prefixes with partial bodies.
- malformed Hello / HelloAck refusal frames.
- inbound yamux streams from the wrong side at high rate.
- pending outbound open requests when yamux closes or shutdown wins the poll cycle.

## Expected Output

- Regression tests for every confirmed bug.
- Findings and open questions added to `task/journal.md`.
- Small fixes in the relevant crate, avoiding API churn unless needed.

## Candidate Fixes

- Harden zero-capacity or zero-buffer paths in `H2Duplex`.
- Add explicit close/error tests for pending open replies in `drive_tunnel`.
- Add malformed-frame tests that prove memory stays bounded by `MAX_CONTROL_FRAME_BYTES`.

## Verification

- `cargo test -p chan-tunnel-proto`
- `cargo test -p chan-tunnel-server`
- `cargo test -p chan-tunnel-client`
