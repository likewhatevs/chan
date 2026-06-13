# Item 2 — Rich Prompt queue visibility

Lane: @@PromptQueue, end-to-end (server half first, web half second).
The Pane.svelte badge edit (§4d) WAITS for @@Editor's Pane restructure.
Line numbers from main @ 3ebee587; verify before editing.

## Goal

Copying Claude Desktop / Codex Desktop: a message submitted from the
Rich Prompt (Cmd+Enter) STAYS VISIBLE in the prompt until the agent is
ready to consume it; the user can see that it is enqueued and not yet
consumed. Additionally surface overall queue depth (the queue is
shared with `cs terminal write` team pokes).

## Today's architecture (verified)

- Per-session `write_queue: Mutex<VecDeque<Vec<u8>>>`
  (terminal_sessions.rs ~982), cap WRITE_QUEUE_CAP=100; registry
  drainer ticks 150ms (spawn_drainer ~866) calling
  `Session::try_drain_one` (~1258-1301): output-quiet >= 800ms gate +
  post-delivery generation-start await (2s cap). enqueue_write
  (~1306-1316) returns Some(len-after-push) | None when full.
- WS: ClientFrame::Prompt { data, agent } (routes/terminal.rs
  ~106-145, handled ~653-672) → submit_writes() (chan-shell
  submit.rs; appends the agent submit chord; gemini = TWO writes) →
  enqueue per write. No ack. ServerFrame (~149-188) has NO queue
  frame. Session events flow Session::broadcast(SessionEvent) →
  output_tx (~1415) → AttachHandle.rx → WS select loop translates to
  frames (~690-737) — Activity/Exit/Closed already ride this; new
  queue events reuse it unchanged. broadcast::Sender::send is sync; no
  Session method awaits; compute depth inside the std-mutex guard,
  drop, then broadcast.
- SPA: RichPrompt.svelte Mod-Enter → submit() (~139-150) →
  sendPromptToTerminal (tabs.svelte.ts ~1663) → per-terminal sink
  (TerminalTab.svelte ~250-261, ~888-920) → WS frame. Submit clears
  the doc + flushes the drafts-backed draft (.Drafts/<name>/draft.md,
  400ms debounce, pasted images in the folder).
- **Pre-existing bug fixed in passing:** the Prompt arm enqueues
  per-write with `let _ =` — at 99 entries a gemini submit enqueues
  the body and silently DROPS the CR. New enqueue is all-or-nothing.

## Wire contract

ClientFrame::Prompt gains an optional client id:

```rust
#[serde(rename = "prompt")]
Prompt {
    data: String,
    #[serde(default)] agent: Option<String>,
    /// Client-generated message id; when present the server acks the
    /// enqueue and emits prompt-delivered when the message's LAST
    /// write reaches the PTY. Absent = legacy fire-and-forget (the
    /// team orchestrator's lead-identity prompt stays untagged).
    #[serde(default)] id: Option<String>,
},
```

ServerFrame additions (+ `queue_depth: usize` on `Session`, built at
~564 from a new `AttachHandle::queue_depth()`, so every (re)attach
re-syncs):

```rust
#[serde(rename = "prompt-ack")]
PromptAck { id: String, queued: bool, depth: usize },
// queued ack: depth after push == the message's 1-based position
#[serde(rename = "prompt-delivered")]
PromptDelivered { id: String, depth: usize },
#[serde(rename = "queue")]
Queue { depth: usize },   // broadcast on every message-depth change
```

JSON: `{"type":"prompt","data":"…","agent":"gemini","id":"<uuid>"}`,
`{"type":"prompt-ack","id":"…","queued":true,"depth":2}`,
`{"type":"prompt-delivered","id":"…","depth":1}`,
`{"type":"queue","depth":3}`. Depths are ABSOLUTE message counts —
idempotent under duplicate events, multi-window safe.

SessionEvent additions: `QueueDepth(usize)`,
`PromptDelivered { id: String, depth: usize }`. Emission: every
enqueue (both paths) and every TAIL drain broadcasts QueueDepth; a
tagged tail drain broadcasts PromptDelivered first. Non-tail drains
(gemini body) emit nothing — message depth didn't change.

## Server changes — terminal_sessions.rs

Queue entry (replaces VecDeque<Vec<u8>>):

```rust
struct QueuedWrite {
    data: Vec<u8>,
    /// Rich Prompt message id (None for cs-terminal-write pokes).
    prompt_id: Option<String>,
    /// True on a message's FINAL write (every single-write message,
    /// and the gemini chord). Depth counts tails; PromptDelivered
    /// fires on a tagged tail's drain.
    tail: bool,
}
```

Tagging EVERY write with prompt_id (not just the tail) is deliberate:
a future cancel-by-id is a pure retain-filter (documented v2).

