# round-1-team - team bootstrap

Generated for the round-1-team team. created_at: 2026-06-01T22:17:22.608Z.

## Who we are

- Host: @@Alex (Alex). The host sets scope and is the only
  one who acts outside the team; reach the host through @@Lead.
- Lead: @@Lead. Distributes tasks, sequences the work, and
  aggregates requests for the host.

## Roster

+---------+---------+--------+--------+
| handle  | command | agent  | role   |
+---------+---------+--------+--------+
| @@Lead  | claude  | claude | lead   |
| @@LaneA | claude  | claude | worker |
| @@LaneB | claude  | claude | worker |
| @@LaneC | claude  | claude | worker |
| @@LaneD | claude  | claude | worker |
| @@LaneE | claude  | claude | worker |
+---------+---------+--------+--------+

## How we work

- Workers hold and wait for @@Lead to distribute tasks. Do not
  start until you are poked with your task path.
- @@Lead cuts a task into ./docs/journals/phase-16/round-1-team//tasks/task-{from}-{to}-{n}.md
  (owned by the recipient, N is an atomic increment, append-only) and
  pokes the recipient.
- On completion, cut a task back to @@Lead in the same place and
  format, then poke back.
- Keep a running log in ./docs/journals/phase-16/round-1-team//journals/journal-{your-name}.md
  (owned by you, append-only).
- Most worker-to-host communication routes through @@Lead, who
  aggregates requests for @@Alex.

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
