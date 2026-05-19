# Phase 7 journal

Author: @@Architect
Date: 2026-05-18

Canonical phase-wide journal per
[../process.md](../process.md). Carries the plan summary,
request checklist, capacity proposal, dispatch table, decisions
log, and the extended-requests trail.

Append-only. New entries go at the bottom under a dated heading.
Corrections are new appends with a back-link, not rewrites.

## Plan summary

Phase 7 has two rounds:

1. **Round 1 — Maintenance.** Project hygiene (this phase's
   process tidy-up, `docs/journals/` migration, `docs/agents/`
   contacts), enhancements (file-browser side panes, unified
   style toolbar, terminal-menu parity), and a wide bugfix wave
   across editor, list/image interactions, find/index UX,
   panes, terminal shortcuts, and broadcast.
2. **Round 2 — Features.** Three feature lanes plus a round-1
   closeout step:
   * **Closeout**: commit wave-1 work in order, bump patch
     version, push. Local pre-push gate (macOS) substitutes
     for CI until keys land.
   * **Survey-style protocol**: @@Architect/agents fire events
     to @@Alex as numbered surveys (option 1, 2, 3...). @@Alex
     picks one, appends reply, pokes back. Surveys must
     include a "grant subsequent approvals for this topic"
     option so we trade per-tool-execution friction for
     topic-scoped throughput.
   * **Notification bubbles over the rich prompt**: extend the
     rich prompt with a watcher on a chosen directory
     (initially `docs/journals/phase-7/alex/event-*.md`).
     WhatsApp-style chat bubbles: @@Alex's messages left,
     into the terminal; agent notifications floating over the
     top with text + links + slash-button to
     approve/reply/deny against the survey. Hiding the prompt
     hides the whole overlay. Echoes the spirit of the old
     Assistant OverlayShell.
   * **Programmatic agent spawning**: @@Architect (via events)
     can create + name terminal tabs, restart to pick up the
     name, and execute an agent CLI (claude / codex /
     gemini) with profile flags. Agents run with full
     permission; gating runs through the survey protocol.
     Pre-flight handles auth / login / first-run quirks.
     Packaged as a `docs/agents/skills/orchestration.md` SKILL
     so the setup is reusable by anyone running chan.

The three Round 2 features compound: the survey protocol is
the substrate, the bubble overlay is the human-facing surface
for it, and agent spawning is what fills the protocol with
agents to gate. Round 2 starts after Round 1 closeout
ships.

## Request checklist

### Round 1 / Project hygiene

* [x] New process: @@Architect cuts tasks to @@Alex in
  `alex/{task}-{n}.md`, poke-driven, no @@Alex-to-@@Architect
  task channel.
* [x] Inter-agent pokes via `alex/poke-{from}-{to}.md`.
* [x] Phase-7 roster reshuffle: @@FullStack (Backend+Frontend),
  @@Systacean (Syseng+Rustacean), webtest-a/b retained.
  Directories renamed accordingly. `frontend/` dropped.
* [x] `docs/agents/` standup: contact cards for the active
  roster, skill guides copied from `~/dev/github.com/fiorix/dotfiles/ai/skills/`.
* [x] `docs/journals/` migration: all `phase-*`
  dirs moved under `docs/journals/`, single external link
  fixed in phase-7 process.md.
* [ ] Date: 2026-05-18 backfill across all phase-7 journals
  (architect, alex, webtest-a, webtest-b, fullstack, systacean
  all green; older-phase journals out of scope per pragmatic
  read, pending @@Alex confirmation in setup-1).
* [ ] `@@{name}` text normalization in older-phase journals
  (deferred, pending @@Alex on backfill scope in setup-1).

### Round 1 / Enhancements

* [ ] File browser side-pane (stick to left/right) inspired by
  GitHub, with overlay still available.
* [ ] Cmd+F when Find buffer is already open: re-focus / toggle,
  don't no-op.
* [ ] Unified style toolbar across file editor and rich prompt;
  icon set audit; include `<hr>`; external links open in
  system browser; preview bubbles if achievable.
* [ ] Terminal menu (split left/right/settings) styled like the
  file menu (sections + separators, matching item order).
* [ ] Terminal name-change indicator when renamed without
  restart; restart-button confirmation prompt.
* [ ] `chan open <path>` CLI from inside a chan-spawned
  terminal: opens a file tab for `.md` (create if missing) or
  the file browser for everything else. Discovery via
  `$CHAN_TAB_NAME` / `$CHAN_DRIVE_NAME` (or session id) so the
  call targets *this* window's chan-server; shell completion
  desirable.
* [ ] Activity indicator on terminal tabs: visual cue when a
  terminal has produced output since last focus; clears on
  focus. Wave 2.
* [ ] Pane menu reorg: move Reload + toggle-web-inspector
  from the right-click pane menu to the pane hamburger;
  move Split + Close into the right-click menu. Pairs with
  B15 (left-click should not open the right-click menu).
  Wave 1.5 candidate (small @@FullStack task).
* [ ] Pane focus border color option (per-pane, persisted):
  switch between blue (default), green, pink. Lives in the
  same right-click menu as Split + Close. Wave 1.5; folds
  into the pane menu reorg task above.
* [ ] Next / previous pane navigation: menu entries + shortcuts
  `Cmd+]` / `Cmd+[` on native, `Cmd+Alt+]` / `Cmd+Alt+[` on
  web (browsers reserve `Cmd+[/]` for back/forward). Same
  native-vs-web detection pattern as `Cmd+T`. Folds into the
  pane menu reorg task.
* [ ] MCP auto-discovery for external agents: publish our
  MCP server under the standard (un-prefixed) env / config
  surfaces each agent reads (claude, codex, gemini).
  Constraints: coexist with existing user MCP setup (append,
  don't overwrite); don't be left out (verify the descriptor
  lands where the agent actually reads). Wave 2; depends on
  systacean-1's env design landing first to share the env-
  export seam.

### Round 1 / Bugfixes

* [ ] Shift+Tab outside lists steals focus to pane hamburger
  (block when not in a list).
* [ ] Image paste inside a list jumps to BOL of next line;
  should append a single space after the image; Enter without
  using the space should drop the space.
* [ ] Find menu additions: highlight trailing space, toggle
  code blocks (markdown only), remove trailing space (with
  on-save / on-auto-save tick).
* [ ] `[[`-completion: show "indexing..." state instead of
  silence / "No matches".
* [ ] `![`-image search: same indexing state + empty-state
  text + spinner.
* [ ] "Empty search, type something" prompt for blank queries.
* [ ] Eventual write timeouts ("failed to write after 10s") on
  small `.md` files; the indexer should never block writes.
* [ ] Spurious cursor jumps observed while typing in `request.md`.
* [ ] Source / rendered switch: place cursor correctly
  (image-aware mapping in both directions).
* [ ] No-matches view: indicate "searched N documents" / spinner
  while indexing, above the separator.
* [ ] Rich prompt: right-click menu with toggle source, toggle
  style toolbar, prompt width; remove toggle-source from the
  visible toolbar; add "Link to File" that drops `read
  [[link]]` into the buffer.
* [ ] End-of-page typing scrolls unexpectedly.
* [ ] Typing on a list moves cursor before the marker.
* [ ] Doc/terminal tab switch leaves editor tab blank until
  click / cursor move.
* [ ] Left-click on empty pane opens right-click menu; should
  only select (right-click only for menu); breaks tab D&D.
* [ ] `Cmd+\`` clashes with macOS window cycle; switch to
  `Cmd+T` on chan.app native + `Cmd+Alt+T` on the web
  variant (browsers reserve `Cmd+T`). Bind both on native
  so muscle memory carries over.
* [ ] `Cmd+Shift+I` mute toggle should always toggle all tabs,
  preserving per-tab MUTE on subsequent edits.
* [ ] Broadcast-icon menu item: can't click mute (incomplete
  observation in source).
