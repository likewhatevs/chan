# Drain Queued Terminal Notifications in One Agent Turn

> Carried forward from v0.71.0 and implemented for v0.72.0 on 2026-07-19.

Status: implemented and unit-tested. The live acceptance matrix has NOT been run against the current harness, so batching is unproven live. The original plan was grounded against `a27007f5` (`v0.70.3`) on 2026-07-18.

## Implementation Evidence

- The repeatable live harness is `scripts/e2e/terminal-queue-drain.sh`, with three cases: `batch`, `boundaries`, and `late`. Every run gets its own tab inside one probe group, and the group is closed on exit. Its load-bearing oracles are the queue depth polled from `cs terminal list --json` (server state) and sentinels the AGENT builds from the number of notification blocks it received, which no literal in its input can satisfy. Everything read out of the scrollback ring is ADVISORY and recorded in the result row: that ring holds PTY output only, so a framed envelope appears there only when the agent renders the pasted body verbatim, and Claude prints `[Pasted text #1 ...]` instead at these sizes.
- No result from this harness exists yet. It replaced an earlier script whose only oracle was the ordered tail tokens in scrollback, which a serial five-turn delivery satisfies exactly as well as one batched turn. The earlier script's runs (Codex 0.144.6 and Claude Code 2.1.215, 3/3 at 1, 16, and 64 KiB, plus 3/3 for Claude at 64 KiB with 50, 100, 200, and 400 ms body/chord gaps, plus advisory 256 KiB runs on a scratch build with a raised ceiling) therefore establish delivery, submission, and FIFO order across payload sizes and gaps. They do not establish that a prefix was batched, that depth moved in one step, or that a boundary held.
- `WRITE_QUEUE_INPUT_GAP` is 50 ms, the smallest gap that passed those earlier delivery runs. `CHAN_TERMINAL_INPUT_GAP_MS` (1..799 ms, mirroring `parse_input_gap`) re-runs a sweep without a rebuild.
- Unit tests pin FIFO boundaries, no skipping, the 64 KiB ceiling, oversized-head progress, singleton bytes, one Codex input, one Claude input sequence, Rich Prompt event ordering, enqueue-after-selection behavior, and shared fresh/restored PTY sequence writes. The enqueue-after-selection property (Required Behavior item 4) is covered ONLY there: selection and pop happen under one queue lock inside a drainer tick, so no external process can enqueue between them.
- Gemini was not installed on the validation host, so it was not promoted. Gemini, OpenCode, Rich Prompt, raw input, and runtime overrides remain single-message boundaries.

### Live Matrix Results

Fill this in from the harness result rows, one line per run. Until a row exists, the matrix has not been run.

| agent | case | size | gap | runs | result | notes |
| --- | --- | --- | --- | --- | --- | --- |
| | | | | | | |

## Divergences From the Plan

Three things landed differently from the design above, and the code is the authority:

- Gemini keeps TWO idle-gated queue entries, not one atomic `InputSequence`. Design section 5 reads as if a Gemini body and CR could share one controller sequence with `WRITE_QUEUE_INPUT_GAP` between them, but that gap is measured only against Claude Code; leaving ~20 ms of margin against Gemini's live-probed 30 ms Shift+Return window, on an agent nobody validated, is not a safe trade for an agent that gains nothing from batching. `WRITE_QUEUE_INPUT_GAP` has exactly one user: the batched Claude body/chord split.
- The queue tracks ENTRIES, not `write_cost` units. With Gemini split back into two entries, every entry costs exactly one raw write, so a separate cost field would always equal the entry count. `WRITE_QUEUE_CAP` bounds entries, a message is still all-or-nothing at the cap, and depth still counts logical messages (tail entries).
- `cs terminal list --json` gained `queue_depth`. Depth was previously observable only through the SPA's WebSocket `queue` frame, which left the live harness with no way to prove a batch drains in one step instead of five.

## Summary

Keep the current correctness contract: `cs terminal write` remains a bounded per-session FIFO, never writes into a busy agent, and preserves arrival order. Change what happens at one safe drain opportunity: instead of submitting exactly one queued `cs terminal write` notification, take the largest eligible FIFO prefix, frame those notifications as one chronological prompt, and submit that prompt once.

