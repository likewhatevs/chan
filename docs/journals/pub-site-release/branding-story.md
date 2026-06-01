# Chan branding story

Author: Claude (for @@Alex)
Date: 2026-06-01
Status: DIRECTION REVISED 2026-06-01 (see banner below). The earlier
four-question pass is preserved for history, but @@Alex re-steered the
positioning after it; where this doc and the revision banner disagree,
the banner wins. README/design/manual/CLAUDE/AGENTS have been updated to
the revised line; the in-app About slide and the marketing site follow
via @@LaneB / @@LaneC.

## REVISION 2026-06-01: re-steered positioning

@@Alex re-steered the brand after the four-question pass. Net changes to
the spine:

- Lead with the AI ENGINE + MULTI-AGENT story, not sovereignty. Modern
  engineers drive projects in MARKDOWN (design docs, specs, tasks); AI
  agents create, review, refine, and harden that work and then execute.
  Chan is a HYBRID, MULTI-AGENT environment where multiple agents (Claude,
  Codex, Gemini) run in the terminal and COORDINATE WITH EACH OTHER via
  `cs` + the MCP server. This promotes the old Pillar 2 to the lead.
- Positioning line: "AI-native IDE for the modern engineer" (was "for
  plain files"). Keep "AI-native".
- "keyboard-first": DROPPED from all copy.
- "local-first" / "plain files" (the sovereignty pillar): DEMOTED to a
  light trust fact (loopback default, opt-in tunnel, files on disk), no
  longer a pillar or a headline. The sovereignty VALUE still lives in the
  mission + founder note, just not in the pitch.
- Editor is MARKDOWN-FIRST. Section 6's "edits source code too" is walked
  back: the editor is "meh" for source; do not sell source editing.
- "first / unique / first-of-a-kind": SHOW, DON'T CLAIM (the section 6
  "first" retire stands and now covers the superlative generally).
- "sigma": stays retired in copy; it is @@Alex's rationale only.
- Motif "Simple stroke. Powerful engine." STAYS; "powerful engine" now
  reads as the AI engine.
- Section 9 pillars are rewritten below; section 13 records these as
  resolved.

The four decisions:

```
Mission    Speed + sovereignty just cause (see section 4). Words still
           iterating; the spine is fixed.
Audience   Inclusive cause, developer-led front door. The just cause
           speaks to everyone who builds; the product front door and
           the feature grid lead developer.
Tagline    Primary: "An IDE that moves at the speed of thought, and
           gets out of your way." Motif: "Simple stroke. Powerful
           engine." "Calm" is demoted to the VISUAL identity only.
Story      First-person "Why I built chan" founder note for the site
           (draft in founder-note.md), plus @@Alex's six motivations
           verbatim as a sourced appendix here (section 15).
```

This doc is written in plain factual prose. Copy written in the brand
voice is fenced and labelled "PROPOSED COPY" so the two never blur.


## 1. The decisions this builds on

You answered four forking questions. Recording them here so the doc
stands on its own:

```
Topic         Choice              What it means
------------  ------------------  --------------------------------------
Voice         Confident,          Aspirational mission + a bold tagline,
              grounded            but no unverifiable claims (no "100x").
                                  Docs stay factual; the site gets room.
Positioning   AI-native IDE       Chan is a dev environment, not just a
                                  notes app. Audience leans developer.
Spine         Zen / ensō          The ensō is an ACT: one decisive
                                  stroke. "Simple stroke. Powerful
                                  engine." is the central metaphor
                                  across every surface (reframed
                                  2026-06-01 from "Calm surface...";
                                  see section 8).
Scope (now)   Story doc first     Review the direction here before I
                                  rewrite any shipped copy.
```

The interesting part is that these three content choices are in
tension on purpose. "AI-native IDE" is a loud, crowded category. A
surface that takes one stroke and then gets out of your way is the
opposite of how IDEs usually present. Resolving that tension IS the
brand.


## 2. The core idea

Chan is the AI-native IDE that moves at the speed of thought and gets
out of your way.

