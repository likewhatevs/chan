# architect-1: orchestration SKILL initial drop

Owner: @@Architect
Cut by: @@Architect (self-assigned)
Date: 2026-05-19

## Goal

Land the initial drop of the orchestration SKILL at
`docs/agents/orchestration/`. This is the external-user-
facing guide for running chan as an orchestration host:
how to wire watchers, how to write event files atomically,
how to spawn agents through chan-server's HTTP control
channel, how to publish MCP descriptors for external
agents.

Three files initially:

* `docs/agents/orchestration/README.md` — overview +
  navigation.
* `docs/agents/orchestration/atomic-writes.md` — the
  writer-side temp+rename contract with per-language
  examples.
* `docs/agents/orchestration/spawn-protocol.md` — the
  HTTP control channel contract + pre-flight survey
  shape + sample event files.

We'll add an `mcp-discovery.md` later as part of
`systacean-14`'s investigation; cut now would be
premature.

## Relevant links

* Round 2 capacity proposal:
  [../architect/journal.md](../architect/journal.md)
  "2026-05-18 21:00 BST" entry.
* Substrate contracts to document: `systacean-9`
  (watcher), `systacean-11` (event-reply), `systacean-12`
  (spawning), `fullstack-13` / `fullstack-18` (bubble
  UI), `fullstack-19` (reply path).

## Acceptance criteria

### README.md

* 1-2 page overview answering: what is chan as an
  orchestration host, what can external users do with
  it, where do the contracts live.
* Navigation to the per-topic guides.
* Quick-start: "I want to wire my own agent into a
  chan watcher" → atomic-writes.md.
* Quick-start: "I want chan to spawn an agent for me"
  → spawn-protocol.md.

### atomic-writes.md

* The writer-side contract from the engineering
  addendum: every event file written via temp+rename
  in the same directory. Watcher reads once on
  fsnotify, no defensive multi-read.
* Per-language minimal examples:
  * **bash**: `mv "$tmp" "$final"` after
    `printf '%s' "$payload" > "$tmp"`
  * **python**: `os.replace(tmp, final)` after
    writing.
  * **rust**: `std::fs::rename(tmp, final)`.
  * **node/JS**: `fs.renameSync(tmp, final)` or
    `await fs.promises.rename(...)`.
* No-self-loop rule: don't write into a directory you
  also watch (your code may, but chan-server doesn't,
  by design).
* Survey schema reference (link to architect journal
  entry); sample survey JSON; sample survey-reply
  JSON.

### spawn-protocol.md

* Endpoint reference for `POST /api/terminals`,
  `POST /api/terminals/<session>/restart`,
  `DELETE /api/terminals/<session>` (from
  `systacean-12`).
* Auth: bearer token; same as the rest of the API.
* Pre-flight signal: what chan-server matches; how
  the user's UI renders the survey.
* Sample workflow: agent A writes a "spawn @@CodexPair
  with command X" event into a watched dir; user (or
  routing agent) clicks/keys the answer; chan-server
  POSTs `/api/terminals` and the new tab boots.

## Out of scope

* MCP discovery — wait for `systacean-14` to land per-
  agent shapes.
* Tutorial-style examples beyond "minimum reproducible".
  Refer back to the task files for real-world detail.
* Translations / non-English content.

## How to start

1. Draft README.md last (overview easier after the
   per-topic guides exist).
2. atomic-writes.md first — the contract is fully
   spec'd; just write it up.
3. spawn-protocol.md depends on `systacean-12`'s
   endpoint shape — wait for it to land or coordinate
   the doc draft against the task spec.

## Hand-off

Self-owned. Pre-push gate green (markdown only). No
event needed when landed — the SKILL appears in
`docs/agents/orchestration/` and gets referenced from
process.md + bootstrap.md.
