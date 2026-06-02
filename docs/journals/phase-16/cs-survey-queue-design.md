# Survey replies through the cs-write queue — feasibility + design (@@LaneA)

> RESOLVED: @@Host picked **A (leave survey as-is)**. The reply is already
> isolated — a blocking control-socket return to the single waiting caller,
> never a terminal PTY write, so it cannot interleave with the cs-write
> queue. NO CHANGE; this task is a no-op. The design-first grounding
> prevented the unnecessary, survey-breaking async redesign (B). Kept as the
> record of why.


DESIGN-FIRST. @@Host: `cs terminal survey` replies should submit through the
cs-write per-session queue so every input to a terminal/agent is serialized.
@@Lead flagged the crux: the reply may be a BLOCKING CLI return, in which
case "through the queue" means something different. This grounds the actual
path and proposes options. No code yet.

## The actual survey path today (grounded in source)

`cs terminal survey` is an SPA-overlay + blocking-control-socket mechanism;
it does NOT touch any terminal's PTY input at any step:

1. CLI `cmd_shell_survey` (cli.rs) builds a `SurveySpec` and sends a
   `ControlRequest::TermSurvey`. The call BLOCKS (the server holds the
   connection open).
2. Server `handle_survey` (control_socket.rs:829) resolves the target
   window(s) via `window_ids_matching`, mints a `survey_id`, registers a
   oneshot on the `SurveyBus`, and pushes an `OpenSurvey` window_command to
   the SPA window(s) -- an OVERLAY, not a terminal write. Then it `rx.await`s
   (:882).
3. The user answers; the SPA POSTs to `/api/survey/reply`; the route
   (routes/survey.rs) calls `SurveyBus::complete_survey(id, reply)`, firing
   the oneshot.
4. `handle_survey` returns `ControlResponse::Ok { message:
   format_survey_reply(&reply) }` (:883) -- the chosen option label, or the
   `[F]` followup-file path.
5. The blocked `cs terminal survey` process prints `message` to STDOUT
   (cli.rs `println!`). The asking agent reads it as the command's RESULT
   (its Bash-tool output), NOT as terminal/compose input.

The ONLY `write_input_matching` near the survey code (control_socket.rs:776)
is the UNRELATED team-spawn identity poke, not the reply.

## @@Lead's flag: CONFIRMED -- it is a blocking CLI return

The survey reply is delivered as a SYNCHRONOUS control-socket response to the
single blocked `cs terminal survey` caller (-> that process's stdout). It
never enters any terminal's PTY input stream. Consequence: the reply CANNOT
today interleave with the cs-write queue or a Rich-Prompt poke -- it is not a
terminal input at all, and it targets one waiting process, not a shared PTY.
So "serialize the survey reply through the queue (like other terminal
inputs)" does not directly apply: there is nothing multiplexing onto a
terminal to serialize.

## So "through the queue" needs an intent decision

**Option A -- no change (the reply is already isolated by construction).**
If @@Host's concern is "a survey answer must not clobber an agent that is
mid-compose," that is already impossible: the answer is a blocked-caller
stdout return, never a PTY write. Recommend NO CHANGE under this reading.

**Option B -- make the answer a QUEUED INPUT to the asking agent's
terminal.** The only way a reply "submits through the queue" is to stop
returning it to the blocked caller's stdout and instead ENQUEUE it (with the
asker's submit chord) into the ASKING agent's per-session write queue
(`$CHAN_TAB_NAME` of the surveying process -- already captured today as the
followup `from`). The drain then delivers + submits it to the asking agent
when that agent is idle. This is a real SEMANTIC CHANGE to survey:
  - `cs terminal survey` would no longer BLOCK-and-return the answer; it
    becomes fire-and-forget (returns a "survey raised" ack), and the answer
    arrives LATER as a serialized poke into the asker's compose.
  - Every current caller that runs `cs terminal survey` and reads the answer
    SYNCHRONOUSLY from stdout would break -- this is the "do NOT break
    survey" risk. It is only safe if the asking agents are redesigned to
    consume the answer as an inbound poke (the fire-and-forget model the Rich
    Prompt / watcher-automation vision points at).
  - Needs: the asker's tab (have it: `$CHAN_TAB_NAME`), the asker's agent
    type for the chord (default claude), and the reply route / handle_survey
    to call `enqueue_write_matching(asker_tab, answer)` instead of completing
    a blocking oneshot.

**Option B-hybrid -- enqueue AND keep a non-answer ack.** Survey returns
immediately with an ack; the answer is enqueued to the asker. Same break to
synchronous callers as B; listed only for completeness.

## Recommendation

CLARIFY INTENT with @@Host before coding. The reply is a blocking return
today, so:
- If the goal is "answers can't interleave with pokes" -> already true
  (Option A); no work.
- If the goal is "survey becomes async and the answer arrives as a
  serialized poke to the asking agent" (Option B) -> that is a deliberate
  survey-model change (blocking -> fire-and-forget), must be coordinated
  (every `cs terminal survey` caller's contract changes), and only then does
  the queue become the delivery path.

My read: @@Host's "every input to a terminal/agent is serialized" vision
points at Option B (the answer as an inbound queued message), consistent with
the Rich-Prompt/Team-Work "all input through one FIFO" direction. But B
breaks the synchronous-return contract, so I will NOT build it until @@Host
confirms the async survey model is intended. If A, this task is a no-op +
worth documenting why.

## If B is greenlit -- design sketch (no code yet)

- Capture the asker on every survey: add the asker's tab + agent to the
  `TermSurvey` request (today only the `[F]` followup carries `from`).
- On reply: in the reply route (or handle_survey post-await), call
  `enqueue_write_matching(Some(asker_tab), None, chord(answer, asker_agent))`
  -- reusing the queue producer the prompt frame uses. The drain delivers it
  when the asker is idle.
- `cs terminal survey` returns a "raised; answer will arrive in your
  terminal" ack instead of blocking for the answer.
- Keep `[F]` followup-file creation unchanged (the file path can ride the
  enqueued message body).
- Tests: the reply enqueues to the asker's tab; the asker's chord is applied;
  survey raise still fans out to the target window(s). Live-smoke a full
  raise -> answer -> queued-delivery round trip.

## Footprint (B, all-my-lane + the survey seam)

control_socket.rs (handle_survey reply path), routes/survey.rs (reply route
-> enqueue instead of complete-oneshot), chan-shell wire.rs/cli.rs (the
asker tab/agent on TermSurvey + the non-blocking ack). routes/survey.rs is
@@LaneC's historical file -- coordinate if B is chosen.