- `fn msg_depth(q) -> usize` — count of tail entries. A gemini pair is
  ONE message; the badge never lies.
- `fn enqueue_prompt(&self, writes: &[Vec<u8>], prompt_id: Option<String>) -> Option<usize>`
  — single lock; rejects the WHOLE message if
  `q.len() + writes.len() > WRITE_QUEUE_CAP` (all-or-nothing); pushes
  all writes, prompt_id on each, tail on last; returns message depth
  after push (== ack position); drop guard, then
  `broadcast(QueueDepth(depth))`.
- `enqueue_write` (~1306, CLI path) — thin wrapper pushing one
  untagged tail entry. **Return value stays the raw queue length** so
  enqueue_write_matching (~635) and EnqueueOutcome
  (control_socket.rs:1374) are byte-for-byte unchanged. Additionally
  broadcasts QueueDepth(messages). Documented divergence: CLI
  position = raw entries; SPA depth = messages.
- `try_drain_one` (~1258) — restructure ONLY the pop section: one
  lock → pop_front + capture msg_depth of remainder → drop guard →
  send_input + existing last_deliver_at/awaiting_gen stores → if
  w.tail: broadcast PromptDelivered{id,depth} when prompt_id set,
  then QueueDepth(depth). Gating above the pop UNTOUCHED (cap, 800ms
  quiet, gen-start await — the team poke bus must not change). No
  awaits in this fn; broadcasts outside the guard.
- `AttachHandle::queue_depth(&self) -> usize`.

## Server changes — routes/terminal.rs

Prompt arm (~653):

```rust
Ok(ClientFrame::Prompt { data, agent, id }) => {
    let submit = SubmitAgent::from_agent_name(agent.as_deref().unwrap_or("claude"));
    let writes: Vec<Vec<u8>> = submit_writes(data, submit)
        .into_iter().map(String::into_bytes).collect();
    let outcome = session.enqueue_prompt(&writes, id.clone());
    if let Some(id) = id {
        let frame = match outcome {
            Some(depth) => ServerFrame::PromptAck { id, queued: true, depth },
            None => ServerFrame::PromptAck { id, queued: false, depth: session.queue_depth() },
        };
        let _ = send_frame(&mut socket, frame).await;  // inline, same socket
    }
}
```

rx arm (~690): `SessionEvent::QueueDepth(d)` → `ServerFrame::Queue`;
`SessionEvent::PromptDelivered{..}` → frame (break on send error like
Activity). All attached sockets get both; non-owners ignore unknown
ids but read depth. Restart/close replaces the Session (queue dies,
comment ~980) → clients get Closed/Exit and re-sync depth on attach.

## Web changes

### state/tabs.svelte.ts
- TerminalTab fields: `queueDepth?: number` (badge; includes teammate
  pokes) and `pendingPrompt?: { id: string; phase: "sent" | "queued" | "delivered" | "rejected" | "failed"; depth?: number }`.
- Exported setters mirroring setTerminalActivity (~1551):
  `setTerminalQueueDepth(tab, depth)` (0 → undefined),
  `beginPendingPrompt(tab, id)`,
  `resolvePendingPrompt(tab, id, phase, depth?)` — id-guarded, stale/
  foreign ids no-op, `failPendingPrompt(tab)` — unguarded (WS close).
- `TerminalPromptSink` + `sendPromptToTerminal` (~1644, ~1663) gain an
  optional trailing `id?: string`. Team-orchestrator call sites pass
  none → untagged frame, zero behavior change for lead-identity
  bootstrap (the one in-repo contract not to break).

### components/TerminalTab.svelte
- TS ServerFrame union (~113): add the three frames + queue_depth on
  session (the union already leads the Rust enum with
  agent_event_echo; unknown types fall through — established).
- Frame handler (~749): session → setTerminalQueueDepth(queue_depth ?? 0);
  queue → depth; prompt-ack → depth + resolvePendingPrompt(queued ?
  "queued" : "rejected"); prompt-delivered → depth +
  resolvePendingPrompt("delivered"); closed/exit → depth 0 +
  failPendingPrompt.
- ws.onclose (~821): failPendingPrompt + setTerminalQueueDepth(0)
  (session frame re-syncs on reconnect).
- sendPrompt (~916): include `id` when given.

### components/RichPrompt.svelte — state machine
- CodeMirror Compartment holding
  `[EditorState.readOnly.of(pending), EditorView.editable.of(!pending)]`,
  reconfigured on phase change.
- submit() (~139): pending exists → no-op return true. Else:
  `const id = crypto.randomUUID()`; `void flushWrite()` (persist
  exactly what was submitted); sendPromptToTerminal(tab.id, text,
  submitAgent(), id); false → today's keep-text path; true →
  beginPendingPrompt, do NOT clear the doc, start 300ms grace timer
  (chip) + 5s ack-timeout.
