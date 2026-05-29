# @@LaneA request - Phase 13 round 1

You are @@LaneA, the round-1 architect for the **content-surfaces**
lane (Editor + Terminal + Inspector). You MAY spawn 2-4 in-session
subagents via the Agent tool (one per bug or per enhancement section).
You report progress + merge-ready slices to @@Alex; @@LaneB serializes
merges to main and cuts v0.17.0.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-1.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/README.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-a/journal.md` (tail of your prior turns)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-a.md` (inbox)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-b-lane-a.md` (cross-lane from @@LaneB; may not exist yet)

## Worktree + branch

Source ONLY in: `../chan-lane-a` on `phase-13-lane-a`. Create on first
turn:

```
git -C /Users/fiorix/dev/github.com/fiorix/chan worktree add ../chan-lane-a -b phase-13-lane-a
```

Journals + channels + this request file live in the MAIN checkout at
`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/` and
are edited by ABSOLUTE PATH (never the worktree copy).

## Scope

### Bugs

1. **New-document cursor focus on mount.**
   - `web/src/editor/Wysiwyg.svelte` (onMount ~370; `autoFocus` ~520;
     existing deferred-focus "bug 10" precedent at 548-561).
   - Also `web/src/editor/Source.svelte` (plain-text variant; its own
     onMount).

2. **"Unsaved changes from a previous session" prompt on brand-new
   docs.**
   - `web/src/state/store.svelte.ts::noteDraftCreated` ~1021.
   - `web/src/components/DraftCloseModal.svelte`.
   - `web/src/App.svelte` (draft creation orchestration; Cmd+N
     paneMode staging).
   - The bug is the prompt firing when there are no prior changes to
     restore.

3. **List marker preservation** (hyphen / `*` / number render as
   authored, not auto-normalized to bullets).
   - `web/src/editor/commands/list.ts`.
   - `web/src/editor/extensions/list_guide_visibility.ts`.
   - `web/src/editor/paste_html.ts` (`bulletListMarker` normalization).
   - `web/src/editor/Wysiwyg.svelte` list CSS at 945-995
     (`cm-md-ol-marker`, `cm-md-ul-marker`, `cm-md-list-line`).

4. **Shift-Enter in agent prompt submits instead of inserting newline.**
   - `web/src/components/TerminalRichPrompt.svelte::onKeydown` 253-272
     (missing Shift-Enter guard before the Cmd/Ctrl+Enter submit at
     270; placeholder text at line 61 also needs updating).
   - `web/src/terminal/keymap.ts` 48-75 / 214-219
     (`terminalMetaKeyBytes` / `enterModifier`; verify the keymap
     path).

### Enhancement - Hybrid Inspector

The Inspector dispatcher (`web/src/components/InspectorBody.svelte`)
renders across `FileBrowserSurface.svelte`,
`FileEditorTab.svelte`, `GraphPanel.svelte`, and
`SearchStatusOverlay.svelte`. Changes here propagate to all four
surfaces automatically; confirm parity per surface.

- **"Show path" -> absolute path + copy button.**
  - `web/src/components/FileInfoBody.svelte` (`showFullPath` toggle
    ~432; render ~648 currently shows relative path).
  - **Reuse**: existing right-click "Copy path to file" in
    `FileEditorTab.svelte`; clipboard helper
    `copyWorkspaceWarningPath()` in `web/src/state/store.svelte.ts`.
    Don't roll a new clipboard call.

- **KIND chips become Graph-from-here links per kind.**
  - `web/src/components/KindChip.svelte` (chips currently render
    FILE KIND + LANGUAGE; extend for hashtag + contact too).
  - Hook: existing `onSetAsScope` callback wired in
    `FileInfoBody.svelte` ~698 already does `kind=path`; add
    kind-specific dispatch for `tag`, `contact`, `language`.
  - **CROSS-LANE DEPENDENCY**: `kind=tag|contact|language` scope
    routes land in Lane B first. WAIT for @@LaneB's KIND route
    signature on `event-lane-b-lane-a.md` before wiring kind-specific
    click handlers. The absolute-path + copy + workspace-root parity
    slices ship in parallel without blocking on Lane B.

- **Workspace-root inspector parity with folder inspector.**
  - `web/src/components/WorkspaceInfoBody.svelte` (align with
    `FileInfoBody` directory mode; root icon stays different).
  - NOTE: @@LaneB's Dashboard widget reuses this. Coordinate via
    `event-lane-a-lane-b.md` when this slice is merge-ready so
    @@LaneB can rebase its widget on top.

## Subagent budget

2-4 in-session subagents max. Suggested slicing (you own the slicing
call):

- Subagent 1: editor bugs 1+2+3 (same file cluster - Wysiwyg / draft /
  list).
- Subagent 2: terminal Shift-Enter bug 4.
- Subagent 3: Inspector path + copy button + workspace-root parity.
- Subagent 4: Inspector KIND wiring (gated on @@LaneB's KIND route
  signature).

## Coordination rules

- Append-only directional channels; never edit another agent's entries.
- **Each turn, BEFORE acting**, read:
  - `event-alex-lane-a.md` (inbox).
  - `event-lane-b-lane-a.md` (cross-lane from @@LaneB, if exists -
    the KIND route signature will land here).
- Progress + merge-ready: append to `event-lane-a-alex.md`.
- Cross-lane to @@LaneB: append to `event-lane-a-lane-b.md` (create
  on first use).
- Self-document in `lane-a/journal.md` (per
  `feedback_self_document_in_task`).
- Subagents speak through you on the bus; they don't have their own
  channel files.
- Declare unexpected `web/src` overlap with @@LaneB BEFORE editing.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check  &&  npm run build
```

Then append to `event-lane-a-alex.md`:

```
ready to merge: phase-13-lane-a@<sha>  -  <one-line slice summary>
```

Per `feedback_svelte_static_gate_misses_runtime`: browser-smoke
component reactivity changes (the Inspector click handlers and
DraftCloseModal logic fall in this bucket). Per
`feedback_terminal_webgl_wkwebview`: terminal changes need a
chan-desktop smoke, not just Chrome.

## Out of scope

Anything not in `roadmap-round-1.md`. Escalate scope creep on
`event-lane-a-alex.md`. Don't push to origin. Don't merge to main -
@@LaneB does that.

## First turn checklist

1. Create the worktree + branch (above).
2. Read all recovery files (above).
3. Append an opening entry to `lane-a/journal.md` (date/time/intent).
4. Pick your first slice; spawn subagent(s) if useful.
5. Work the slice to the gate; report on `event-lane-a-alex.md`.
