# Terminal manual page -- task brief (@@LaneE, from @@Host via @@Lead)

@@Host wants a SEPARATE manual page about THE TERMINAL: the embedded terminal,
Rich Prompt, survey, pokes, and the `cs terminal` command family. (He
RETRACTED the earlier "one page with the markdown affordances" idea -- the
@today/@date/mermaid/wiki-links/@{contact} docs STAY where they are; do NOT
fold them in here.)

## Scope of the new page

A terminal-centric chapter covering:
- The embedded terminal (what it is: a real PTY in the workspace; the
  CHAN_MCP_* / CHAN_TAB_NAME env it exports; one terminal owns the agent).
- The `cs terminal` command family -- GROUND each in the actual CLI `--help`
  (run `cs terminal --help`, `cs terminal write --help`, `cs terminal survey
  --help`, `cs terminal scrollback`/`sc`, `cs terminal team ...`). Document
  what each does, the selectors (--tab-name / --tab-group), and the submit
  chord behavior (--submit claude|codex|gemini) the way the --help describes
  it. Do NOT invent flags; mirror the binary.
- Pokes: what a "poke" is (a `cs terminal write --submit` to another tab so a
  running agent receives + submits input hands-free) and the submit-chord
  detail.
- Survey: `cs terminal survey` + its real constraint (it needs a live SPA
  window owning the tab; PTY-only sessions return "no live terminal session
  matched"). State the limitation honestly.

## Rich Prompt + the cs-write queue (IN-FLIGHT -- sequence carefully)

Rich Prompt (the floating Cmd+Shift+P markdown bubble) and the always-on
cs-write QUEUE are being BUILT this round (@@LaneB + @@LaneA). Do NOT
confabulate their behavior. Sequence:
1. Write the SHIPPED parts now (embedded terminal + cs terminal family +
   pokes + survey), grounded in the CLI --help.
2. Leave Rich Prompt + the queue as the LAST sections, written from the specs
   (docs/journals/phase-16/round-1-rich-prompt.md +
   cs-write-queue-design.md) BUT verified against the shipped feature before
   round-close. Flag them clearly as the verify-last sections so they don't
   ship describing behavior that changed during the build. Coordinate the
   timing with @@Lead (I'll tell you when @@LaneB/@@LaneA land).

## Page placement (your call, flag it)

`docs/manual/terminal-and-mcp.md` already exists (terminal + MCP). Decide:
(a) EXTEND terminal-and-mcp.md with the cs/Rich-Prompt/survey/poke material,
    or (b) split a dedicated `terminal.md` and keep MCP separate.
Whichever you pick: update `docs/manual/index.md` (the TOC at :14-15) and fix
any internal links. Lean toward the option that keeps the page coherent and
the TOC clean; you own the manual's shape, so make the call and note it.

## Writing rules (repo)

No em-dashes. ASCII tables targeting 80 cols. Factual, no marketing. Explain
WHY where it helps. `web-marketing` `npm run check` must stay green (manual
renders, no broken links) -- this is the gate for docs.

## Deliverable

Post a design-first outline (the page's section list + the
extend-vs-new-page call) to event-lane-e.md for @@Lead sign-off BEFORE
writing the prose, same as the gateway-guide discipline. Then write the
shipped sections; hold Rich Prompt + queue for the verify-last pass.
