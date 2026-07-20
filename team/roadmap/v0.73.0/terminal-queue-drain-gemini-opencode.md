# Queue-Drain Batching for Gemini and OpenCode

Status: accepted scope for v0.73.0. Carried forward from v0.72.0, which shipped chronological batching for Codex and Claude only. Gemini and OpenCode remain single-message boundaries and their submit timing under a batch is unproven.

## Problem

v0.72.0 batches consecutive submitted `cs terminal write` notifications into one agent turn ([terminal-write-queue-drain](../done/terminal-write-queue-drain.md)). That behavior is live-proven for two agents: 18 harness runs, Codex and Claude, three cases each (`batch`, `boundaries`, `late`), three runs per case at 64 KiB with the built-in 50 ms body/chord gap, every run passing and recorded row by row in that item.

The other two agents are excluded in code. `ResolvedSubmit::is_batchable` in `crates/chan-shell/src/submit.rs` returns true only for a built-in template on Claude or Codex, and the prefix selector in `crates/chan-library/src/terminal_sessions.rs` stops with `BatchStopReason::UnbatchableAgent` for anything else. A Gemini or OpenCode session therefore drains one queued notification per agent turn, which is exactly the stale-coordination latency the batching work set out to remove.

Each exclusion has a different reason.

Gemini is deliberately a batch boundary, and its body and Return stay two separately idle-gated queue entries. Gemini 0.51 converts a Return received within 30 ms of inserted text into Shift+Return, including text delivered as bracketed paste, and that 30 ms window is live-probed. `WRITE_QUEUE_INPUT_GAP` is 50 ms, measured only against Claude Code. Leaving roughly 20 ms of margin against a live-probed window, on an agent nobody validated, is not a safe trade for an agent that gains nothing from batching today.

OpenCode simply retains its existing boundary and was never exercised. Its built-in template is bracketed paste plus CR in one PTY write, structurally the same shape as Codex, which suggests it would be the smaller of the two changes. That is a similarity between templates, not evidence about how OpenCode handles a 64 KiB paste followed by a submit.

Neither CLI is installed on the validation host, and both require interactive account authentication, so neither could be installed unattended. Their submit timing is unprovable there.

## Desired contract

- An agent stays single-message until the live matrix passes for that agent. Template similarity to a proven agent does not promote it.
- Promoting an agent means all of: adding it to `ResolvedSubmit::is_batchable`; deciding its batched delivery shape in `plan_submitted_input`, which for Gemini is the question of whether a batched body and its bare CR may become one atomic `PtyCommand::InputSequence` under a gap that clears its Shift+Return window, or whether they stay two separately idle-gated queue entries and the agent stays a boundary; and recording the measured gap that decision rests on, not an assumed one.
- Rich Prompt messages, writes without `--submit`, and any runtime submit-template override remain boundaries regardless of agent. Nothing in this item changes those rules.
- If an agent is measured and the answer is that it must stay a boundary, that is a valid close. Record the measurement and the reason in place of a promotion.

## What running the matrix involves

The harness already exists and is committed: `scripts/e2e/terminal-queue-drain.sh`, with cases `batch`, `boundaries`, and `late`. Its load-bearing oracles are the queue depth polled from `cs terminal list --json`, which is server state rather than screen content, and sentinels the agent builds from the number of notification blocks it counted, which no literal in its input can satisfy. Everything read out of the scrollback ring is advisory and recorded in the result row. Every run gets its own tab inside one probe group, and the group is closed on exit.

Two things are needed to run it for a new agent.

First, a harness arm. The script accepts `--agent codex|claude` and rejects anything else. Each arm supplies a launch command that starts the agent with tools disabled or read-only, a ready pattern the harness waits for, and an override template that the `boundaries` case uses to force a batch boundary. Adding Gemini or OpenCode means supplying those three values for that CLI.

Second, a host that satisfies the harness preconditions. The agent under test must be installed and already authenticated, and it must already trust the served workspace: an agent that parks on a first-run trust prompt never prints its ready pattern, so every run fails on the ready-pattern timeout rather than on queue behavior. The server under test also has to be reachable through `CHAN_CONTROL_SOCKET` and `CHAN_WINDOW_ID` from the shell running the harness, and a gap sweep means restarting that server with `CHAN_TERMINAL_INPUT_GAP_MS` set, since the harness only asserts the value the server already read.

## Acceptance

Per agent, on a host where that agent is installed and authenticated:

- `batch` passes three consecutive runs at 64 KiB: the agent emits the sentinel it built from five counted notification blocks, the tail tokens arrive in FIFO order, and the depth trace goes from 5 to 0 with no intermediate sample.
- `boundaries` passes three consecutive runs: the trace passes through every one of 4, 3, 2, and 1, no multi-message batch sentinel appears, and the tokens still arrive in FIFO order, so nothing was skipped to batch a later message.
- `late` passes three consecutive runs: the batch sentinel is followed by a separate late sentinel the agent built from that one message.
- The result rows land in this item in the same shape as the shipped Codex and Claude matrix, naming the agent version, payload size, gap, and the run-length-encoded depth trace.
- For Gemini specifically, the chosen body/chord gap is stated with the measurement behind it and the margin it leaves against the 30 ms Shift+Return window, or the item records that no gap gave adequate margin and Gemini stays two entries.

Unit coverage moves with the behavior: the prefix selector, the input plan, and the boundary tests must pin the new agent's batched and singleton encodings before any live run is treated as acceptance.

## Boundaries

- Do not lower `WRITE_QUEUE_BATCH_MAX_BYTES` from 64 KiB, and do not raise it in this item.
- Do not batch Rich Prompt, raw writes, or override-backed writes. Those boundaries are load-bearing and independent of which agents batch.
- Do not reorder, deduplicate, summarize, or semantically cancel notifications. The receiving agent still gets every original message verbatim inside the envelope.
- Do not add a user-visible batching flag. Eligibility stays a property of the resolved submit specification.
- An agent left unpromoted is not a defect. Single-message delivery is correct, just slower.