* [ ] Doc images: partial render -> all-or-nothing
  (logged via chat on 2026-05-18).
* [ ] Markdown tables don't render in the editor (empty space
  + chevrons visible, content below may also be affected); live
  repro in `alex/setup-1.md` Q3 (logged via chat on
  2026-05-18).
* [ ] External fs moves of open files surface raw "i/o error
  file not found" in the tab; detect file-moved-while-open
  (inotify-equivalent on path / inode), auto-follow if
  unambiguous, otherwise show "moved or deleted" state with
  re-open / find / close affordances. Long-term: agents
  route moves via MCP (see auto-discovery enhancement) so we
  never lose visibility. Wave 1 for the UX state; the
  inode-follow can defer to wave 2.
* [ ] Terminal reattach after browser reload: PTY sessions
  go silent (disabled input, no output) after a chan.app
  reload; only menu "Restart" recovers (PTY reset, too
  heavy). WebSocket reconnect must re-attach to the live PTY
  session. Suspect BCAST/mute state ties into the
  input-enable path (related to B17/B18). Wave 1.
* [ ] Light-mode terminal: lighter glyphs lack contrast.
  Bump foreground contrast in light mode only. Wave 1.
* [ ] "Graph from here" on a directory returns empty
  (0/4 nodes, 0/3 edges) with no errors / no progress
  indication despite Details showing 246 files. Either
  fix the scope-filter mismatch surfacing 0 nodes, OR
  surface state ("indexing… N of M" with spinner / "no
  matches for scope X"). Same empty-state ambiguity
  family as B3/B4/B8/B9. Wave 1.

## Capacity proposal

Round 1 work envelope: 5 enhancements + 1 new CLI feature + 21
bugfixes = 27 work items across editor, terminal, panes, find/
index UX, and a new CLI surface.

### Slot map

| Agent       | Slots | Scope                                                              |
|-------------|-------|--------------------------------------------------------------------|
| @@FullStack | 1     | Owns the entire editor / terminal / panes / find-UX implementation.|
| @@Systacean | 1     | `chan open` CLI; write-timeout investigation; project hygiene BG.  |
| @@WebtestA  | 1     | Walkthrough lane: file browser + editor + find/index.              |
| @@WebtestB  | 1     | Walkthrough lane: terminal + broadcast + panes + shortcuts.        |

### Initial fan-out (round 1, wave 1)

| Task              | Owner       | Wraps                                                |
|-------------------|-------------|------------------------------------------------------|
| fullstack-1       | @@FullStack | File browser side panes (E1).                        |
| fullstack-2       | @@FullStack | Unified style toolbar (E2).                          |
| fullstack-3       | @@FullStack | Find UX upgrade (B3+B4+B8+B9+B10+B21).               |
| fullstack-4       | @@FullStack | List + image interaction bugs (B1+B2+B13).           |
| systacean-1       | @@Systacean | `chan open` CLI (E5) incl. env design + completion.  |
| systacean-2       | @@Systacean | Write-timeout investigation (B5).                    |
| webtest-a-1       | @@WebtestA  | Baseline walkthrough: file browser + editor + find.  |
| webtest-b-1       | @@WebtestB  | Baseline walkthrough: terminal + broadcast + panes.  |

Deferred to wave 2 (cut once wave 1 lands):

* E3 terminal menu styling, E4 terminal rename indicator.
* B6 cursor jumps, B7 source/rendered cursor mapping, B11
  rich-prompt right-click menu, B12 EOL scroll, B14
  doc/terminal tab blank, B15 left-click pane menu, B16
  cmd+\` rebind, B17 cmd+shift+I toggle all, B18 BCAST mute,
  B19 image partial render, B20 markdown tables.

### Background work (parked behind fan-out)

| Task     | Owner       | Scope                                              |
|----------|-------------|----------------------------------------------------|
| BG-rename| @@Systacean | `git mv` phase dirs from `chan-pre-release-phase-N` to `phase-N`, ~386 wiki-link |
|          |             | rewrites in bulk.                                  |
| BG-p4    | @@Architect | Import phase-4 material from ChanRoadmap.          |
| BG-hist  | @@Architect | Historical agent contact cards (full history).     |

### Capability assumptions

* @@FullStack carries Svelte / TS / axum / chan-server route
  fluency; can cross over into `chan_drive` for filesystem-
  facing seams; consults @@Systacean for clippy / dependency
  questions.
* @@Systacean drives the CLI subcommand layer in `crates/chan`
  and the indexer side of chan-drive; owns the pre-push gate.
* Webtest lanes drive Chrome via the `mcp__claude-in-chrome__*`
  tools and never edit code; findings come back as task-file
  appends.

### Handoffs

* @@FullStack lands a feature → tags @@WebtestA or @@WebtestB
  for walkthrough → @@Systacean reviews if Rust quality / CI
  surface changes.
* @@Systacean lands a CLI / indexer fix → @@FullStack
  integrates if the frontend needs to react (env wiring for
  `chan open`).

## Dispatch

(populated as tasks fan out)

## Decisions log

(append-only record of decisions made with @@Alex, mirrored from
[../alex/](../alex/) task files)

### 2026-05-18 — setup-1 decisions

Source: [../alex/setup-1.md](../alex/setup-1.md).

* **Q1.** Directory reshuffle confirmed: `backsystacean/` →
  `systacean/`, add `fullstack/`, drop `frontend/`. Already
  landed in the working tree.
* **Q2.** Contact roster: **full history**. Backfill historical
  contacts (`@@Backend`, `@@Frontend`, `@@Syseng`,
  `@@Rustacean`, `@@Backsystacean`, `@@Webtest`) as redirect
  cards under `docs/agents/`. Background after fan-out.
* **Q3.** Skill mapping: keep my proposal (Architect=architect;
  FullStack=webdev+rustacean+pythonic; Systacean=syseng+rustacean;
  Webtest A/B=webdev). Optimize for claude > codex > gemini in
  that priority order; guides stay portable since they're plain
  markdown.
* **Q4.** Phase-4 exists at
  `~/Documents/ChanRoadmap/chan-pre-release-phase-4/` (bugs.md
  + process.md). Treat as a "bug bounty + random updates"
  collection, not a full phase. Import what's coherent into
  `docs/journals/phase-4/` (using the shortened naming below).
* **Q5.** Migration to `docs/journals/` was correct; do follow-on
  cleanup (rename phase dirs from `chan-pre-release-phase-N` to `phase-N`, wiki-link rewrite) in
  background after the team has tasks. Team is idling so
  fan-out comes first.
* **Q6.** Bug phrasings (partial image render, table render,
  chan-open CLI) all confirmed.

### 2026-05-18 — directory naming

Use `phase-N` under `docs/journals/`, not the legacy
`phase-N`. The prefix was a top-level
disambiguator that's no longer needed inside the journals
namespace. Mechanical rename + ~386 wiki-link rewrites — see
[BG] task #10.

### 2026-05-18 — event protocol

Pokes become **events** under `alex/event-{from}-{to}.md`.
Per-pair append-only logs. Types: `poke`, `agent-recycle`,
`permission`, `capacity` (reserved). Most events route through
@@Architect; `permission` events go direct to @@Alex for
interactive grants (terminal / browser launch).

@@Architect signals agent recycling via the `agent-recycle`
event type: outgoing agent writes a handover entry to its
journal, @@Architect fires the event with the target agent
name + link to the handover, @@Alex closes/reopens that
agent's session against the same profile. Detail in
[../process.md](../process.md) under "Agent-recycle protocol".

Forward-looking: @@Alex plans to attach an fsnotify watcher on
`alex/event-*.md` for automation. The typed-event schema
above is the contract that watcher will dispatch on.

### 2026-05-18 — permission approval mechanics

A permission event counts as approved in writing when:

1. @@Alex appends a section starting with "approved", OR
2. @@Architect appends a section titled "approved
   (transcribed by @@Architect)" with a chat timestamp and a
   note that @@Alex approved verbally.

Option (2) lets @@Alex grant verbally without context-
switching into the file; @@Architect handles the audit trail.
Detail in [../process.md](../process.md) under "Approving a
`permission` event".

### 2026-05-18 — wave 1 sign-offs

| Task          | Status                                                | Sign-off                                    |
|---------------|-------------------------------------------------------|---------------------------------------------|
| fullstack-1   | APPROVED, walkthrough queued (webtest-a-2)            | [fullstack-1.md](../fullstack/fullstack-1.md) |
| systacean-1   | proposal APPROVED with 4 amendments; impl proceeding  | [systacean-1.md](../systacean/systacean-1.md) |
| webtest-a-1   | permission approved in writing; baseline in progress  | [event-webtest-a-alex.md](../alex/event-webtest-a-alex.md) |
| webtest-b-1   | permission approved in writing; baseline in progress  | [event-webtest-b-alex.md](../alex/event-webtest-b-alex.md) |

New wave-1.5 task cut: webtest-a-2 (side-pane walkthrough),
runs after webtest-a-1.

Future: fullstack-5 (window_command frontend handler for
chan open) will be cut once @@Systacean finalizes the wire
JSON.

## Extended requests (mid-phase additions)

| Date       | Source           | Ask                                          | Logged at                                  |
|------------|------------------|----------------------------------------------|--------------------------------------------|
| 2026-05-18 | Chat (Alex)      | Partial image render -> all-or-nothing.      | [../request.md](../request.md) Bugfixes    |
| 2026-05-18 | Chat (Alex)      | `chan open <path>` CLI from spawned terminal,| [../request.md](../request.md) Enhancements|
|            |                  | env-driven window/drive scoping.             |                                            |
| 2026-05-18 | Chat (Alex)      | Markdown tables don't render; repro in       | [../request.md](../request.md) Bugfixes    |
|            |                  | `alex/setup-1.md` Q3.                        |                                            |
| 2026-05-18 | Chat (Alex)      | Activity indicator on terminal tabs (active  | [../request.md](../request.md) Enhancements|
|            |                  | vs idling). Wave 2 @@FullStack.              |                                            |
| 2026-05-18 | Chat (Alex)      | External fs moves -> "i/o error" UX is raw;  | [../request.md](../request.md) Bugfixes    |
|            |                  | detect + soften, inode-follow if possible.   |                                            |
| 2026-05-18 | Chat (Alex)      | MCP auto-discovery: publish under standard   | [../request.md](../request.md) Enhancements|
|            |                  | env/config surfaces; coexist additively.     |                                            |
| 2026-05-18 | Chat (Alex)      | Terminal sessions go silent after browser    | [../request.md](../request.md) Bugfixes    |
|            |                  | reload; menu Restart recovers. Suspect       |                                            |
|            |                  | BCAST/mute. Wave 1.                          |                                            |
| 2026-05-18 | Chat (Alex)      | Light-mode terminal contrast bump for        | [../request.md](../request.md) Bugfixes    |
|            |                  | lighter glyphs. Wave 1.                      |                                            |
| 2026-05-18 | Chat (Alex)      | Pane menu reorg: Reload+inspector to         | [../request.md](../request.md) Enhancements|
|            |                  | hamburger; Split+Close to right-click.       |                                            |
| 2026-05-18 | Chat (Alex)      | Per-pane focus-border color option (blue/    | [../request.md](../request.md) Enhancements|
|            |                  | green/pink). Folds into pane menu reorg.     |                                            |
| 2026-05-18 | Chat (Alex)      | Next/Prev pane: menu + Cmd+]/Cmd+[ (native), | [../request.md](../request.md) Enhancements|
|            |                  | Cmd+Alt+]/Cmd+Alt+[ (web).                   |                                            |
| 2026-05-18 | Chat (Alex)      | "Graph from here" on directory returns       | [../request.md](../request.md) Bugfixes    |
|            |                  | empty silently. Fix + surface state.         |                                            |

## 2026-05-18 — tidy-up landed

* Wrote the new [../process.md](../process.md) encoding the
  poke-driven @@Architect ↔ @@Alex flow, the
  `alex/poke-{from}-{to}.md` inter-agent channel, and the new
  roster (@@FullStack, @@Systacean, @@WebtestA, @@WebtestB).
* Phase-7 dirs reshuffled: `backsystacean/` → `systacean/`,
  `frontend/` dropped, `fullstack/` created. Journals carry
  `@@{name}` + Date: 2026-05-18 headers.
* `docs/agents/` stood up with active-roster contact cards
  ([architect](../../../agents/architect.md),
  [fullstack](../../../agents/fullstack.md),
  [systacean](../../../agents/systacean.md),
  [webtest-a](../../../agents/webtest-a.md),
  [webtest-b](../../../agents/webtest-b.md)) plus
  skill copies under each `skills/` subdirectory.
* `docs/journals/` migration executed (`git mv` x 180 file
  paths); the only ascending link inside phase 7
  (`process.md` -> `docs/agents/`) was rewritten to
  `../../agents/`.
* Open ask filed: [../alex/setup-1.md](../alex/setup-1.md) with
  six gating questions (roster names, contact-roster scope,
  skill mapping, phase-4 confirmation, migration timing, bug
  phrasing).
* Proceeding with my recommendations in place. @@Alex can
  amend in `setup-1.md` and the working tree if any call needs
  reversing.

## 2026-05-18 — setup-2 decisions

Source: [../alex/setup-2.md](../alex/setup-2.md).

* **Q1.** Round 1 closeout: **tight** — commit `systacean-1`
  first, then `fullstack-1` post-walkthrough, then patch bump
  + push. Deferred: everything else.
* **Q2.** Survey schema: **Option B / structured JSON**
  (against my recommendation). Reason given: future
  automation friendlier. Process amendment needed —
  documented as a follow-up.
* **Q3.** Survey scope levels confirmed:
  `one-shot` / `topic-session` / `topic-phase`. Default
  `one-shot`; per-topic option upgrades to `topic-session`.
* **Q4.** Bubble UX:
  * Watcher **configurable** (no hardcoded paths).
  * Dismiss model **simplified**: every survey carries a
    "skip / not now" option, so reply IS the dismiss. No
    separate X close.
  * Stack vs tray: **user preference switch** (not a fixed
    choice).
* **Q5.** Agent spawn: **full CLI command in the spawn
  event** (against my profile-reference recommendation).
  Reason: zero-setup is better for now. Pre-flight survey
  options confirmed: open terminal / kill / retry. Add a
  timeout + spinner + time counter + explicit "retry now"
  affordance.
* **Q6.** Orchestration SKILL: for our team,
  `docs/agents/orchestration/...`. For external users, the
  watcher-config dialog also asks them where their skills
  live (may be outside the drive root) or offers to set up
  a new dir in their project.
* **Q7.** Wave 1.5 accepted for terminal-reattach +
  light-mode contrast + fs-move UX wedge. Activity indicator
  + MCP auto-discovery slide to Round 2.

## 2026-05-18 — wave-1 mid-status

* `systacean-1` (chan open CLI): **commit-cleared**
  architect-side; awaiting @@Alex authorization.
* `systacean-2` (write-timeout fix): **commit-cleared**
  architect-side; awaiting @@Alex authorization.
* `fullstack-1` (docked side panes): **commit-cleared**
  architect-side after @@WebtestA's walkthrough; awaiting
  @@Alex authorization.
* `fullstack-2` (style toolbar): code review APPROVED,
  walkthrough needed (cut `webtest-a-3` to cover external-
  link routing).
* `fullstack-3` (Find UX), `fullstack-4` (list/image bugs):
  queued; @@FullStack moves to them now.
* `webtest-a-1` (baseline Lane A): complete, 11 bug
  verdicts at
  [../webtest-a/webtest-a-1.md](../webtest-a/webtest-a-1.md).
  Headliner finds: B20 markdown table render crash
  (`RangeError: Block decorations may not be specified via
  plugins`), B1 Shift+Tab focus theft, B13 typing-before-
  marker, B9 image-bubble stray separator.
* `webtest-b-1` (baseline Lane B): status pending.
* New tasks cut today:
  * `fullstack-5` — workspace tab D&D regression
    (drag active onto adjacent inactive = delete; surfaced
    by @@WebtestA during `webtest-a-2`).
  * `webtest-a-3` — `fullstack-2` walkthrough.

Coordination flag from @@WebtestA: chrome-MCP shares one
browser between webtest lanes; their tab gets pulled
between 8801 / 8810 when @@WebtestB navigates. Mitigation:
re-assert `window.location.assign` at top of each batch.
Long-term: separate Chrome profiles per lane (logged for
Round 2 setup work).

Commit-order plan (for @@Alex to authorize sequentially):

1. `systacean-1` (smallest cross-task overlap risk; lands
   `CHAN_WINDOW_ID` + `CHAN_CONTROL_SOCKET` + `chan open`).
2. `fullstack-1` (docked side panes).
3. `systacean-2` (write-timeout fix).
4. `fullstack-2` (after `webtest-a-3` walkthrough passes).
5. `fullstack-5` (tab D&D regression) — fold into the same
   release wave if it lands in time; otherwise next wave.

Patch version bump after #4 lands. Push if the local
pre-push gate (macOS) is green.

## 2026-05-18 14:50 BST — wave-1 commits landing + WebtestB findings

### Commits on main

* `6c53c2d` — `systacean-1` (chan open CLI).
* `87a9a36` — `fullstack-1` (docked side panes).
* `c03d6f2` — `fullstack-5` (tab D&D regression + reopen
  closed tabs).
* `systacean-2` — cleared to commit (signalled @@Systacean
  at 14:50 BST). Expect a small rebase on `tabs.svelte.ts`
  vs `fullstack-5`.
* `fullstack-2` — blocked on `webtest-a-3` external-link
  walkthrough.

### Lane B walkthrough findings (`webtest-b-1` + gap-fill +
adjacent pass)

* **B14** (terminal sessions silent after browser reload):
  **NOT REPRO** on current main. Likely fixed incidentally
  by recent terminal work. Will mark closed if a focused
  check confirms.
* **B19** rescope: PTY reattach + input enable work. The
  remaining gap is **scrollback retention** only. Re-scope
  the bug entry; narrower fix.
* **B15** reproduced (left-click on empty pane opens
  right-click menu). Confirmed; folds into the pane menu
  reorg + B15 cluster.
* **B16** partial (single-window Chrome).
* **B17** confirmed (per-tab mute state not preserved across
  `Cmd+Shift+I`).
* **B18** clarified: strip-level mute IS clickable; the
  `[BCAST]` pill is a status indicator only. Original
  complaint may have been about per-tab mute on the pill,
  which isn't an interactive control.
* **B20** light-mode contrast: concrete repro provided
  (`\e[37m` invisible white-on-white; green/yellow/cyan too
  pale). Use as the implementation target.
* **E3 doc tab menu**: bigger than the request implied —
  doc tab has NO right-click menu (terminal has 22 items).
  Need to BUILD a doc tab menu, not reorder the terminal's.
  Updating the pane menu reorg task scope.

### New finds (logged for next-wave triage)

* **`chan open <dir>` opens parent + highlights**, not into
  the directory's listing. Small nit — file as a
  `systacean-N` follow-up after `systacean-2` ships.
* **Cross-drive nav drift** (Lane B → Lane A: 8810 → 8801
  via stray clicks in the welcome-state pane menu's `Files`
  entry). Welcome-state pane menu's global drives picker
  defaults to most-recent drive. Once a tab is open in the
  pane the menu collapses to 3 items and the trigger
  vanishes. Needs targeted repro before pointing at code;
  @@WebtestB will revisit when convenient.

### URL hand-off

* Lane A (still up): `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`
  on `/tmp/chan-webtest-a-1/`.
* Lane B (still up): `http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
  on `/private/tmp/chan-webtest-b-1`.

## 2026-05-18 15:35 BST — round-1 closeout plan

@@Alex's directive: close round 1 = land remaining commits,
patch bump to 0.10.1, build Chan.app desktop bundle, push to
origin so other hosts can pull and rebuild. Then recycle all
agent sessions to come back fresh on the new version.

### Critical path

1. `fullstack-3` commit — @@Alex AUTH'd 15:25; @@FullStack
   committing imminently.
2. `fullstack-2` revision — @@FullStack revising Tauri
   `shell.open` dispatch (impl already had `plugin:opener`
   but not firing in Chan.app — debug). Then commit.
3. `webtest-a-3` external-link walkthrough — three scenarios
   (browser / desktop / tunnel-loop). Gates `fullstack-2`
   commit.
4. `systacean-5` closeout (cut today) — patch bump + Chan.app
   build + push. Drives the close.

### Wave-1.5 task files cut today (queued for post-recycle)

* `fullstack-4` — list + image bugs B1/B2/B13 (cut earlier).
* `fullstack-6` — pane menu reorg + B15 click semantics +
  per-pane focus-border color + Next/Prev pane + doc-tab
  right-click menu (consolidated cluster).
* `fullstack-7` — light-mode terminal contrast bump.
* `systacean-3` — cross-drive nav drift investigation.
* `systacean-4` — `chan open <dir>` enters the dir.
* `webtest-b-2` — wave-1.5 Lane B walkthroughs.

These wait in the tree for the fresh post-recycle agents.

### Recycle protocol heads-up for @@Alex

When `systacean-5` confirms push + Chan.app build is done,
I'll fire a single chat ping summarizing the closeout SHAs,
the version tag, the Chan.app bundle path, and the list of
task files ready for fresh agents to pick up. Recycle uses
the same bootstrap prompts from `docs/agents/bootstrap.md`
(no changes needed; the path-mismatch issue from this
morning is long gone).

## 2026-05-18 16:00 BST — closeout path clear

Five wave-1 commits on main:

1. `6c53c2d` systacean-1
2. `87a9a36` fullstack-1
3. `c03d6f2` fullstack-5 + autosave serialization
4. `1a937e8` systacean-2
5. `064d3e7` fullstack-3

Ready to commit (architect-cleared, awaiting @@Alex):

* `fullstack-2` — Tauri opener dispatch + unified style
  toolbar. @@WebtestA's three-scenario walkthrough done
  (scenario 1 live, scenarios 2+3 by code audit — Chrome
  MCP can't drive WKWebView on macOS). Verdict accepted;
  architectural constraint satisfied by construction.
* `systacean-4` — chan open dir enters the directory.
  Implemented by @@Systacean during idle cycles. Small,
  isolated, fold-in TBD by @@Alex.

Then `systacean-5` drives:

* Patch bump to **0.10.1**.
* Local pre-push gate (macOS): fmt + clippy `-D warnings` +
  test + `--no-default-features` build + npm check + test +
  build.
* Chan.app desktop bundle build.
* Push `main` + `v0.10.1` tag.

@@WebtestB's final sweep also caught:

* fs-move on open file reproduced — wave-1.5 scope.
* Rich-prompt right-click missing — fold into `fullstack-6`.
* E1 docked file browser confirmed working.

## 2026-05-18 17:00 BST — Round 1 SHIPPED

Tag `v0.10.1` pushed to `origin/main`. Closeout commits in
order:

1. `6c53c2d` — systacean-1 (window-scoped chan open)
2. `87a9a36` — fullstack-1 (docked file-browser side panes)
3. `c03d6f2` — fullstack-5 (tab drag reorder + reopen closed
   tabs, includes autosave serialization)
4. `1a937e8` — systacean-2 (file writes off Tokio workers)
5. `064d3e7` — fullstack-3 (Find / link-bubble UX state ladders)
6. `3ab0aac` — fullstack-2 (external links via desktop shell)
7. `f8014a9` — fix: restore terminal prompt mode toggle (caught
   by `npm test` after fullstack-2's toolbar unification; small
   inline regression fix by @@Systacean during closeout)
8. `f996f4c` — systacean-4 (chan open dir enters listing)
9. `9e48367` — chore: bump version to 0.10.1

### Bundle

* macOS Chan.app: `target/release/bundle/macos/Chan.app`
  (ad-hoc signed, no notarization — acceptable for private
  repo / single-user case).
* DMG path (`make build`) failed in `bundle_dmg.sh`. Worked
  around with `cargo tauri build --bundles app`.
* `/Applications/Chan.app` was NOT replaced — the running
  Chan.app couldn't be deleted safely. @@Alex needs to quit
  the running app and manually replace.

### Disk

* @@Systacean reclaimed a 53G `target/` tree before the
  clean rebuild. Post-rebuild is ~11G. Worth noting for
  whoever else clones the repo.

### Notable

* @@Systacean's inline regression-fix commit `f8014a9` is
  worth a callout in the phase summary. The shared
  StyleToolbar unification from `fullstack-2` had
  inadvertently lost the source/rendered toggle on the
  terminal prompt; `npm test` caught it, @@Systacean fixed
  it in the closeout window without escalating. Good
  systems-eng catch.

### Fresh-agent task queue (post-recycle)

Task files sitting in the tree, ready for fresh sessions:

* `fullstack-4` — list + image bugs (B1/B2/B13).
* `fullstack-6` — pane menu reorg + B15 + per-pane focus
  color + Next/Prev pane + new doc-tab right-click menu +
  (folded in) rich-prompt right-click menu.
* `fullstack-7` — light-mode terminal contrast bump.
* `systacean-3` — cross-drive nav drift investigation.
* `webtest-b-2` — wave-1.5 Lane B walkthroughs.

Bootstrap prompts in [docs/agents/bootstrap.md](../../../agents/bootstrap.md)
unchanged; agents resume from `docs/journals/phase-7/`.

## 2026-05-18 17:05 BST — Handover (architect-recycle)

@@Alex is recycling all agent sessions, me included.
This append is the load-bearing context for whoever boots
as @@Architect next.

### State right now

* `origin/main` at `9e48367`, tagged `v0.10.1`. Push
  verified (0 ahead / 0 behind).
* All four working agents recycling. None mid-task.
* Wave-1 shipped: 9 commits listed above. Chan.app desktop
  bundle built at `target/release/bundle/macos/Chan.app`;
  @@Alex still needs to manually replace
  `/Applications/Chan.app` (running app couldn't be
  deleted safely during the build).

### Read these first when you boot

In order:

1. This journal entry (you're here).
2. The Plan summary section at the top of this file — it's
   accurate for both Round 1 (done) and Round 2 (next).
3. [../alex/setup-1.md](../alex/setup-1.md) and
   [../alex/setup-2.md](../alex/setup-2.md) — every gating
   decision @@Alex has made this phase, with their replies.
4. [../request.md](../request.md) — source of truth; pay
   attention to Round 2 features (survey protocol, bubble
   overlay, agent spawning, orchestration SKILL).
5. [../process.md](../process.md) — particularly "Events",
   "Approving a permission event", "Agent-recycle protocol".
   These changed mid-phase.

### Immediate next moves when you resume

1. **Do NOT cut new tasks yet.** First confirm @@Alex is
   ready to start Round 2 fan-out, or whether they're still
   on the build / install side of the 0.10.1 recycle.
2. **Round 2 capacity proposal**: I drafted the plan
   summary but never cut a formal capacity proposal for
   Round 2. That's your first architect deliverable when
   @@Alex says "go on Round 2".
3. **Fresh fullstack/systacean/webtest sessions** will
   bootstrap and find these task files waiting:
   * `fullstack/fullstack-4.md` (list + image bugs).
   * `fullstack/fullstack-6.md` (pane cluster — also
     subsumes rich-prompt right-click menu per
     @@WebtestB's late finding).
   * `fullstack/fullstack-7.md` (light-mode terminal
     contrast).
   * `systacean/systacean-3.md` (cross-drive nav drift
     investigation — non-trivial, browser-cache /
     ServiceWorker territory).
   * `webtest-b/webtest-b-2.md` (wave-1.5 walkthroughs).
   @@WebtestA has no queued task — Round 2 fan-out is
   where they get work.

### Decisions @@Alex has made (mirror is here for fast lookup)

* **setup-1**: full historical contact roster, phase-4
  imported as a stub, phase dir rename to `phase-N`,
  migrate to `docs/journals/` now.
* **setup-2 (Round 2)**:
  * Q1: tight closeout. (Done.)
  * Q2: **structured JSON** for survey schema (against
    my recommendation; their choice for automation
    friendliness).
  * Q3: scope tags `one-shot` / `topic-session` /
    `topic-phase`. Default one-shot; per-topic grant
    upgrades to topic-session.
  * Q4: configurable watcher; every survey carries a
    "skip/not now" option so reply IS the dismiss; stack
    vs tray = user preference switch.
  * Q5: **full CLI command** in spawn event (against my
    profile-references recommendation; zero-setup
    preference). Pre-flight survey: open-terminal /
    kill / retry, with timeout + spinner + counter +
    "retry now" affordance.
  * Q6: `docs/agents/orchestration/` directory (for our
    team); the bubble watcher dialog asks external users
    where their skills live.
  * Q7: accepted wave-1.5 sequencing (terminal-reattach,
    light-mode contrast, fs-move UX wedge). Activity
    indicator + MCP auto-discovery slide to Round 2.

### Things to watch out for (subtle / non-obvious)

* **Append-only is fragile under multi-author edits.**
  I had to clean up after a `git mv` rename + sed sweep
  early today; the sed nuked narrative references describing
  the rename itself. When doing mechanical text changes,
  carve out anchors that document the change (`legacy
  chan-pre-release-phase-N`, ChanRoadmap paths, etc.).
* **Curated status reports**: @@Alex explicitly wants
  highlights/lowlights/contention summaries, not full
  tabular dumps. Saved as memory; do NOT relapse into
  long status tables in chat.
* **Permission approval mechanism**: permission events
  count as "approved in writing" when either @@Alex
  appends "approved" themselves OR @@Architect appends
  "approved (transcribed by @@Architect)" with a chat
  timestamp. Use the transcription path so @@Alex doesn't
  context-switch into files.
* **Commits**: only @@Alex authorizes commits per
  `CLAUDE.md`. Architect-side clearance is necessary but
  not sufficient; always say "gated on @@Alex".
* **Cross-drive drift bug**: @@WebtestB found a real
  multi-tunnel drift. Hop happens BEFORE any page JS runs;
  browser-cache / service-worker / rust-embed identical-
  bundle territory. Routed to @@Systacean as
  `systacean-3`. Don't conflate with anything SPA-side.
* **DMG bundle path broken**: `make build` failed in
  `bundle_dmg.sh`. Worked around with
  `cargo tauri build --bundles app`. Worth a Round-2
  follow-up to fix the DMG step properly.
* **`f8014a9` regression catch**: `fullstack-2`'s shared
  StyleToolbar unification had silently dropped the
  terminal prompt's source/rendered toggle. Caught by
  `npm test`, fixed inline by @@Systacean. Worth a credit
  in the eventual `summary.md`.

### Memory (already saved, will carry forward)

* `feedback_curated_status_reports` — H/L/contention only.
* `feedback_pre_push_checks` — fmt + clippy + test +
  svelte-check + npm build before every push.
* `feedback_test_server_workflow` — ask before spinning up
  a test server (drive choice + seed).
* `feedback_chan_invariants` — pragmatic invariant
  handling.
* `project_chan_code_variant`, `project_media_browser`,
  `project_native_ios_accessory_abandoned` — context.
* `user_shell` — bash on macOS, not zsh.

### Sign-off

Round 1 of phase 7 closed cleanly on `v0.10.1`. Round 2
queued. Fresh me: read the items above in order, then
ask @@Alex if they're ready to start Round 2 fan-out.

— @@Architect, 2026-05-18 17:05 BST

## 2026-05-18 18:10 BST — fresh-architect resumption

Resumed post-recycle. Three things needed catching up on
before any Round-2 fan-out.

### Docs migration commit (recovered)

The `docs/journals/` + `docs/agents/` migration claimed
done in the 17:05 handover was actually never committed —
the staging dropped during the v0.10.1 closeout. Working
tree had 187 unstaged deletes under `chan-pre-release-
phase-*` and `docs/agents/` + `docs/journals/` entirely
untracked.

Caught + landed as `2fc286e` (`chore: migrate phase
journals to docs/journals + add docs/agents`). Pre-push
gate green, pushed to `origin/main`. Stray `untitled.md`
at repo root wiped per @@Alex.

### Fresh agents jumped early

Three of the four fresh agents booted before I did and
started their queued tasks without architect direction.
Process violation but caught quickly; work is sound:

* **@@FullStack** — 5 files of `fullstack-4` (B1/B2/B13).
  `outdentListItem` now consumes Shift-Tab even when
  there's nothing to outdent (no focus theft). Image
  paste in lists inserts ` ` not `\n`. New
  `listCaretGuard` clamps click position past the marker.
  `stripUnusedInlineImageSpaceOnEnter` for the
  retract-on-Enter case. Tests added. **Architect-side
  cleared to commit**, gated on @@Alex.
* **@@Systacean** — `systacean-3` proposal + patch
  landed in the working tree. `Cache-Control: no-store`
  on SPA shell, `public, max-age=31536000, immutable` +
  `Vary: Host` on hashed assets. Tests added. Clean,
  minimal, matches the diagnostic theory. **Architect-
  side cleared to commit**, gated on @@Alex.
* **@@WebtestB** — re-verified B14/B19 on post-recycle
  main. **B14 NOT REPRO**, B19 PTY re-attach works, only
  scrollback retention remains. Rescope B19 accordingly.

### Round 2 decisions taken with @@Alex this turn

* **Atomic-write contract for fswatcher events**. Writers
  (every agent + chan itself) write event files via temp
  file + rename in the same dir. Watcher (chan-server)
  fires once on fsnotify, reads once. No defensive
  multi-read on the server side — that complexity drops
  vs request.md line 106.
* **No watcher self-loops**. chan-server's reaction to a
  watched event must never write back into the watched
  directory. Default posture is structural separation
  (poke → PTY, not → disk-in-watched-dir). If we ever do
  write inside a watched dir, reuse the existing
  `crates/chan-server/src/self_writes.rs` for notify
  suppression.
* **Contract lives in process.md now, escalates to
  orchestration SKILL later**. For our roster the rule
  is one paragraph in `process.md`. For external users
  (Round 2 deliverable) the same rule lands in
  `docs/agents/orchestration/` with per-language
  temp+rename examples.

### Request.md re-pass — new items relative to handover

Reviewed at @@Alex's request. New asks:

* **B22 — Copy Path on directory leaves file-browser
  pane in a stuck `Loading…` state.** Recovery requires
  Reload, currently surfacing only via left-click on
  empty pane (B15 cluster). Pairs with the pane-menu
  reorg work — promote `fullstack-6` ahead of
  `fullstack-4` / `fullstack-7` so Reload moves to the
  hamburger sooner. **@@Alex authorized the promotion.**
* **4×3 multi-topic survey variant** for batched
  architect→@@Alex dispatches (2-4 topics × 1-3
  options).
* **Standing "check my comments first" survey option**
  on every survey.
* **Watcher status bullet on terminal tab**, blinks when
  replies arrive (parallel to file-save bullet).
* **HTTP control channel for agent spawning** (preferred
  over MCP). Token shape vs `--no-token` mode needs
  design.
* **Gemini test affordance**: @@Alex offers their gemini
  for ~/.gemini settings-reset reproduction during
  spawn-pre-flight design.
* **Broadcast unifies with the bubble notification
  system** — not a separate channel.

Setup-2 Q5 amendment: the spawn event still carries the
full CLI command (zero-setup), but the *back-channel*
between spawned agent and chan-server is HTTP, not MCP.
Logging this as a refinement, not a reversal.

### Wave-1.5 sequence (authorized)

Promotion takes effect. New order:

1. `fullstack-6` — pane menu reorg + B15 + per-pane
   focus color + Next/Prev pane + new doc-tab right-
   click menu + (folded in) rich-prompt right-click
   menu + B22 stuck-Loading addendum.
2. `fullstack-4` — list + image bugs (B1/B2/B13).
   @@FullStack already drafted; commits behind
   `fullstack-6` if needed for review ordering, or
   beside it if independent.
3. `fullstack-7` — light-mode terminal contrast bump.
4. `systacean-3` — cross-drive nav drift. @@Systacean
   already drafted patch; landing requires @@WebtestB
   re-repro on the new headers.

### Outstanding

* 5 unstaged code files (in `web/src/editor/` + 1 in
  `crates/chan-server/src/`) remain after the docs
  migration push — they ARE @@FullStack's + @@Systacean's
  in-progress work, intentionally left in tree.
* Round 2 capacity proposal still owed. Will draft once
  `fullstack-6` is cut and the wave-1.5 commits start
  landing.

— @@Architect, 2026-05-18 18:10 BST

## 2026-05-18 18:50 BST — BCAST/mute cluster scope expansion

@@Alex repro on a 6-terminal stress test surfaced new
symptoms in the B17/B18 cluster (already wave-2 deferred):

* `[BCAST]` text pill on tabs should be replaced by the
  broadcast (radio) icon used in the membership chip area
  (consistent with the menu UI). Original B18 wording had
  this buried; promoting it to first-class spec.
* BCAST membership menu toggle is leaking across tabs:
  ticking one terminal flips others. Toggle must isolate
  to the clicked terminal.
* Select-all / deselect-all in the membership menu must
  preserve each tab's individual pre-existing MUTE state.
  BCAST membership and per-tab MUTE are independent axes.

Captured in request.md as a sub-bullet under the existing
B18 line, not as a new bug entry. Wave-2 still owns it; no
promotion. When `fullstack-6` ships and we cut wave-2
tasks, BCAST/mute gets its own `fullstack-N.md` with the
above as acceptance criteria.

— @@Architect, 2026-05-18 18:50 BST

## 2026-05-18 19:30 BST — wave-1.5 commits landing

@@FullStack picked up the standing topic-level clearance and
pushed both their queued patches:

* `d4b11d2` — `fullstack-4` (B1/B2/B13 list/image bugs).
* `67a637f` — `fullstack-6` (pane menu reorg + B15 click
  semantics + per-pane focus color + Next/Prev pane +
  doc-tab right-click menu + rich-prompt right-click menu +
  B22 stuck-Loading cleanup).
* `13eadfb` — `fullstack-7` (light-mode terminal contrast
  bump).

@@Systacean is reading their commit-auth poke now; their
`systacean-3` (cache headers) lands next.

After `systacean-3` is on `main`, @@WebtestB gets the
re-repro poke for the Lane-A coexistence drift on the new
headers. That closes wave-1.5 on the fix side.

@@WebtestA's self-initiated `webtest-a-4` regression sweep
is running against `d4b11d2`+. They'll top up scope as more
commits land.

Notification-system gap surfaced today: agents don't auto-
detect event-file appends. Pokes queue until @@Alex wakes
the agent's terminal. Round 2's bubble overlay closes that.

— @@Architect, 2026-05-18 19:30 BST

## 2026-05-18 20:00 BST — wave-2 fan-out (stepped)

@@Alex chose the stepped approach: fan out wave-2 bugs +
walkthroughs now, draft the Round 2 capacity proposal in
parallel, kick off Round 2 features after wave-2 lands.

### Tasks cut

* `fullstack-8` — BCAST/mute cluster (B17+B18+6-terminal).
* `fullstack-9` — markdown pipe-table crash (B20).
* `fullstack-10` — editor cursor + scroll cluster
  (B6+B7+B12).
* `fullstack-11` — fs-move UX wedge.
* `fullstack-12` — `Cmd+\`` → `Cmd+T` / `Cmd+Alt+T` (B16).
* `webtest-a-5` — Lane A rolling walkthrough (wave-1.5
  + wave-2 as commits land).
* `webtest-b-3` — Lane B rolling walkthrough (same).
* `systacean-6` — cross-drive drift, SPA persistent-state
  phase (already cut earlier this turn).

### Slot occupancy

* @@FullStack: 5-task wave-2 queue, serial execution.
* @@Systacean: `systacean-6` in flight.
* @@WebtestA: rolling walkthroughs.
* @@WebtestB: rolling walkthroughs.

All four slots saturated. @@Alex will wake terminals when
ready; pokes are queued in event logs.

### Notification gap (architectural note)

Pokes pile up in event files; agents only read them when
@@Alex wakes their terminal. This trip's signature pattern:
each agent landed work, parked, was woken once, landed
more work, parked again. The Round 2 bubble-overlay +
fsnotify-watcher feature closes that loop.

### Standing topic-level commit clearance

@@Alex's "make intelligent decisions" mandate covers
gate-green → commit → push for architect-cleared work in
the active wave. Reaffirmed for wave-2 bugs.

— @@Architect, 2026-05-18 20:00 BST

## 2026-05-18 21:00 BST — Round 2 capacity proposal

@@Alex authorized Round 2 fan-out with the same 4-slot
capacity. Round 2 has four feature streams; this proposal
fans out the substrate (streams #1 + #2) now, defers
spawning + orchestration SKILL to a second wave once the
substrate lands.

### Feature streams

| Stream | Name                                           | Wave |
|--------|------------------------------------------------|------|
| F1     | Survey-style event protocol (schema + UI)      | A    |
| F2     | Notification bubbles over rich prompt          | A    |
|        | (fsnotify watcher + bubble overlay)            |      |
| F3     | Programmatic agent spawning (HTTP back-channel)| B    |
| F4     | Orchestration SKILL (docs/agents/orchestration)| B    |

F1 and F2 share substrate (fsnotify event ingestion +
typed event schema). F3 builds on F1 (uses the survey
mechanism for pre-flight troubleshooting). F4 is the
external-user-facing documentation of the whole stack.

### Wave-A fan-out (substrate)

| Agent       | Task          | Scope                                                        |
|-------------|---------------|--------------------------------------------------------------|
| @@Systacean | `systacean-9` | fsnotify watcher + survey-event ingestion + atomic-write + no-self-loop |
| @@FullStack | `fullstack-13`| Bubble overlay UI + watcher-set dialog + survey rendering + reply path |
| @@WebtestA  | `webtest-a-6` | Rolling walkthrough on frontend pieces                       |
| @@WebtestB  | `webtest-b-4` | Rolling walkthrough on backend pieces + watcher → PTY path   |

Architect-side parallel work: orchestration SKILL initial
draft scaffold so it's ready when wave-B tasks land.

### Survey schema (design lock for wave-A)

Event file shape, written atomically by the producer:

```json
{
  "id": "<unique-id>",
  "type": "survey",
  "from": "@@SomeAgent",
  "to": "@@Alex",
  "topic": "<short-topic-tag>",
  "questions": [
    {
      "header": "<short label, max 12 chars>",
      "text": "<question text>",
      "options": [
        {"key": "1", "label": "<short option>"},
        {"key": "2", "label": "<short option>"},
        {"key": "3", "label": "<short option>"}
      ]
    }
  ],
  "standing_options": [
    {"key": "C", "label": "Check my comments first"}
  ],
  "scope": "one-shot" | "topic-session" | "topic-phase"
}
```

Multi-topic (4×3) variant: up to 4 `questions`, each with
up to 3 `options`. Single-topic = 1 question.

Reply, written atomically by @@Alex via the bubble UI:

```json
{
  "id": "<survey-id-being-replied-to>",
  "type": "survey-reply",
  "from": "@@Alex",
  "to": "@@SomeAgent",
  "answers": [{"question_index": 0, "key": "1"}],
  "scope_grant": "one-shot" | "topic-session" | "topic-phase",
  "note": "<optional free text>"
}
```

### Standing rules

* Writers always use temp+rename atomic writes.
* Watcher reads once on fsnotify; no defensive multi-read.
* chan-server must never write back into a watched dir
  (poke → PTY, not → disk-in-watched-dir).
* Watcher lifecycle = terminal-session-scoped; dropped on
  terminal exit; not re-created.

### Wave-B (deferred, cut after wave-A lands)

* `systacean-N`: HTTP agent control channel
  (spawn / name terminal / execute command / restart).
* `fullstack-N`: Spawn-from-rich-prompt UI + pre-flight
  survey (open-terminal / kill / retry, with timeout +
  spinner + counter + "retry now").
* `architect-N`: Orchestration SKILL —
  `docs/agents/orchestration/` with atomic-write contract
  per-language examples + spawn protocol guide.

### Carry-over polish (low priority backfill)

* Light-mode contrast: `\e[37m` borderline + `B97` collapse
  to `C30`. Wait until @@FullStack idles or fold into a
  wave-B task.
* Pane hamburger ↔ right-click menu auto-dismiss
  interaction (@@WebtestA's cosmetic note).

— @@Architect, 2026-05-18 21:00 BST

## 2026-05-19 00:45 BST — Round 2 substrate + Phase 1 + Phase 2 shipped, Wave-B queued

Throughput milestone. Round 2 wave-A landed, Phase 1
overlays-to-tabs landed, Phase 2 Hybrid pane model
landed in two parts (substrate + Cmd+K transactional
mode), polish bundle landed.

### Round 2 + Phase 1/2 commits on main (today)

| Commit  | Task           | What                                                       |
|---------|----------------|------------------------------------------------------------|
| cd88b0c | systacean-9    | fsnotify watcher + event ingestion + dispatch              |
| 1f2f6fc | fullstack-13   | bubble overlay + watcher dialog + survey UI                |
| 2d1c719 | fullstack-18   | TUI density simplification (numbered keys, drop Submit)    |
| 530e30f | systacean-11   | event-reply atomic-write endpoint (bypasses drive gate)    |
| 7bc2897 | fullstack-19   | SPA reply path switched to chan-server endpoint            |
| 1cd4ef2 | systacean-8 v2 | PTY reattach by (window_id, tab_name) on reload            |
| 4ca7dc4 | systacean-10   | revert systacean-6 (confirmed no-op vs systacean-3 alone)  |
| a2fb205 | fullstack-14   | Phase 1 — Graph + File Browser as first-class tabs         |
| e4f9d28 | fullstack-15   | Phase 2 substrate — binary-tree + drag-detach + persistence|
| 44d9749 | fullstack-16   | Phase 2 Cmd+K transactional pane mode + WASD/arrows/resize |
| 0c2faa7 | fullstack-17   | polish bundle (7 nits including watcher staleness)         |
| dfcad1c | architect-1    | wave-B fan-out + orchestration SKILL initial drop          |

Twelve commits in roughly six hours. Pre-recycle stamp
was `v0.10.1` at `9e48367`; this entire run rides on top
of `9e48367` post-recycle and post-docs-migration
(`2fc286e`).

### Round 2 wave-B queued

| Task          | Owner       | Scope                                                                              |
|---------------|-------------|------------------------------------------------------------------------------------|
| systacean-12  | @@Systacean | HTTP agent control channel (spawn / name / execute / restart)                      |
| systacean-13  | @@Systacean | Activity indicator on terminal tabs (PTY output-since-focus)                       |
| systacean-14  | @@Systacean | MCP auto-discovery (claude / codex / gemini)                                       |
| fullstack-20  | @@FullStack | Spawn-from-rich-prompt UI + pre-flight survey                                      |
| architect-1   | @@Architect | Orchestration SKILL — README + atomic-writes + spawn-protocol shipped; mcp-discovery deferred to -14 |

Walkthrough lanes: `webtest-a-7` (frontend angle),
`webtest-b-5` (backend + deferred `fullstack-15`
pane-detach catch-up).

### Patterns that worked this round

* **Standing topic-level commit clearance.** Once @@Alex
  said "make intelligent decisions", I authorized agent
  commits inline (still recording the auth in their
  event log per the permission-approval mechanic). Saved
  a per-commit ping-pong. 12 commits cleared this way
  without surprise rollback.
* **Cross-stack ownership where it makes sense.**
  @@Systacean wrote SPA storage scoping in `systacean-6`
  (then reverted in `-10` when @@WebtestA showed it
  wasn't load-bearing) and the event-reply endpoint in
  `-11`. @@FullStack switched the SPA reply caller in
  `-19`. No lane-policing friction.
* **Wider repro folding.** When @@WebtestB found the
  watcher-staleness bug also fired on URL-hash nav
  (not just reload), I appended the wider context to
  the existing `fullstack-17` polish entry rather than
  cutting a new task. Same fix, more thorough spec.

### Subtle / non-obvious

* **fsnotify-watcher in chan-server reads ONCE.** The
  whole substrate trusts writer-side atomic temp+rename.
  No defensive multi-read on the server. This is the
  load-bearing engineering decision from the request.md
  addendum — anyone touching the watcher must preserve
  it.
* **Survey-reply path bypasses chan-drive.** Event
  files are infra traffic, not user content; they go
  through `tokio::fs` in `systacean-11`'s endpoint, NOT
  through `chan_drive::Drive::write_text`. The drive's
  editable-text gate is correct for editor saves and
  wrong for this surface. Keeping the drive boundary
  clean was the architectural reason.
* **systacean-6 was reverted** after @@WebtestA showed
  `systacean-3`'s `Vary: Host` was sufficient. The
  storage-key namespacing is gone; if drift surfaces
  again, look at the welcome-state Files action path
  next.
* **Phase 2 was easier than expected.** Existing
  layout model was already a binary-tree of splits with
  URL/session persistence. `fullstack-15` was just the
  drag-detach addition; `fullstack-16` was Cmd+K +
  keybinds on top.
* **TUI density survey UI** (`fullstack-18`) replaced
  the v0 bubble overlay's Submit / Scope dropdown /
  Skip / standing-option-row chrome. Multi-topic 4×3
  gets a horizontal tab strip with auto-advance +
  auto-commit. No Submit button anywhere.

### Outstanding architect work

* `architect-1` — `mcp-discovery.md` waits on
  `systacean-14`'s per-agent investigation.
* Phase-4 import from `~/Documents/ChanRoadmap/` —
  needs explicit @@Alex go-ahead to read outside
  repo (auto-mode blocks).
* Eventual phase summary — wait until wave-B ships.

### Recycle-prep note for next @@Architect

If you boot fresh: read this entry, then `git log
--oneline -25` to confirm the commit sequence above.
Wave-B is in flight; the four pokes at the bottom of
each event log (~22:30-00:30 BST appendices) are the
authoritative dispatch.

— @@Architect, 2026-05-19 00:45 BST
