# @@Architect task 4: phase-6 wrap plan

Owner: @@Architect
Status: IN_PROGRESS (locked by Alex 2026-05-18, final version:
wrap now, carry three items to phase 6.1)

## Goal

Drive phase 6 to a clean close. Land the must-haves end-to-end,
verify, commit + push, then open phase 6.1 for the three carry-
over items.

## Decision (Alex 2026-05-18, final)

* **Drive to REVIEW** before commit:
  * [frontend-2](./frontend-2.md) (PARTIAL): broadcast bar
    peer-group + mute icon + member `[x]` + Select All + bubble
    menu reorder + terminal right-click expansion + PANE
    Inspector toggle + outside-overlay menu finish + tab-rename
    stale-env prompt + file browser opens collapsed.
  * [frontend-10](./frontend-10.md): FB header full path +
    "Terminal from here" with file-prompt seed.
  * [frontend-12](./frontend-12.md) dir half: directory
    "Graph from here" in the graph inspector.
  * [frontend-13](./frontend-13.md): modifier-Enter chord gap
    (Cmd+Enter / Ctrl+Enter via CSI-u). Alex needs this for
    personal live testing before they can dig back into the
    test service.
  * [frontend-14](./frontend-14.md): rich-prompt overlay on
    top of a terminal (medium-size feature added late: markdown
    composer triggered by Alt+Space / right-click, ships raw
    markdown to the PTY on Cmd+Enter, image paste via
    attachments, "New File from here" save).
  * [backsystacean-10](./backsystacean-10.md): expose PTY CWD
    on terminal session metadata. Unblocks
    [frontend-2](./frontend-2.md)'s CWD-dependent right-click
    rows (currently rendered with the fallback status).
  * [frontend-15](./frontend-15.md): pin the window-scoped
    broadcast invariant (audit + test). Broadcast must not
    cross window boundaries.
  * OBS-WT6-L backend-only restart in
    [webtest-1](./webtest-1.md) so the live `/api/health`
    indexer block is exercisable.
* **Carry to phase 6.1** (recorded in
  [summary.md](./summary.md) "Remaining follow-ups" when phase
  6 closes):
  * [frontend-5](./frontend-5.md) broad compat-sensitive
    identifier codemod (`kind: "folder"`, graph filters,
    persisted scope keys, internal canvas aliases). Needs a
    deliberate wire-format compatibility pass.
  * [frontend-11](./frontend-11.md) graph filter chip counter
    overcount fix (OBS-WT6-WTA-9). Cosmetic; underlying data
    correct.
  * [frontend-12](./frontend-12.md) breadcrumb half. UX nice-
    to-have; the dir "Graph from here" half lands this phase.

## Must-land lanes

| Lane | Owner | Why it must land this phase |
|------|-------|-----------------------------|
| [frontend-2](./frontend-2.md) | @@Frontend | Headline UX: broadcast bar (peer group + mute icon + member `[x]` + Select All + `[off]`), right-click expansion (Copy CWD / Show Dir / Graph dir / New Terminal / New File / splits / search / settings), PANE Inspector toggle, outside-overlay menu finish, terminal bubble menu reorder, tab-rename stale-env prompt, file browser opens collapsed on first open. |
| [frontend-10](./frontend-10.md) | @@Frontend | File browser overlay header = selected entry's full path; "Terminal from here" on dirs + files (file variant seeds the prompt as `$ <cursor> path` via leading-space + Ctrl+A). |
| [frontend-12](./frontend-12.md), dir half only | @@Frontend | Directory nodes in the graph inspector currently have no "Graph from here" (`GraphPanel.svelte:1139` gates `onSetAsScope` to `fsKind === "file"`). Without this the architectural ask "filesystem as primary graph layer" leaves a usability gap (file Graph from here works, dir does not). Small extension to the existing gate. Breadcrumb half parks to next phase. |
| OBS-WT6-L | @@WebtestA | Backend-only restart so the live `/api/health` indexer block from [backsystacean-7](./backsystacean-7.md) is exercisable. Also rebuilds for the merged `/api/graph` from [backsystacean-9](./backsystacean-9.md). |

## Phase 6.1 manifest

Three items carry over (re-park after the brief un-park). Each
needs a fresh task file in `phase-6.1/` when
Alex opens that phase:

| Item | Source task | Why deferred |
|------|-------------|--------------|
| Broad `folder` -> `directory` identifier codemod | [frontend-5](./frontend-5.md) | Wire-format compat-sensitive (`kind: "folder"`, graph filters, persisted scope keys, internal canvas aliases). User-visible copy + wire vocab already landed. |
| Graph filter chip counter overcount | [frontend-11](./frontend-11.md) | OBS-WT6-WTA-9. Cosmetic mismatch; underlying data correct. |
| Graph overlay scope breadcrumb | [frontend-12](./frontend-12.md) | UX nice-to-have. Directory "Graph from here" half lands this phase. |

## Verification gates before push

* Pre-push gate on the final HEAD:
  * `cargo fmt --check`
  * `cargo clippy --all-targets -- -D warnings`
  * `cargo build --no-default-features`
  * `cargo test --workspace`
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
* @@WebtestA live click-through against the rebuilt service:
  * Merged `/api/graph` chip counts non-zero on the seeded
    drive (`folder N`, `language N`, etc.).
  * Inspector renders for drive / directory / markdown /
    contact / text / binary / media / special.
  * Frontend-2 must-haves: PANE Inspector toggle present;
    outside-overlay shows the 2-button menu; terminal
    right-click menu has every advertised action; broadcast
    bar appears in broadcast mode with mute toggle + members
    + `[off]`; tab-rename prompt fires with Restart / Later.
  * Frontend-10 must-haves: FB header shows the selected
    entry's full path; "Terminal from here" on a directory
    opens a terminal at that CWD; on a file lands with the
    prompt seeded.
  * Frontend-12 dir half: clicking a directory node in the
    graph inspector exposes "Graph from here" and re-scopes.
  * Backsystacean-7 indexer block live response shape.

## Commit + push

* [architect-3](./architect-3.md) carries the commit-grouping
  draft. Refresh to cover the new REVIEW lanes (backsystacean-6,
  -7, -8, -9 + frontend-3, -4, -6, -7, -8, -10, -12-dir-half +
  the parked frontend-5 partial). Six-commit shape stays.
* Working tree at ~57 files now; expect ~70 by the time
  frontend-2 + -10 + -12 ship.
* Push to `origin/main` on Alex's explicit go signal.

## Summary at close

[summary.md](./summary.md) with:

* Outcome and completion status.
* Highlights.
* Lowlights.
* Bugs found and fixed (with OBS-WT6-* trail).
* Test and hardening coverage.
* Remaining follow-ups (the parking lot above).
* Agent rankings and constructive feedback.

## Progress

* 2026-05-18 Wrap plan locked by Alex.

## Completion notes

(populated when push lands and summary is final)
