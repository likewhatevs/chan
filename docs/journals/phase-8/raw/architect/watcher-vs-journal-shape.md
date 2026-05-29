# Watcher-vs-journal shape — coordination design gap

Author: @@Architect
Date: 2026-05-21

Status: **design artifact**. Captures the structural split
between today's agent-coordination journals and the chan
runtime watcher's event shape. Drives a decision in the
rich-prompt session-evolution wave
([`rich-prompt-session-evolution.md`](rich-prompt-session-evolution.md))
about how the two shapes converge (or stay separate).

## Surfaced by

Live smoke test 2026-05-21: @@Alex pointed the rich-prompt
watcher at `docs/journals/phase-8/alex/`; @@Architect cut
echo-round-trip pokes to @@FullStackA + @@FullStackB by
appending to `event-architect-fullstack-{a,b}.md`. Watcher
surfaced nothing. Investigation hit two hard structural
mismatches; the smoke test was paused per @@Alex's "if
anything breaks we pause and analyse" directive.

## The two shapes

### Journal shape (today's coordination)

Used by every `event-<from>-<to>.md` under `alex/`.

| Property        | Value                                                  |
|-----------------|--------------------------------------------------------|
| File lifecycle  | Long-lived; one file per directed channel              |
| Mutation        | Append-only; new poke = new dated `## YYYY-MM-DD —` heading |
| Content         | Markdown narrative (free-form prose, bullets, tables)  |
| Read model      | Agents read whole file on bootstrap; "latest message" = bottom |
| Filename        | `event-<from>-<to>.md` (e.g. `event-architect-fullstack-a.md`) |
| Audit value     | Maximum — full conversation history preserved          |
| Growth          | Monotonic per channel (5-30 KB at phase-8 end)          |

Strengths: rich, human-readable, reconstructible. Weaknesses:
not machine-tail-able; no per-message identity; no read-receipts;
grows forever.

### Wire shape (chan runtime watcher today)

Used by `chan-server::event_watcher`, the rich-prompt watcher
dialog, and the survey/bubble surface in the SPA.

| Property        | Value                                                  |
|-----------------|--------------------------------------------------------|
| File lifecycle  | Single-message per file; written once, then static     |
| Mutation        | None (atomic create via tempfile + rename)             |
| Content         | JSON conforming to `AgentEvent` schema                  |
| Read model      | Watcher emits each new file as a dispatch event        |
| Filename        | `event-<id>.{md,json}` or `pre-flight-<id>.{md,json}`  |
| Identity        | `id` field; `SeenEventIds` dedup table on the server   |
| Schema          | `id`, `type` (survey/survey-reply/poke), `from`, `to`, optional `topic`/`questions`/`answers`/`scope`/`note` |

Strengths: machine-routable; per-message identity; survives
SPA reload via `SeenEventIds`; integrates with the bubble
queue + survey-reply round-trip. Weaknesses: no
multi-message coherence (each file stands alone); content is
constrained to the schema; not human-skimmable.

## The mismatch

`chan-server/src/event_watcher.rs` is the integration point:

* **Event-kind filter** (line 134-142): handles
  `EventKind::Create(_)` and `EventKind::Modify(ModifyKind::Name(_))`
  (rename-into). Plain `Modify(Data)` (file append) falls
  through to `None`. Journal appends NEVER fire the watcher.
* **Content filter** (line 48-61, parse path): every fired
  file is parsed as `AgentEvent` JSON. Markdown narrative
  bodies fail the parse, increment `dropped_events`, emit
  warn. Even if a Create fired on a journal file, the
  watcher would warn-and-drop rather than dispatch.
* **Filename filter** (line 224, `is_watcher_event_filename`):
  matches our journal filenames (`event-<from>-<to>.md`)
  because the regex `^(event|pre-flight)-<id>\.(md|json)$`
  treats `<from>-<to>` as a valid `<id>`. So filenames pass
  but content fails — the dropped_events case, not the
  silently-skipped case.

The two shapes co-exist under the same filename prefix +
the same directory + the same watcher regex, but talk past
each other at the event-kind and content layers.

## Why this matters

Two motivating contexts:

1. **Live monitoring of agent coordination by @@Alex** — the
   immediate ask. @@Alex wanted to point the watcher at the
   coordination dir and see agent traffic surface as
   bubbles in the SPA. Today's journals can't drive that.
2. **Eventual rich-prompt + watcher-driven automation**
   (per memory `project_dispatch_is_automation_blueprint`)
   — the phase-8 dispatch shape is the blueprint for the
   automation that follows. If the blueprint can't bridge
   to the runtime watcher, the automation needs either a
   shape change or a bridge layer.

