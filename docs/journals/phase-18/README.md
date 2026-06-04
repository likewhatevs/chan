# Phase 18

Opened after v0.25.0 (the phase-17 close). Phase-18 lands @@Alex's
v0.26.0 TODO: editor list/scroll/`[[` fixes, graph fixes + copy-link, a
file-browser context-menu + shortcut-hint pass, the inspector pill
redesign, terminal focus/copy-paste/UTF-8 fixes, the chan-desktop
pre-flight removal, and a repo/docs cleanup that consolidates the journals
into docs/phases/phase-N.md.

## round-1/

@@Alex's round-1 draft (moved here from ./dev): the v0.26.0 bug fixes,
enhancements, the inspector redesign, and the repo cleanup.

- draft.md          the round-1 spec (was dev/TODO.md)
- image*.png        screenshots referenced inline by draft.md

## team/

The Team Work bus for the session, ready to launch as a dedicated lead +
6 worker lanes (7 agents).

- config.toml       7-member team (@@Lead + @@LaneA..F), 4x2 layout
- bootstrap.md      the team PROCESS, server-generated (the canonical
                    template, incl. the `cs terminal survey` channel to
                    @@Alex). Tool-owned; do not hand-edit.
- round-1-plan.md   the round WORK: lane assignments, owned files,
                    shared-file rules, waves, gate. The Lead's playbook.
- tasks/ journals/ followups/   empty; filled live during the round.

Launch the round with:

    cs terminal team load docs/journals/phase-18/team

`load` reads config.toml, spawns lead-first, and pokes each agent to read
bootstrap.md (identity + process). It does NOT regenerate bootstrap.md, so
the placed template stays put. Then point the lead at the work:

    cs terminal write --tab-name=@@Lead --submit=claude \
        $'poke from @@Alex: read docs/journals/phase-18/team/round-1-plan.md and dispatch round-1'

(bootstrap.md is the generated process doc and has no round-plan pointer;
this one poke hands the lead the playbook. Workers then wait for the
lead's task pokes.)

Use `load`, not `new`: `new` regenerates bootstrap.md from config.toml
(identical here, but it also spawns immediately). The round plan lives in
round-1-plan.md, NOT inside bootstrap.md, so a regenerate cannot wipe it.
