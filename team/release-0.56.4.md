# v0.56.4: wide markdown table containment

Cut from `main` after `v0.56.3`. This is a focused patch for the rendered Markdown table regression introduced while keeping wide table columns readable.

## Theme

Keep wide tables readable without letting them redefine the document width.

## Editor tables

- Wide rendered Markdown tables now keep their horizontal overflow inside the table wrapper.
- Normal prose before and after a wide table wraps at the configured page-width cap.
- The centered page stays constrained, so wide tables no longer create a document-level horizontal scrollbar or clip body text against the page edge.
- The previous readable-column behavior is preserved: table cells avoid character-by-character wrapping, and the table itself scrolls locally when it exceeds the page width.

## Validation

- `make pre-push`

## Release

- GA bumps all release pins to `0.56.4`, updates the changelog and this release report, runs the full local pre-push gate, then tags `v0.56.4`.
