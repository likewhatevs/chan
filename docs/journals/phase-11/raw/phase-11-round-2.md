# Phase 11 round 2

Round 2 opens with the phase-10 carryover. The items below were the open
phase-10 surface at the close of Track A item 4 (Tauri icon + desktop
docs/config audit). Track C closed in phase 10; the Rich Prompt watcher-audit
follow-ups landed in `baac10e`. Decisions recorded here are @@Architect's,
ratified by @@Alex on 2026-05-26.

## Phase 10 carryover: still open

### 1. Linux desktop launch

Status: open, launch blocker.

The Linux desktop app opened a white window with no rendered content, menus
limited to Edit/Window with no File menu, and a blank duplicate window from
Window -> Drives, for at least one manual tester. Validate and fix:

- shell launches and the embedded server starts;
- first-window routing renders the drive list or first-run flow;
- menus expose the expected platform actions.

Local repro is available via lima-vm + sdme (aarch64); CI still owns x86_64.

### 2. macOS CLI-to-desktop handoff

Status: open, design checkpoint first.

Scoped to macOS for now. Linux is blocked behind item 1; Windows desktop is
deferred (see phase-10 desktop docs audit). Produce a short design note with
options, trade-offs, and one recommendation before implementing:

- should `chan serve <path>` attach to a running desktop-owned server, ask
  desktop to open the drive, or keep owning a separate server process;
- how same-user desktop discovery works (Unix-domain socket on macOS);
- how ownership, bearer-token discovery, lifecycle, version, and capability
  mismatch are represented;
- the no-desktop-running fallback;
- which flags force standalone server behavior.

## Phase 10 carryover: superseded or no longer tracked

### Linux native drag-out: superseded

The phase-10 Track A item was native File Browser drag-out on Linux. Round 1
decided to remove File Browser drag in/out entirely on macOS and Linux and
operate through Upload/Download buttons instead (with a native-desktop
download indicator). The drag-out item is therefore dropped, not carried.
The replacement work lives in the round-1 bug list.

### Release verification: no longer a blocker

The phase-10 Track B item `npm run verify:release` (no skip flags) was
blocked on the repository being private and the next `chan-v*` tag carrying
the manual bundle. The repository is now public and the release process has
been ironed out, so both blockers are gone. This is no longer tracked as open
work; confirm once at the next tag rather than carrying it as a round-2 item.

The phase-10 Track A "release validation" operational checks fold into the
same ironed-out release process. Same treatment: confirm at the next cut, not
a standing round-2 item.

## Phase 10 carryover: deferred

### Manual/site copy for streaming and transfer behavior

The phase-10 Track B item was updating `docs/manual/` and the generated public
manual for the streaming open, progressive relationship loading, graph
streaming, and inspector Upload/Download behavior. Round 1 reworks partial
load, File Browser, and Graph (the exact surfaces that copy would describe),
so writing it now would be stale within the round. Deferred until the round-1
partial-load / File Browser / Graph rework settles, then write the manual copy
against the final behavior and run `cd web-marketing && npm run check`.
