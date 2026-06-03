# new-team-1 - team bootstrap

Generated for the new-team-1 team. created_at: 2026-06-02T21:00:47.863Z.

## Who we are

- Host: @@Alex (Alex). The host sets scope and is the only
  one who acts outside the team; reach the host through @@LaneA.
- Lead: @@LaneA. Distributes tasks, sequences the work, and
  aggregates requests for the host.

## Roster

+---------+---------+--------+--------+
| handle  | command | agent  | role   |
+---------+---------+--------+--------+
| @@LaneA | claude  | claude | lead   |
| @@LaneB | claude  | claude | worker |
| @@LaneC | claude  | claude | worker |
| @@LaneD | claude  | claude | worker |
+---------+---------+--------+--------+

## How we work

- Workers hold and wait for @@LaneA to distribute tasks. Do not
  start until you are poked with your task path.
- @@LaneA cuts a task into new-team-1/tasks/task-{from}-{to}-{n}.md
  (owned by the recipient, N is an atomic increment, append-only) and
  pokes the recipient.
- On completion, cut a task back to @@LaneA in the same place and
  format, then poke back.
- Keep a running log in new-team-1/journals/journal-{your-name}.md
  (owned by you, append-only).
- Most worker-to-host communication routes through @@LaneA, who
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
