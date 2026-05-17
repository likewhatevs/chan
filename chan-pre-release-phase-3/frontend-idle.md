# frontend-idle: @@Frontend ready for next work

Owner: @@Architect (assignment task back to architect).

Status: REVIEW.

Related:

- [frontend-1.md](./frontend-1.md)
- [frontend-2.md](./frontend-2.md)
- [frontend-3.md](./frontend-3.md)
- [backend-3.md](./backend-3.md)
- [frontend-b-1.md](./frontend-b-1.md)
- [journal.md](./journal.md)

## Summary

@@Frontend has consumed every actionable item across the three
assigned task files plus the architect-reconciled unblock and
@@FrontendB's review hand-offs. All work is type-checked
(`cd web && npm run check`: 0 errors, 0 warnings), tested
(`cd web && npm test -- --run`: 145 tests pass; 34 new across 4
new test files since phase-3 start), production-built
(`npm run build` succeeds with no new warnings), and the one
Rust change (`crates/chan/src/main.rs` SERVE_LONG_ABOUT regen)
passes `cargo check -p chan`.

## Per-task status

- **[frontend-2.md](./frontend-2.md)** — REVIEW.
  Cmd+F cursor placement, File Browser Cmd+F, new-file tab-
  complete, multi-level indent regression, list-guide auto-
  hide, GitHub-style icons, the post-REVIEW context menu
  positioning fix, and now also the stale-selection-rectangles
  defense (CM6 `drawSelection()` extension added — pulled from
  the deferred cluster after @@FrontendB's analysis pinned the
  root cause). Two image-related deferred items remain
  (cursor-height inherited from image, image-line guide bars
  break around images); both still need browser repro with
  @@Webtest before I can pick the right fix.

- **[frontend-1.md](./frontend-1.md)** — REVIEW (only
  [backend-3.md](./backend-3.md) layout config gating one final
  item).
  Landed: visible Assistant→Agent rename + SERVE_LONG_ABOUT
  regen, dashboard shell, URL state (`search_scope=`), Agent
  overlay Cmd+F over chat history, **CODEx-on-CLAUDE banner
  state-sync fix** (per `InlineAssist.svelte::conversationBackend()`),
  and **AppStatusBar event-click routing** (index → Settings,
  import → File Browser, transient status → clear). Agent
  activity intentionally not surfaced in the status bar per
  the source comment + @@FrontendB's confirmation. Only
  [backend-3.md](./backend-3.md) (Settings layout
  standard/compact LineSpacing enum rename) remains gated on
  @@Backend.

- **[frontend-3.md](./frontend-3.md)** — REVIEW.
  Resource colors centralized (`--g-binary` FILE blue,
  `--g-folder` grey added to App.svelte palette, kinds.ts
  updated, design.md table refreshed), parent dir + common
  ancestor scope options auto-derived, graph folder filter
  chip added for filesystem mode. Full cross-mode filter
  normalization (folder/path overlay in markdown mode,
  markdown-link overlay in folder mode) deferred as a
  substantial follow-up — @@FrontendB's read-only audit
  documents the implementation map should that follow-up
  be greenlit.

## Asks back to @@Architect

1. **@@Webtest browser smoke** per the test-expectations sections
   of each task file. Focused list:
   - Cmd+F Enter cursor placement in editor.
   - File Browser Cmd+F over expanded entries.
   - PathPromptModal Tab-complete vs. cycling.
   - Nested-list wrap with a long sentence at depth ≥ 2.
   - List guide fade after 1.5s.
   - GitHub-style chevron + folder icons.
   - File Browser right-click context menu now lands adjacent
     to the clicked row (regression fix).
   - Dashboard header on the empty-pane background.
   - URL-hash round-trip for search scope, graph folder filter,
     overlay state.
   - Agent overlay Cmd+F over chat history.
   - AppStatusBar section clicks open the right overlay (index
     → Settings, import → File Browser, transient → cleared).
   - **Banner state-sync repro** ([frontend-b-1.md](./frontend-b-1.md)
     "Banner per-agent"): select CODEX in the inspector, open a
     saved Claude conversation; banner should now say
     "CLAUDE CLI" (was the CODEx-on-CLAUDE bug).
   - Resource colors consistent across file tree / inspector /
     search / agent / graph.
   - Parent dir + common ancestor scope options.
   - Graph folder filter chip in filesystem mode hides folder
     nodes + edges.
   - **Image residual repros** from [frontend-b-1.md](./frontend-b-1.md):
     the `drawSelection()` extension should remove stale
     selection rectangles around image/list blocks. Cursor
     height after images and image-line guide bar breakage
     still need a screenshot / fixture so I can pick the fix.

2. **Coordinate the SERVE_LONG_ABOUT commit** with the frontend
   shortcuts.ts "Agent" label flip and the existing
   [backend-1.md](./backend-1.md) `crates/chan/src/main.rs`
   wording change. One Rust diff in main.rs + the frontend
   shortcuts.ts already landed = a single coherent unit per
   the journal's commit-coordination guidance.

3. **Settings layout standard/compact** lands as a pair with
   [backend-3.md](./backend-3.md) once @@Backend renames the
   `LineSpacing` enum. Frontend half is one screen of
   labels/types/CSS values; precisely scoped in
   [frontend-1.md](./frontend-1.md) "Still blocked on @@Backend".

4. **Optional follow-up** (not blocking phase-3 delivery):
   full per-mode graph filter normalization per
   [frontend-3.md](./frontend-3.md) "Deferred / partial" with
   the implementation map in [frontend-b-1.md](./frontend-b-1.md)
   "Graph filter chips" + "Markdown graph parent-folder /
   path-to-root overlay" + "Folder graph: markdown cross-link
   overlay".

## Idle and ready

2026-05-16 @@Architect: Alex reassigned @@Frontend into @@WebtestB. Continue in
[webtest-2.md](./webtest-2.md) and do not take more implementation work unless
reassigned.
