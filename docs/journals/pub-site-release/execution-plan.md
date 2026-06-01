# Chan branding rollout - execution plan

Author: Claude (for @@Alex)
Date: 2026-06-01
Status: READY for a later implementation session. The brand direction is
locked in branding-story.md; this is the agent-facing "how to apply it"
to the shipped surfaces. Entry point for agents: read bootstrap.md first.

This applies the locked positioning (chan is an AI-native IDE for plain
files, not a "notes app for markdown workspaces") across README, design,
manual, the agent-contract files, the in-app About slide, and the
marketing site, and publishes a first-person founder /story page.


## How this runs: 3 lanes, 2 waves

Each agent learns its identity from the CHAN_TAB_NAME environment
variable (see bootstrap.md) and does ONLY its lane. Lanes own
non-overlapping files so a shared worktree never cross-contaminates.

```
Lane     Wave  Owns (no file overlap)                     Build/verify
-------  ----  -----------------------------------------  ------------
@@LaneA   1    README.md, design.md, docs/manual/         none (text)
               index.md, CLAUDE.md, AGENTS.md, + sync
               branding-story.md
@@LaneB   1    web/src/components/EmptyPaneCarousel        npm build +
               .svelte (Dashboard About slide)             cargo + smoke
@@LaneC   2    web-marketing/ (home.html, scripts/         site check
   GATED       build.mjs, src/pages/story.html)            (npm run check)
```

- WAVE 1 (@@LaneA + @@LaneB) runs in parallel NOW.
- WAVE 2 (@@LaneC) is GATED: start only after @@Alex drops the new Team
  Work + refreshed screenshots into web-marketing/assets/.
- Fewer-lanes fallback: @@LaneA + @@LaneB can be one agent; @@LaneC stays
  separate and gated.


## Shared rules (every lane)

1. Identity: `echo "$CHAN_TAB_NAME"`. If it is empty or not LaneA/LaneB/
   LaneC, STOP and ask @@Alex. Do not guess your lane.
2. No em dashes anywhere. ASCII tables to 80 cols. Comments explain WHY.
3. Voice by surface: README, design.md, manual, CLAUDE.md, AGENTS.md stay
   factual and plain (no marketing). Brand/marketing voice is allowed
   ONLY on the marketing site and the /story page.
