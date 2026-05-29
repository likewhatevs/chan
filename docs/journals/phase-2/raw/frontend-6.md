# @@Frontend task 6

Status: Ready for review.

Goal: Fix graph folder / multi-file scope layout so seed documents do not stack on top of each other.

Relevant links: [[phase-2/request.md]], [[phase-2/journal.md]]

Acceptance criteria:

- Single focal scopes still center the focal node.
- Folder and multi-file scopes spread focal nodes apart before force layout settles.
- Non-focal nodes still participate in the force simulation.

Test expectations:

- Run `cd web && npm run check`.

Progress notes:

- Confirmed `web/src/components/GraphCanvas.svelte` pinned every focal node at `fx=0`, `fy=0`, which stacks folder-scope seed files.
- Multi-focal scopes now pin seed nodes around a deterministic ring; single-focal scopes still pin at origin.

Completion notes:

- Files changed: `web/src/components/GraphCanvas.svelte`, `phase-2/frontend-6.md`.
- Tests run: `cd web && npm run check` (pass).
- Known risks: browser smoke should confirm the ring spread feels right for small and large folders.
- Commit readiness: ready after visual smoke/review.
