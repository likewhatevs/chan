# Phase 11 New File / draft-save / editor-mode fixes (new tasks)

From @@Alex (2026-05-26). Three related items around the create/open flow
and editor modes. Filed into lanes per @@Alex's instruction.

## Item 1 (OWNER: @@LaneB) - source-code mode must not run markdown input rules

Bug: in source-code editing mode, typing "* " (and presumably "- ",
"1. ") still triggers markdown list mode. Source/code mode must NOT apply
markdown input rules (lists, etc.); those belong only to markdown
(rendered / editable-markdown) editing. Gate the input rules to the
markdown mode. File: `web/src/editor/Source.svelte` (and wherever the
list/markdown input rules are registered for the editor; see
`web/src/editor/commands/list.ts`). Small, editor-internal; Lane B can
slot it on its next webdev turn (e.g. while awaiting @@Alex CLI-handoff
ratification).

## Item 2 (OWNER: @@LaneA) - New File / New File or Dir open-after-create

Bug: the New File menu (and likely all New File right-click menus) creates
the file but does not open it. Expected:
- The dialog's job is to CREATE only (file, or dir for New File or Dir).
- After a successful create, if the path is an EDITABLE file, open it in
  the Hybrid Editor: markdown opens RENDERED, other editable/source files
  open in SOURCE-CODE mode. Open even if read-only.
- If a DIRECTORY was created, SELECT it in the tree shortly after creating.
- Use the existing editable rules (`web/src/state/fileTypes.ts`
  `isEditableText` / `crates/chan-drive/src/fs_ops.rs is_editable_text`),
  the existing editor-open helper (`web/src/state/tabs.svelte.ts`
  open-in-pane), and the existing tree-select helper
  (`store.svelte.ts revealAndSelect`). Hook the open/select at the
  create-resolution layer, not inside FileTree's menu markup, to avoid
  churn on `FileTree.svelte`.

This is File Browser create-flow behaviour -> @@LaneA. PathPromptModal /
pathValidate are free now (Lane B's bug-4 work there is merged), so
@@LaneA can take them for this. Sequence with the FB-capabilities work.

## Item 3 (OWNER: @@LaneA) - Save-from-draft must reuse the New File/Dir dialog

Bug/annoyance: the "Save" action in the draft right-click menu uses a
different dialog than New File; it should reuse the SAME dialog
(PathPromptModal) so the user gets autocomplete etc. Catch - a draft is
one of two shapes:
- a lone `draft.md` (the only thing in the draft dir) -> save as a FILE
  (file target through the dialog).
- a DIRECTORY (the user pasted images, opened a terminal, wrote files in
  the draft dir) -> invoke the dialog in a DIR-ONLY mode and INFORM the
  user that the whole draft directory is being saved as a directory.

Needs: a Dir-only mode added to PathPromptModal, and draft file-vs-dir
detection (is the draft dir just `draft.md` or more?). Draft + dialog
flow -> @@LaneA (it owns the dialog for this and the FB create/save
surface). Reuse the same create-then-open behaviour from Item 2 where it
applies after the save.

## Notes
- Items 2 and 3 share PathPromptModal and the create/save flow; do them
  together, sequenced with the FB-capabilities feature (all FB-create
  surface). Item 1 is independent editor-internal work for @@LaneB.
- Key files: `web/src/components/PathPromptModal.svelte`,
  `web/src/state/pathValidate.ts`, `web/src/state/store.svelte.ts`,
  `web/src/state/tabs.svelte.ts`, `web/src/editor/Source.svelte`,
  `web/src/editor/commands/list.ts`, draft handling in chan-server/
  chan-drive.
