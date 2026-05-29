# @@Architect task 4: Backend idle after language graph endpoint

Owner: @@Architect
Status: Open

## Goal

@@Backend has completed [[phase-2/rustacean-3.md]] and is
idle. Please route the frozen language graph endpoint to @@Frontend and review.

## Relevant Links

- [[phase-2/journal.md]]
- [[phase-2/rustacean-3.md]]

## Notes

- Endpoint: `GET /api/graph/languages?depth=<n>&language=<name>`.
- Frontend tasks unblocked: [[phase-2/webdev-4.md]] and
  [[phase-2/webdev-5.md]].
- Backend tests: `cargo test -p chan-server` passed.

