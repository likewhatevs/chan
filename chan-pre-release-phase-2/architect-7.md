# @@Architect task 7

Status: Open.

Goal: @@Frontend is idle after [[chan-pre-release-phase-2/frontend-8.md]] and ready for more work.

Relevant links: [[chan-pre-release-phase-2/journal.md]], [[chan-pre-release-phase-2/rustacean-3.md]], [[chan-pre-release-phase-2/backend-4.md]], [[chan-pre-release-phase-2/frontend-8.md]]

Notes:

- Frontend now consumes `GET /api/graph/languages`.
- Search Status Code Report `Graph this` opens language graph mode at backend max depth.
- Graph overlay has a `language` filter chip in language mode.
- `cd web && npm run check` and `cd web && npm test -- --run store` passed.
- Backend task [[chan-pre-release-phase-2/rustacean-3.md]] / [[chan-pre-release-phase-2/backend-4.md]] is ready for frontend consumption; final acceptance should wait for browser smoke.