## Resolution options

Three coherent shapes; each is a real piece of work, none
trivial.

### A. Dual-write (journal + wire event per append)

Every architect/agent append to a journal channel ALSO
drops a single-message JSON event file in the same
directory. The journal stays the audit-trail-of-record;
the wire event is the watcher-driven runtime signal.

Pseudo-shape:
```
docs/journals/phase-8/alex/
  event-architect-fullstack-a.md           # journal (append-only)
  event-arch-fa-poke-<id>.json             # wire (one per poke)
  event-arch-fa-poke-<id+1>.json
  ...
```

Pros:
* Zero-touch to existing coordination model.
* Watcher infra already works.
* Dual-write happens at append time (a small helper
  function the architect + agents call instead of raw
  append).

Cons:
* Two source-of-truth surfaces; risk of drift if one path
  fails.
* Wire-event files accumulate (need GC, ties into the
  earlier read-receipt / inbox-LRU-trash discussion).
* Each agent's bootstrap walk learns the helper.

### B. Watcher learns modify+tail-diff

Extend `event_watcher.rs` to handle `Modify(Data)` events
on known journal files: tail the new content since last
emit, parse the new dated heading as a synthetic
`AgentEvent` (poke type, body = heading prose).

Pros:
* No change to existing journal coordination.
* Single source of truth.

Cons:
* Server keeps per-file tail cursors (state in a previously
  stateless watcher).
* Parsing markdown headings into AgentEvent fields is
  lossy + heuristic (id derivation, topic extraction).
* Edge cases: file rotation, partial appends mid-flush,
  multi-agent concurrent appends, restart cursor recovery.
* Bigger watcher footprint; new failure modes.

### C. Channel migration (journals retire, wire events
become the coordination medium)

Drop the markdown journals; agents post one JSON event
file per poke. A renderer (chan-server route or SPA
component) presents the per-channel conversation by
listing + sorting the wire events.

Pros:
* Single shape; clean.
* Watcher-native; agents talk in the wire shape directly.

Cons:
* Loses the human-readable narrative surface (bullets,
  tables, link-to-other-file). Each event is constrained
  to the AgentEvent schema's free-form `note` field.
* Massive churn: every existing channel + the bootstrap
  walk + the architect-skill discipline rewires.
* High risk of regression on coordination clarity. The
  journal narrative IS the architect's working tool.

## Recommendation

**A** is the lowest-risk path that preserves the journal
discipline AND lights up @@Alex's watcher. Pair with the
inbox-LRU-trash discussion from 2026-05-21 (recommended
deferral): the wire events become the inbox+trash surface
(per-message identity makes GC tractable); the journals
stay the archive.

Sequencing fits naturally into the rich-prompt
session-evolution wave (Round-2 wave-2 or wave-3): the
session-evolution work already touches the architect-skill
discipline + the bootstrap step that reads inbound
channels.

## Decisions @@Alex needs to make (when wave-2/3 cuts)

1. **Resolution path**: A (dual-write), B (watcher tail-diff),
   or C (channel migration)? Recommendation: A.
2. **Wire-event GC**: if A lands, ride the read-receipt
   discussion's option-A (read-marker) + option-C
   (LRU-to-trash at phase close) per
   [the read-receipt design exchange 2026-05-21]. Or fresh
   shape.
3. **Sequencing**: in the rich-prompt session-evolution
   wave (where the bootstrap walk is already being
   touched), or earlier as a dedicated mini-task?
4. **Wave-2 architect-discipline update**: if A lands, the
   architect's poke helper has to dual-write. Cut a small
   architect-3 task to ship the helper before the
   FullStack agents adopt it.

## Out of scope here

* Cross-host coordination (phase-9 desktop-native
  territory).
* MCP-server bridging (chan-llm's MCP boundary is
  unchanged by this).
* SPA UI for browsing wire events (separate UX task).

## Cross-references

* [`rich-prompt-session-evolution.md`](rich-prompt-session-evolution.md)
  — the wave where this likely lands.
* `crates/chan-server/src/event_watcher.rs` — the watcher
  source.
* `docs/journals/phase-8/process.md` §"Watcher event-file
  naming convention" — the regex + filter sites.
* `systacean-9` / `systacean-10` — the filename-filter +
  non-matching-skip plumbing.
* Memory `project_dispatch_is_automation_blueprint` —
  the longer-arc motivation.
