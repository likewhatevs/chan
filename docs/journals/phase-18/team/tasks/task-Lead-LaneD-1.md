# task Lead -> LaneD (1): Inspector

You are @@LaneD - Inspector lane. Round-1, Wave 1. START NOW.

## Read first (context lives here, not in this poke)
- Process: docs/journals/phase-18/team/bootstrap.md
- Plan + your lane section + gate/quality bar:
  docs/journals/phase-18/team/round-1-plan.md  (section "@@LaneD - Inspector")
- Verbatim spec: docs/journals/phase-18/round-1/draft.md  ("### Inspector")
- Re-verify line anchors against HEAD; they drift.

## Wave 1 scope (one redesign, applied per item category)
Replace the flat action buttons with a single PILL (main action) + dropdown
(secondary actions) per item category. Reuse existing handlers (Open, Upload,
Download, View/Zoom, Show File, Graph-from-here).
- Directory: main "Open" (new file-browser tab); dropdown: Upload file here,
  Download tarball, New terminal here, Graph from here.
- File (editable): main "Open" (Hybrid Editor); dropdown: Download file,
  New terminal here, Graph from here.
- Media: main "View / Zoom"; dropdown: Download file, New terminal here,
  Graph from here.
- Binary (incl symlinks): main "Download file"; dropdown: Graph from here.
- Editor "Show Details": main "Show file" (file-browser tab, file selected);
  dropdown: Download file, New terminal here, Graph from here.
"New terminal here" seeds the terminal with "{cursor}{space}{relative-path}".

## Owned files (edit ONLY these)
web/src/components/{FileInfoBody.svelte,InspectorBody.svelte,Inspector.svelte},
web/src/terminal/fromHere.ts.

## Shared-file rules (plan "Shared-file contention")
- fromHere.ts: YOU own seed-format changes (terminalFromHereTarget). @@LaneC
  consumes the helper as-is. If the seed format / signature must change, that
  affects @@LaneC -> flag to @@Lead BEFORE landing the signature change.

## Gate before any "done" report
make web-check + svelte-check + npm run build. Browser-smoke the pill +
dropdown per category (Svelte-5 reactivity; static gates miss runtime errors).

## On completion
Cut task-LaneD-Lead-1.md (own-gate-green + pathspec sha + per-category status +
note whether you changed fromHere.ts seed format), poke me
(--tab-name=@@Lead --submit=claude). Journal: journal-LaneD.md.
