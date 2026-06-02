# Terminal manual page - design-first outline (@@LaneE)

For @@Lead sign-off BEFORE prose, per the gateway-guide discipline. Brief:
round-1-part-e-terminal-page.md.

## Extend-vs-new call (my decision, flag for override)

RENAME `docs/manual/terminal-and-mcp.md` -> `docs/manual/terminal.md` and
make it the one comprehensive Terminal chapter, with MCP discovery folded
in as a section (not split off). Reasons:
- The terminal is ONE surface; its env exports (CHAN_TAB_NAME/GROUP AND
  CHAN_MCP_*) belong on one page. Splitting them (brief option b) would
  fragment "what the terminal exports".
- The cs family, pokes, and survey all operate ON the terminal -> one
  coherent chapter, one clean TOC entry.
- Pre-launch is the right time to rename (no external bookmarks yet). The
  ONLY inbound link is index.md:17; I update its label + target. (A
  historical journal mention in pub-site-release/branding-story.md:423 is
  a closed retrospective - leave it.)

If you'd rather keep the filename `terminal-and-mcp.md` (zero URL churn), I
extend it in place instead and only relabel the TOC. Your call; default =
rename.

## Page section list

SHIPPED (write now, grounded in CLI --help / source):
1. The embedded terminal. A real PTY rooted at the workspace; one terminal
   owns one agent. Env it exports: CHAN_TAB_NAME, CHAN_TAB_GROUP (default
   "default"), and the CHAN_MCP_* discovery set when the MCP bridge is up.
2. The `cs terminal` command family (prefix matching: `cs t n/w/l`). One
   subsection or a compact table per command, each mirroring its --help:
   - `new [PATH]` (--tab-name, --tab-group): open a tab.
   - `list` (--json): live sessions grouped by group.
   - `write` (--tab-name/--tab-group, --submit claude|codex|gemini,
     --stdin): raw bytes to sessions; no newline appended.
   - `restart` (--tab-name/--tab-group): relaunch preserving spawn cmd+env.
   - `scrollback` (--tab-name ONLY, no group axis): dump a session's replay
     ring to stdout. [merged 8b21edd9; grounded in source - installed cs is
     stale, I re-verify --help on a fresh binary before final.]
   - `team new|load` (--script): the CLI form of the Cmd+P team dialog.
   Selectors: --tab-name (one) / --tab-group (broadcast). "At least one
   selector required" for write/restart/survey.
3. Pokes. A poke = `cs terminal write --submit <agent>` to another tab so a
   running agent receives AND submits the bytes hands-free. The submit
   chord differs per agent (claude = the Cmd+Enter modifyOtherKeys CSI;
   codex/gemini = plain CR); omitting --submit parks the bytes unsubmitted.
4. Survey. `cs terminal survey` raises a blocking survey over the SPA
   window that OWNS the target tab and prints the chosen option (or, with
   --followup, the [F] followup file path). Flags: --title, --option (1..4),
   --followup/--followup-dir/--from/--to, --stdin. HONEST LIMIT: it needs a
   live SPA window owning the tab; a PTY-only session (no window) matches
   nothing. [Verify the exact "no live terminal session matched"-style
   error string against the binary before final.]
5. MCP discovery (folded from the old page). The CHAN_MCP_* env set;
   external agents translate it into their own MCP config. External agents
   only - no in-app chat / assistant HTTP API.

VERIFY-LAST (in-flight; @@LaneB/@@LaneA; write from spec, HOLD for sign-off
when they land, verify against the shipped feature before round-close):
6. Rich Prompt (the floating Cmd+Shift+P markdown bubble) - spec:
   round-1-rich-prompt.md. @@LaneB.
7. The always-on cs-write queue - spec: cs-write-queue-design.md. @@LaneA.
   These two stay clearly marked as the verify-last sections so we don't
   ship behavior that changed during the build.

## Out of scope for this page

`cs pane` (C3, merged de5dcbfd/c118bdc1) is a separate window/pane command
family, not in the brief's terminal scope. Note its existence at most;
don't document it here.

## Verify

web-marketing `npm run check` green (manual renders, no broken links) is
the docs gate. I run it after the rename + prose.
