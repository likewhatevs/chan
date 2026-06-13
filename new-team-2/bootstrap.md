# new-team-2 - team bootstrap

Generated for the new-team-2 team. created_at: 2026-06-12T19:54:54.700Z.

## Who we are

- Host: @@Alex (Alex). The host sets scope and is the only
  one who acts outside the team; reach the host through @@Conductor.
- Lead: @@Conductor. Distributes tasks, sequences the work, and
  aggregates requests for the host.

## Roster

+---------------+---------+--------+--------+
| handle        | command | agent  | role   |
+---------------+---------+--------+--------+
| @@Conductor   | claude  | claude | lead   |
| @@Editor      | claude  | claude | worker |
| @@PromptQueue | claude  | claude | worker |
| @@TeamFlow    | claude  | claude | worker |
| @@Desktop     | claude  | claude | worker |
| @@CtxPass     | claude  | claude | worker |
+---------------+---------+--------+--------+

## How we work

- Workers hold and wait for @@Conductor to distribute tasks. Do not
  start until you are poked with your task path.
- @@Conductor cuts a task into new-team-2/tasks/task-{from}-{to}-{n}.md
  (owned by the recipient, N is an atomic increment, append-only) and
  pokes the recipient.
- On completion, cut a task back to @@Conductor in the same place and
  format, then poke back.
- Keep a running log in new-team-2/journals/journal-{your-name}.md
  (owned by you, append-only).
- Worker-to-host communication routes through @@Conductor (see
  "Reaching the host" below); workers do not contact @@Alex directly.

## Reaching the host

When a decision needs @@Alex, do NOT survey the host directly from a
worker, and do NOT use a TUI / in-editor survey (AskUserQuestion). Cut the
question to @@Conductor (a task, or folded into your completion task).
@@Conductor consolidates the open questions and raises a survey to
@@Alex with `cs terminal survey` (a blocking overlay in the host's
window), keeping each survey focused (one decision, up to 4 options) and
batching or sequencing several pending questions rather than firing many
tiny ones:

    cs terminal survey --tab-name=@@Alex --title '<topic>' \
        --option '<a>' --option '<b>' $'<question / context, markdown>'

Every survey also offers @@Alex an `[F]` follow-up (defers with a
paper-trail under new-team-2/followups/) and a Dismiss, so the host can pick
an option, follow up, or drop it; the reply tells @@Conductor which.
Prefer `cs terminal survey` over any TUI survey: it blocks in the host's
window and routes the answer back to @@Conductor (see `cs terminal survey
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
e.g. tasks/task-Lead-Alice-1.md.
