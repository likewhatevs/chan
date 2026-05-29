# alex/setup-1.md

Owner: @@Alex
Cut by: @@Architect
Date: 2026-05-18

## Goal

Lock down the phase-7 tidy-up before any feature work starts.
Decisions here unblock the directory reshuffle, the
`docs/agents/` build-out, and the `docs/journals/` migration.

## Relevant links

* [../request.md](../request.md) — round 1 / project hygiene.
* [../process.md](../process.md) — updated for the new
  poke-driven flow + roster.
* [../architect/journal.md](../architect/journal.md) — phase
  journal (still mostly stubs).

## Questions

### Q1. Author directory names

Phase 6 used `backsystacean/`. Phase 7 scaffolded
`backsystacean/` but the request renames the role to
`@@Systacean`. Phase 5 used `systacean/`.

Proposed: rename `backsystacean/` → `systacean/`, add new
`fullstack/`, **delete** the empty scaffolded `frontend/`
directory (`@@Frontend` folds into `@@FullStack`).

* [@@Architect recommends] yes, rename + add fullstack + drop
  frontend.
* Alternative: keep `frontend/` as a stub with a redirect note
  pointing at `fullstack/`.

### Q2. docs/agents roster

The request asks for `docs/agents/{name}/contact.md` so we can
graph phase work. Two scopes:

* **Active roster only**: Architect, FullStack, Systacean,
  WebtestA, WebtestB. Anything older (Backend, Frontend,
  Syseng, Rustacean, Backsystacean, Webtest) gets a note in
  the contact pointing at its successor.
* **Full history**: a contact file per *every* historical agent
  name across phases 1-7, so backfilled journals link to live
  contacts. Each historical contact says "rolled into @@X".

* [@@Architect recommends] full history. Cheap, and makes the
  backfill self-contained when we graph.

### Q3. Skills source

The request points at `~/dev/github.com/fiorix/dotfiles/ai/skills/`.
Five skills there: architect, pythonic, rustacean, syseng,
webdev. Each is a single `guide.md`.

Mapping:

| Agent       | Skills                              |
|-------------|-------------------------------------|
| @@Architect | architect                           |
| @@FullStack | webdev, rustacean, pythonic         |
| @@Systacean | syseng, rustacean                   |
| @@WebtestA  | webdev (browser-driving lane)       |
| @@WebtestB  | webdev (browser-driving lane)       |

* [@@Architect recommends] copy the guides as `skills/{name}.md`
  inside each agent's directory; contact links to them. Copies,
  not symlinks, so the repo is self-contained.
* Confirm the mapping, or call out gaps.

### Q4. Phase 4 gap

`phase-{1,2,3,5,6,7}` exist; phase 4 is
absent. Confirm phase 4 was either skipped or renamed (so the
backfill doesn't go looking for it).

### Q5. docs/journals migration timing

The move of `phase-*` → `docs/journals/` will
break relative links inside the journals + any external
references (e.g., `CLAUDE.md`, `design.md`) that point at them.
We have two windows:

* **Now**: do the migration before round 1 work starts, with
  the working tree clean. Single commit, one big `git mv`
  pass, then link-fix pass.
* **At phase close**: keep `phase-7/` at the
  repo root during the phase; migrate everything in one
  trailing commit so phase-7 internal links don't churn.

* [@@Architect recommends] **now**. Phase 7 just started; the
  journal is empty; the cost is bounded. Doing it later means
  rewriting every phase-7 link too.

### Q6. New bug from chat

Logged as a bullet at the bottom of
[../request.md](../request.md) Bugfixes:

> When opening a doc with images, some thumbnails render and
> some don't (partial render); preference is all-or-nothing.

Confirm phrasing reflects the intent, or amend.

## Downstream tasks gated

* `architect/architect-tidyup-{n}.md` — reshuffle phase-7
  dirs (blocked by Q1).
* `architect/architect-contacts-{n}.md` — stand up
  `docs/agents/` (blocked by Q2, Q3).
* `architect/architect-journals-migration-{n}.md` —
  `docs/journals/` move (blocked by Q4, Q5).

## How Alex replies

Append a section below titled `## 2026-MM-DD reply` with one
bullet per Q. Poke @@Architect when done.

Q1. correct!
Q2. ok full history is fine
Q3. do what you think will work for most agents.. claude, codex, gemini we care about in this order; gemini less so (today, during our dev)
Q4. phase 4 ended up as a bug bounty + random updates.. i have the files in ~/Documents/ChanRoadmap/phase-4/ but it's not the whole story.. you can do your best to import some of it; we also dont need such long directory names since we're in a more contained namespace under ./docs now
Q5. now, yes.. although this is mostly important to you, not to everyone else; maybe you can do the previous ones in background after you start cutting tasks to the team.. they are idling 
Q6. include the ones ive reported yes, tks

One extra thing about the process: when you poke me, you can also request that the agent's context is cleared; what i want is to optimise the agents for execution and thus completing phases should allow us to recycle the agent, bring in a fresh new agent that can take a handover journal and start fresh new technical work; you should tell me when to do that

we need a small protocol for how we're going to use these pokes, or perhaps we call them events instead?

later i will want to put an fsnotify in the dir where we place these event-{from}-{to}.md and start building automation.. for example: the event may be from anyone to me, but ideally from anyone to you, and from you to me most of the time - except for permissions on their terminal, permissions on the test chrome browser they're launching, etc; still, try to prep properly all through you, msot of the time.. when the requests come to me we need to establish what kind of request: poke (from A to B, no extra context), agent recycle (which agent); later we can think of capacity request

