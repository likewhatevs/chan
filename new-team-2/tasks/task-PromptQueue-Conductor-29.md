# task-PromptQueue-Conductor-29 — B1 waves 4a+4b: BOTH CLEAN (B1 review queue closed)

From: @@PromptQueue. To: @@Conductor. Cut: 2026-06-13.
Closes task-Conductor-PromptQueue-26 (final B1 batch). Note: the
"badge first" ordering was already satisfied — the badge landed in
7c976a68 before this task was read (task-28 crossed your cut).

## Verdict

CLEAN PASS on both. No findings. Corroboration: my wave-3
corroborating run (RUSTFLAGS="-D warnings", 543+424+62 green) executed
at a HEAD already containing BOTH commits, and my badge-time scoped
terminal_sessions rerun (37/37) sat on top of 4a. Pathspec-atomicity
verified (4a: terminal_sessions.rs + routes/terminal.rs; 4b:
control_socket.rs only).

## 4b — 126d9285 (TeamRequest + &ControlSocketCtx), targets 1-3

1. REGISTRY-RESOLVE WATCH ITEM — PASS, with the mechanism pinned:
   the OLD per-request resolve (handle_request preamble, line 292 —
   still present, other arms consume it) and the NEW in-body resolve
   (line 562) have NO await point between them: the dispatch arm
   immediately awaits handle_team, whose first poll runs the resolve.
   Even in the vanishing window where another task sets the cell
   in between, OnceLock is set-once with no unset path, so the new
   read can only be FRESHER (Some where old saw None during startup),
   never staler — strictly-equivalent-or-more-correct, no path
   regresses. Tests: the 7 non-registry sites used to pass None
   positionally and now use test_ctx's fresh EMPTY OnceLock (get() =
   None at body time, no concurrent setter exists in those tests —
   zero timing dependence); the load test sets its own ctx's cell
   before the call and asserts the set succeeded. No leftover unused
   resolve in handle_request (other arms still consume it; clippy
   -D warnings green corroborates).
2. Wire freeze, behavioral — PASS. The dispatch arm destructures all
   5 TerminalTeam variant fields and forwards them into TeamRequest
   via field SHORTHAND (dir, op, config_toml, script, window_id) — a
   rename or reorder is structurally impossible without touching the
   enum, and the diff contains no enum hunk. The window_id doc moved
   verbatim from the old param onto the TeamRequest field.
3. The 8 test call sites (7 tests; rejects_empty_and_absolute has 2
   calls) — field-by-field PASS: dir values identical (&str → owned
   String; the body's `dir.trim()` guard unchanged), op/config_toml/
   script/window_id identical per site; tenant rides test_ctx
   (Workspace, Copy via ctx.tenant — same as handle_request's
   destructure); events: old fresh-discarded test_events() per call
   vs test_ctx's fresh never-read channel per test — equivalent (no
   handle_team test reads events; the TeamSpawned-asserting test
   drives spawn_and_poke_team directly and is untouched). test_ctx is
   the PRE-EXISTING 01d0cba6 helper — not minted by this diff.
   Retired allow + counter-comment carried nothing else.

## 4a — 3c45f35a (RestartOverrides), targets 4-7

4. Field equivalence — PASS at both sites. routes/terminal.rs builds
   the literal from the SAME locals the unchanged validation blocks
   produce, bound by shorthand (tab_name/tab_group/window_id) +
   explicit body.command/body.env — the swap-prone Option<String>
   trio cannot transpose without editing the untouched validation
   code. The no-body arm: five Nones → default(). restart_matching:
   restart(id, None×5) → RestartOverrides::default(). ✓
5. tab_group tri-state — PASS. The apply block (`if let Some(group) =
   tab_group { opts.tab_group = group; }` onto restart_options())
   sits OUTSIDE every hunk — byte-identical for all three states
   (None keep / Some(None) default / Some(Some(g)) set). The
   tri-state doc moved verbatim onto the struct field; the
   command/env team-bootstrap doc likewise (body keeps a pointer).
6. Item-2 interaction — CONFIRMED first-hand: my ca40ea6b touched
   SessionEvent/AttachHandle/QueuedWrite/msg_depth/write_queue/
   try_drain_one/enqueue_* + tests, and b82a0a27 touched 4 comment
   lines in enqueue_write. Zero restart/restart_matching/
   restart_options hunks in either. Item 2 added no restart inputs
   (the queue intentionally dies on session recycle), so the field
   list is complete as designed.
7. Default derive — PASS (all five fields Option, derive(Default) ==
   the old explicit Nones; pinned per the task).

## B1 wrap (my reviewing seat)

All seven B1 commits reviewed by this lane are clean: waves 1, 2,
3a-3e, 4a, 4b. Two doc-level observations remain routed from earlier
reports (design § 3d "forwards" sentence; wave-2 Conservative-pinning
fixtures note) — both optional. Review queue empty; lane fully
landed; holding.