Every other IDE answers "more power" with "more panels, more chrome,
more noise." Chan answers it the way the ensō is drawn: one decisive
stroke, then the engine does the rest. A single binary, a single
workspace, a keyboard-first surface that gets out of your way, and a
powerful engine (search, graph, reports, agents) running underneath.
The name and the mark already say this; the product already looks like
this; the words just have not caught up yet.

  Simple stroke. Powerful engine.

That line is the whole brand compressed: you make one simple stroke
(your intent), the engine executes and delivers. The rest of this doc
unpacks it and shows where each half lands. (Calm is still the FEELING
of the surface and lives in the visual identity, section 11; it is no
longer the verbal pitch.)


## 3. Name and mark: the spine

This is the thread that ties a minimal monochrome UI to a
high-velocity AI mission without either one feeling bolted on.

- chan reads as Chan / Chan Buddhism (the school that became Zen in
  Japan). The associations are presence, economy of motion, and doing
  exactly enough.
- The enso (the circle drawn in a single brush stroke) is already the
  brand mark: it is the launcher logo, the loading spinner, the
  brand-orange favicon (web/index.html, web/src/design.md). It is the
  one warm accent on an otherwise monochrome dark UI.
- The enso means wholeness and the single breath. That maps cleanly to
  the product's hard constraints, which become brand virtues:
  one static binary, one workspace as the boundary, no control files
  left in your project, local-first by default.

So the Zen spine is not decoration we are adding. It is the honest
description of decisions already made in the code, given a name.

Practical guardrail: lean on the FEELING (calm, single breath, just
enough, gets out of the way), not on Buddhist vocabulary. We say
"calm" and "out of your way," not "achieve enlightenment." Zen is the
posture, not the punchline.


## 4. Mission (the just cause)

DECIDED: the mission is framed as a Simon Sinek "infinite game" just
cause, not a feature promise. A just cause is the horizon a community
moves toward and never "wins"; it should be simple, powerful, and
ultimately unachievable. The brainstorm line "make the world move
faster with artificial intelligence" had the right ambition but read
like a generic AI slogan. The earlier A/B/C candidates were product
promises, not causes; they are retired. The locked spine braids
@@Alex's three deepest values from the founder story: speed (speed of
thought), partnership (AI as a partner, bring your own agent), and
sovereignty (lifelong Linux/open-source roots, local-first, plain
files, run your own, nothing left behind).

PROPOSED COPY (mission, spine locked, wording still iterating):

```
A world where anyone can build at the speed of thought, with AI as a
partner and without ever giving up control of their tools or their
work.
```

Sinek just-cause check:

```
Criterion    How this cause meets it
-----------  ---------------------------------------------------------
For (not      "A world where..." - an affirmative future to move
 against)     toward, not an enemy to beat.
Inclusive     "anyone can build" - open to everyone who builds
              anything, not just developers (see section 5, audience).
Service       "their tools, their work" - the benefit is the
              builder's, not the company's.
Resilient     "AI as a partner" is tech-agnostic; the cause (speed +
              sovereignty) survives whatever the agents become.
Idealistic    "anyone... at the speed of thought" is an absolute we
              can always approach and never finish. Deliberately
              unachievable, so the community keeps moving.
```

