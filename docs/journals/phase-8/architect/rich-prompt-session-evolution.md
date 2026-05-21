# Rich prompt session evolution (draft, pre-discussion with @@Alex)

Author: @@Architect
Date: 2026-05-20

Status: **draft, design decisions locked 2026-05-20**. Not
dispatched yet — fan-out gated on the broader Round-2 sequencing
confirmation in [`round-2-plan.md`](round-2-plan.md). Captures
@@Alex's 2026-05-20 ask for the rich prompt + spawn-agent surface
to become chan's multi-agent session conductor: history + cwd
preflight + shell-vs-agent submit mode + spawn-agent identity
broadcast + multi-agent spawn form.

## Decisions locked 2026-05-20 (clean sweep on @@Architect's recommended options)

| Topic                | Locked                                                        |
|----------------------|---------------------------------------------------------------|
| History storage      | On-disk `.md` under `.chan/rich-prompt-history/`. Durable + future cross-host transferable via Round-3 metadata import/export. |
| Cwd preflight timing | Always-visible header field inside the rich prompt. No separate preflight overlay. Composes with the team-row form below. |
| Submit-mode toggle   | Per-prompt toolbar icon in the prompt header. Toolbar pattern from `fullstack-a-24`. |
| Multi-agent surface  | Inside the rich prompt as a new team step. Rich prompt becomes the single session-conductor surface (cwd + history + team rows + eyeball preflight + broadcast). |

These four decisions interlock: items B + E both place new
surfaces inside the rich prompt, so items A/B/C/D/E ship as a
cohesive evolution of one surface rather than spreading across
multiple components.

## Source ask (paraphrased from chat 2026-05-20)

1. **History backlog**: Cmd+Enter rolls the prompt content into a
   history backlog and recycles the prompt window back to empty.
   History stored as `.md` files (chat-style transcript).
2. **Working-directory preflight**: rich prompt asks for a working
   directory from the start. Later this preflight grows into the
   team / agents setup checklist.
3. **Shell vs agent submit mode**: today's rich prompt sends Enter
   to the underlying shell on Cmd+Enter. Agents running in the
   shell need a Cmd+Enter chord of their own to submit. Add a UI
   switch to flip between the two modes.
4. **Spawn-agent identity placement preflight**: spawn-agent
   already works + `$CHAN_TAB_NAME` is wired correctly. Add a
   preflight where the user eyeballs every agent terminal to
   confirm authenticated + configured. Then an "identity
   placement" broadcast fires the canned message to each:
   "you are $CHAN_TAB_NAME, read docs/agents/bootstrap.md". That
   completes the session boot.
5. **Multi-agent spawn form**: simpler form with rows of
   `[agent-name] [command] [env]` + `+`/`-` to add more, plus a
   "launch in the back" checkbox to drop them into the back of a
   Hybrid. The user finds the flip-pane affordance via the
   hamburger menu (which `fullstack-a-27` already added).

## Grounded reading of today's surface

Source-of-truth audit (Explore agent, 2026-05-20):

| Surface                | Today                                                                  |
|------------------------|------------------------------------------------------------------------|
| Rich-prompt submit     | `TerminalRichPrompt::submit` → `onSubmit(buffer)` → `submitRichPrompt` → `sendUserInput(source)` writes raw text frames to the PTY |
| Trailing newline       | No explicit append in JS; whatever the Wysiwyg / Source buffer ends with is what arrives. @@Alex's read ("sends Enter to the shell") matches the empirical behaviour — must be the trailing newline left by the editor. To confirm at task-cut time. |
| Prompt clear on submit | Buffer is NOT cleared after submit; same draft sits there until the user deletes it. `focusNonce` bumps but the text stays. |
| Command history        | None today. Single in-memory draft + `rpb` session-persisted buffer in SerTab. |
| `CHAN_TAB_NAME`        | `SpawnDialog` user-typed name → `api.spawnTerminal({ name })` → `CreateOptions::tab_name` → `cmd.env("CHAN_TAB_NAME", tab_name)` on the spawned process. Verified working. |
| Spawn target pane      | New tabs land in `pane.tabs` (currently visible side). No back-side targeting; would be net-new wiring. |
| Bulk spawn             | None. One SpawnDialog submit = one terminal. |
| Send-literal-chord     | None. The buffer is sent as raw text frames; there is no path for "send a Cmd+Enter chord to the terminal program." |

