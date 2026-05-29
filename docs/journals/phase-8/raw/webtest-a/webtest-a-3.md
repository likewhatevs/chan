# webtest-a-3 — `-a-43` Hybrid back-side refactor + `-b-23` web-marketing static site walkthrough

Owner: @@WebtestA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Walk two pieces of Round-2 wave-2 work that landed in HEAD
after the v0.11.2 cut:

1. **`fullstack-a-43`** (HEAD `b36ca96`) — Hybrid back-side
   architecture refactor: per-surface config view. The
   back side of a Hybrid pane stops being "another tab
   collection" and becomes a per-surface configuration
   surface keyed to the active front tab type
   (Terminal / Editor / Graph / File Browser).
2. **`fullstack-b-23`** (HEAD `bc9e1f8`) — chan.app
   marketing site source ported into `web-marketing/`
   including the donation QR placement.

These shipped pre-recycle and have no walkthrough verdict
yet. @@WebtestA is the reactive lane; this is the first
wave-3 dispatch for you post-recycle.

## Background

### `-a-43` Hybrid back-side refactor

@@FullStackA's commit-clearance poke is at the tail of
[`../alex/event-fullstack-a-architect.md`](../alex/event-fullstack-a-architect.md)
"2026-05-21 — Task A — Hybrid back-side architecture
refactor: cleared for review (suggested commit)" / the
follow-up "committed at `b36ca96`".

Implementation note in
[`../fullstack-a/fullstack-a-43.md`](../fullstack-a/fullstack-a-43.md)
tail. Single commit; 11 files (5 modified + 4 new
`HybridXConfig.svelte` stubs + the task file + the
fullstack-a journal).

Design context in `architect/round-2-plan.md` §"Hybrid
back-side revisited" + journal entry "2026-05-21 — design
decision: Hybrid back-side becomes per-surface config".

What changed:

* Pane.svelte's flip behaviour reads the active front-tab
  type + mounts the matching back-side component.
* Four new components: `HybridTerminalConfig.svelte`,
  `HybridEditorConfig.svelte`, `HybridGraphConfig.svelte`,
  `HybridFileBrowserConfig.svelte` — STUBS in this task.
* Front/back independent theme dropped — both sides of a
  Hybrid share a single per-Hybrid theme value.

What did NOT change (yet):

* Settings migrations (Tasks B/C/D/E/F/G) are subsequent
  tasks. The four `HybridXConfig.svelte` components are
  stubs in `-a-43`; they get populated in `-a-45..-a-48`.
* The Settings overlay still hosts Terminal / Editor
  settings until Tasks B + C migrate them out.

### `-b-23` web-marketing static site

@@FullStackB's commit-clearance poke is at the tail of
[`../alex/event-fullstack-b-architect.md`](../alex/event-fullstack-b-architect.md)
"2026-05-21 — poke (fullstack-b-23 committed; pre-recycle
wrap)". Implementation note in
[`../fullstack-b/fullstack-b-23.md`](../fullstack-b/fullstack-b-23.md).

11 files, pure additive (zero deletions). Static site
source ported into `web-marketing/`:

* `index.html`, `favicon.ico`, `chan-mark.png`.
* `qr-donate.png` (matches `web/public/qr-donate.png` from
  `-a-42`'s prep commit).
* `install.sh`, `install.ps1`.
* `assets/editor-dark.png`, `assets/editor-recipes.png`.
* Donation QR section per @@Alex's framing.

Round-2 backlog item 6 ("Website migration") is the
container for this work; this commit is the static-source
land. DNS cutover + GH Pages CI + manual content are
subsequent tasks.

## Coverage slice (lane A)

This walk has two surfaces:

* **SPA surface** (for `-a-43`): build chan, serve, open
  the editor, drive the Hybrid back-side flip in each of
  the four front-tab types, capture screenshots.
* **Static-site surface** (for `-b-23`): serve
  `web-marketing/` via `python3 -m http.server` (or any
  static server), open in Chrome MCP, walk the page +
  install-script links, verify the donation QR renders +
  matches the embedded chan repo asset.

## Acceptance criteria

### `-a-43` — Hybrid back-side per-surface flip

1. Build chan at HEAD `b36ca96` (or any descendant). `cargo
   build -p chan` + `web/npm run build` (in that order if
   rust-embed needs the fresh frontend).
2. Spin up a test server against a throwaway drive (e.g.
   `/tmp/chan-test-phase8-wa-r3-a43/`). Per the standard
   test-server-workflow, seed with a small markdown set OR
   the chan repo itself. Ask @@Architect (or default to
   chan-source seed) if undecided.
3. Open the SPA + walk the four Hybrid front-tab types:
   * **Hybrid Terminal**: open a terminal in a Hybrid pane;
     flip to back. Expected: `HybridTerminalConfig.svelte`
     stub mounted; title band "Hybrid Terminal" visible.
     Stub copy may be a placeholder ("Terminal settings
     will land in fullstack-a-45" or similar — confirm
     against the actual stub copy in the source).
   * **Hybrid Editor**: open a file in a Hybrid pane; flip
     to back. Expected: `HybridEditorConfig.svelte` stub
     mounted; "Hybrid Editor" title.
   * **Hybrid Graph**: open the graph in a Hybrid pane;
     flip to back. Expected: `HybridGraphConfig.svelte`
     stub mounted; "Hybrid Graph" title.
   * **Hybrid File Browser**: open the FB in a Hybrid pane;
     flip to back. Expected:
     `HybridFileBrowserConfig.svelte` stub mounted; "Hybrid
     File Browser" title.
4. **Per-Hybrid theme test**: set a custom theme on one
   Hybrid pane via the hamburger menu; flip front/back
   several times; confirm BOTH sides carry the same theme
   value (no front/back split anymore). Open a second
   Hybrid; confirm theme is per-pane (the second Hybrid
   stays default until set independently).
5. **Flip animation**: confirm `-a-22`'s half-flip
   animation still plays correctly with the new component
   shape. No visual regression.
6. **Switch-front-while-flipped**: with a Hybrid flipped to
   back, switch the active front tab to a different type
   (e.g., from Terminal-back to switching the front to an
   Editor tab). Expected: the back-side content swaps to
   match the new front type, with the title band updating.
   This is the load-bearing flip-reveals-config-for-the-
   current-surface behaviour.

### `-b-23` — web-marketing static site

1. From the repo root: `cd web-marketing && python3 -m
   http.server 8090` (or any port). Open
   `http://localhost:8090/` in Chrome MCP.
2. Verify the landing page renders:
   * Logo + branding (`chan-mark.png`).
   * Hero copy + screenshots (`assets/editor-dark.png`,
     `assets/editor-recipes.png`).
   * Donation QR section (`qr-donate.png`) — visible,
     not broken-link. Confirm it's the same image as
     `web/public/qr-donate.png` (`shasum -a 256` both
     files OR visual inspection).
   * Install-script links: `install.sh`, `install.ps1`.
     Click each; confirm they download / render as
     expected (likely the curl-pipe-bash shape).
3. Verify favicon loads. Verify no broken external CDN /
   relative-path links.
4. Check the page in a small + large viewport (resize
   the browser window or use Chrome dev-tools mobile
   preview). Static site should be responsive or at
   least readable on mobile.

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-21 — fullstack-a-43 + fullstack-b-23
walkthroughs (wave-3 cleared work)`. Capture:

* Each acceptance subsection (a-43's six checks + b-23's
  four checks) with HOLD / FAIL / PARTIAL verdict.
* Screenshots at each step (especially the four Hybrid
  back-side flips for `-a-43`).
* Side observations for the bug list (e.g. stub copy that
  could be clearer, edge cases the flip semantic doesn't
  handle, marketing-site issues like broken links or
  alignment).
* Tear-down evidence (test server killed, throwaway drive
  `rm -rf`'d, `chan remove <path>` registry cleanup, any
  Chrome MCP tabs closed).

## How to start

1. `git status` to confirm clean tree; `git log
   --oneline -10` to verify `b36ca96` and `bc9e1f8` are
   in HEAD.
2. Decide on the throwaway-drive seed shape: chan-source
   (matches `webtest-a-2.md`'s pattern) OR a small
   ad-hoc fixture. Document the choice in the
   walkthrough verdict.
3. Build chan (`cargo build -p chan`); spin up
   `./target/debug/chan serve <drive-path>` per the
   standing test-server-workflow.
4. Open the SPA in Chrome MCP; walk the `-a-43` six
   checks; capture per-check verdicts.
5. Stop the chan server (or leave it for later use);
   `cd web-marketing && python3 -m http.server 8090`;
   open in a new Chrome MCP tab; walk the `-b-23` four
   checks.
6. Append the verdict to `webtest-a-1.md`; fire a poke
   to @@Architect via
   `event-webtest-a-architect.md` when done.
7. Tear down per the standing rule (kill servers, rm
   throwaway drive, chan remove, close Chrome MCP tabs).

## Coordination

* @@WebtestA lane (reactive).
* Standing terminal + Chrome MCP perm covers everything
  in this task (throwaway-drive shape).
* If you find regression-class issues:
  * `-a-43` regression → bug-list entry + @@FullStackA
    follow-up routing (likely affects Tasks B/C/D/E
    sequencing in the wave-3 fan-out).
  * `-b-23` regression → bug-list entry + @@FullStackB
    follow-up.
* If you find a side observation worth dispatching but
  not regression-class, file in the bug list with
  "NOT YET DISPATCHED — Round-2 wave-3 candidate" and
  flag in your poke.

## Numbering

Highest committed `webtest-a-N` is `-2` (the v0.11.2 cut
walkthrough verdict, committed in `3262e61` / adjacent
pre-recycle commits); this is `-3`.

## Out of scope

* `-b-22` orphan-sidecar runtime walk — that's
  @@WebtestB's lane (`webtest-b-3.md`), needs the
  standing chan-desktop runtime perm.
* `-a-44` drag-to-rearrange — not yet committed
  (@@FullStackA's queue next pickup post-recycle).
  Cut a separate `webtest-a-N` when it lands.
* Hybrid back-side migrations (Tasks B/C/D/E/F/G) —
  those are subsequent `-a-N` tasks. Walk each when
  it lands; don't pre-validate stubs.
* DNS cutover / GH Pages CI / manual content for the
  web-marketing site — those are subsequent backlog-
  item-6 sub-tasks, not part of `-b-23`.

## 2026-05-21 — walkthrough complete (8/8 HOLD)

Walked both surfaces on HEAD `22fd878`. Verdict + per-check
evidence appended to
[`webtest-a-1.md`](webtest-a-1.md) under
"## 2026-05-21 — fullstack-a-43 + fullstack-b-23 walkthroughs
(wave-3 cleared work)".

* `-a-43` all 7 sub-checks (3a/b/c/d + #4 + #5 + #6) HOLD.
* `-b-23` all 4 sub-checks HOLD; #4 viewport-responsiveness
  marked HOLD-partial because Chrome MCP `resize_window`
  did NOT shrink the reported `innerWidth` (stayed at 1595)
  even at 480×800 — fluid centered layout + correct meta
  viewport strongly suggest mobile rendering works but a
  real-device or DevTools-emulator pass is not in this
  walk's evidence.

Side observations filed (not regression-class, no fresh
bug-list entry yet — flagged in `webtest-a-1.md` under
"Side observations"):

1. Cmd+. Tab Return single-key-sequence flaky in Chrome MCP
   when focused pane front is a terminal; webtest-tooling
   note only.
2. Hybrid back-side stubs use `var(--text)` + `var(--border)`
   but no explicit `--bg`; works today via transparent bg
   inheriting parent. Tasks B/C/E/F populating the stubs
   should stay disciplined here.
3. `-b-23` task background mentions "11 files"; actual file
   count is 10. Minor doc-drift in task spec.

Test server + static server + throwaway drive + Chrome MCP
tabs all torn down per the standing rule.