The receiving agent then sees message 0 through message N in one turn and can reconcile them before acting. A later notification that says a bug is fixed or a hold is lifted is visible before the agent spends a full turn on an earlier stale report.

Do not concatenate the raw queue entries that exist today. Their submit bytes have already been encoded, so concatenation can place multiple submit chords inside one prompt and agent paste handling differs. Refactor the queue to retain logical message text plus resolved submit metadata until drain time, form the batch, then encode the batch once.

Initial batching scope:

- Batch only consecutive submitted messages originating from `cs terminal write`.
- Keep Rich Prompt messages as one turn each. They remain in the same FIFO and act as batch boundaries.
- Keep writes without `--submit` as opaque, single-message writes. They may be shell input or intentionally parked compose text.
- Batch the built-in Codex and Claude submit modes.
- Keep Gemini single-message until the same live matrix proves batched body plus separate CR is reliable.
- Treat any runtime submit-template override as single-message unless its batch behavior is explicitly validated. Preserve the override exactly instead of guessing how to split it.

## Problem

The current serializer prevents conflicting terminal input, but its one-message-per-agent-turn policy amplifies stale coordination:

```text
queue at the next idle opportunity

0  bug report A
1  extra context for A
2  report B
3  correction to A
4  A is fixed at <sha>; do not investigate it
```

Today the agent receives 0, generates a full response, returns idle, receives 1, and repeats. It may spend several long turns investigating A before it reaches 4. The queue is correct and lossless, but it prevents the receiver from reconciling the queue as a whole.

Desired behavior at that same idle opportunity:

```text
one submitted turn

Queued notifications, oldest first. Read the whole batch before acting;
later messages may update or supersede earlier messages.

--- notification 1/5 ---
bug report A
--- notification 2/5 ---
extra context for A
...
--- notification 5/5 ---
A is fixed at <sha>; do not investigate it
```

This is a latency and coordination improvement, not a correctness fix. The current idle gate, queue bound, ordering, selectors, restart behavior, and raw terminal-input path remain load-bearing.

## Current Contract

### Queue and drain

`crates/chan-library/src/terminal_sessions.rs` owns the per-session queue:

- `WRITE_QUEUE_CAP` is 100 raw write entries.
- `WRITE_QUEUE_QUIET_MS` is 800 ms. PTY output silence is the only idle signal.
- `WRITE_QUEUE_DRAIN_TICK` is 150 ms.
- `WRITE_QUEUE_GEN_START_CAP_MS` is 2 seconds.
- `Session::try_drain_one` pops one `QueuedWrite`, sends it to the PTY, then waits for output newer than the delivery timestamp or the 2-second cap before it can pop another.
- `Registry::spawn_drainer` scans every live session on the tick.

The original implementation rationale is preserved in commit `3d6d144e`: one per-session FIFO, output-quiescence idle detection, and a generation-start wait prevent chained pokes from stacking in one compose buffer. This plan keeps those safety properties around a batch rather than around every notification inside the batch.

### Producers

Two producers share the queue:

- `cs terminal write` is parsed in `crates/chan-shell/src/cli.rs`, where `submit_writes` applies the selected agent encoding before the control request is sent. `ControlRequest::TermWrite` in `crates/chan-shell/src/wire.rs` therefore carries already-encoded `data` only. `crates/chan-server/src/control_socket.rs::term_write` enqueues those bytes as an untagged entry.
- The terminal WebSocket `prompt` frame in `crates/chan-server/src/routes/terminal.rs` carries logical text, agent name, and an optional prompt id. The route calls `submit_writes` immediately and enqueues the resulting one or two raw writes as one logical Rich Prompt message.

Early encoding is the architectural blocker. By drain time the library cannot safely recover the original text, selected agent, or submit-template source from arbitrary bytes.

### Submit encodings

`crates/chan-shell/src/submit.rs` is the source of truth:

- Codex default: bracketed-paste open, text, bracketed-paste close, then CR in one PTY write.
- Claude default: text followed by the xterm modifyOtherKeys Cmd+Enter CSI in one PTY write.
- Gemini default: text and CR as two separate queue writes because a coalesced text plus CR does not submit.
- `CHAN_SUBMIT_<AGENT>` and `<config>/chan/submit.toml` can override the resolved template.