Why this over a tighter line: a sovereignty-forward variant ("the
fastest way to build with AI is the one that keeps your work entirely
yours") and a timeless craft variant ("the tools you think in never
slow you down") were both on the table. @@Alex chose the speed +
sovereignty braid as the spine; the two alternates remain as dials if
later refinement wants more ownership-forward or more tech-agnostic
phrasing.

NOTE: the spine is fixed; the exact words above are a starting point
to refine with @@Alex, not final hero copy.


## 5. Positioning statement

The internal one-liner that every surface inherits from, in the
classic for/unlike form:

PROPOSED COPY (positioning):

```
For builders who live in the keyboard, Chan is an AI-native IDE
for plain files: a calm, tiling workspace that unifies a
markdown-and-code editor, persistent terminals, multi-agent
sessions, a workspace graph, and hybrid search in one local-first
binary. Unlike heavyweight IDEs that answer power with clutter,
Chan keeps the surface quiet and leaves nothing behind in your
project.
```

This is not shipped copy. It is the source the hero, the README line,
and the app's About slide all compress from, so they agree.

DECIDED (audience altitude): the brand runs at two altitudes, and they
do not fight.

```
Altitude        Speaks to            Lives in
--------------  -------------------  -------------------------------
Mission /        everyone who         The just cause (section 4) and
just cause       builds anything      the founder note. Inclusive by
                                      design: @@Alex's own use spans
                                      engineering, taxes, recipes,
                                      family trips, local and remote.
Positioning /    developers first     The hero, the four-pillar grid,
front door                            the screenshots (Team Work, MCP,
                                      terminal, code reports). The
                                      product's beachhead is dev-shaped.
```

So the "whole working life" universality is real and it lives in the
mission + founder story, while the feature grid and hero lead
developer. One brand, two registers; the positioning statement above
is the developer-front-door source.

## 6. The honest claim: what "IDE" does and does not mean here

You chose the AI-native IDE positioning. With a developer audience,
the fastest way to lose trust is to claim a category and then miss its
table stakes. So we define the term on our own terms and we are honest
about the edges.

What Chan means by IDE (integrated, by evidence in the product):

- One environment for editor + terminal + agents + graph + search +
  reports, in a tiling tab/pane UI.
- Edits markdown first, and edits source code too (the screenshots
  show C with syntax highlighting, per-file and per-directory code and
  language reports, SLOC and COCOMO).
- Persistent server-side terminal sessions (tmux -CC feel) with
  broadcast groups.
- Agents are first-class: Team Work orchestrates multi-agent terminal
  sessions; the in-process MCP server exposes the workspace to Claude,
  Codex, and Gemini.

What Chan is deliberately NOT (say this, do not hide it):

- Not a heavyweight, language-server-per-language, debugger-and-
  refactoring-suite IDE. Calm is a feature, not a missing checklist.
- Not cloud-required. It is local-first; the cloud (tunnel, gateway)
  is opt-in.
- Not an in-app chatbot. There is no in-app agent UI; agents connect
  through MCP and the terminal. (This is a differentiator, not a gap:
  bring the agent you already trust.)

Two words from the brainstorm to retire on shipped surfaces:

- "first" (as in "the first IDE for..."): unverifiable superlative,
  invites a fact-check we lose. Drop it.
- "sigma generation" and "100x": funny in a brainstorm, but they read
  as unserious to the exact developer audience we are courting, and
  "100x" is a number we cannot defend. Retire from durable copy. If
  you want one wink, the lowest-risk home for it is a single line deep
  in the manual or a code comment, your call, not the hero.


@@Alex: This all makes sense and I like to have these "what is" and "what is not" well clear to new users who will be discovering Chan soon.

## 7. Voice and tone

Confident, grounded, calm, precise. We earn the bold tagline by being
exact everywhere else.

```
Do                              Don't
------------------------------  ------------------------------------
State what it does, plainly.    Claim multipliers ("100x", "10x").
Let real numbers speak (BM25,   Use hype adjectives ("revolution-
SLOC, COCOMO, one binary).      ary", "game-changing", "insane").
Short sentences. White space.   Stack three buzzwords in a row.
"Gets out of your way."         "Unleash your potential."
Name the agents (Claude,        Imply Chan replaces the developer.
Codex, Gemini) as partners.
Say "local-first," mean it.     Imply AI or cloud is required.
```

Tonal range by surface (same brand, different register):

```
Surface           Register          Latitude for the bold voice
----------------  ----------------  ----------------------------------
Marketing site    Confident, warm   High. Hero + taglines live here.
In-app copy       Calm, minimal     Low. Micro-copy; never sell to a
                                    user who already installed.
README            Technical         Low. One accurate positioning
                                    line, then engineering facts.
Manual            Instructional     None. Task-first, plain, factual.
Gateway pages     Plain, trusted    Low. It handles accounts/tokens.
```

This resolves the apparent conflict with CLAUDE.md ("no marketing
language in comments or documentation"). That rule governs the docs
and the code. The marketing site is the one surface whose job IS
marketing, so it gets the latitude. README and manual stay under the
rule.


## 8. Tagline system

DECIDED. The earlier primary "The AI-native IDE that stays calm" is
retired: @@Alex flagged "stays calm" as passive and weird, and "calm"
as overindexed across the set. The fix is the enso reframe. The enso
is not a serene picture; it is an ACT, one decisive brush stroke. So:
the user makes one simple stroke (intent) and the engine executes and
delivers. "Calm" now lives only in the VISUAL identity (section 11:
monochrome, white space, the single orange mark); the verbal taglines
lead with the act and the velocity, not with serenity.

PROPOSED COPY (taglines):

```
Primary (hero lede / subhead):
  An IDE that moves at the speed of thought, and gets out of
  your way.

Repeating motif (section dividers, social card, About slide):
  Simple stroke. Powerful engine.

Spine-forward alternates (same act, different register):
  One stroke in. The engine does the rest.
  Your editor, terminal, and agents. One workspace, all yours.

Plain-files-forward alternate:
  The AI-native IDE for plain files.

Short lockup (nav, social, footer):
  chan - the AI-native IDE
```

Recommendation, locked: lead with "An IDE that moves at the speed of
thought, and gets out of your way." (the line @@Alex endorsed) as the
hero subhead, and stamp "Simple stroke. Powerful engine." everywhere
as the repeating motif. The first carries the velocity and the
get-out-of-the-way differentiator; the second is the enso act in four
words (one decisive stroke -> powerful delivery) and replaces the old
"Calm surface. Powerful engine." Note it keeps "powerful engine,"
which @@Alex did not object to, and swaps the passive "calm surface"
for the active "simple stroke."


## 9. Brand pillars (the "what do I get")

The brainstorm's long bullet list reorganized into four pillars. Each
pillar has a claim (voice), the evidence (so we can defend it), and
where it shows up. These four become the site's feature grid, the
README feature list, and the app's About carousel, all from one
source.

REVISED 2026-06-01: the pillars are reordered and relabeled. AI + the
multi-agent fleet now lead; the old "Plain files, local-first" pillar is
demoted to a light trust fact (not a pillar). Shipped wording lives in
execution-plan.md; the headline set is now:

```
1. AI is the engine       agents create/review/refine/harden/execute the
                          Markdown docs and tasks that drive the project.
                          (Markdown-first.)
2. A fleet that works     Claude, Codex, Gemini run in the terminal and
   together               coordinate with EACH OTHER via cs + MCP. You
                          conduct; they collaborate. No in-app chatbot.
3. One hybrid workspace   editor, terminal, Team Work, file browser,
                          graph, dashboard as tiling tabs/panes.
4. Knows your workspace   hybrid BM25 + embedding search, live graph,
                          code reports (SLOC, COCOMO).
```

The PILLAR 1-4 blocks below are retained for their evidence notes, but
where they say "keyboard-first" or treat plain-files/local-first as a
headline pillar, the revision above wins. The old Pillar 2 ("we stopped
hiding the agents") is the spirit of the new #1 and #2.

PILLAR 1 - One hybrid UI, one workspace

```
Claim:    One keyboard-first workspace instead of ten windows.
Evidence: Tiling tabs and panes; tab types Editor, Terminal, Team
          Work, File Browser, Graph, Dashboard. Inspired by a mix of
          hyprland tiling and iTerm2 panes. Every surface also ships
          CLI tooling for ad-hoc use and automation.
Shows up: Site hero + grid; launcher screen; README intro.
```

PILLAR 2 - We stopped hiding the agents (bring your own fleet)

This is the standout differentiator and gets its own narrative beat on
the site, not a grid cell. The origin (founder note, motivation 4): the
journey ran from API integrations, to brute-force headless-agent
embedding, to giving up on hiding the agents and making the terminal
(the pty) a first-class orchestration substrate. The "TUI wars" got
settled by accepting the terminal instead of fighting it. So agents are
not a chatbot bolted into a sidebar; they are a fleet you conduct.

```
Claim:    Agents are first-class, and they are the agents you already
          trust. You conduct a fleet, you do not talk to a sidebar.
Evidence: Team Work orchestrates multi-agent terminal sessions (Claude,
          Codex, Gemini) and works out of the box on any project. The
          terminal is the orchestration layer: persistent server-side
          sessions (tmux -CC feel), broadcast groups, and `cs` CLI
          tooling let agents poke and write to each other. The
          in-process MCP server lets agents READ AND WRITE the
          workspace (read_file, write_file, list_files, search_content,
          graph, reports) over a Unix socket; terminals export CHAN_MCP_*
          discovery. No in-app chatbot, by design and by hard-won
          conviction.
Shows up: Site "we stopped hiding the agents" split + Team Work shot;
          manual terminal-and-mcp.
```

Precision guardrail: MCP exposes the WORKSPACE to agents (content,
search, graph, reports, writes). Fleet ORCHESTRATION (broadcast, groups,
spawning, poking) is the terminal + `cs`, not MCP. Do not write "agents
drive the UI through MCP" in shipped copy; that overclaims the MCP
surface. The accurate, and cleaner, line is: "agents read and write your
workspace through MCP; you conduct the fleet from the terminal."

PILLAR 3 - Plain files, local-first

```
Claim:    Your work stays yours: plain files on disk, nothing left
          behind in your project.
Evidence: Markdown and source on disk; workspace is the sandboxed
          boundary; Chan stores no control files inside your project
          tree. Loopback by default, per-launch bearer token; opt-in
          tunnel via workspace.chan.app; the gateway is self-hostable.
Shows up: Site "local by default" section; README; manual tunnel.
```

PILLAR 4 - It understands your workspace

```
Claim:    Search, graph, and reports that actually know your tree.
Evidence: Hybrid search (BM25 + BGE-small embeddings, bundled in the
          binary); multi-layer graph rooted on the filesystem tree,
          layered by markdown links, contacts/mentions, hashtags, and
          per-file/per-directory code and language reports (SLOC,
          COCOMO). Dashboard carousel surfaces workspace and add-on
          status.
Shows up: Site grid; Graph + Search + Dashboard screenshots; manual.
```


## 10. Message map per surface

How the one story renders on each surface. This is the part that
"communicate this branding throughout" asks for.

### 10.1 Marketing site (chan.app)

Boldest register. The brainstorm's structure is mostly right; it just
needs the grounded voice and the spine.

Hero, replacing the current "local-first markdown workspaces / A
desktop and CLI editor for plain markdown folders":

PROPOSED COPY (home hero, decided):

```
eyebrow:  the AI-native IDE
h1:       chan
lede:     An IDE that moves at the speed of thought, and gets out of
          your way. One keyboard-first workspace for your editor,
          terminal, and AI agents. Powerful underneath: hybrid search,
          a live workspace graph, code reports. Local-first by default.
buttons:  [ Install ]  [ Read the manual ]
```

Motif stamped near the hero and on section dividers: "Simple stroke.
Powerful engine."

Section flow (maps brainstorm -> site, all four pillars):

```
1. Hero (above)                         spine + category
2. Feature grid = the 4 pillars         the "what do I get"
3. "We stopped hiding the agents" split pillar 2, the differentiator
   (Team Work shot)
4. Local-first split (loopback/tunnel)  pillar 3, trust
5. Run your own (gateway is OSS)         self-host, dev credibility
6. Install (desktop-first, CLI too)     existing section, keep
7. Support / donate                      existing section, keep
```

Screenshots: the brainstorm already flags two TODOs that block launch:
refresh the Team Work shot with the real prompt (image-8), and swap
the name/avatar on the id.chan.app shots (image-10, image-11). Note
these as launch-blockers, not copy.

<title> and <meta description> in base.html should move off any
"markdown folders" phrasing to the new positioning line so search and
social previews match.

### 10.2 Chan itself (in-app)

Lowest-sell register. The user already installed; in-app copy should
reassure and orient, never market. The spine shows up as restraint:
the enso, the monochrome calm, the white space. Touch points:

```
Surface              Today / role            Proposed direction
-------------------  ----------------------  -----------------------
Launcher screen      enso + workspace name   Keep. Optionally one
(App.svelte,         + action tiles          quiet line under the
EmptyPaneWelcome)    (Terminal, Team Work,   name in the brand
                     File Browser, Graph,    voice; calm, not a
                     Search, Dashboard)      pitch.
Dashboard "About"    Carousel empty-state    This is the in-app home
carousel slide       UX surface              for "Simple stroke.
(DashboardTab,       (web/src state)         Powerful engine." + the
EmptyPaneCarousel)                           4 pillars, one per
                                             slide, factual phrasing.
Empty pane hints     shortcut hints          Keep functional; the
                                             calm IS the message.
```

PROPOSED COPY (optional launcher subline, under workspace name):

```
Simple stroke. Powerful engine.
```

Guardrail: functional UI strings (buttons, errors, menus) stay
literal. Brand voice lives only in the deliberately "about" surfaces
(the About carousel, maybe one launcher subline). Do not sprinkle
taglines into the working UI.

### 10.3 README (github.com/fiorix/chan)

Technical/contributor audience. Under the CLAUDE.md no-marketing rule.
The only branding move is to fix the opening so the project does not
describe itself as a "notes app" while the screenshots show an IDE.
Everything below the first paragraph stays engineering prose.

Today:

```
# chan
Notes app for plain markdown workspaces. ...
```

PROPOSED COPY (README opener):

```
# chan

An AI-native IDE for plain files: a single static binary that
bundles a CLI and a local HTTP server, serving a keyboard-first
tiling workspace (editor, terminal, multi-agent Team Work, file
browser, graph, dashboard) over a plain folder on disk. Markdown
first, source code too. Hybrid BM25 + embedding search, a live
workspace graph, and code reports are built in.

Single-user, single-machine, local-first. Loopback HTTP by
default; an opt-in tunnel publishes the same workspace at
https://{user}.workspace.chan.app/{workspace}/* for cross-device
access.
```

design.md line 3 ("the user-facing notes app") should change in the
same spirit so the canonical design reference agrees with the README.
CLAUDE.md's own one-line description ("the user-facing notes app")
can stay as-is or follow; flag for your call since it is the agent
contract file, not user-facing.

### 10.4 Manual (docs/manual)

End-user instructional. No marketing latitude. The only change is the
framing sentence on the landing page so a reader is not told Chan is
"plain markdown workspaces" when they downloaded an IDE. The
how-to pages (install, workspaces, wiki-links, search, terminal,
tunnel, upgrade) stay exactly as factual as they are.

Today (index.md):

```
Chan works with plain markdown workspaces. A workspace is a folder
on disk that Chan opens through the desktop app or ... chan serve.
```

PROPOSED COPY (manual index framing):

```
Chan is an AI-native IDE for plain files. You point it at a folder
on disk (a workspace) and edit, search, graph, run terminals, and
drive AI agents over that tree, through the desktop app or the
standalone chan serve command. Your files stay ordinary files.
```

The "What stays on disk" and "What is local" sections below it are
already perfect for the spine (plain files, local by default). Keep.

### 10.5 Gateway (id.chan.app, workspace.chan.app)

Account, sign-in, tokens, workspace proxy. Plain and trustworthy
register; it handles credentials. Branding touch is light: consistent
"chan" lockup and enso in the header (already present per image-10),
and one factual line that it is open source and self-hostable, which
doubles as developer-trust signal and matches the brainstorm's "You
can run your own!" note. No taglines on auth screens.


## 11. Visual identity (carry, do not reinvent)

The visuals already say "calm." Codify, do not redesign.

```
Element       Current state              Keep / note
------------  -------------------------  --------------------------
Mark          Orange enso                Keep. The one warm accent;
                                         it is the whole identity.
Palette       Monochrome dark UI,        Keep. Calm = restraint +
              orange brand accent,       white space. Do not add a
              semantic hues per concept  second brand color.
              (web/src/design.md)
Motion        enso as loading spinner    Keep. The single breath in
                                         motion is on-brand.
Type/space    minimal, generous space    Keep. Space is the message.
Marketing     light + dark site themes   Keep dark as the hero;
site                                     screenshots are dark, which
                                         already reads "calm IDE."
```


## 12. Risks and things to flag

```
Risk                          Mitigation / call to make
----------------------------  ------------------------------------
"AI-native IDE" overpromise   Section 6 honest-edges copy; never
to a dev audience.            imply debugger/LSP-suite parity.
Implying AI is required.      Always pair "born with AI" with
                              "local-first," "works offline."
SEO/name collision: "chan"    Brand the full "chan - the AI-native
reads as the -chan honorific  IDE" lockup; lean on chan.app + the
or imageboards.               enso in metadata.
"first" / "100x" / "sigma"    RETIRED from durable copy (section 6,
fact-check and credibility.   Q3). No easter egg; brainstorm.md keeps
                              them as the historical draft.
CLAUDE.md no-marketing rule.  Resolved: site is the marketing
                              surface; README/manual stay factual.
Launch-blocking screenshots.  image-8 Team Work prompt is a TODO;
                              image-10/11 need name+avatar swap.
```


## 13. Decisions made, and what is still open

RESOLVED 2026-06-01 (the four-question pass):

```
1. Tagline      Primary = "An IDE that moves at the speed of thought,
                and gets out of your way." Motif = "Simple stroke.
                Powerful engine." Old "stays calm" retired. (Section 8)
2. Mission      Speed + sovereignty just cause, framed as an infinite
                game. Spine locked; words still iterating. (Section 4)
3. The wink     RETIRE. "sigma" / "100x" / "first IDE" stay out of all
                durable copy. brainstorm.md keeps them as the historical
                draft; no easter egg. (Section 6)
4. Audience     Inclusive cause, developer-led front door. (Section 5)
5. Self-desc    UPDATE ALL. README, design.md, manual, the site, AND the
                CLAUDE.md / AGENTS.md agent-contract one-liners move to
                the AI-native-IDE-for-plain-files positioning. (Sec. 10)
6. Story        First-person founder note (founder-note.md) + the six
                motivations as a sourced appendix; the /story page
                publishes from the draft. (Section 15)
```

RESOLVED 2026-06-01 (re-steer, supersedes parts of the above):

```
7.  Position    "AI-native IDE for the modern engineer" (not "for plain
                files"). Keep "AI-native". (Banner, sec 2/5)
8.  Lead story  AI is the engine + multi-agent fleet coordinating in the
                terminal; Markdown-driven work. Old Pillar 2 leads.
9.  keyboard    DROP "keyboard-first" from all copy.
10. local/plain DEMOTE local-first/plain-files to a light trust fact; no
                longer a pillar. Sovereignty value lives in mission +
                founder note only.
11. Editor      MARKDOWN-FIRST; do not sell source editing (sec 6 "edits
                source too" walked back; editor is "meh" for source).
12. first/uniq  SHOW, DON'T CLAIM; no "first/unique" superlative.
13. sigma       Stays retired in copy; rationale only.
14. Screenshots Team Work shots staged in web-marketing/assets/; stale
                editor-*.png deleted.
```

STILL OPEN:

```
6. Mission      Exact wording of the just cause (the spine is fixed).
   words
7. Pillar       Anything Chan does today that the four pillars still
   accuracy      undersell or misstate? Pillar 2 was rewritten and the
                MCP-vs-terminal line corrected; @@Alex knows the product
                best, so a final read is worth it.
```


## 14. Apply plan (direction approved; apply pass staged)

The direction is locked (section 13). The apply pass is mechanical and
small, grouped so each is a clean commit. Hero/motif/mission below are
the decided copy, not candidates:

```
Group        Files                                  Change
-----------  -------------------------------------  ------------------
Site copy    web-marketing/src/pages/home.html      hero (eyebrow/h1/
             web-marketing/src/templates/base.html  lede=speed-of-
             (build.mjs page <title>/<desc> data)   thought) + 4-pillar
                                                    grid + "we stopped
                                                    hiding the agents"
                                                    split (Pillar 2) +
                                                    founder-note link;
                                                    title/desc off
                                                    "markdown folders"
README       README.md (opener), design.md (l.3)    positioning line,
                                                    factual (no mktg)
Manual       docs/manual/index.md                   framing sentence
In-app       web/src DashboardTab/EmptyPaneCarousel  About-slide copy
             web/src App.svelte/EmptyPaneWelcome     (motif + 4 pillars,
                                                    factual); optional
                                                    launcher subline.
                                                    Calm/motif only in
                                                    About, never working
                                                    UI strings.
Founder      web-marketing /story page              publish from the
note         (from founder-note.md, @@Alex voice)   draft (decided);
                                                    rides Wave 2 w/ site
Self-desc    CLAUDE.md, AGENTS.md one-liners         UPDATE too (decided,
                                                    Q5); same line as
                                                    design.md
```

Execution is staged into 3 lanes / 2 waves; see execution-plan.md +
bootstrap.md in this directory. Wave 2 (the site) is GATED on @@Alex's
new Team Work + refreshed screenshots.

Notes for the apply pass, from project memory and CLAUDE.md:
- The marketing site has its own gate: run npm run check in
  web-marketing before claiming the site builds.
- Any web/src in-app copy change needs the full reload cycle (npm run
  build in web/, cargo build -p chan, restart) and a browser smoke;
  static checks miss Svelte runtime issues.
- No em dashes, ASCII tables to 80 cols, factual in README/manual.
- Screenshot refresh (image-8 prompt, image-10/11 name+avatar) is a
  separate launch task, not part of the copy commits.
- In-app smoke must NOT kill the running chan.app: build a renamed
  binary, serve a throwaway workspace on a separate port, scope any
  pkill to that path, tear it down. Leave @@Alex's live app alone.


## 15. Motivations and inspiration (source, @@Alex)

@@Alex's own account of why Chan exists, captured verbatim-in-spirit
as the source the mission, the pillars, and the founder note all draw
from. Lightly cleaned for readability; the substance is his. (No
implication that all of this is public copy; this is the well the
public copy is drawn from.)

The through-line: Chan is the tool a lifelong systems person built so
he would stop switching tools. One environment for a whole working
life, local or remote, with agents as a native layer rather than a
bolted-on chat box.

1. ONE TOOL FOR A WHOLE WORKING LIFE. An engineering manager with a
   long career and deep Linux roots since the mid-90s, who writes
   documents constantly. In 2026 he needed a portable, reliable editor
   for work and home, spanning engineering (code, design docs) and
   ordinary life (taxes, cooking recipes, organising family trips). It
   has to work across all those environments, local and remote: on the
   laptop, on a remote machine over HTTP, inbound or outbound (tunnels).

2. A DECENT TERMINAL. A lifelong console user: DOS, bare tty on Linux
   for years, window managers (WindowMaker, AfterStep, XFCE, contributed
   to XFCE's xfsound in 1998), and a lot of GTK code in the late 90s and
   early 2000s for video applications. Always needed a decent terminal
   to write code and orchestrate builds. Today he needs it to operate
   multiple agent instances (Claude, Codex, Gemini) across hybrid
   environments, because different agents do different work: at home,
   managed by cost; at work, different agents for different tasks; plus
   hybrid coding sessions (e.g. 4 Claude, 2 Codex). A long-time iTerm2
   and tmux -CC user; Chan's terminal is heavily inspired by that combo,
   with its own broadcast and groups and command-line tooling to manage
   and interact with the terminals.

3. THE SECOND BRAIN. Retaining and feeding project-related information
   to the agents. Projects like qmd were inspirational and the original
   seed of Chan's early days as a text editor. Chan now provides its own
   MCP server so agents can interact with the workspace content (and the
   terminal/`cs` tooling drives orchestration). This enables powerful,
   never-seen-before automation and orchestration.

4. DECENT LLM INTEGRATION. A long journey: first API support for
   Anthropic, Gemini, OpenAI; then learning the hard integration points
   compared to local TUI agents; then brute-force attempts to integrate
   headless agents into the editor; and finally giving up on hiding the
   agents and adding a full-featured terminal so the "TUI wars" could be
   settled with the TUI as a fundamental layer of Chan. Instead of
   fighting, he accepted that having a terminal mattered more than trying
   to hide the agents. Because they run on a pty, that became the
   orchestration layer: now agents can write to each other, poke, and so
   on.

5. FILE BROWSER. A necessity that turned out to be a very decent
   addition. The integration between the `cs` command line, the file
   browser, and the rest of Chan makes the work fluid when these tools
   are combined. The inspector was born in the file browser and is now
   present in most hybrid tabs.

6. THE HYBRID ITSELF. An absurd idea that turned out to work extremely
   well for a fluid workflow: a widget that mixes tabs of different kinds
   that all interoperate, for a seamless local and remote experience over
   something that does NOT feel like a web browser, and that even
   survives window reloads. (Note: the tail of the original capture was
   garbled by a terminal glitch; this is the reconstructed intent. Worth
   confirming with @@Alex.)