## Decomposition

### Item A — history backlog + recycle

Two halves:
* **Recycle on submit**: after Cmd+Enter, clear the buffer. Trivial
  one-line wire-up; should land as a small SPA fix.
* **Chat-style `.md` history**: each submitted buffer rolls into
  a transcript. Open design questions:
  * Storage layer (in-memory / session-persisted / on-disk).
  * Surface (history scroll above the prompt? a separate panel?
    @@Alex's "history backlog" suggests the chat-bubble pattern
    `fullstack-a-24` already established).
  * Per what scope (per-rich-prompt instance / per-terminal-tab /
    per-drive)?

### Item B — working-directory preflight

A "cwd unset → preflight first" gate on the rich prompt. Composes
with item D's broader preflight checklist (agent-team setup). Open
questions:
* When does the preflight fire — at prompt mount, or first
  Cmd+Enter, or always-visible header?
* Does cwd `cd` the underlying shell, or only annotate the
  recorded history? (If `cd`, sequencing matters: cwd fires
  before the buffer.)
* Where does the cwd persist (per-prompt / per-tab / per-drive)?

### Item C — shell vs agent submit mode

Today's flow sends raw buffer text into the PTY; multi-line
buffers act as a sequence of shell commands because the shell
treats embedded `\n` as Enter. For an interactive agent running
inside the terminal (Claude Code, codex), Enter inserts a
newline into the agent's input draft; the agent needs its own
Cmd+Enter chord to submit-as-one-message.

Two practical implementations:

* **Mode A "shell" (current)**: send buffer verbatim; trailing
  newline executes the last line.
* **Mode B "agent"**: send buffer with newlines mapped to the
  agent's "newline-within-input" convention (typically the same
  raw `\n` since most agent input modes use it that way), then
  append the agent's submit chord. The submit chord encoding is
  not universal — common shapes are `\x1b[27;9;13~` (xterm
  modifier-other-keys) or a literal `\x0d` (just CR). **Needs
  per-agent investigation at task-cut.**

Toggle scope:
* In the rich-prompt toolbar (per-prompt / per-tab); OR
* Settings global default + per-tab override.

### Item D — spawn-agent preflight + identity broadcast

Two new steps after the existing spawn-agent dialog:

1. **Eyeball preflight**: after spawning N agent terminals, the
   user sees a checklist that shows each terminal's current
   output snapshot (so they can verify Claude Code / codex /
   gemini login state). Each row has a "ready" checkbox; user
   confirms each.