The queue must retain enough resolved metadata to honor these rules and overrides after batching.

### Queue visibility and recall

Rich Prompt queue state is message-based even though the backing queue is write-based:

- `QueuedWrite::tail` makes a Gemini body plus chord count as one Rich Prompt message.
- `SessionEvent::QueueDepth` drives the terminal badge.
- `SessionEvent::PromptDelivered` fires only when the tagged tail is popped.
- `cancel_prompt` removes every still-queued raw write carrying the prompt id.
- `queued_prompt_ids` re-synchronizes pending Rich Prompt ids on attach.

Batching only untagged `cs terminal write` messages keeps the first implementation away from Rich Prompt recall and per-id delivery semantics. The logical-queue refactor must still preserve those semantics for single Rich Prompt entries.

### Documentation that encodes the old behavior

- `crates/chan-shell/src/cli.rs` help says the queue delivers one message at a time.
- `crates/chan-shell/design.md` describes submit encoding before queue delivery.
- `crates/chan-server/src/routes/team_config.rs::generate_bootstrap_md` tells every generated team that the next poke arrives only after another full idle cycle.
- `web/packages/marketing/src/pages/manual.html` describes Rich Prompt and `cs terminal write` as one shared serialized queue.

## Preliminary Live Validation

Disposable terminals were spawned through the live `cs` control socket and removed after the probe. Environment:

- chan baseline `a27007f5`.
- Codex CLI `0.144.5`.
- Claude Code `2.1.214`.
- One synthetic 18 KiB payload containing five framed notifications, with a unique token at the end of each notification.
- The agent had to return all five tokens in order without using tools.

Results:

- Codex accepted the payload through the existing `--submit=codex` bracketed-paste plus CR encoding, submitted one turn, read all five tokens, and returned the expected sentinel.
- Claude recognized the same `--submit=claude` payload as a large paste and left it parked as `[Pasted text #1 +16 lines]`. The submit CSI appended to the same PTY write did not submit it.
- Sending Claude's submit CSI as a later, separate PTY write submitted the parked batch. Claude read all five tokens and returned the expected sentinel.

Conclusion: batching is viable for both agents, but the delivery sequence is agent-specific. Codex can receive the encoded batch atomically. Claude needs the batch body and submit chord as distinct PTY writes once paste handling activates. Encoding each queued notification first or blindly appending one Claude chord to a large write is unsafe.

This was a feasibility probe, not the full acceptance matrix. It proves the design direction and the Claude failure mode at 18 KiB. It does not establish the minimum safe Claude inter-write gap, a maximum payload, Gemini behavior, or cross-version stability.

## Required Behavior

1. A busy agent receives nothing, exactly as today.
2. At the first safe idle opportunity, every eligible notification already in the selected FIFO prefix is removed atomically and submitted as one agent turn.
3. Notification order is unchanged and made explicit in the batch envelope.
4. A notification enqueued after the prefix is selected stays queued for the next opportunity.
5. A Rich Prompt message, an unsubmitted raw write, Gemini, a different resolved submit spec, or the byte ceiling ends the prefix. The drainer never skips the boundary to batch later messages.
6. One queued notification is delivered byte-for-byte under its existing single-message encoding. Do not add a batch envelope to a singleton.
7. Queue depth decreases by the number of logical messages drained. No intermediate `N-1`, `N-2`, ... badge churn for one batch.
8. The generation-start wait applies once after the complete batch delivery sequence, not between notifications inside the batch.
9. Runtime submit overrides retain their current single-message behavior. An unrecognized template shape disables batching for that message.
10. No payload text is logged.

## Design

### 1. Carry a resolved logical submit specification over the control wire

Add wire-safe submit metadata in `crates/chan-shell/src/submit.rs` and re-export it from `lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubmitAgent {
    Claude,
    Codex,
    Gemini,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedSubmit {
    pub agent: SubmitAgent,
    pub template: String,
    pub source: SubmitTemplateSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmitTemplateSource {
    BuiltIn,
    Override,
}
```

`ResolvedSubmit::resolve(agent)` applies the existing precedence and unescaping once. The source bit is behavioral, not diagnostic decoration: only known built-in templates are initially batch-enabled. It also prevents the server from silently replacing a `CHAN_SUBMIT_*` value that was set only in the short-lived `cs` process.

