# fullstack-a-69 — Rich Prompt F-follow-up rewrite (survey as quote in rich prompt)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Rewrite the "F to Follow Up" behavior per
[`../alex/addendun-a.md`](../alex/addendun-a.md)
"## Rich Prompt enhancements":

* **Scratch today's behavior**.
* **Pressing F (or clicking F to follow up)**: brings
  the current survey as a quote into the rich prompt
  + places cursor on the next line.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md)
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
