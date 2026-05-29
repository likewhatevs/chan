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

## 2026-05-22 — implementation note (ready for commit clearance)

Both pieces landed in one atomic delta. Same single-commit
shape as the earlier `-b-*` fixes (chan-desktop Rust + SPA
JS + styles in one bundle, no chan-server cross-lane touch).

### Heuristic tightening — positional argv check

`crates/chan-server/src/control_socket.rs` was already gated
in `-24`; the heuristic lives in
`desktop/src-tauri/src/serve.rs`. Old shape: three
independent substring checks (`rest.contains("chan")` +
`rest.contains(" serve ")` + `rest.contains(key)`). New
shape:

* `argv[0]` basename must equal `chan` exactly (via
  `Path::new(tokens[0]).file_name()`).
* `argv[1]` must equal `serve`.
* `key` must appear as a standalone token in `argv[2..]`
  (slice `.contains(&key)` — clippy::manual_contains).

False-positives caught by the bug list (wrappers like
`strace chan serve <key>` / `lldb -p N chan serve <key>` /
`/bin/sh -c chan serve <key>`, path-substring matches like
`chan serve /tmp/notes-other`) no longer trip the heuristic.

### OrphanCandidate + new IPC

Pulled the existing `find_orphan_chan_serve_pids` to
`find_orphan_chan_serve_candidates`. The parsing core
(`parse_ps_lines_for_chan_serve`) now returns
`Vec<OrphanCandidate>` (`pid: u32` + `command: String`)
instead of `Vec<u32>`, so the dialog gets both pieces in
one pass. `reclaim_drive_lock` maps the candidates back to
PIDs for the SIGTERM loop.

New IPC `find_drive_lock_candidates(path) -> Vec<OrphanCandidate>`
returns the candidate list to the SPA without touching the
processes. Race window between `find_drive_lock_candidates`
+ `reclaim_drive_lock` is intentional: reclaim re-enumerates
internally before the kill, so a candidate disappearing
between the two calls just means the kill loop has fewer
PIDs.

`OrphanCandidate` is defined unconditionally
(`#[derive(Debug, Clone, Serialize)]`). The serde derives
keep the fields "used" on Windows so clippy's dead-code
sweep stays quiet there.

### SPA-side custom modal

`desktop/src/main.js::promptDriveLockTakeover` now:

1. Calls `find_drive_lock_candidates`.
2. If empty → non-destructive `message()` notice ("lock may
   be held by an unrelated process").
3. Otherwise → renders `showReclaimDialog(key, candidates)`
   with PID + command line per row.
4. On Reclaim → existing `reclaim_drive_lock` path
   unchanged.

`showReclaimDialog` is a vanilla-JS modal (vanilla shape
matches the rest of `main.js` — desktop SPA is plain JS,
not Svelte). Backdrop click + Escape cancel; Reclaim
button gets initial focus + Enter triggers it. Styles
appended to `desktop/src/styles.css` under
`/* fullstack-b-25: drive-lock reclaim modal */`.

### Tests

| New test (chan-desktop, `serve::tests::`)                              | Acceptance criterion |
|------------------------------------------------------------------------|----------------------|
| `parse_ps_lines_picks_chan_serve_against_key_but_skips_self` (updated) | Real `chan serve` match still works; return shape pivots to `OrphanCandidate` |
| `parse_ps_lines_rejects_wrapper_programs_running_chan_serve` (new)     | Wrappers (`strace` / `lldb` / `wrapper-chan-serve.sh` / `/bin/sh -c chan serve …`) rejected |
| `parse_ps_lines_rejects_path_substring_only_match` (new)               | Keys must match as standalone tokens; `/tmp/notes-other` and `/tmp/notes/sub-drive` don't trip the heuristic when key is `/tmp/notes` |
| `parse_ps_lines_carries_command_line_into_candidate` (new)             | `OrphanCandidate.command` carries the full argv for the dialog |
| `invoke_handler_registers_find_drive_lock_candidates` (new)            | `find_drive_lock_candidates` registered in `generate_handler!` + `fn find_drive_lock_candidates` exists |
| `serve_failed_payload_drive_lock_field_is_consumed_by_launcher` (extended) | main.js invokes `find_drive_lock_candidates` before `reclaim_drive_lock` |

chan-desktop count: 39 → 43 (+4 net; one of the 5
parse_ps_lines tests replaces the old PID-only fixture).

### Pre-push gate (local, macOS aarch64)

| Surface                                                            | State                                          |
|--------------------------------------------------------------------|------------------------------------------------|
| `cargo fmt --check`                                                | Clean for my files (one unrelated diff in `chan-server/src/terminal_sessions.rs` from another agent's WIP; not mine to format). |
| `cargo clippy --workspace --all-targets -- -D warnings`            | Clean (one `clippy::manual_contains` caught locally, fixed in-flight before commit). |
| `cargo test --workspace`                                           | All pass.                                     |
| `cargo build --workspace --no-default-features`                    | Clean.                                         |
| `web/` `npx svelte-check`                                          | 10 errors in `GraphPanel.svelte` — not from -25 (`web/` WIP from another agent). |
| `web/` `npm run build`                                             | Not re-run (no SPA change in `-25`; desktop SPA is plain JS in `desktop/src/`). |

### Files to stage

```
desktop/src-tauri/src/main.rs
desktop/src-tauri/src/serve.rs
desktop/src/main.js
desktop/src/styles.css
docs/journals/phase-8/fullstack-b/fullstack-b-25.md
```

Atomic per-path `git commit --only` per
`feedback_shared_worktree_commits`. Multiple unrelated
agents' WIP in the tree right now (`chan-server`,
`web/components`, `Cargo.lock`).

### Suggested commit subject

```
chan-desktop: tighten orphan-detect heuristic + render candidate PIDs in reclaim dialog (fullstack-b-25)
```

### Runtime walkthrough

Source-side tests + structural pins are comprehensive.
Leaving the runtime visual smoke to @@WebtestB per the
task body's "Walkthrough (@@WebtestB walks runtime once
committed — that's a separate dispatch)" out-of-scope
clause. My standing chan-desktop runtime perm is available
if you'd rather I run a quick `make run` + manually create
an orphan + observe the modal myself.
