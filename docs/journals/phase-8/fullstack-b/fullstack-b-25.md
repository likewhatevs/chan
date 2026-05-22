# fullstack-b-25 — chan-desktop orphan-detection heuristic tightening + dialog PID display (-b-22 follow-up)

Owner: @@FullStackB
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Tighten the orphan-detection heuristic + add candidate-
PID display to the reclaim dialog so the destructive-
action confirmation isn't opaque. Both pieces from
@@WebtestB's `-b-22` walkthrough finding (`webtest-b-3`
verdict; filed in `phase-8-bugs.md`).

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "chan-desktop
orphan-detection heuristic too loose (false-positive
risk in noisy shell environments)" — full bug body
with the heuristic's current shape + the two follow-up
pieces.

## Scope

Two pieces in one commit:

### 1. Tighten the heuristic

Currently matches ANY process whose command line
contains `chan` + ` serve ` + drive-key as three
INDEPENDENT substrings. A `tail -f chan-serve.log
<drive-key>`, an IDE inspecting the directory, a tmux
pane with `chan serve <drive-key>` in visible
scrollback — all COULD enter the candidate set.

Fix: match `chan serve <drive-key>` as a contiguous
argv sequence (regex or positional argv check) instead
of three independent substrings. Audit
`chan-desktop`'s orphan-detect path
(`desktop/src-tauri/src/` per `-b-22`'s commit
`3987e73` + smoke-fixup chain).

### 2. Render candidate PIDs in the reclaim dialog

Currently `promptDriveLockTakeover()` uses Tauri's
plain `ask()` (yes/no shape only). User can't see
what's about to be SIGTERM'd.

Fix: replace with a custom modal that lists the
candidate PIDs + their command-line (or a relevant
substring) so the user sees exactly what reclaim
will kill. Decline buttons remain (Reclaim / Cancel).
For multi-candidate cases, show all of them.

## Acceptance

### Tightened heuristic

1. **Real chan serve match**: a genuine `chan serve
   <drive-key>` process matches the heuristic + appears
   in the candidate set.
2. **False-positive case**: a process with `chan
   serve` AND `<drive-key>` in NON-contiguous argv
   (e.g. `tail -f chan-serve.log -k <drive-key>`)
   does NOT match.
3. **Standalone process names**: a process called
   "chan" without the `serve` subcommand doesn't
   match.

### Dialog with PID display

4. **Reclaim dialog renders PID list**: when triggered
   with a candidate set, dialog shows each PID +
   command-line.
5. **User can decline**: Cancel button works; no
   processes touched.
6. **User can confirm**: Reclaim button SIGTERMs the
   listed PIDs; toast surfaces ("Reclaimed drive from
   <N> orphan process(es)").

### Tests

* Vitest pin (Rust side): heuristic match function
  positive + negative cases.
* If the dialog ships as a Tauri-side webview modal,
  E2E pin via Chrome MCP optional (you have standing
  perm); otherwise rely on visual smoke.

### Gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-desktop`: green (+ new test
  pins).
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`:
  green.
* If SPA-side modal: `npm test` + `npm run check` +
  `npm run build` green.

## Coordination

* @@FullStackB lane (chan-desktop runtime owner).
* Atomic-audit-commit discipline.
* Standing chan-desktop runtime permission covers any
  runtime verification (throwaway-drive shape only).

## Authorization

**Yes** for `desktop/src-tauri/src/*.rs` (heuristic
tighten + dialog wiring) + SPA-side if a custom modal
is added. Plus task tail + outbound.

## Numbering

Highest committed `fullstack-b-N` is `-24` (Windows
dead_code lint sweep + smoke fixup cascade). This is
`-25`.

## Out of scope

* Re-architecting the lock-takeover protocol beyond
  heuristic + dialog UX. Stay narrow.
* Server-side lock primitive changes (Round-3 polish
  per `phase-8-bugs.md` "Windows lock contract parity").
* Walkthrough (@@WebtestB walks runtime once committed
  — that's a separate dispatch).
