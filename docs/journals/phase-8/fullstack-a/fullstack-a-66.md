# fullstack-a-66 — SPA New Draft action + FB Drafts folder rendering + Rich Prompt history reuse

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Dependency: `systacean-24` (chan-drive Drafts primitive)

## Goal

SPA implementation of the New Draft flow:

1. **Cmd+N from Hybrid hamburger / global keymap** →
   create `Drafts/untitled-N/` directory + populate
   `draft.md` + open in Hybrid Editor.
2. **FB Drafts folder rendering** — first element;
   distinct color (yellow w/ light/dark variants).
   Inspector shows "lives outside drive's root"
   notice.
3. **Rich Prompt history reuse**: Rich Prompt history
   stored as `Drafts/rich-prompt-N/...` via the same
   Drafts mechanism.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md):
"## Flow for the 'New Draft' action" + "### Extra"
sections.

## Dependency on -24

@@Systacean's `systacean-24` lands the chan-drive
backend (filesystem primitive + indexer + graph emit).
This task consumes that API surface. **Wait for `-24`
commit-readiness** OR start SPA shell + integration
points with stubbed API + wire in once `-24` lands.
Implementer picks.

## Scope (SPA)

### Cmd+N handler

* Bind Cmd+N → call `chan_drive::Drive::create_draft_dir`
  (via chan-server route OR directly through Tauri
  IPC if needed). Pick smallest `N` unused.
* Open `draft.md` inside the new dir in the Hybrid
  Editor.
* No further user prompt — direct create + open.

### FB Drafts folder rendering

* First element in the FB tree (above the drive root
  contents).
* Distinct color — yellow w/ light/dark variants.
  Define CSS vars `--fb-drafts-light` / `--fb-drafts-dark`;
  apply to the row.
* Inspector view on click: same shape as a regular
  folder, but with a header notice: "Drafts lives
  outside the drive's root" (per `addendun-a.md`).

### Rich Prompt history reuse

* Refactor Rich Prompt history persistence to use
  `Drafts/rich-prompt-N/` paths instead of wherever
  it lives today (per the addendum: this means user
  has GitHub-style access to their history via the
  FB).
* Each prompt history entry is a `rich-prompt-N/`
  dir under Drafts.

### Graph integration

* Drafts root node renders with a distinct color +
  inspector matches FB inspector (same shape).
* Distinct edge style for drive → Drafts edge per
  `-24`'s emit attribute.
* Files inside Drafts in the graph behave like
  drive files.

## Acceptance

1. Cmd+N creates `Drafts/untitled-N/` + opens
   `draft.md` in Hybrid Editor. First N=1; second
   N=2 etc.
2. FB shows Drafts as the first row in yellow color
   (light + dark mode both).
3. Drafts folder click → inspector with "outside
   drive's root" notice.
4. Rich Prompt history persists into
   `Drafts/rich-prompt-N/` (verifiable via FB +
   filesystem inspection).
5. Graph view: Drafts root node distinct;
   click-inspector matches FB shape.

### Tests

Vitest pins for chord binding + chord handler +
Drafts row rendering + inspector header notice.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA SPA primary.
* Depends on `systacean-24` API surface.
* If `-24`'s API shape differs from this task's
  expectation, fire scope poke + adjust.

## Authorization

Yes for SPA files + tests + task tail + outbound.
Pause-then-resume if `-24` not yet ready.

## Numbering

This is `-a-66`.