4. Wink retired: keep "sigma", "100x", and "first IDE" (and "the first
   ... IDE") OUT of all new copy. Do NOT edit brainstorm.md; it is
   @@Alex's historical draft and keeps those phrases on purpose.
5. Do NOT kill the running chan.app. Any in-app smoke uses a renamed
   binary copy on a throwaway workspace and a separate port, with any
   pkill scoped to that path. Tear it down after.
6. Do NOT commit or push unless @@Alex says so. If told to commit in the
   shared worktree, stage your lane's paths only and commit with an
   explicit pathspec: `git commit -F msg -- <your paths>` (flags BEFORE
   the `--`). Verify with `git diff --staged --stat` before and
   `git show --stat HEAD` after.
7. Frontend reload reality (LaneB, LaneC site): rust-embed bakes the
   bundle at compile time. For web/src changes: `npm run build` in web/
   FIRST, THEN `cargo build -p chan`, then a hard browser reload. Static
   checks (svelte-check, ?raw vitest) miss Svelte-5 runtime errors, so
   browser-smoke reactive changes.
8. The exact NEW copy below is derived from branding-story.md (the
   locked source). If anything reads wrong in context, prefer
   branding-story.md and flag it to @@Alex rather than improvising.


## WAVE 1


### @@LaneA - positioning text (no build needed)

Five tiny factual swaps plus a doc sync. The one-liner must be identical
across design.md, CLAUDE.md, and AGENTS.md, which is why one lane owns
all of them. Match each file's existing line-wrap width.

--- README.md (top of file) ---

OLD (lines 1-7):
```
# chan

Notes app for plain markdown workspaces. `chan` is a single static binary
that bundles a CLI and a local HTTP server; the server serves a
Svelte WYSIWYG editor that the user edits notes in. Cross-file
`[[wiki-link]]` autocomplete, BM25 + embedding hybrid search, link
graphs, reports, and embedded terminal tabs are built in.
```

NEW:
```
# chan

An AI-native IDE for plain files. `chan` is a single static binary that
bundles a CLI and a local HTTP server; the server serves a keyboard-first
tiling workspace (editor, terminal, multi-agent Team Work, file browser,
graph, dashboard) over a plain folder on disk. Markdown first, source
code too. Cross-file `[[wiki-link]]` autocomplete, BM25 + embedding
hybrid search, link graphs, code reports, and embedded terminal tabs are
built in.

Single-user, single-machine, local-first. The HTTP server binds loopback
by default; an opt-in tunnel publishes the same workspace at
`https://{user}.workspace.chan.app/{workspace}/*` for cross-device access.
```

--- design.md (opening sentence) ---

OLD (lines 3-5):
```
`chan` is the user-facing notes app: a CLI plus an HTTP server that
serves a Svelte WYSIWYG editor for plain markdown workspaces. This
document is the canonical design reference for the workspace.
```

NEW:
```
`chan` is the user-facing AI-native IDE for plain files: a CLI plus an
HTTP server that serves a keyboard-first tiling workspace (editor,
terminal, Team Work, file browser, graph, dashboard) over a plain folder
on disk. This document is the canonical design reference for the
workspace.
```

--- docs/manual/index.md (framing sentence) ---

OLD (lines 3-5):
```
Chan works with plain markdown workspaces. A workspace is a folder on disk that
Chan opens through the desktop app or through the standalone `chan serve`
command.
```

NEW:
```
Chan is an AI-native IDE for plain files. You point it at a folder on disk
(a workspace) and edit, search, graph, run terminals, and drive AI agents
over that tree, through the desktop app or the standalone `chan serve`
command. Your files stay ordinary files.
```

Leave the "What stays on disk" and "What is local" sections below it
unchanged; they already fit the spine.

--- CLAUDE.md and AGENTS.md (identical one-liner, lines 8-10) ---

OLD (both files):
```
`chan` is the user-facing notes app: a CLI plus an HTTP server
that serves an embedded Svelte WYSIWYG editor for plain markdown
workspaces.
```

NEW (rewrap to each file's existing width):
```
`chan` is the user-facing AI-native IDE for plain files: a CLI plus
an HTTP server that serves an embedded keyboard-first tiling workspace
(editor, terminal, Team Work, file browser, graph, dashboard) over a
plain folder on disk.
```

--- branding-story.md sync (this same directory) ---

Confirm section 13 marks the wink (Q3 = retire) and self-descriptions
(Q5 = update all, incl. CLAUDE.md/AGENTS.md) as RESOLVED, and the
founder page as publish-now. Confirm section 14's Self-desc and
Founder-note rows say "decided." (This sync may already be done; verify
and finish any leftover row.)

@@LaneA verification: `git diff` shows exactly these five files plus
branding-story.md changed; no other files; no em dashes; the one-liner
is byte-identical (modulo wrap) across design.md/CLAUDE.md/AGENTS.md;
grep the five files for "notes app", "markdown workspaces", "sigma",
"100x", "first IDE" and confirm zero hits in the new copy.


### @@LaneB - in-app Dashboard About slide (build + smoke)

File: web/src/components/EmptyPaneCarousel.svelte

The About carousel is hardcoded markup blocks (slideCount = 3); the
About slide is `{#if slideIndex === 0}` (around line 467) with a
`<div class="slide-title">About</div>` near line 473. Add, right after
that title, two quiet lines in the slide's existing style (use the
secondary text color, small size; mirror neighboring classes):

```
Simple stroke. Powerful engine.
```
```
An AI-native IDE for plain files: your editor, terminal, and AI agents
in one keyboard-first workspace. Local-first by default.
```

Keep it minimal and factual; this is the lowest-sell surface. Do NOT
expand the carousel to more slides, do NOT touch DashboardTab SLOTS, and
do NOT touch functional UI strings (the spawn tiles / menu rows / license
labels). The launcher subline in EmptyPaneWelcome.svelte is intentionally
NOT changed (the launcher is a working surface).

@@LaneB verification: `cd web && npm run build`, then `cargo build -p
chan` at repo root. Serve a THROWAWAY workspace with a renamed binary on
a separate port (never pkill the shared chan.app), open the Dashboard
tab, confirm the About slide shows the two new lines and the carousel
still cycles through all 3 slides with no Svelte runtime error in the
console. Tear the test server down.


## WAVE 2 (GATED on @@Alex's new Team Work + refreshed screenshots)

### @@LaneC - marketing site + founder /story page

GATE: confirm the new Team Work + refreshed screenshots are in
web-marketing/assets/ before starting. If not, STOP; this wave is gated.

Brand voice is allowed here. Motif to stamp once near the hero:
"Simple stroke. Powerful engine."

--- web-marketing/src/pages/home.html ---

Hero (lines 1-19):
- eyebrow (l.3): `local-first markdown workspaces` -> `the AI-native IDE`
- h1 (l.4): keep `chan`
- lede (l.5-8) NEW:
```
An IDE that moves at the speed of thought, and gets out of your way. One
keyboard-first workspace for your editor, terminal, and AI agents,
powerful underneath with hybrid search, a live graph, and code reports.
Local-first by default, and your files stay ordinary files on disk.
```
- image caption (l.18): `Chan running against a local notes workspace.`
  -> `Chan running against a local workspace.`
- Stamp the motif once near the hero (a small line under the lede or as
  the feature-grid eyebrow).

Feature grid (lines 21-50): keep the 4-card markup/classes; rewrite the
4 cards to the 4 pillars:

```
1. One hybrid UI
   One keyboard-first workspace instead of ten windows. Tiling tabs and
   panes for editor, terminal, Team Work, file browser, graph, and
   dashboard, each with command-line tooling.

2. Bring your own fleet
   Team Work orchestrates multi-agent terminal sessions with the agents
   you already trust: Claude, Codex, Gemini. The MCP server lets them
   read and write the workspace; you conduct the fleet from the terminal.
   No in-app chatbot.

3. Plain files, local-first
   Markdown and source stay ordinary files under the workspace folder,
   with nothing left behind in your project. Loopback by default; an
   opt-in tunnel publishes the same workspace for cross-device access.

4. It understands your workspace
   Hybrid BM25 and embedding search, a multi-layer graph over links,
   tags, and mentions, and per-file and per-directory code reports
   (SLOC, COCOMO), all built in.
```

Keep the existing install split, local-first split, and support section.
Once the Team Work screenshot exists, optionally add a dedicated
"we stopped hiding the agents" image-split (pillar 2) using it; otherwise
pillar 2 lives in the grid card only.

--- web-marketing/scripts/build.mjs ---

- Home `<title>` (l.61): `chan - local-first markdown editor`
  -> `chan - the AI-native IDE`
- Home description (l.62-63) NEW:
```
Chan is an AI-native IDE for plain files: a keyboard-first workspace for
your editor, terminal, and AI agents, with hybrid search, a live graph,
and code reports. Local-first by default.
```
- Register the new story page: add it to requiredInputs and add a
  writePage call (active key "story", its own title/description), mirror
  the existing home/install page registration.
  - story page `<title>`: `Why I built chan`
  - story page description: `Why chan exists, in the maker's words.`
- Add the nav link in renderSiteNav (l.160-165):
  `["story", "/story/", "Story"]`

--- web-marketing/src/pages/story.html (NEW) ---

Render the founder note from founder-note.md as HTML: an `<h1>Why I
built chan</h1>` followed by the prose paragraphs as `<p>` elements.
DROP the internal "Author/Date/Status" front-matter block at the top of
founder-note.md; that is internal only. Match the page wrapper/markup
conventions used by src/pages/install.html. This ships @@Alex's draft;
he may do a voice pass on the live copy afterward.

@@LaneC verification: `cd web-marketing && npm run check` (builds dist +
smokes). Confirm the home page renders the new hero/grid + the new
screenshots, the /story page builds and is linked in the nav, and the
page title/description are updated. Grep the built site for "sigma",
"100x", "first IDE", "notes app", "markdown workspaces" and confirm zero
hits.


## Source docs (read for context)

- branding-story.md - the locked brand decisions (mission as a just
  cause, primary tagline, motif, 4 pillars, voice-by-surface, visual
  identity). The single source of truth.
- founder-note.md - the first-person /story page draft (LaneC).
- brainstorm.md - @@Alex's original seed doc. Historical; do NOT edit.