2. **Identity broadcast**: a single action sends the canned
   message to every confirmed terminal's PTY:
   `you are $CHAN_TAB_NAME, read docs/agents/bootstrap.md`
   (with each tab's actual `$CHAN_TAB_NAME` substituted client-
   side from the spawn dialog's name input, since the server-
   side env var isn't readable by the SPA).

Open questions:
* Surface: extension of SpawnDialog, or new overlay (cleaner
  for the multi-row UI in item E)?
* Output snapshot: live `read_output` polling on each session,
  or static "press a key to refresh" pattern? Polling is the
  better UX but more wiring.
* Broadcast send: same text-frame WS path as `sendUserInput`,
  N tabs in parallel. No back-end change needed.

### Item E — multi-agent spawn form

Replaces one-at-a-time SpawnDialog with a multi-row form:

```
+---------------------------------------------------------------+
| [Agent name]    [Command]              [Env (KEY=val)]   [x]  |
| [Agent name]    [Command]              [Env (KEY=val)]   [x]  |
| [+ add row]                                                   |
|                                                               |
| [ ] launch in the back (Hybrid back-side)                     |
|                                                               |
|              [ Cancel ]  [ Spawn N agents ]                   |
+---------------------------------------------------------------+
```

* On "Spawn N agents": fires N `api.spawnTerminal` calls in
  parallel. Each tab lands in the active pane (or back-side if
  the checkbox is set — net-new wiring in `tabs.svelte.ts`).
* The "launch in back" path needs to ensure the pane is Hybrid
  (auto-Hybridize if leaf), then push to `pane.back.tabs`
  instead of `pane.tabs`. Composes with `fullstack-a-27`'s
  hamburger flip entry — the user can flip to see the spawned
  agents after the form closes.
* After spawn, the form transitions to item D's preflight
  (eyeball + broadcast), making this a single end-to-end
  flow: rows → spawn → eyeball → broadcast → session ready.

## Coupling with existing work

* **Round-2 chord migration** (Cmd+P rich prompt, in
  `round-2-plan.md`): wave-1 task lands the Cmd+P binding.
  Items A/B/C extend the rich prompt's behaviour AFTER chord
  migration. Sequence: chord migration first, then A/B/C in
  the same lane.
* **Round-2 item 2 (pre-flight + BOOT)**: that work is the
  per-drive pre-flight at chan-server startup. Item B's rich-
  prompt cwd preflight is a parallel pattern at a different
  layer (per-prompt, not per-drive). Same UX vocabulary
  ("preflight checklist") so visual / copy patterns should
  cross-reference.
* **`fullstack-a-27` Hybrid hamburger**: already added "Flip
  pane" entry. Item E's "launch in back" relies on the user
  finding flip via the hamburger; @@Alex's framing ("by then
  they'll have it in the hamburger menu") is satisfied.
* **`fullstack-a-24` floating-pill redesign**: history backlog
  (item A) should compose with the bubble overlay above the
  prompt — same visual language.

## Sequencing recommendation

Round-2 wave 1 already has the chord migration + carousel +
Infographics + BOOT work. This stack is a logical Round-2
wave 2 / wave 3 addition:

| Wave | Task                                                     | Owner       | Approx size |
|------|----------------------------------------------------------|-------------|-------------|
| W2   | A.1: clear-buffer-on-submit (recycle)                    | @@FullStackA | XS          |
| W2   | A.2: chat-style history backlog (.md transcript)         | @@FullStackA | M           |
| W2   | B: cwd preflight on rich-prompt open                     | @@FullStackA | S-M         |
| W2   | C: shell/agent submit-mode toggle                        | @@FullStackB | S-M         |
| W3   | E: multi-row spawn form + "launch in back" wiring        | @@FullStackA | M           |
| W3   | D: spawn-agent eyeball preflight + identity broadcast    | @@FullStackA | M           |

D + E pair tightly (D consumes E's output); land together.

## Survey + decisions (resolved 2026-05-20)

Four topics surveyed via the standard AskUserQuestion flow;
@@Alex picked the architect-recommended option on all four
(history → on-disk, cwd → always-visible header, mode toggle
→ per-prompt toolbar icon, team surface → inside rich
prompt). Survey rendered to a one-line decisions table at
the top of this artifact; locked.

### Implementation notes flowing from the locks

* **`.chan/rich-prompt-history/` layout**: per-drive,
  scoped by tab name + timestamp. Suggested shape:
  `.chan/rich-prompt-history/<tab-name>/<YYYY-MM-DD>-<HHMMSS>-<short-hash>.md`.
  Each `.md` file is one submitted buffer; the file body
  is the buffer text verbatim, with a YAML frontmatter for
  metadata (cwd, mode, submitted-at, submitted-from-tab).
  History panel in the prompt reads the directory listing
  sorted by mtime descending. Goes through `chan-drive`
  for atomic writes (path under `.chan/` is infra, not
  user content — review @@Systacean's call on whether
  the `Drive` helpers cover this or if a sibling helper
  in chan-drive's `index` layer is the right home).

* **Always-visible cwd header**: lives at the top of the
  rich prompt above the composer. Sits in the same band
  as the team-rows once item E lands. Editable text field
  with a path validator (live check against chan-server
  `/api/files?dir=...` or a new endpoint); empty / unset
  state allowed but flagged visually so the user
  understands the prompt will inherit the shell's cwd
  until they set one. Persists per-prompt-session
  (SerTab `rpd?: string` field).

* **Per-prompt toolbar icon for shell/agent mode**: icon
  + label "Shell" / "Agent" with a click to flip. Tooltip
  explains the chord encoding difference. Persists
  per-prompt-session (SerTab `rpsm?: "shell" | "agent"`
  field). Default "shell" (today's behaviour).

* **Rich prompt as team conductor**: the prompt grows three
  new bands above the composer (in order):
  1. **cwd header** (single field; from item B).
  2. **Team rows** (item E: agent-name + command + env per
     row, `+`/`-`, "launch in back" checkbox). Hidden if
     no rows added.
  3. **Eyeball preflight + broadcast** (item D: triggered
     after team rows spawn; shows N tiles with each
     terminal's recent output snapshot + "ready"
     checkbox; "Broadcast identity" button when all
     checked).

  The composer (chat-style history above the prompt + the
  Wysiwyg / Source buffer below) lives BELOW the conductor
  bands. The collapse/expand from `fullstack-a-24` should
  collapse the composer band ONLY — the conductor bands
  stay visible since they're the load-bearing state.

  Open layout question for the implementer: vertical band
  ordering OR a tabbed surface (cwd / team / preflight as
  tabs)? Recommend vertical bands for v1 — matches the
  preflight checklist metaphor + avoids hiding state
  behind tabs. Implementer's call at task time if the
  rich prompt's vertical real estate runs out.

## What this plan is NOT

* A task fan-out. No tasks dispatched until @@Alex confirms
  the survey above.
* A schema. The .md transcript format, the eyeball preflight
  data shape, the broadcast chord encoding are all
  implementation choices for the owning agent at task-cut.
* A commitment that all 5 items land in Round 2. If @@Alex
  defers items D + E (multi-agent flow) to Round 3, the
  rich-prompt evolution (A + B + C) stands alone cleanly.

## 2026-05-21 — Item A (history backlog) animation spec from @@Alex

@@Alex 2026-05-21 added a specific UX shape for item A
(rich-prompt history backlog):

> when we implement the rich prompt history feature later,
> i want to make the following: the rich prompt text area
> will use the same flip effect but here a horizontal flip
> every time the user press cmd+enter.. and it'll come back
> with an empty prompt.. we will have a button that flips
> the same area into the history list of prompts

### Locked animation behaviour

* **On Cmd+Enter submit**: rich-prompt text area performs a
  **horizontal flip** (Y-axis flip; visual side-swap matching
  the `-a-22` Hybrid pane half-flip aesthetic, but rotated
  90° — horizontal axis swap instead of pane flip). Comes
  back with an EMPTY prompt + caret ready. The submitted
  buffer disappears from the composer + lands in the
  on-disk history (per the locked decision: on-disk `.md`
  per drive under `.chan/rich-prompt-history/<tab>/`).
* **History toggle button**: new button on the rich-prompt
  toolbar (next to Send / Collapse / shell-vs-agent toggle
  from `-b-13` / page-width slider from `-a-30`). Click
  flips the SAME area into the history list of prompts
  (chronological, newest at top). Click again flips back
  to the composer.

### Animation primitives to reuse

* `-a-22` pane-flip animation (Y-axis rotate 0° → 90° → 0°
  with mid-point content swap during invisible edge-on
  moment). Item A's submit-flip is the SAME primitive
  applied to the rich-prompt area instead of a pane —
  axis rotated 90° (horizontal instead of vertical).
* `prefers-reduced-motion: reduce` honored (mirror
  `-a-22`'s respect for the user preference).

### Why this shape works

* **Submit-flip provides explicit visual feedback** that the
  buffer was captured + the user can keep typing immediately
  (vs the current "buffer clears with no animation" which
  reads as ambiguous — did the submit happen?).
* **Same-area history view** preserves the screen real
  estate; no popover / drawer / modal disrupts the composer's
  spatial position.
* **Composable with the shell-vs-agent toggle from `-b-13`**
  + the page-width slider from `-a-30` + the collapse
  chevron from `-a-24`. The toolbar gains one more icon
  button; layout stays consistent.

### Sequencing notes

Item A (history backlog with this animation spec) lands as
part of Round-2 wave-2 per the original plan. The flip
animation primitive from `-a-22` is already in HEAD
(committed as `6ed7ebb`); item A reuses it without new
animation infrastructure.

### Task-spec update

When item A's task file cuts at fan-out (`-a-N` slot;
provisionally `-a-42` post-v0.11.2's `-a-36..-41` set),
include this animation spec in the acceptance criteria.
History-list flip-back button placement next to the
existing toolbar buttons (Send / Collapse / submit-mode
toggle / page-width slider).
