# alex/setup-2.md

Owner: @@Alex
Cut by: @@Architect
Date: 2026-05-18

## Goal

Lock down the Round 2 design space before any task fan-out.
Round 2 is three compounding features (survey protocol,
notification bubbles, agent spawning) plus a Round 1 closeout
step. Decisions here unblock the Round 2 capacity proposal +
task cuts.

## Relevant links

* [../request.md](../request.md) Round 2 section (the three
  feature blocks + the closeout preamble).
* [../architect/journal.md](../architect/journal.md) Plan
  summary now reflects Round 2.
* [../process.md](../process.md) current event protocol +
  "Approving a permission event" section (the substrate the
  survey upgrade builds on).

## Questions

### Q1. Round 1 closeout commit grouping

Wave 1 produced two ready-to-ship pieces in the working tree:

* `systacean-1` — chan open CLI, end-to-end, @@Architect
  APPROVED, waiting on you.
* `fullstack-1` — docked side panes, @@Architect APPROVED,
  walkthrough by @@WebtestA still pending.

Wave 1 also has follow-on work that may land before closeout:
`fullstack-2/3/4`, `systacean-2`, `webtest-a-1/2`,
`webtest-b-1`. Bug/enhancement additions logged today
(activity indicator, MCP auto-discovery, fs-move UX,
terminal-reattach, light-mode contrast) are not yet cut as
tasks and won't be in this closeout unless you say so.

Options:

1. **Tight closeout**: commit `systacean-1` first, then
   `fullstack-1` once @@WebtestA signs off, then patch bump +
   push. Defer everything else (including
   `fullstack-2/3/4`, the new bugs) to a follow-up wave +
   release.
2. **Wide closeout**: hold the release until the entire wave
   1 fan-out lands (all the fullstack-N + systacean-N +
   webtest-N tasks). Single fatter release.
3. **Rolling**: commit each task as it lands and push
   continuously (still gated on your `approved`); cut the
   patch bump at a natural stopping point you call.

* [@@Architect recommends] **(1) tight closeout**. Each piece
  of wave 1 is independently valuable and the chan-open CLI
  unlocks Round-2-style flows in everyday use. Keeping the
  release tight also makes it easy to bisect if a bug
  surfaces.

### Q2. Survey schema

Round 2 protocol: events become numbered surveys. Need to
pick the wire shape that lives in an `event-*.md` file. Two
candidates:

**Option A — Inline numbered list in markdown:**

```markdown
## 2026-MM-DD HH:MM — survey

Question: ...

1. Option A description.
2. Option B description.
3. Option C description (and grant this topic for the rest
   of the session).

Recommendation: 1
```

@@Alex appends `## reply: 1` (or 2/3) to the same file.

**Option B — Structured JSON + markdown narrative:**

A fenced JSON block carries the machine-readable bits; the
prose explains. fsnotify watcher parses the JSON; @@Alex
sees the prose.

* [@@Architect recommends] **(A) inline markdown**. Cheaper
  to read for humans, parseable enough by a regex (`^\d+\. `
  in the survey section, then the `## reply: N` append).
  When fsnotify automation arrives, the parser is ~30 lines.
  Structured JSON is over-engineering for what's effectively
  a 1-of-N choice.

Also confirm the per-topic grant option naming. Proposal:
every survey may include an option phrased
"... and grant subsequent approvals for this topic". When
@@Alex picks it, future events of the same `topic` tag from
the same agent auto-approve without a fresh survey for a
session-bounded window. The agent declares the topic in the
survey header.

### Q3. Survey scope: per-tool vs per-topic vs per-session

Per the request, you want to "optimise for throughput" with
topic-level grants instead of per-tool-execution. Proposing
this tag hierarchy:

* `topic`: a coarse area (e.g., `test-server-setup`,
  `webtest-walkthrough`, `agent-spawn`).
* `scope`: where the grant lasts:
  * `one-shot` — single command, no re-use.
  * `topic-session` — same agent, same topic, current
    session (cleared on agent recycle).
  * `topic-phase` — same agent, same topic, current phase
    (cleared on phase close).

Default scope is `one-shot`; the per-topic grant option
upgrades to `topic-session`. `topic-phase` is rare and
should require an explicit survey option.

Confirm the levels or call out gaps.

### Q4. Notification bubble watcher target(s)

Initial watcher target for testing is
`docs/journals/phase-7/alex/event-*.md`. Open follow-ups:

* Do you want the watcher configurable (any drive
  directory, picked via the "new file"-style dialog with
  completion), or hard-coded to the alex events directory
  initially?
* Bubble dismiss behavior: auto-dismiss after slash-button
  reply, manual dismiss with X, or both?
* If multiple events arrive at once, do bubbles stack (a la
  iMessage) or queue (one visible, rest in a tray)?

* [@@Architect recommends] configurable from day 1 (the
  "new file" picker is cheap to reuse); auto-dismiss after
  reply + manual X; stack when count ≤ 3, then queue with
  a "+N more" affordance.

### Q5. Agent spawn CLI menu

Architecture: @@Architect fires an event to spawn an agent.
That event names the profile (claude / codex / gemini),
which model/mode, which task assignment, and a name for the
new terminal tab.

* Should the spawn-event carry the full CLI command, or
  reference a profile that chan-server knows about? Profile
  references mean we can update the underlying flags
  without rewriting old event files.
* Pre-flight: when an agent comes up requesting auth /
  login / restart, where does that surface? Proposing: a
  bubble notification on the user's rich prompt with the
  raw agent output and a "open the terminal" / "kill" /
  "retry" survey.

* [@@Architect recommends] profile references stored under
  `docs/agents/profiles/{claude,codex,gemini}.toml` with
  the canonical CLI command + default flags. Spawn events
  name the profile + per-spawn overrides.

### Q6. Orchestration SKILL packaging

Where does the "spin up chan with agents" SKILL live?

* `docs/agents/skills/orchestration.md` — sits alongside
  the existing skill guides (architect, webdev, etc.).
* `docs/skills/orchestration.md` — at the docs/ root, since
  it's for *humans setting up chan*, not for the agents
  themselves.
* `docs/agents/orchestration/` — its own subdir with
  multiple files (setup steps, profiles, troubleshooting).

* [@@Architect recommends] `docs/agents/orchestration/` as
  a directory: setup.md, profiles.md, troubleshooting.md.
  The skill content is structured enough that splitting
  helps; readers can land on troubleshooting.md when something
  breaks without scrolling through setup.

### Q7. New bugs / enhancements logged today

Today the request added: activity indicator, MCP
auto-discovery, fs-move UX, terminal-reattach, light-mode
contrast. None of these are cut as tasks yet.

Options for sequencing:

1. Fold the wave-1-able ones (terminal-reattach, light-mode
   contrast, fs-move UX wave-1 bit) into a small **wave 1.5**
   before closeout.
2. Push all of them to Round 2 (after closeout).
3. Skip wave 1.5 and let @@FullStack pick the high-value ones
   up between fullstack-2/3/4 as opportunistic fixes.

* [@@Architect recommends] **(1) wave 1.5** for the three
  wave-1-able items. Terminal-reattach is operationally
  painful (it just killed your agents). Light-mode contrast
  is a 1-hour CSS fix. fs-move UX (the wave-1 wedge: soften
  the i/o error) is also tractable. Activity indicator + MCP
  auto-discovery slide to Round 2.

## Downstream tasks gated

* Round 1 closeout commit plan (gated by Q1).
* Round 2 capacity proposal (gated by Q2–Q7).
* `docs/agents/orchestration/` skill scaffold (gated by Q6).

## How @@Alex replies

Append a section below titled `## 2026-05-18 reply` with
one bullet per Q. Then poke @@Architect via chat (or, once
the survey protocol lands, via the survey reply mechanism
this file is proposing).

Alex's reply:
Q1. tight closeout
Q2. Option B, structured JSON
Q3. Agreed
Q4. Open follow ups:
- Watcher configurable, no hardcode
- Both methods to dismiss - cant we make this always be survey instead? would make it easier
- Can we have a switch for stack and tray? users may want different options here

Q5. spawn
- full cli command for now; not sure i understand the point about old event files
  - the reason im making this choice is to have less setup required; zero-setup=better
- these suggestions is what i was thinking: open the terminal, kill, retry
- timeout for this? need a spinner, time counter, a "retry now"

Q6. SKILL packaging
For us, this is indeed docs/agents/... for others, they will have to pick when they choose to put the watcher on.. maybe in the same dialog where they pick the folder, they are asked the path to their skills - whcih may be outside the drive's root btw, or to setup a new dir in their project

Q7. new bugs
Sequencing:
- accepting your suggestion