Change `ControlRequest::TermWrite` to carry logical text and `submit: Option<ResolvedSubmit>`. `None` is an opaque raw write. `crates/chan-shell/src/cli.rs` sends one control request per invocation rather than calling `submit_writes` and sending pre-encoded requests.

This is a pre-release internal wire, so no migration layer is required. Pin the serialized JSON. An old `cs` process talking to a new server remains a short-lived upgrade-skew case, not a supported compatibility contract.

Keep `apply_submit_chord` and `submit_writes` for existing direct team-spawn and compatibility callers until all construction sites are deliberately migrated. Do not mix that cleanup into the batching change.

### 2. Store logical messages, not encoded writes

Replace `VecDeque<QueuedWrite>` with a queue of logical messages in `crates/chan-library/src/terminal_sessions.rs`:

```rust
enum QueueSource {
    CsWrite,
    RichPrompt,
}

struct QueuedMessage {
    data: String,
    submit: Option<ResolvedSubmit>,
    source: QueueSource,
    prompt_id: Option<String>,
    write_cost: usize,
}
```

`write_cost` preserves the current raw-write capacity accounting while the queue becomes logical: one for raw, Codex, and Claude; two for Gemini. Track the total cost under the same queue mutex and refuse an enqueue that would exceed 100. `queue_depth` is the number of logical messages. The CLI's reported position can remain the cumulative write cost so existing position wording does not need to change.

Provide two narrow enqueue methods:

- `enqueue_cs_write(data, submit)` creates an untagged `QueueSource::CsWrite` message.
- `enqueue_prompt(data, submit, prompt_id)` creates one tagged `QueueSource::RichPrompt` message.

The server route no longer expands a Rich Prompt into raw writes before enqueue. `cancel_prompt` becomes a single-message retain filter, and `queued_prompt_ids` reads one id per queued logical message. This removes the current need for `tail` bookkeeping while keeping the external state machine unchanged.

### 3. Select a maximal eligible FIFO prefix

Add a pure selector used under the queue lock:

```rust
fn select_batch_prefix(
    queue: &VecDeque<QueuedMessage>,
    max_bytes: usize,
) -> BatchSelection;
```

The head is batchable only when all of these hold:

- `source == QueueSource::CsWrite`.
- `submit` is present.
- the resolved template source is `BuiltIn`.
- the agent is Codex or Claude.

Continue through consecutive entries while source, agent, and the entire resolved submit spec are equal and the framed payload stays within `WRITE_QUEUE_BATCH_MAX_BYTES`. Stop at the first mismatch. Never scan past a boundary.

If fewer than two messages qualify, use the existing single-message encoding and no envelope. If the head alone exceeds the byte ceiling, deliver it alone so a large message cannot wedge the queue.

Start with `WRITE_QUEUE_BATCH_MAX_BYTES = 64 * 1024`. Phase 1 may raise it after repeated live proof, but should not lower it without revisiting the objective: 64 KiB comfortably holds the intended bounded queue of lean one-line coordination pokes while avoiding an unbounded compose paste. Count the final UTF-8 envelope bytes, not Rust character count.

### 4. Frame the chronological batch once

Add a pure `format_notification_batch(&[QueuedMessage]) -> String` in the queue module. Keep the envelope ASCII and stable enough for exact unit tests:

```text
# Queued terminal notifications

5 messages, oldest first. Read the entire batch before acting. Later
messages may update or supersede earlier messages.

--- notification 1/5 ---
<message 1>
--- end notification 1/5 ---

...
```

Apply the existing trailing-newline trim to each submitted message before framing, matching today's per-message `apply_submit_chord` behavior. Preserve interior newlines and empty messages. Delimiters are an agent-reading aid, not a security boundary; queue content is still instruction text by design.

Do not summarize, deduplicate, reorder, or attempt to infer supersession in chan. The receiving agent owns reconciliation and gets every original message verbatim inside the envelope.

### 5. Encode after the batch is formed

Centralize encoding in `chan-shell` as a pure input-plan builder so the CLI, server, library, and tests do not grow another submit map:

```rust
pub struct PtyInputPlan {
    pub parts: Vec<Vec<u8>>,
}

pub fn plan_submitted_input(
    text: String,
    submit: Option<&ResolvedSubmit>,
    batched: bool,
) -> PtyInputPlan;
```

Rules:

- No submit: one verbatim part.
- Codex built-in: one part using the existing bracketed-paste template plus trailing CR, for singleton and batch.
- Claude built-in singleton: retain the current one-part encoding.
- Claude built-in batch: two parts, the batch body followed by the bare submit CSI.
- Gemini: retain its current body plus bare CR parts and remain single-message.
- Override: retain the existing single-message expansion and mark it unbatchable.

This keeps the default bytes in one source of truth and prevents the queue from parsing already-encoded terminal control sequences.

### 6. Deliver multi-part input atomically with a tested gap

Add an input-sequence command to the PTY controller in `crates/chan-library/src/terminal_sessions.rs`:

```rust
enum PtyCommand {
    Input(Vec<u8>),
    InputSequence {
        parts: Vec<Vec<u8>>,
        gap: Duration,
    },
    // Resize, Redraw, Kill
}
```

The controller writes and flushes each part, waits the validated gap between parts, and finishes the sequence before processing another controller command. This prevents keyboard input or another queued notification from interleaving between a pasted Claude body and its submit chord. Implement the same branch in fresh and fdstore-restored PTY controllers, preferably through one small shared write helper so their behavior cannot drift.

Use the existing direct team-spawn precedent of a 400 ms split gap as the initial candidate, but freeze the final value only after Phase 1 tests 50, 100, 200, and 400 ms on current Claude. The chosen gap must be below the 800 ms queue-idle threshold. Name it next to the other queue timing constants and state the live evidence in its comment.

`Session::try_drain_one` becomes `try_drain_batch`:

1. Preserve the empty, awaiting-generation, and output-quiet checks.
2. Select and pop the batch under the queue mutex.
3. Compute remaining logical depth under that same mutex.
4. Format and encode outside the mutex.
5. enqueue one `PtyCommand::Input` or `InputSequence`.
6. Set `last_deliver_at` and `awaiting_gen` once.
7. Emit one `QueueDepth(remaining_depth)` event.

The current delivery acknowledgment means "accepted by the PTY controller channel", not "confirmed written by the kernel". Keep that contract. A later controller write failure already emits `SessionEvent::Error` and tears down the session.

### 7. Preserve Rich Prompt events

Rich Prompt is deliberately a batch boundary in this version. When its single logical message drains:

- Build its normal one-message input plan.
- Emit `PromptDelivered { id, depth }` before `QueueDepth(depth)`, preserving the current ordering.
- A cancel racing after the logical message is popped returns `removed: false`, which is the honest result because delivery has begun.
- `queued_prompt_ids` contains only still-cancellable queued messages.

No terminal WebSocket frame shape changes are required. The SPA already treats depth as an absolute count and unknown delivered ids as ignorable.

### 8. Add trace-only observability

Emit one trace event per drain without content:

```text
terminal_write_drain session=<id> messages=5 bytes=6120 parts=2 remaining=0 agent=claude
```

Include whether delivery was `single` or `batch` and why an apparent prefix stopped at a boundary at trace level if useful. Never log payload text, submit-template bytes, or prompt ids at info level.

## Implementation Phases

### Phase 1: validation and scratch prototype

Do this before landing the queue refactor.

1. Turn the preliminary probe into a repeatable local harness under `scripts/e2e/terminal-queue-drain.sh` or a small Rust integration test driver. It must use unique tab names/groups, poll `cs terminal scrollback`, and trap cleanup with `cs terminal close --tab-group=<probe-group>`.
2. Run Codex and Claude with tools disabled/read-only. Use five messages containing tokens only at each message tail, then require one exact response containing every token in order.
3. Test framed payloads at 1 KiB, 16 KiB, 64 KiB, and 256 KiB. Run each supported case three times. Record whether the TUI renders ordinary input or a paste placeholder, whether it submits, and whether all tail tokens arrive.
4. For Claude, test separate body/chord gaps of 50, 100, 200, and 400 ms. Choose the smallest value with three consecutive successes at 64 KiB, then repeat it at 256 KiB as advisory evidence.
5. Build a scratch logical-queue prototype and enqueue five separate `cs terminal write` calls while the agent is busy. Verify the next idle opportunity yields exactly one user turn and one response, not merely that one pre-concatenated payload works.
6. Enqueue a sixth message after the prototype has selected its batch. Verify it remains for the next turn.
7. Put a Rich Prompt message, a no-submit write, and an override-backed write between eligible notifications. Verify FIFO boundaries and no skipping.
8. Probe Gemini separately. Promote it to batchable only if body plus distinct CR succeeds across the same three-run matrix; otherwise retain the planned single-message behavior.