- $effect on tab.pendingPrompt?.phase:
  - queued: cancel ack-timeout; `.rp-label` (~309) shows
    "queued — waiting for agent" (+ "(#N)" when ack depth > 1);
    read-only, content dimmed.
  - delivered: clear doc, `void flushWrite()` (DRAFT CLEARS HERE),
    clear pending, restore editable, refocus, cancel timers.
  - rejected: restore editable, keep text, transient
    "queue full — try again", clear pending.
  - failed (WS close / ack timeout / session end): restore editable,
    keep text, "connection lost — message may still be queued".
- Idle label when tab.queueDepth > 0:
  "N queued · submit with cmd+enter" (teammate pokes visible in the
  prompt itself).
- Hide/show while pending just works: pending lives on the tab; the
  draft still holds the text until delivery.

### components/Pane.svelte — tab-strip badge (AFTER @@Editor lands)
Next to the activity dot (~1201), same affordance family: a small
count pill `{t.queueDepth}` when `t.kind === "terminal" &&
(t.queueDepth ?? 0) > 0`, title "queued terminal messages".

## UX decisions (made; @@Alex can amend at review)
1. Editing while pending: READ-ONLY (bytes already queued; edits would
   desync). v2 documented: cancel/dequeue via
   `ClientFrame "prompt-cancel" {id}` → retain-filter on prompt_id +
   QueueDepth broadcast + prompt-cancelled ack. Not in v1 (needs UI
   affordance + cancel-vs-inflight-drain race story).
2. Second Cmd+Enter while pending: no-op (replace needs cancel; defer
   together).
3. Reconnect/reload: pending resolves to "failed" on socket loss —
   unlock, keep text, honest label; never destroy user text on an
   ambiguous signal; depth re-syncs from session frame. A reloaded
   user can resubmit → visible recoverable duplicate; accepted v1
   (durable pending = queued-prompt-ids on the session frame, v2).
4. Fast path: read-only immediately, chip only after 300ms grace —
   idle-agent delivery lands within ~1 drainer tick, no flash on
   routine submits.

## Tests

cargo (`cargo test -p chan-server`; update the 4 existing queue tests
~1964-2025 for QueuedWrite):
- enqueue_prompt_is_all_or_nothing_at_cap (99 + gemini pair → None,
  unchanged queue; 1-write fits).
- queue_depth_counts_messages_not_writes (gemini pair → raw 2,
  depth 1; +CLI write → depth 2).
- drain_emits_delivered_on_last_write_only (subscribe output_tx
  before; drive try_drain_one with the timestamp-manipulation
  pattern; body drain → no events; chord drain →
  PromptDelivered{id,0} then QueueDepth(0)).
- enqueue_broadcasts_queue_depth (both paths); enqueue_prompt return
  == position == depth.

routes/terminal.rs (~1025): prompt decode with/without id; serde
snapshots for prompt-ack (both), prompt-delivered, queue, session
with queue_depth; optionally extend the WS event-flow test (~1379
pattern) to observe Queue after enqueue.

vitest: state tests (begin → queued → delivered; rejection;
failPendingPrompt; stale-id ignored; depth 0 → undefined);
richPromptComponent.test.ts source pins (randomUUID in submit; submit
does NOT clear doc; delivered clears doc + flushWrite; readOnly
compartment; grace/timeout constants; pending no-op guard);
richPromptTerminalWiring.test.ts (sendPrompt carries id; the three
frame arms; session applies queue_depth; onclose fails pending; Pane
badge markup — after the badge lands).

## Manual verification

1. Busy agent: `while true; do date; sleep 0.3; done` (sub-800ms gaps
   hold the quiet gate — deterministic).
2. Submit over it → text stays, dims read-only, chip after ~300ms;
   tab badge 1.
3. `cs terminal write --tab-name <name> 'echo poke' --submit=claude`
   ×3 → badge 2, 3, 4 (and in a second attached window).
4. Ctrl-C the loop → drains one message per idle/gen-start cycle;
   badge counts down; prompt clears exactly when its message prints.
5. Gemini rule (plain shell → submitAgent "gemini", 2 writes): text
   drains first (still pending, depth unchanged), CR drain clears.
6. Idle fast path: no chip flash, clears within ~1s.
7. Reload mid-pending → draft text restored, badge correct from
   session frame, queued copy still delivers.
8. Regression: `cs terminal write` stdout (queued/full/position)
   unchanged; cap still 100.

## Sequencing
(1) QueuedWrite + enqueue_prompt + events + drain emission + cargo
tests → (2) WS frames + ack + session queue_depth → (3) tabs store →
(4) TerminalTab frames → (5) RichPrompt state machine → (6) Pane badge
(after @@Editor) → (7) vitest + manual recipe.

Protocol versioning: pre-release, none needed — id is
#[serde(default)], unknown server frames fall through in the SPA.
