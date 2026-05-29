# fullstack-23: TUI vertical survey layout + mark-as-follow-up

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Revise the bubble survey rendering to a tighter TUI
layout that scales legibly for 1-2 line option labels,
and add a "mark as follow up" reply state so @@Alex can
defer a survey without losing it from view.

## Relevant links

* [../request.md](../request.md) — see the BCAST cluster
  bullets for the 2026-05-19 follow-up sub-bullet on
  this layout.
* @@Alex's direction in chat (2026-05-19 02:30 BST + 02:40 BST).
* Predecessor: [./fullstack-18.md](./fullstack-18.md)
  (TUI density first pass).

## Acceptance criteria

### Layout — single-topic survey

* Vertical list, one row per option. Each row:
  * `[N]` numbered prefix (the keyboard binding) on the
    left in a small fixed-width slot.
  * Option label as wrapping text to the right of `[N]`.
    Labels can run 1-2 lines without breaking layout.
* Question text appears above the option list (not in
  the rows).
* Click the row (anywhere) = reply with that option.
* Press `1`/`2`/`3` on keyboard = same.

### Layout — multi-topic survey

```
[topic a] [topic b]            ← topic tab strip
topic description / question text
[1] option text, 1-2 lines fine
[2] option text
[3] option text
```

* Horizontal topic-tab strip across the top of the
  bubble.
* Below the tabs: focused topic's description text.
* Below the description: vertical numbered options
  matching the single-topic shape above.
* Default focused tab = first. After answering, focus
  auto-advances; auto-commit when all tabs answered
  (same as today).

### Bounds (enforced architect-side; frontend just renders)

* Single-topic: 1-3 options.
* Multi-topic: 2-4 topics, each with 1-3 options.
* If a survey arrives with more than 3 options or more
  than 4 topics, render what fits and surface a one-line
  "truncated, this many extra options" hint at the
  bottom. Producers shouldn't send oversized surveys but
  defensive rendering avoids a crash.

### Mark as follow up (async)

* Third reply state alongside answer-on-click and Esc-
  to-skip.
* Affordance: press `F` while a bubble is focused, OR
  click a small "follow up" link/button under the
  options row (subtle — don't crowd the bubble chrome).
* Behavior (async-unblock contract):
  * Reply is **emitted immediately** with `follow_up:
    true` in the JSON. The producer agent receives it,
    treats it as "user deferred — don't wait, move on,
    expect maybe a real answer later." Producer
    UNBLOCKS as soon as the follow-up reply arrives.
  * Bubble **stays visible** in the stack/tray on the
    user's side with a small "follow up" badge. This
    is a reminder for the user — not a producer-side
    affordance.
  * If the user later acts on the bubble (picks an
    option, or skips), a NEW survey-reply is written
    that supersedes the follow-up. Producer agent
    dedups by survey `id`, latest reply wins. The
    spec lives in the orchestration SKILL when we
    document producer-side handling.

### Reply schema addition

* `survey-reply` JSON gains an optional `follow_up:
  bool` field. Default absent = false.
* No other schema changes. Backend `systacean-11`
  already accepts opaque JSON; the new field rides
  through.

### Esc / hard skip stays as today

* Esc on a bubble = reply with empty `answers` array,
  bubble dismisses immediately. Same as `fullstack-18`.
* Esc on a follow-up-badged bubble = dismisses + writes
  a final empty reply (overrides the earlier follow-up).

## Out of scope

* Producer-side semantics for handling `follow_up: true`
  replies. That's per-agent behavior; documented in the
  orchestration SKILL later.
* Reordering / sorting follow-up bubbles in the tray.
* Markdown rendering inside option labels.

## How to start

1. `web/src/components/BubbleOverlay.svelte` —
   restructure the option-render loop from horizontal
   to vertical row layout. Each row = `[N]` + wrapping
   label.
2. Multi-topic: topic tab strip + description block
   above the option list.
3. Add the `F` keystroke handler in the same scope as
   `1`/`2`/`3` (focused bubble + no text input focus).
4. Add the follow-up affordance (small link/button or
   keyboard hint).
5. Bubble state machine: gain a `marked_as_follow_up`
   flag; render the badge when set.
6. Reply path: when follow-up fires, write reply with
   `follow_up: true`, do NOT dismiss the bubble.

## Hand-off

Standard. Pre-push gate green. Coordinate with @@WebtestA
for the layout walkthrough. Insert after `fullstack-22`
in your queue. Ping via
`alex/event-fullstack-architect.md`.

## Result

2026-05-19 05:35 BST — Implemented in `web/src/components/BubbleOverlay.svelte`,
`web/src/state/watcherEvents.ts`, and `web/src/api/client.ts`.
Survey options now render as vertical numbered rows with wrapping labels.
Multi-topic surveys keep a top tab strip and render the focused topic's
question above the vertical options. Oversized surveys are defensively
bounded to four topics and three visible options with a one-line hidden
item hint.

Added the async follow-up state: pressing `F` or clicking `follow up`
writes an immediate `survey-reply` with `follow_up: true`, leaves the
bubble visible, and shows a `follow up` badge. A later option pick or
Esc writes a normal superseding reply and dismisses the bubble.

Verification:

* `npm run test -- BubbleOverlay watcherEvents`
* `npm run check`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` in a clean temporary
  worktree with only the `fullstack-23` patch applied.

Note: the shared live worktree had unrelated Systacean MCP-discovery
files dirty and failing `cargo fmt --check`, so the full gate was run
from a clean worktree to avoid touching another lane's files.