Phase 1 exit criteria:

- Codex and Claude pass three consecutive five-notification runs at 64 KiB.
- Claude's split gap is measured, not assumed.
- Five separate queue entries produce one submitted turn under the prototype.
- Cleanup leaves no probe terminals or processes.
- The journal records agent versions, payload sizes, gap, success counts, and representative scrollback sentinels.

### Phase 2: chan-shell logical submit wire

Files:

- `crates/chan-shell/src/submit.rs`: serializable `ResolvedSubmit`, template-source tracking, pure input-plan builder, default/override tests.
- `crates/chan-shell/src/wire.rs`: add the optional resolved submit spec to `TermWrite`; pin JSON.
- `crates/chan-shell/src/cli.rs`: send one logical request; update help.
- `crates/chan-shell/src/lib.rs`: re-export the narrow types/functions used by chan-library and chan-server.

Exit criteria: chan-shell tests prove exact Codex, Claude singleton, Claude batch, Gemini, no-submit, env override, config override, and wire bytes.

### Phase 3: logical queue and batched drain

Files:

- `crates/chan-library/src/terminal_sessions.rs`: logical queue, capacity accounting, prefix selection, envelope, input sequence, batched drain, event preservation, trace event, tests.

Exit criteria: all queue state-machine tests pass without a live agent and both fresh and fdstore-restored PTY controllers share the same sequence writer.

### Phase 4: producer integration

Files:

- `crates/chan-server/src/control_socket.rs`: accept logical `data + submit`, enqueue one `QueueSource::CsWrite`, retain selector and response behavior.
- `crates/chan-server/src/routes/terminal.rs`: resolve the Rich Prompt submit spec server-side and enqueue one `QueueSource::RichPrompt`; remove early `submit_writes` expansion from this path.
- `crates/chan-server/src/routes/team_config.rs`: update generated queue-drain guidance and its source-pin tests.

Do not migrate direct team-spawn identity pokes unless required by the new submit API. They are outside the notification FIFO and already own an explicit multi-write gap.

### Phase 5: documentation, gates, and live acceptance

Files:

- `crates/chan-shell/design.md`: logical wire payload, drain-time encoding, agent-specific batch delivery.
- `web/packages/marketing/src/pages/manual.html`: multiple queued `cs` notifications can arrive as one chronological agent prompt.
- `CHANGELOG.md`: user-visible latency behavior and Claude paste-safe split.

Run the scoped tests first, then the full gate required for a release change.

## Automated Tests

### chan-shell

- `SubmitAgent` and `ResolvedSubmit` serde round trips use lowercase wire names.
- `TermWrite` JSON includes logical data and the resolved submit spec.
- Built-in resolution is marked `BuiltIn`; env/config resolution is marked `Override` and retains exact unescaped bytes.
- Singleton encoding stays byte-identical to today's Claude and Codex defaults.
- Claude batch encoding produces exactly two parts: body and CSI.
- Codex batch encoding produces exactly one bracketed-paste plus CR part.
- Gemini remains two parts and is not batchable.
- No-submit remains one verbatim part, including trailing newlines.

### chan-library

- Busy output holds the whole queue.
- Five eligible Codex messages drain as one batch and set one generation wait.
- Five eligible Claude messages create one two-part controller sequence.
- A singleton has no envelope and retains exact bytes.
- FIFO prefix stops on Rich Prompt, no-submit, Gemini, different agent, different template, override, and byte ceiling.
- A message enqueued after prefix selection remains queued.
- Capacity remains all-or-nothing at 100 write-cost units.
- One batch emits one absolute queue-depth change.
- Rich Prompt delivered/depth event order is unchanged.
- Rich Prompt cancellation removes a queued logical message and cannot remove one already popped for delivery.
- `queued_prompt_ids` preserves FIFO order.
- Batch formatting preserves message order, UTF-8, interior newlines, empty messages, and per-message trailing-newline trimming.
- A first message larger than the byte ceiling drains alone rather than wedging.
- The shared controller helper flushes parts separately and invokes the gap only between parts. Use an injected sleeper/recorder so unit tests do not sleep.
- Fresh and restored session construction initializes the same queue state.

