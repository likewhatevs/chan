# Phase 9 Release Parking Lot

Date: 2026-05-24
Owner: @@CoreArchitect
Status: Release triage recorded

## Release Decisions

- Native/desktop literal `Cmd+P` validation is skipped for this release.
  @@Alex will validate against the next `Chan.app` build and report if it
  fails.
- Page-break and PDF export ownership is accepted for this release. The code is
  already merged in `b0869b1` (`Add editor PDF export and page breaks`).
- `mtime_ns` editor CAS adoption is already wired through Web and server. No
  new release code was needed in this session.

## Post-Release Tasks

1. Rich Prompt visual validation in a browser environment that can type into
   CodeMirror and use clipboard APIs.
   - Validate non-empty prompt submit archives the exact buffer and clears only
     when the user has not edited during submit.
   - Validate full Spawn agents preflight, including slider/config paste and
     confirmation flow.

2. Low-FD live stress.
   - Reproduce with many terminal sessions plus active indexing under
     `ulimit -n 256`.
   - Decide whether background search/index admission needs a separate throttle
     beyond the current fd budget and terminal session admission.

3. Remaining Wave 1 follow-ups.
   - Rapid-edit browser/server repro for stale editor/index state races.
   - Product decision for the `[[` search contract, then endpoint tests for the
     selected semantics.
   - Audit remaining direct sync calls from async server paths.

## Release Checks Run

- `npm run test -- --run src/editor/print.test.ts src/editor/commands/page_break.test.ts src/components/editorRightClickRevamp.test.ts src/state/tabs.test.ts`
- `cargo test -p chan-server routes::files::write_tests --lib`
