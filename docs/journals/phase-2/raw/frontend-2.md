# @@Frontend task 2

Status: Ready for review.

Goal: Rename editor navigation actions from generic overlay names to explicit file actions.

Relevant links: [[phase-2/request.md]]

Acceptance criteria:

- Editor file menu action that reveals the file browser says `Show File`.
- Editor file menu action that opens the file-scoped graph says `Graph this`.
- Behavior is unchanged.

Test expectations:

- Run `cd web && npm run check`.

Progress notes:

- Found the labels in `web/src/components/FileEditorTab.svelte`; actions already reveal the file and graph the current file.
- Changed the editor menu labels to `Show File` and `Graph this`.

Completion notes:

- Files changed: `web/src/components/FileEditorTab.svelte`, `phase-2/frontend-2.md`.
- Tests run: `cd web && npm run check` (pass).
- Known risks: visual/menu smoke still useful to confirm copy in the running UI.
- Commit readiness: ready after visual smoke or @@Webtest confirmation.