### chan-server

- `term_write` keeps the selector-required, no-match, capacity, single-target position, and group fan-out responses.
- The terminal `prompt` frame still defaults an omitted agent to Claude, acks tagged enqueue, and emits delivered only on drain.
- Generated bootstrap text says pending pokes are delivered together in chronological batches at idle, and tells agents to read the entire batch before acting.

### Live acceptance

- While each agent is generating, enqueue five real `cs terminal write --submit=<agent>` notifications including a correction and a final "already fixed" message.
- Verify queue positions increase immediately while no input reaches the busy compose box.
- On idle, verify scrollback shows one submitted user turn containing all five framed notifications.
- Verify the agent acknowledges the final state without independently acting on the superseded report.
- Repeat for Codex and Claude at the Phase 1 payload ceiling.
- Verify a sixth late enqueue produces the next turn.
- Verify queue depth jumps from five to zero for the batch.
- Verify one normal Rich Prompt before or after the notifications stays its own turn.
- Restart/close a terminal with a pending queue and verify the existing drop-on-session-recycle behavior remains.

Suggested scoped commands:

```sh
cargo fmt --check
cargo test -p chan-shell submit
cargo test -p chan-library terminal_sessions
cargo test -p chan-server routes::terminal
cargo test -p chan-server team_config
cargo clippy -p chan-shell -p chan-library -p chan-server --all-targets -- -D warnings
```

Then run the repository pre-push gate per `.agents/skills/gate/SKILL.md`.

## Risks and Mitigations

- Claude paste mode swallows a coalesced submit chord. Mitigation: body and chord are separate controller writes with a live-validated gap.
- A very large queue can create an oversized agent prompt. Mitigation: a named byte ceiling selects a maximal FIFO prefix and always allows an oversized head to drain alone.
- Batching could cross a semantic input boundary. Mitigation: only consecutive `QueueSource::CsWrite` messages with an identical built-in submit spec are eligible; Rich Prompt and raw input are hard boundaries.
- Runtime chord overrides can encode arbitrary prefix/suffix behavior. Mitigation: carry the resolved override over the wire and keep it single-message until explicitly supported.
- A new notification can race batch selection. Mitigation: select and pop the prefix under the existing queue mutex; later enqueue stays in the remainder.
- Queue badge or recall state can drift during the raw-to-logical refactor. Mitigation: make depth logical, keep event ordering pinned, and retain tagged Rich Prompt tests before enabling batching.
- Controller input could interleave between Claude body and chord. Mitigation: one `PtyCommand::InputSequence` owns all parts atomically inside the per-session controller thread.
- TUI versions can change paste/submit handling. Mitigation: keep submit templates runtime-overridable, keep overrides single-message, retain the local live probe as a release smoke, and record tested agent versions.

## Non-goals

- Replacing output quiescence with shell integration, prompt detection, or a new agent protocol.
- Detecting whether a human has a paused half-typed compose buffer.
- Reordering, deduplicating, summarizing, or semantically cancelling notifications in chan.
- Batching raw keyboard input or shell commands.
- Batching Rich Prompt submissions in the first implementation.
- Changing the queue's drop-on-terminal-restart behavior.
- Adding a user-visible batching flag. The optimization is automatic for known-safe notification traffic.

## Recommended Landing Shape

Land this as two reviewable commits after Phase 1 evidence exists:

1. `refactor(terminal): queue logical submitted messages` - wire metadata, logical queue, exact singleton behavior, Rich Prompt state preservation, no batching enabled.
2. `perf(terminal): drain queued notifications as one agent turn` - prefix selection, envelope, Claude input sequence, docs, live harness, and batch tests.

The first commit must prove no behavioral drift under the existing unit suite and singleton live smoke. The second commit contains the latency behavior and can be reverted independently if an agent version invalidates the batching matrix.
