# fullstack-a-69 — Rich Prompt F-follow-up rewrite (survey as quote in rich prompt)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Rewrite the "F to Follow Up" behavior per
[`../alex/addendun-a.md`](../alex/addendum-a.md)
"## Rich Prompt enhancements":

* **Scratch today's behavior**.
* **Pressing F (or clicking F to follow up)**: brings
  the current survey as a quote into the rich prompt
  + places cursor on the next line.

## Reference

[`../alex/addendun-a.md`](../alex/addendum-a.md)
verbatim:

> Click F to Follow Up process:
>   - [ ] Whatever it is doing today, scratch it
>   - [ ] Pressing F or clicking F to follow up just brings the current survey as a quote into the rich prompt, and places the cursor on the next line

## Scope

Audit current F-follow-up behavior + replace with the
new shape. New behavior:

1. User looking at a survey bubble (the BubbleOverlay).
2. Presses F (or clicks the F-follow-up affordance).
3. The current survey TEXT gets injected into the
   rich prompt as a markdown quote (each line prefixed
   with `> `).
4. Cursor lands on a fresh new line BELOW the quote.

The user can then type their follow-up freely; the
quoted survey provides context.

## Acceptance

1. **F triggers quote injection**: F key OR
   F-follow-up button click → current survey is
   quoted into the rich prompt.
2. **Quote format**: each survey line prefixed with
   `> ` (markdown quote syntax).
3. **Cursor placement**: ends up on a fresh new line
   immediately below the quote block.
4. **Old behavior removed**: whatever F did before
   (likely survey-reply form or similar) is gone.

### Tests

Vitest pins for the F-chord handler + quote-format
helper + cursor-placement.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.

## Authorization

Yes for rich-prompt + BubbleOverlay-related SPA files
+ tests + task tail + outbound.

## Numbering

This is `-a-69`.

## 2026-05-22 — ready for review

Three-file change. SPA-only.

### Audit verdict

Pre-`-a-69` F-follow-up:
* F key OR follow-up button → `markFollowUp(event)`
  → `commit(event, {}, -1, true)` → server-side
  `writeSurveyReply(...)` with `follow_up: true`.
* Server-side stash marked the survey as
  needing-follow-up; UI rendered a "follow up"
  badge on the bubble.

@@Alex's "scratch today's behavior" framing:
remove the server-side semantic, replace with a
client-only quote-into-the-Rich-Prompt action.

### What landed

`web/src/components/BubbleOverlay.svelte`:

* New `onQuoteToPrompt?: (markdown: string) =>
  void` prop. Mounted from TerminalTab.svelte.
* New `surveyAsQuoteMarkdown(event)` helper —
  formats the survey as markdown quote lines
  (topic / from / per-question header / text /
  options each prefixed with `> `). Falls back
  to `event.note` for non-survey bubbles.
* New `quoteSurveyToPrompt(event)` — wraps the
  formatter + the callback in a single entry
  point.
* F-key handler + follow-up button onclick
  both call `quoteSurveyToPrompt(event)`.
* Removed `markFollowUp` function entirely (no
  remaining UI callers).
* `followUps` state + `follow-badge` UI stay in
  the file as dead code; never set anymore so
  the badge never renders. Cleanup can land in
  a follow-up; removing them in this commit
  would touch `commit()`'s `followUp` param +
  ripple to the chan-server contract on
  `writeSurveyReply`. Out of scope for the
  rewrite per the addendum's narrow framing.

`web/src/components/TerminalTab.svelte`:

* New `quoteIntoRichPrompt(markdown)` function.
  Appends the markdown to `tab.richPrompt.buffer`
  (with `\n\n` separator if the buffer isn't
  empty), opens the rich prompt, bumps
  `focusNonce` so the Wysiwyg/Source re-focus
  + re-mount the new buffer cleanly.
* BubbleOverlay mount passes
  `onQuoteToPrompt={(markdown) => quoteIntoRichPrompt(markdown)}`.

`web/src/components/BubbleOverlay.test.ts`:
* `renderOverlay` helper signature changed to
  accept options object
  (`{ onWatcherDetached?, onQuoteToPrompt? }`)
  so tests can inject a spy for the new prop.
* Two existing tests rewritten:
  * "follow-up click writes async reply" →
    "follow-up click calls onQuoteToPrompt with
    the survey-as-quote markdown". Asserts the
    quote contains topic + question header +
    text + options; asserts NO server reply
    fires.
  * "F marks the focused survey as follow-up
    and a later answer supersedes it" → "F key
    calls onQuoteToPrompt for the focused
    survey; subsequent answer still works".
    The "answer supersedes" expectation is
    gone (no server reply on F); the
    subsequent-answer path is preserved (still
    works via the normal answer flow).

`web/src/components/richPromptFollowUp.test.ts`
(new): 9 raw-source pins covering the helper,
the F-key + button wiring, the markFollowUp
removal, the prop shape, the TerminalTab
callback wiring + buffer-append shape.

### Acceptance

1. **F triggers quote injection** ✓ —
   mechanism via tests; @@WebtestA walk for
   empirical.
2. **Quote format**: each line prefixed with
   `> ` ✓ — topic / from / question header /
   question text / options all properly
   quoted.
3. **Cursor placement**: rich-prompt-injected
   text ends with `\n`; the editor lands the
   caret at the end-of-content position which
   is the new line below the quote ✓.
4. **Old behavior removed** ✓ — markFollowUp
   function gone; no UI call site fires
   writeSurveyReply with `follow_up: true`.

### Gate

* vitest **838 / 838** (+9 net from `-a-71`'s
  829).
* svelte-check 0 errors / 0 warnings across
  4012 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Callback-based prop** for the quote
  injection rather than reaching into the
  global tab state from BubbleOverlay — keeps
  the component decoupled. TerminalTab owns
  the rich-prompt mutation.
* **`focusNonce` bump** to re-mount the
  editor with the new buffer + restore caret
  to end-of-content (where the new line below
  the quote sits).
* **Don't remove `followUps` state +
  `follow-badge` UI** in this commit — they're
  dead code now but their removal ripples to
  `commit()`'s `followUp` param + the
  `writeSurveyReply` chan-server contract.
  Punt cleanup to a follow-up so the rewrite's
  blast radius stays contained.
* **Existing tests rewritten** rather than
  deleted — preserves regression coverage for
  the new behavior + the subsequent-answer
  path.

### Suggested commit subject

```
Rich Prompt F-follow-up: quote current survey into prompt instead of marking server-side (fullstack-a-69)
```

Single commit. Component + callback + helper
+ tests tightly coupled around the rewrite.

### Files for `git add` (per-path discipline)

* `web/src/components/BubbleOverlay.svelte`
* `web/src/components/BubbleOverlay.test.ts`
* `web/src/components/TerminalTab.svelte`
* `web/src/components/richPromptFollowUp.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-69.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
