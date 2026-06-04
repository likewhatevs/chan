# task LaneD -> Lead (1): Inspector pill + dropdown - DONE (own-gate-green)

Round-1 Wave-1 Inspector redesign complete. Structural redesign landed in
the working tree; own-gate-green; live browser-smoke flagged for Wave 2.

## Pathspec + fingerprint (for your verification / commit)
- base HEAD: d5f7dd38
- changed (ONLY these): 
  - web/src/components/FileInfoBody.svelte        (source)
  - web/src/components/inspectorActionsLayout.test.ts   (rewritten contract)
  - web/src/components/dashboardTabAndCarousel.test.ts  (A4 pins updated)
  - web/src/components/fileTreeDragOut.test.ts          (3 pins updated)
- pathspec diff fingerprint: 70290a427f103eb48a9527942f29abc2fdc1bb4f
  (`git diff -- <the 4 paths> | git hash-object --stdin`)
- UNCHANGED (owned but not touched): web/src/terminal/fromHere.ts,
  web/src/components/InspectorBody.svelte, web/src/components/Inspector.svelte

## fromHere.ts seed format: NOT changed (no @@LaneC impact)
Verified empirically rather than assumed: TerminalTab.maybeSeedPrompt
already wraps the seed as ` ${seedInput}\x01` (leading SPACE + Ctrl-A =
cursor-to-start). That IS "{cursor}{space}{relative-path}". The existing
terminalFromHereTarget (cwd=parent, seedInput=basename) is already correct,
so I left fromHere.ts and its unit test untouched. @@LaneC consumes the
helper as-is; no signature/seed-format change, no flag needed.

## Design: whole redesign fits in FileInfoBody.svelte
A $derived.by `actionModel` picks one main + a secondary[] per category;
the snippet renders a pill (primary) + caret that drops the secondaries.
Stayed inside owned files by wiring the two NEW universal actions to store
primitives instead of new caller props:
- dir "Open" -> revealPathInBrowser(path,{enter:true}) (store), or the
  host onReveal when present (dashboard) - both give "open in an FB tab".
- "New terminal here" -> onNewTerminal when present (dashboard), else
  openTerminalInActivePane(terminalFromHereTarget(path,isDir)).
Kept the onNewTerminal/allowUpload props so EmptyPaneCarousel (not my
file) still type-checks. InspectorBody/Inspector needed NO change.

## Per-category status (all 5 implemented)
- Directory: main "Open" (new FB tab); dropdown Upload file here
  (allowUpload-gated), Download tarball, New terminal here, Graph from here. DONE
- File (editable): main "Open" (onOpen -> Hybrid Editor); dropdown
  Download file, New terminal here, [Export to PDF if md], Graph from here. DONE
- Media: main "View / Zoom" (image) / "View PDF" (pdf); dropdown Download
  file, New terminal here, Graph from here. DONE
- Binary (incl symlinks): main "Download file"; dropdown Graph from here. DONE
- Editor "Show Details" (no onOpen, has onReveal): main "Show file";
  dropdown Download file, New terminal here, [Export to PDF], Graph from here. DONE
Notes: Upload is now directory-only (per spec). Graph-from-here / New
terminal / Export-PDF items drop out when their handler/condition is absent.

## One judgment call to ratify (not a blocker)
Kept "Export to PDF" as a markdown-only DROPDOWN item. The spec's
per-category dropdown lists don't mention it, but phase-17 (A3-iii)
deliberately moved Export-to-PDF OUT of the editor menu and INTO this
Inspector (pinned by editorRightClickRevamp.test.ts: "moved to the
Inspector"). Dropping it would regress that feature, so I demoted it to a
secondary action rather than delete it. Flag to @@Alex if he wants it gone.

## Gate
- svelte-check: 0 errors in my files. The ONE tree error is @@LaneC's
  FileBrowserSurface WIP (`refreshTree` - file actively churning under me);
  not my scope.
- vitest (scoped own-gate, 10 files incl all inspector + fromHere tests):
  162/162 PASS.
- vite build: green (only pre-existing chunk-size / dynamic-import warnings).
- Full-tree `make web-check` is transiently RED on PEER WIP only:
  LaneC fileBrowserRightClickRevamp + FileBrowserSurface(refreshTree),
  LaneA blocks.test.ts (3). None in my scope. Recommend your isolated
  gate.sh worktree (committed state) for the authoritative full-tree pass.

## Browser smoke: deferred to Wave 2 (per plan)
Not run live yet. Rationale: (1) round plan puts empirical smokes in Wave 2
on a clean persistent server, with the Chrome-vs-WKWebView client still a
pending @@Alex survey; (2) the shared tree can't build a clean server right
now (LaneC/LaneA active WIP) without an isolated worktree; (3) the change's
reactivity is sound-by-construction - actionModel is a pure $derived.by (no
state mutation), menuOpen reset + listener wiring are standard $effects.
I'm available to drive the Chrome smoke at convergence. What to eyeball:
pill label per category, caret opens/closes the dropdown, outside-click +
Esc close it, and each action fires (esp. dir "Open" -> new FB tab and
"New terminal here" seeding ` path` with cursor at col 0).
