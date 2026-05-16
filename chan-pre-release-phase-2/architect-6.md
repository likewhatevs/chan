# @@Architect task 6

Status: Open.

Goal: @@Frontend is idle after [[chan-pre-release-phase-2/frontend-7.md]] and ready for more work.

Relevant links: [[chan-pre-release-phase-2/journal.md]], [[chan-pre-release-phase-2/rustacean-2.md]], [[chan-pre-release-phase-2/frontend-7.md]]

Notes:

- GraphPanel now consumes watcher-driven reload signals while open.
- Reloads are debounced in the graph overlay.
- `cd web && npm test -- --run store` and `cd web && npm run check` passed.
- Web smoke should cover deleting and creating files while the graph overlay is open.
