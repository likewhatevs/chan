# new-team-1 - team bootstrap

Generated for the new-team-1 team. created_at: 2026-06-12T09:12:57.944Z.

## Who we are

- Host: @@Alex (Alex). The host sets scope and is the only
  one who acts outside the team; reach the host through @@Lead.
- Lead: @@Lead. Distributes tasks, sequences the work, and
  aggregates requests for the host.

## Roster

+---------------+---------+--------+--------+
| handle        | command | agent  | role   |
+---------------+---------+--------+--------+
| @@Lead        | claude  | claude | lead   |
| @@Chan        | claude  | claude | worker |
| @@ChanDesktop | claude  | claude | worker |
| @@ChanGateway | claude  | claude | worker |
+---------------+---------+--------+--------+

## How we work

- Workers hold and wait for @@Lead to distribute tasks. Do not
  start until you are poked with your task path.
- @@Lead cuts a task into new-team-1/tasks/task-{from}-{to}-{n}.md
  (owned by the recipient, N is an atomic increment, append-only) and
  pokes the recipient.
- On completion, cut a task back to @@Lead in the same place and
  format, then poke back.
- Keep a running log in new-team-1/journals/journal-{your-name}.md
  (owned by you, append-only).
- Worker-to-host communication routes through @@Lead (see
  "Reaching the host" below); workers do not contact @@Alex directly.

## Reaching the host

When a decision needs @@Alex, do NOT survey the host directly from a
worker, and do NOT use a TUI / in-editor survey (AskUserQuestion). Cut the
question to @@Lead (a task, or folded into your completion task).
@@Lead consolidates the open questions and raises a survey to
@@Alex with `cs terminal survey` (a blocking overlay in the host's
window), keeping each survey focused (one decision, up to 4 options) and
batching or sequencing several pending questions rather than firing many
tiny ones:

    cs terminal survey --tab-name=@@Alex --title '<topic>' \
        --option '<a>' --option '<b>' $'<question / context, markdown>'

Every survey also offers @@Alex an `[F]` follow-up (defers with a
paper-trail under new-team-1/followups/) and a Dismiss, so the host can pick
an option, follow up, or drop it; the reply tells @@Lead which.
Prefer `cs terminal survey` over any TUI survey: it blocks in the host's
window and routes the answer back to @@Lead (see `cs terminal survey
--help` for the current flags).

## The poke 1-liner

Pokes are one-line pointers, not fat context. The context lives in the
task file you point to.

    cs terminal write --tab-name=<target> --submit=<target-agent> \
        $'poke from <me>: <1-line>; read <path>'

`--submit=<target-agent>` appends the submit chord the TARGET agent reads,
so the poke fires instead of parking in the compose box. Use the target's
`agent` from the roster above:

- claude: --submit=claude (chord \x1b[27;9;13~)

A shell member is not an agent: drop --submit and the buffer's trailing
newline submits it. Without --submit the poke parks unsubmitted in an
agent's compose box.

## Files

- config.toml    the team config (you may hand-edit; revalidated on reload)
- bootstrap.md   this file (generated from config.toml)
- tasks/         task-{from}-{to}-{n}.md, owned by the recipient, append-only
- journals/      journal-{member}.md, owned by each member, append-only
- followups/     followup-{from}-{to}-{n}.md, owned by the recipient

Task and followup filenames use the bare name (handle without the @@),
e.g. tasks/task-Lead-LaneA-1.md.
