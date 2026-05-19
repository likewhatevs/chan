# fullstack-13: Round 2 substrate — bubble overlay + watcher dialog + survey UI

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Land the frontend substrate for Round 2 feature streams F1
(survey protocol) and F2 (notification bubbles). The
backend in `systacean-9` provides the fsnotify watcher +
event dispatch over `poke\n` to the PTY. This task builds:
(1) the rich-prompt affordance to set up a watcher,
(2) the bubble overlay that floats over the terminal pane
when a watcher is active, (3) the survey rendering + reply
path that writes the reply JSON atomically.

## Relevant links

* Backend partner: [../systacean/systacean-9.md](../systacean/systacean-9.md).
* Survey schema (design lock):
  [../architect/journal.md](../architect/journal.md)
  ("2026-05-18 21:00 BST — Round 2 capacity proposal").
* @@Alex's intent:
  [../request.md](../request.md) — "Notification system
  over the rich prompt" section + engineering addendum.

## Acceptance criteria

### Watcher-set dialog (rich prompt)

* Rich prompt grows a menu item / button "Watch directory"
  that opens the existing new-file dialog (with directory
  completion) for the user to pick a watch root.
* Submit calls `POST /api/terminal/<session>/watcher`
  with the chosen path. Success = watcher state stored in
  the tab's frontend state; failure = surface error in
  the prompt.
* "Stop watching" removes via
  `DELETE /api/terminal/<session>/watcher`.
* Hiding the rich prompt hides the entire watcher UI (no
  detached overlay).

### Bubble overlay

* When a watcher is active on the current terminal tab,
  events arrive via the existing PTY input path
  (`poke\n`). The frontend tracks the watcher's directory
  separately and reads event files via existing drive
  read API to render bubble content.
* Bubbles float over the top portion of the terminal
  pane; they have a background but are visually layered
  over xterm output (the terminal underneath remains
  visible).
* User preference toggle: **stack** (bubbles pile up
  vertically) vs **tray** (collapse to a tray indicator
  with a count, expand on click). Persists per-user via
  the existing preferences endpoint.
* Each bubble shows: text content, links (clickable —
  external links route through the existing
  `linkout` handler), and survey options if `type ==
  survey`.
* "Skip / not now" is always available on every survey
  (it's just declining to pick an option — replying
  with `scope_grant: one-shot` and no answer). The
  standing option `"Check my comments first"` is
  always rendered as one of the choices.

### Survey rendering + reply

* Single-topic survey: 1×N (N ≤ 3) buttons under the
  question text.
* Multi-topic 4×3 survey: up to 4 questions stacked,
  each with up to 3 options. User can pick one option per
  question. A single "Submit" button at the bottom emits
  the reply.
* Standing options (e.g. "Check my comments first") are
  rendered below the per-question options, separated by
  a hairline.
* Scope-grant selector (one-shot / topic-session /
  topic-phase) defaults to one-shot, with per-survey
  upgrade affordance.
* Reply path: build the JSON per the schema, write
  atomically (temp + rename) to the same watch directory
  with filename `event-reply-<survey-id>.md` (or
  similar). The producer agent reads it from their own
  outbox.

### Terminal tab status bullet

* When a terminal tab has a watcher attached, show a
  small bullet/dot on the tab strip (parallel to the
  file-save dirty bullet). Tooltip: "watcher active".
* When unread bubbles / replies arrive while the rich
  prompt is hidden, the bullet **blinks**. Clears on
  prompt re-open.

## Out of scope

* Agent spawning UI (wave-B, separate task).
* Pre-flight troubleshooting survey (wave-B).
* Stack-vs-tray detailed animation polish (a reasonable
  default is fine; refine later).
* Markdown rendering inside bubbles (plain text + links
  is enough for v1).

## How to start

1. Pair with @@Systacean on the `POST/DELETE
   /api/terminal/<session>/watcher` shape before building
   the frontend caller.
2. Bubble overlay: new component
   `web/src/components/BubbleOverlay.svelte` rendered as
   a child of the terminal pane, positioned absolute over
   the xterm container.
3. Survey rendering: reuse existing button / chip
   primitives where possible.
4. Reply atomic write: use the same temp+rename pattern
   the file editor already uses for saves. Confirm with
   @@Systacean if there's an existing helper to call.
5. Status bullet: parallel to the dirty / unsaved bullet
   path already in the tab strip.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@Systacean
on the API contract; with @@WebtestA for walkthrough; with
@@WebtestB for end-to-end terminal-side validation.

## 2026-05-18 19:19 BST — implementation ready

Implemented the frontend substrate:

* Rich prompt now has Watch directory / Stop watching controls, using
  `POST` / `DELETE /api/terminal/<session>/watcher` with the locked
  body shape `{ "path": "..." }`.
* Terminal tabs store watcher state, poll/read event files from the
  chosen watch dir, refresh immediately when PTY output includes
  `poke\n`, and show a watcher bullet that blinks for unread events
  while the rich prompt is hidden.
* `BubbleOverlay.svelte` renders stack/tray modes, plain text links via
  the existing external-link opener, survey questions/options,
  standing "Check my comments first", scope grants, Submit, and
  Skip / not now.
* Survey replies write via temp create + same-dir rename to
  `event-reply-<survey-id>.md`.
* Added persisted `bubble_overlay_mode` preference plumbing.

Verification:

* `npm run test -- TerminalRichPrompt watcherEvents`
* `npm run check`
* `npm run build`
* `cargo check -p chan-server`
* `scripts/pre-push`

Note: @@Systacean's `systacean-9` backend files are also dirty in the
shared worktree; this FullStack commit should stage only the files in
this implementation plus the small preference plumbing above.
