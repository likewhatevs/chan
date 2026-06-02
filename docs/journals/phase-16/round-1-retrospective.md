# Phase-16 round-1 retrospective (@@Lead)

Round shape: a structured wave-1/2/3 core dispatch (cs lead-tooling, graph,
preflight, gateway docs, CI) that opened the round, followed by a long
@@Host-driven feature + polish stream that became the bulk of the work. Five
worker lanes (@@LaneA..E) + a dedicated @@Lead architect, coordinated over
on-disk task files + append-only event channels, gated through an isolated
worktree. Heading into a 0.24.0 release.

## Done (merged to round-1 main)

Core waves (pre-feature-stream): cs lead-tooling (C2 scrollback / C3 pane /
C1 team-load / S1 / window-id), G1 graph dir-spine, P1 preflight, P2
onboarding nudge, F-series editor/UI fixes, TW1 team dialog, D1 gateway
reframe, B1 CI bump, gateway guide-v2. (~26 slices.)

@@Host feature + polish stream:
- Mermaid: cursor-based render (no flip button), horizontal flip, up/down
  step-in, reverse-flip symmetry, visible selection inside code blocks,
  error line/col locatability.
- Image-viewer prev/next; image-drag source-row indicator.
- Reports on by default + actionable onboarding card; preflight OK button.
- Dashboard: carousel navigator, real-engine screensaver preview, carousel-nav
  centering, carousel moved into the OK footer row, screensaver preview inside
  the Screen-lock box + shown only when locked.
- Per-workspace directory blocklist: backend (global baseline + per-workspace
  additions, union filter, off-loop re-walk) + file-browser settings UI.
- About-slide motif/pitch removal.
- cs-write QUEUE: always-on per-session FIFO + idle-drain, serializing all
  terminal/agent input.
- Rich Prompt: floating Cmd+Shift+P bubble, markdown-list continuation, then
  re-architected to be Drafts-backed with editor-style image paste.
- Team Work: decoupled from regular terminals, then the in-terminal bubble
  deleted entirely (lead becomes a normal terminal; identity via the queue).
- Graph Drafts-node inspector fix; path-COPY switched to the code-block icon.
- Terminal manual page (terminal.md: cs family + pokes + survey + MCP).
- Survey-through-queue: closed as a no-op (already isolated).

## Pending (carryover into the release tail / post-break)

- @@LaneB: Team Work bubble full-delete + lead-identity-via-queue (in flight at
  the break); terminal tab-nav fix (Alt+Shift+[/] swallowed by the PTY).
- @@LaneE: terminal.md sections 6 (Rich Prompt) + 7 (queue) - deferred to
  post-break to finalize against the settled code.
- Release: final :8787 validation of the finished Team Work + Drafts Rich
  Prompt, unified version bump to 0.24.0, full pre-push gate (all workspaces),
  publish=false dry-run, tag.

## Highlights

- A unifying principle emerged and held: EVERY input to a terminal/agent (cs
  terminal write, Rich Prompt, Team Work, and the question of survey replies)
  flows through ONE serialized per-session queue. The feature set converged on
  a clean architecture rather than a pile of one-offs.
- The Drafts-backed Rich Prompt reframe (@@Host) dissolved the image-paste
  problem: instead of base64/per-agent-path hacks, pasted images are real files
  in a Drafts folder that any agent reads via MCP/disk. The best design win of
  the round, and it came from grounding the premise rather than building it.
- Design-first repeatedly prevented bad builds: @@LaneA's survey finding
  (already isolated -> no-op, avoided a survey-breaking async redesign),
  @@LaneB's image finding (base64 not agent-consumable -> the Drafts reframe),
  and the read-only Team Work classification that absorbed every re-scope
  without wasting code.
- Gate hygiene caught real stale tests (blocklist + carousel source-pin tests)
  via the isolate-gate (committed-state, immune to peer WIP) + full vitest.
- The md5-anchored freshness check on the rebuilt :8787 caught a false
  "stale bundle" alarm (pipe truncation on a 1.5MB stream), preventing a
  confabulated bug report.

## Lowlights

- The Team Work scope churned about four times (whole-GUI-delete -> bubble-only
  -> preserve+decouple+tie-to-lead -> full-delete + lead-is-normal). It cost
  real cycles and several @@LaneB holds. No wasted code (read-only classify
  first), but a lot of dispatch turbulence.
- @@LaneD's own-gate skipped vitest twice (blocklist + carousel), both caught
  by the isolate-gate. Own-gates must run the full `make web-check`.
- A leftover lane server (:7841) caused @@Host a false "unknown variant
  prompt" bug; multiple concurrent test servers created avoidable confusion.

## Feedback: the agents

- @@LaneA: exemplary design-first grounding (queue design, survey finding).
  Refusing to build survey-B blindly was exactly right.
- @@LaneB: carried the hardest, most-churned surface (Rich Prompt + Team Work)
  and classified read-only before any deletion, so the re-scopes never cost
  code. Strong grounding (image finding, the primeTeamWork-only-delivery catch
  that would have stranded the lead).
- @@LaneC: clean, well-scoped inspector + About-slide work; good FileInfoBody
  parity.
- @@LaneD: solid dashboard work; tighten the own-gate to full web-check so the
  vitest-skip pattern does not recur.
- @@LaneE: well-grounded docs, especially insisting on clap-source grounding
  over a stale installed binary for the terminal page.

## Feedback: @@Host (Alex)

- The Drafts-backed insight was the round's high point: that kind of
  architectural reframe is where the most value came from, and it turned a
  messy per-agent image hack into a clean files+MCP design.
- The Team Work model was reframed four times live. The final model is clean
  and correct, but settling it before dispatch ("the bubble IS the old rich
  prompt, delete it; the lead is just a terminal that bootstraps a team")
  would have saved three re-scopes. Real-time exploration is fine; a beat to
  lock the model before it hits the lanes would reduce churn.
- Validating on a stale lane server produced a false bug. The metadata-id in
  the terminal path is the tell for which server/drive is in front of you.

## Feedback: the architect (@@Lead, me)

- I over-committed to each interpretation of the Team Work scope instead of
  surfacing the instability early. When "preserve the old bubble" arrived right
  after I had mapped a full deletion, I should have flagged the contradiction
  and asked one crisp either/or, rather than re-scoping reactively message by
  message. I also carried a wrong mental model (a separate "Team Work bubble
  composer") that @@Host had to correct outright.
- Recurring operational slip: I launched watchers/gates with `&`/disown
  (untracked) out of habit despite knowing run_in_background is required.
  Caught and corrected each time, but it is a pattern to kill.
- What worked: the gate -> rebuild -> md5-verify pipeline held; design-first
  routing prevented bad builds; the lean-poke-bus + append-only event channels
  kept five lanes collision-free across a very long, mutating feature stream.

## Process notes (for next round)

- Design-first for anything with a premise risk was the round's MVP. Keep it as
  the default for "this should work seamlessly / everywhere" style asks - the
  premise is often where the bug is.
- The isolate-gate (committed-state, peer-WIP-immune) is non-negotiable with
  multiple same-area lanes; pair it with a standing rule that own-gates run the
  full web-check, not a subset.
- When @@Host's model is still moving, lock it with one explicit restatement +
  confirm before dispatching, instead of re-scoping per message.
