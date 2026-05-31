# Lane C journal — Terminal + chan shell

Self-documented progress for @@LaneC (subagents C-term, C-shell). Curated
status goes to @@Alex; full context lives here.

## Round 1 decisions (from @@Alex, 2026-05-30)
- `cs` provisioning: docs-only this round (dispatch + symlink test, no PATH/env
  injection in terminal_sessions.rs).
- `cs term write`: raw bytes only, no implicit newline; `--stdin` raw.
- BUG-3: validate on a test server with a real agent; confirm drive + seed
  first; flag empirically-unverified if no agent CLI reachable.
- Carousel: single-r spelling everywhere (roadmap's `carrousel` is a typo).

## Work order
1. (in progress) C1 BUG-3 Shift+Enter stale-zero-after-reconnect.
2. C2 tab-groups + group-scoped broadcast -> reach CK-GROUP.
3. SHELL-1 `Command::Shell` + `cmd_shell` + `argv[0]=="cs"` + symlink test.
4. SHELL-2 control_socket.rs extensions (category 1 push + category 2 registry
   read handle). Group fan-out gated on CK-GROUP.
5. SHELL-3 `handleWindowCommand` SPA arms.

## Log

### C1 — BUG-3 Shift+Enter (in progress)
Root cause confirmed in source: `terminalMetaKeyBytes` (keymap.ts:56-59) emits
the modified-Enter sequence only if the SPA observed the agent's protocol
negotiation. `start()` resets protocol state (TerminalTab.svelte:579) and
`keyboardProtocol` is a fresh object per mount (:175). On reattach to a
surviving PTY (`connect()` sees `tab.terminalSessionId`, :626) a long-lived
agent never re-emits its CSI negotiation -> stale-zero -> Shift+Enter -> plain
`\r` -> submit.

Empirical lifecycle finding: there is NO same-instance WS auto-reconnect
(host-resume listeners only repaint; ws.onclose just sets status). The
reconnect-to-surviving-PTY path is a FULL component remount, so the
component-local `keyboardProtocol` const is born fresh-zero -> candidate (a)
"don't reset" is meaningless on a component-local object, and candidate (c)
"sane default" is unsafe (it would emit escape bytes into a plain shell that
never negotiated). Correct fix: RELOCATE the protocol state onto the `tab`
object (survives remount) and reset it only on a genuine fresh spawn.

Implemented:
- `TerminalTab.keyboardProtocol?` field + `ensureTerminalKeyboardProtocol(tab,
  fresh)` helper in tabs.svelte.ts (lazy create; reset in place only when
  fresh). Not serialized -> page-reload/session-restore reattach is the one
  remaining stale-zero window (documented limitation, flagged to @@Alex).
- TerminalTab.svelte: start() resets only when `!tab.terminalSessionId`
  (fresh); key handler reads `tab.keyboardProtocol`; controlled restart resets
  in place (reused session id but fresh shell would otherwise inherit the
  killed agent's modifyOtherKeys -> inverse regression).
- Tests: 3 cases in tabs.test.ts (fresh creates zero; reattach keeps; fresh
  resets in place, same object ref).

Gated-green: vitest 164 pass (tabs+keymap), TerminalTab 9 pass, svelte-check
0/0. Still needs: real-agent empirical validation (batched with C2 on one test
server).

### C2 — tab-groups + group broadcast (code-complete)
Group is a plain string (no lifecycle), default "default" via
`terminalTabGroup()`. Frontend:
- `TerminalTab.group?` field + `terminalTabGroup`/`setTerminalGroup`/
  `DEFAULT_TERMINAL_GROUP` in tabs.svelte.ts; `OpenTerminalOptions.group`.
- Broadcast scoped to source's group: `terminalBroadcastMemberIds`,
  `broadcastTerminalInput`, `toggleActiveTerminalBroadcastSelectAll`
  (Cmd+Shift+I), and the component's `broadcastTargets` picker all filter to
  same-group. 3 group tests added (membership scope, fan-out boundary,
  grouped select-all).
- Serialized as `tg` (non-default only) in serializeTab + restore, so a
  reattach after reload keeps the group consistent with the server.
- "Group" context-menu field below "Name", PENDING-until-restart model: typing
  stages `groupDraft`; `restart()` commits it past the cancel gate. This keeps
  the SPA `tab.group` (broadcast) and the server `tab_group` in lockstep (both
  change only at restart) so they never diverge. session.ts carries
  `tab_group` on the WS query (non-default only); restart body carries `group`.

Server:
- `CreateOptions.tab_group` + `Session.tab_group`; `$CHAN_TAB_GROUP` always
  exported (default when unset) beside `CHAN_TAB_NAME`; added to clear-list.
- `routes/terminal.rs`: `TerminalQuery.tab_group` + `normalize_tab_group`
  (blank/"default" -> None); `RestartTerminalBody.group`; restart uses
  `Option<Option<String>>` (keep / set-default / set-group).
- **CK-GROUP surface (for C-shell):** `Registry::session_summaries()` ->
  `Vec<TerminalSessionSummary{session_id,tab_name,tab_group,cwd}>` and
  `Registry::write_input_matching(tab_name, tab_group, data) -> usize`. These
  two are dead until SHELL-2 consumes them (expected; same lane, same round).

Gated: cargo check -p chan-server --tests OK (2 dead-code warnings = the
CK-GROUP accessors, cleared by SHELL-2). svelte-check 0/0; vitest session+tabs
160 pass. Still needs: real-agent validation + 4-terminal (2 default, 2 foobar)
broadcast walk on a test server.

**CK-GROUP reached** -> C-shell can wire `cs term list`/`write` to the
registry read handle.

### SHELL-1/2/3 — chan shell / cs (code-complete)
- **SHELL-1 (main.rs):** `Command::Shell { action: ShellAction }` + `cmd_shell`
  / `cmd_shell_term`; `parse_cli()` rewrites argv when argv[0] basename ==
  "cs" so `cs <action>` == `chan shell <action>` (symlink is the user's to
  make; build ships none). `term` is a nested group: new/write/list.
  Integration test `crates/chan/tests/cs_alias.rs` (3 cases: cs dispatches,
  cs --help lists shell actions, plain `chan term` is rejected).
- **SHELL-2 (control_socket.rs):** ControlRequest gains OpenGraph /
  OpenTermNew / OpenDashboard (category 1, push window_command) + TermWrite /
  TermList (category 2, registry). Registry read handle plumbed via a
  set-once `TerminalRegistryCell` (OnceLock) filled in lib.rs after the
  registry is built (breaks the start-order cycle, preserves the
  bind-failure control_socket_path semantics). `term write` requires >=1
  selector + errors on no match; `term list` returns `{"groups":{...}}`.
  Client `send_control_request` now returns the Ok message so `term list`
  prints JSON to stdout; everything else to stderr. `term write` = raw bytes,
  no implicit newline (@@Alex decision). 8 new control_socket tests.
- **SHELL-3 (store.svelte.ts):** `WindowCommandFrame` + `handleWindowCommand`
  arms for open_graph (file/dir/workspace), open_term_new (cwd/tab_name ->
  title/tab_group -> group), open_dashboard (+carousel_index set on the new
  DashboardTab; carouselSlide is Lane A's stable field, set from my region,
  no Lane A edit). No SPA arm for term write/list (server-side).

Scope notes for @@Alex:
- `cs dashboard` ships `--carousel-index` only; dropped the roadmap's
  `--carrousel-on` (auto-advance on/off is entangled with Lane A's dashboard
  redesign; deferred to round-2). Flagging.
- `cs term write --stdin` reads stdin to EOF then sends one TermWrite (raw
  UTF-8). NOT chunk-streamed; `tail -f | cs term write --stdin` would buffer
  until EOF. True streaming is a round-2 refinement (control-socket protocol
  is one-request-one-response). Flagging.
- `cs` provisioning is docs-only (your decision); help text documents the
  symlink. No PATH injection.

Gated: cargo check -p chan-server -p chan --tests 0 warnings; control_socket
11 pass; cs_alias 3 pass; svelte-check 0/0. Pending: full gate (fmt/clippy/
--no-default-features/npm build) + empirical test-server validation.

### Full static gate (C-term + C-shell) — GREEN for my scope
- cargo fmt --check: clean.
- cargo clippy --all-targets -- -D warnings: clean (added
  `#[allow(clippy::enum_variant_names)]` on WindowCommand: the shared `Open`
  prefix IS the wire contract, renaming would rename the open_* command
  strings the SPA matches).
- cargo test -p chan-server (315) + -p chan (59 unit + cs_alias 3): all pass.
  Added a real-PTY assertion that a no-group terminal exports
  $CHAN_TAB_GROUP=default.
- cargo build --no-default-features: clean.
- svelte-check 0/0; npm run build: clean.
- vitest: 1575 pass. Updated 3 source-pattern tests broken by my structural
  changes (terminalGeneratedReplyFanout: reset -> ensureTerminalKeyboardProtocol;
  altSpaceXtermHandlerRemoved: keyboardProtocol -> tab.keyboardProtocol;
  terminalRightClickRevamp: Name -> Group -> status row).

**Not mine (flag to @@Alex):** 2 vitest reds in
`screensaverThemes.test.ts` come from @@LaneA's in-flight MatrixRain work in
the shared worktree (MatrixRain.svelte modified + new MatrixRainPreview.svelte
/ matrixRain.ts). I did not touch screensaver; left it for @@LaneA.

### Empirical validation (DONE, 2026-05-30) — test server /tmp/chan-test-lanec
Served from /tmp/lanecsrv (renamed copy) on port 8799, scoped pkill, seeded
notes/projects. All three agent CLIs (claude/codex/gemini) are present.

**BUG-3 — the empirical investigation changed the fix.** Probing with `cat -v`
(shows the exact bytes Shift+Enter emits: modifyOtherKeys -> `^[[27;2;13~`):
1. Pre-reload, modifyOtherKeys on: `A^[[27;2;13~B` -> keymap correct.
2. Page reload, RECENT negotiation: `C^[[27;2;13~D` -> the reattach REPLAY
   re-feeds the still-recent negotiation to the parser, re-establishing state
   on its own (this always worked; the in-memory relocation is irrelevant to a
   page reload, whose heap is gone).
3. Page reload, negotiation EVICTED from the 1 MiB replay ring (ran
   `seq 1 250000` after it): `E`<newline>`F` -> Shift+Enter SUBMITTED. The real
   BUG-3, and the in-memory fix does NOT cover it (reload drops the heap; the
   evicted negotiation isn't in replay to re-establish).

**Conclusion:** the page-reload-with-long-lived-agent case (the likely primary
real-world trigger) needs the negotiated state SERIALIZED. Added:
- keymap.ts: `serializeKeyboardProtocolState` / `restoreKeyboardProtocolState`
  (compact `{x,km,ka,s}`; null for default). 3 unit tests.
- tabs.svelte.ts: `SerTab.kp` emitted in serializeTab for a live session with
  non-default state (NOT in the shareable hash); restored onto
  `tab.keyboardProtocol` in the deserializer via `savedTerm?.kp`. 2 tests.
- The negotiation output itself triggers a session save (recordOutputBytes ->
  scheduleTerminalSessionSave), so kp is persisted promptly; no explicit
  save-on-change needed.
4. Re-ran step 3 with the rebuilt binary: `G^[[27;2;13~H` -> **FIXED**.

So the complete BUG-3 fix is TWO layers: (1) in-memory tab relocation covers
heap-intact remounts (tab move, old-design flip); (2) kp serialization covers
page reload past the replay window. Both empirically confirmed.

**C2 groups + broadcast — validated:**
- `$CHAN_TAB_GROUP=default` injected at runtime; control socket + window id
  exported.
- "Group" context-menu field present below Name; typing a new group shows the
  "applies on restart" prompt and does NOT mutate `tab.group` (hash unchanged)
  -> pending-until-restart works; SPA + server never diverge.
- Restart commits: shell respawns with `$CHAN_TAB_GROUP=foobar`, hash gains
  `tg:foobar`, and `cs term list` moves the session under group "foobar"
  (new session_id) -> SPA + registry update in lockstep at restart.
- Broadcast picker is group-scoped: a default terminal's picker lists only
  itself (self), never the foobar "worker".

**C-shell — validated end-to-end (real `cs -> chan` symlink on PATH):**
- `cs term list` -> grouped JSON to stdout; no-selector `cs term write` errors;
  `cs term write "echo X"` types but does NOT execute (raw bytes, no newline);
  `cs term write --tab-group foobar` writes to all foobar sessions.
- Category 1 all open the right tab (verified in the session hash):
  `cs open notes/ideas.md` -> ideas.md editor; `cs graph projects` ->
  graph `gs:dir:projects`; `cs term new --tab-name worker --tab-group foobar`
  -> terminal `n:worker, tg:foobar`; `cs dashboard --carousel-index 2` ->
  dashboard `cs:2` (carousel dot at slide 2).

Final gate after the serialization addition: svelte-check 0/0; vitest 1582
pass / 0 fail (the earlier 2 screensaver reds cleared as @@LaneA aligned their
MatrixRain tests); npm build clean. Rust gate unchanged (no Rust edits in the
serialization addition): fmt/clippy/test/`--no-default-features` still green.
