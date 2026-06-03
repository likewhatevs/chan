# journal-LaneB (terminal & cs core)

Append-only. Owner: @@LaneB.

## 2026-06-02 - boot

- Read team bootstrap.md. Self-identified from $CHAN_TAB_NAME=@@LaneB.
- Lane: terminal & cs core. Owned files:
  - web/src/App.svelte
  - web/src/components/{RichPrompt,TerminalTab,BubbleOverlay,Pane}.svelte
  - web/src/state/richPrompt.svelte.ts
  - web/src/terminal/submitMode.ts
  - web/src/state/tabs.svelte.ts (pane-mode region only)
  - crates/chan-shell/* (cli.rs, submit.rs)
  - crates/chan-server/src/control_socket.rs (pane-exec region only)
- Assigned items: B1 (rich prompt per-terminal), B4 (cs pane split/close),
  B8 (cs --submit codex). Wave 1 start = B8 (reproduce first).
- Status: HOLDING. Waiting for @@LaneA to cut tasks/task-LaneA-LaneB-1.md
  and poke me. Will not start until poked with the task path.

## 2026-06-02 - B8 empirical probe (cs --submit codex)

Poked by @@LaneA: Wave-1 = B8. Task: tasks/task-LaneA-LaneB-1.md.

Setup: codex-cli 0.136.0 (authed, gpt-5.5). Spawned an isolated probe tab
`@@CodexProbe` (group `probe`) in the live desktop session via `cs terminal
new`, launched codex in it. Observed codex's rendered screen by dumping the
replay ring (`cs terminal scrollback`) and replaying it through pyte (codex
draws in place; raw ANSI is unreadable). Probe script: /tmp/render_codex.py.

Input-mode recon (grep of the raw ring): codex enables bracketed paste
(\e[?2004h) AND the kitty keyboard protocol with flags \e[>7u (1 disambiguate
+ 2 report-event-types + 4 report-alternate-keys, but NOT 8 report-all-keys).
Without flag 8, an UNMODIFIED Enter stays the legacy CR -> codex's Enter IS
\r; the bug is purely paste-burst coalescing.

Probe results (each input written as ONE `cs terminal write`, the exact
shape `apply_submit_chord` produces):

  input (one write)                          | result
  -------------------------------------------|---------------------------
  text + \r              (current chord)      | NO submit - parks as newline
  text + \x1b[13u        (kitty CSI-u Enter)  | NO submit - parks
  text + \x1b[27;9;13~   (claude chord)       | NO submit - consumed, parks
  text, then \r as a SEPARATE write           | SUBMITS
  \x1b[200~ text \x1b[201~ \r  (one write)    | SUBMITS  <- the fix

Root cause: codex coalesces a single text+CR write into a paste burst and
treats the trailing CR as a literal newline (matches @@Alex's "command then a
newline, no submit"). Wrapping the text in explicit bracketed-paste delimiters
makes codex insert it as a paste, so the CR after the paste-end marker is read
as a distinct Enter keypress and submits. Stays within the single-write
apply_submit_chord model.

Fix: submit.rs apply_submit_chord wraps codex text as \e[200~{text}\e[201~\r;
mirror in submitMode.ts encodeForAgentSubmit. claude/gemini unchanged (plain
suffix chord). codex's Enter byte is still \r (submit_chord), the wrap is the
delivery.

NOTE for @@LaneA (cross-lane, doc-only mirror): crates/chan-server/src/routes/
team_config.rs::submit_chord_literal (used only by render_poke_chords -> a
bootstrap.md bullet "- codex: --submit=codex (chord \r)") is a 3rd mirror of
the chord map. Still technically accurate (codex's Enter byte is \r) but the
"(chord \r)" parenthetical no longer tells the whole story. Outside my lane
(routes/), so flagging rather than editing. Cosmetic; not blocking.

## 2026-06-02 - B8 DONE (own-gate green + empirically verified)

Files (all my lane):
  crates/chan-shell/src/submit.rs       blob eb3c2a4 (apply_submit_chord
        wraps codex: \e[200~{text}\e[201~\r; docs+tests updated)
  crates/chan-shell/src/cli.rs          blob abc3bb8 (--submit help text:
        codex = bracketed-paste wrap + CR)
  web/src/terminal/submitMode.ts        blob 049eec1 (encodeForAgentSubmit
        wraps codex; mirror of apply_submit_chord byte-for-byte)
  web/src/terminal/submitMode.test.ts   blob 1929653 (codex expectation +
        multi-line preservation)
Other changed tracked files in the tree belong to other lanes (verified).

Own-gate (scoped):
  - cargo fmt -p chan-shell --check        PASS (whole-tree fmt red is
        @@LaneD's chan-workspace WIP, not mine)
  - cargo clippy -p chan-shell --all-targets -D warnings   PASS
  - cargo test -p chan-shell               PASS (34 tests)
  - npm test (full vitest)                 PASS (1646 tests / 167 files)
  - npm run check (svelte-check)           0 ERRORS (1 pre-existing RichPrompt
        a11y WARNING, not from B8)
  - npm run build                          OK

Empirical (live codex-cli 0.136.0, fresh ./target/debug/chan against the
desktop control socket): `cs terminal write --submit=codex $'echo
CHAN_VERIFY_FINAL'` -> codex SUBMITTED and ran it (transcript: "Ran echo
CHAN_VERIFY_FINAL"); composer parked-count = 0. Probe tab torn down (closed
1 tab, 0 sessions). /tmp probe artifacts removed.

Reported to @@LaneA via tasks/task-LaneB-LaneA-1.md. Holding for Wave-2
(B1 + B4) dispatch per the task's "After B8: Hold".

Poke caveat: `cs terminal write --tab-name=@@LaneA` hit 2 sessions - there is a
leftover `new-team-1` group with its own @@LaneA..D alongside our `phase-17`
group. Harmless (read-only pointer) but noisy. FUTURE pokes: add
`--tab-group=phase-17` to scope to our team.

## 2026-06-02 - B1 DONE (rich prompt per-terminal + data-loss fix)

Task: tasks/task-LaneA-LaneB-2.md (Wave-2 B1 + @@Alex's live data-loss bug).

Root cause of @@Alex's data loss: RichPrompt.submit() routed via
sendPromptToActiveTerminal (FOCUSED pane's active tab), but the bubble was
shown WINDOW-GLOBAL (richPrompt.visible bool x TerminalTab `active` = every
pane's active terminal). So the user could type into a bubble that wasn't the
focused terminal, hit cmd+enter, and the text cleared while landing on the
wrong terminal / nowhere visible.

Fix (all in-lane):
- Per-terminal visibility: richPrompt.svelte.ts is now keyed by tab id
  (byTab Record) instead of a global bool. cmd+shift+p (App.svelte) resolves
  the focused terminal via activeTerminalTab() and toggles ONLY that terminal;
  no-op when the focused tab is not a terminal. TerminalTab renders the bubble
  on `active && isRichPromptVisible(tab.id)`; menu + Escape + close are
  per-tab. Two terminals can show their own bubble independently.
- Routing/data-loss: RichPrompt.submit() routes to its OWN tab
  (sendPromptToTerminal(tab.id, text)) and does NOT reap the composer unless
  the prompt frame actually went out to that terminal's OPEN socket (a failed
  send keeps the text to retry). Removed the now-dead
  sendPromptToActiveTerminal (no caller left; pre-release drop).
- Resize-top: a top grab handle drags the bubble's top up to the terminal top
  (mirrors the 12px bottom inset; capped at parent height - 24). Per-prompt.
- Survey z-order: BubbleOverlay (fixed, z:39000, App root) already paints above
  the bubble (z:20 inside terminal-tab z:2); verified, no change needed.

Files (blob fingerprints):
  web/src/state/richPrompt.svelte.ts        f2d44dc
  web/src/App.svelte                        9bd4511
  web/src/components/TerminalTab.svelte     39e75ed
  web/src/components/RichPrompt.svelte      06da00e
  web/src/state/tabs.svelte.ts              f802b46 (removed dead sender; prompt
        region ~1428, far from @@LaneC saveDraft + my pane region)
  web/src/components/richPromptComponent.test.ts      1c638ce
  web/src/components/richPromptTerminalWiring.test.ts 00c0392

Own-gate GREEN: npm test (1646), svelte-check (0 errors; 1 PRE-EXISTING
RichPrompt root-div a11y WARNING that svelte-ignore does not suppress - it was
there before B1 at 188:1, exit 0), npm run build OK.

Empirical (Chrome, fresh binary /tmp/chan-laneb-b1 serving /tmp/chan-b1-ws,
2 panes/2 terminals): per-terminal toggle (focused terminal ONLY, not all
panes); focus+cursor in prompt on show; toggle-off hides; two prompts coexist
independently; submit lands in the bubble's OWN terminal + composer clears on
success; resize-top grows upward + caps at terminal top - inset (per-prompt);
survey renders ABOVE the focused tall prompt; no-op when the focused tab is a
File Browser (and the bubble hides when its terminal tab goes inactive). Tore
down: closed my Chrome tab, killed server by PID, chan remove, rm temp.

ROUTED to @@LaneA (server-side, per the task's STOP+route): @@Alex also asked
for a loader + cancel "confirm-before-reap until the prompt reached the
terminal". The WS `prompt` frame is fire-and-forget; `send()` returns true iff
the WS is OPEN + the frame was written (confirms delivery to the CORRECT
terminal's socket, not server enqueue). A TRUE ack with a visible loader/cancel
needs a chan-server prompt-handler ack (shared with @@LaneD). The in-lane fix
ALREADY prevents @@Alex's data loss (correct routing + reap-only-on-delivery);
the loader/cancel ack is an enhancement for @@LaneA to sequence.

## 2026-06-02 - B12 DONE (direct dashboard chord, out of hybrid nav)

Task: tasks/task-LaneA-LaneB-3.md. @@Alex wants a direct dashboard chord
(today only Hybrid Nav `Mod+. i`).

Implemented (@@LaneA authorized the shortcuts.ts edit in the task):
- shortcuts.ts app.dashboard.open: web "Alt+Shift+D", native "Mod+Shift+D"
  (Cmd+Shift+D mac / Ctrl+Shift+D linux), escapeTerminal:true, note "or
  Mod+. i". Web uses Alt+Shift+D because Cmd/Ctrl+Shift+D is the browser's
  bookmark-all (not preventable) - same split as tab/pane nav.
- App.svelte onWindowKey: a KeyD branch next to cmd+shift+p. Native (guarded
  by isTauriDesktop() + currentOS() per the reload-chord precedent):
  Cmd+Shift+D mac / Ctrl+Shift+D linux. Web: Alt+Shift+D. -> openDashboardIn
  ActivePane(). The `chan:command app.dashboard.open` bridge case already
  existed (App.svelte ~926), so if chan-desktop's KEY_BRIDGE_JS later
  intercepts Cmd+Shift+D it routes there (stopImmediatePropagation, no
  double-fire); otherwise the onWindowKey native branch handles it.
- shortcuts.test.ts: updated the dashboard chord assertion (web Alt+Shift+D,
  native Cmd+Shift+D). The bridge-case test + EmptyPaneWelcome test reference
  the chord via chordLabel(chordId) dynamically - unchanged.

Files (blob fingerprints):
  web/src/state/shortcuts.ts        3e7ed08
  web/src/App.svelte                629cd60 (also carries B1; B12 = the KeyD
        branch)
  web/src/state/shortcuts.test.ts   e2dbac4

Own-gate GREEN: shortcuts.test + dashboard tests (59), full vitest (1647),
svelte-check (0 errors; same 1 pre-existing B1 warning), build OK.

Empirical (Chrome, fresh /tmp/chan-laneb-b12 on :8792): Alt+Shift+D opens the
dashboard in the active pane from a focused TERMINAL (escapeTerminal works) AND
from a focused EDITOR (notes.md). Chord hint "Alt+Shift+D" shows in the
launcher Dashboard tile + the pane hamburger menu (registry propagated). Did
NOT press Cmd+Shift+D in Chrome (browser bookmark-all dialog; our web branch
binds only Alt+Shift+D, isTauriDesktop() false in Chrome -> we don't fight it).
Torn down (server by PID, chan remove, rm temp; @@LaneD's :8810 untouched).

For @@LaneA: native Cmd+Shift+D (mac WKWebView) is @@Alex's hand-smoke (agents
can't drive WKWebView). If WKWebView eats the chord before JS, add Cmd+Shift+D
to chan-desktop KEY_BRIDGE_JS (desktop/src, your lane) - the bridge case
already routes app.dashboard.open, so no further App.svelte change needed.

## 2026-06-02 - cs team mcp_env surface DONE (B5 CLI half)

Task: tasks/task-LaneA-LaneB-4.md. Build the `cs terminal team` surface on
@@LaneD's landed TeamConfig.mcp_env (serde default false, top-level TOML key).
chan-shell ONLY (did NOT touch routes/team_config.rs or control_socket.rs).

Design note: the CLI forwards a raw config TOML string to the server
(config_toml: Option<String>); chan-shell has no toml parser + no TeamConfig.
So `--mcp-env` must INJECT the key into the forwarded TOML client-side. Added
`toml` to chan-shell behind the `client` feature (server still links wire-only;
verified --no-default-features builds) and a parse + set-root-key +
re-serialize (set_team_mcp_env) so mcp_env lands at the document root before
the [[members]] tables (a naive string append can't guarantee that). The
server re-parses + regenerates config.toml, so re-serialization is safe.

Implemented:
- McpEnvToggle ValueEnum (on|off) + `--mcp-env <on|off>` on `team new`
  (Option; omitted = leave the config's value / server default OFF).
- set_team_mcp_env(config_toml, bool): toml::Table insert mcp_env + re-emit.
- Help text: default OFF, what it controls, overrides input config's value.
- `load` needs no CLI change (serde round-trips the field) - verified
  empirically it preserves mcp_env.
- Tests: clap on/off/bogus parse; set_team_mcp_env sets/overrides at root +
  preserves [[members]].

Files (blob fingerprints):
  crates/chan-shell/Cargo.toml   cabb805 (toml dep behind client feature)
  crates/chan-shell/src/cli.rs   9e2ae76

Own-gate GREEN: cargo fmt -p chan-shell --check, clippy -p chan-shell
--all-targets -D warnings, cargo test -p chan-shell (37), cargo build
-p chan-shell --no-default-features (toml stays client-gated, server clean).

Empirical (fresh /tmp/chan-laneb-mcp on :8793, shell-only `command=true`
config so no real agents spawn): team new --mcp-env on -> config.toml
`mcp_env = true`; no flag -> `mcp_env = false`; --mcp-env off ->
`mcp_env = false`; team load --script round-trips (config still
`mcp_env = true`, 3493-byte bootstrap emitted). Help shows the flag + OFF
default + [possible values: on, off]. Torn down (server by PID, chan remove,
rm temp).

Note for @@LaneA: --script emits the bootstrap SPAWN script, not a config
dump, so mcp_env doesn't appear literally in --script output (it's consumed by
the server's spawn-options, @@LaneD's half). That matches the task ("emits it
if it emits the other team fields" - it doesn't emit team fields as such).

## 2026-06-03 - B4 (task-LaneA-LaneB-5): STOPPED + routed (cross-lane)

Reconciled @@LaneA's poke: "the cs-surface (B-4)" = the cs mcp_env surface
(task-LaneA-LaneB-4), already DONE; "B4 still held until D lands B5" + B5 having
landed = B4 unblocked via task-LaneA-LaneB-5. The poke crossed my mcp_env
completion.

Recon'd the full `cs pane split` flow before editing. B4 CANNOT be done within
the task-listed files: both parts land in store.svelte.ts::applyPaneExec
(@@LaneC's lane, not listed), and @@LaneC is likely in store.svelte.ts for B9.
- Flow: cs pane split -> control socket -> chan-server handle_pane_exec
  (forwards PaneOp opaquely, NO dir interpretation -> control_socket.rs needs
  NO change) -> store.svelte.ts applyPaneExec -> splitPane.
- Part 1 (RIGHT|BOTTOM): chan-shell cli.rs SplitDirArg + wire.rs SplitDir
  Left->Right (MINE) must land in LOCKSTEP with applyPaneExec's dir string
  (PaneExecOp `dir: "left"|"bottom"` -> `"right"|"bottom"` + the `op.dir ===
  "left"` split case -> "right" row/after) (@@LaneC's store.svelte.ts).
- Part 2 (focus steal): applyPaneExec -> splitPane (tabs.svelte.ts:2971) sets
  activePaneId = newPane -> cs split steals focus. Fix belongs in the cs path
  (applyPaneExec preserve the sender's focus for split) = store.svelte.ts.
- Part 2 (transaction mode): cs path uses splitPane/closePane (NOT
  paneModeSplit), so it does NOT enter transaction mode today - appears
  already-satisfied; confirm on smoke.

STOPPED per the bootstrap "route if a fix pulls into another lane's file."
Routed to @@LaneA via tasks/task-LaneB-LaneA-5.md with two options (A: authorize
me the applyPaneExec region, coordinate the store.svelte.ts merge with @@LaneC;
B: reassign that piece to @@LaneC + I land the chan-shell half in lockstep).
Recommended A (wire dir + applyPaneExec must change together). Holding ALL B4
edits pending the ruling; chan-shell half ready to land once ownership is set.

## 2026-06-03 - B4 DONE (Option A ruling; landed atomically + verified)

@@LaneA ruled Option A (followup-LaneA-LaneB-1): I own the applyPaneExec region
for B4. Pre-burst quiescence check passed (store.svelte.ts mtime stable;
@@LaneC's only WIP is ~2004-2014 graph, far from applyPaneExec 781-813).

Landed as ONE lockstep burst:
- chan-shell wire.rs SplitDir { Left, Bottom } -> { Right, Bottom } (+ doc).
- chan-shell cli.rs SplitDirArg Left->Right (+ From impl + ShellAction::Pane /
  PaneAction::Split help + 2 tests).
- store.svelte.ts applyPaneExec (AUTHORIZED region): PaneExecOp split dir
  "left"|"bottom" -> "right"|"bottom" (L781); split case op.dir==="right" ->
  splitPane row/after, else column/after (L813); + focus-preserve (capture
  keepActive = layout.activePaneId before splitPane, restore after) so a
  one-shot cs split does NOT steal focus to the new empty pane (L813-822).
  Chose capture/restore over a splitPane focusNew arg - most contained, no
  ripple to splitPane's other callers.
- control_socket.rs: NO change (forwards opaquely, confirmed).

Files (blob fingerprints):
  crates/chan-shell/src/wire.rs   188d8e3
  crates/chan-shell/src/cli.rs    d5394976 (also carries mcp_env; B4 = the
        SplitDir bits)
  web/src/state/store.svelte.ts   bf5bf1d  (B4 = lines 781 + 813-822 ONLY;
        @@LaneC's B9 region is ~2004-2014, no overlap)

Own-gate GREEN: cargo fmt/clippy/test -p chan-shell (37), cargo check -p
chan-server (unaffected), npm test (1656), npm run build OK. svelte-check
full-tree shows 8 errors but NONE in my files - they are other lanes' WIP
(@@LaneA B5 dialog TeamDialogConfig.mcpEnv test fixtures + a SearchPanel union
type). store.svelte.ts itself is svelte-check-clean.

Empirical (Chrome, fresh /tmp/chan-laneb-b4 :8794, cs run FROM the SPA
terminal): `cs pane split right` -> new pane RIGHT, focus STAYS on the sending
Terminal-1 (no steal), no transaction bar; `cs pane split bottom` -> new pane
BELOW, focus stays; `cs pane split left` -> rejected ("invalid value 'left'
[possible values: right, bottom]"); `cs pane` query confirms pane-1 (the
sender) stayed active after both splits; `cs pane close-pane --pane pane-3`
(non-active) -> closed, Terminal-1 keeps focus, no transaction mode. All of
@@Alex's report addressed. Torn down (server by PID, chan remove, rm temp).

This was my last round-1 item. Round-2 R2-3 (per-terminal survey) comes next
per the task.

## 2026-06-03 - R2-3 (task-LaneA-LaneB-6): STOPPED + routed (contract + C/D)

Round-1 pushed (origin/main 03bb91f8). R2-3 = per-terminal survey. Viewed
image-3 (survey anchored over its terminal, not a window-wide modal).

Recon'd the full survey flow + round-3-survey-contract.md (the @@Architect-held
C<->D seam). The blocker is structural: the `open_survey` frame carries NO
target tab (control_socket.rs OpenSurvey ~91 + store.svelte.ts:700 = just
`survey`), so the SPA cannot attach a survey to a specific terminal. Surveys are
WINDOW-targeted today. Per-terminal needs the tab on the frame -> a contract
SHAPE change spanning @@LaneD (transport) + @@LaneC (SPA) + survey.svelte.ts
(store) + BubbleOverlay (mine).

STOPPED per the task ("if the survey state is outside your owned list, route")
+ the B4 precedent. Routed to @@LaneA (task-LaneB-LaneA-7) with the full plan +
a 1-field contract amendment proposal (tab on the open_survey frame; SurveySpec
+ reply path unchanged) + an ownership split (LaneA ratifies contract, LaneD
~2-line transport, ME the full SPA side incl BubbleOverlay per-terminal render
like B1's rich prompt). Holding edits pending the ratification.

## 2026-06-03 - R2-3 DONE (Option A ratified; landed + verified end-to-end)

@@LaneA ratified (followup-LaneA-LaneB-2): open_survey frame gains
`tabName?: string|null` (Some=that terminal, None=window-wide). Authorized me
the full SPA side incl the store.svelte.ts open_survey HANDLER region.

Implemented (B1 rich-prompt pattern for surveys):
- survey.svelte.ts: state keyed by slot - byTab Record<tabId, {spec,busy}> +
  a windowWide slot; showSurvey/pickOption/requestFollowup/surveyFor/surveyBusy
  all take a SurveySlot (tab id or null). Two terminals reply independently.
- store.svelte.ts (authorized handler region): open_survey frame type +
  `tabName` field; terminalSlotForName(name) resolves frame.tabName ->
  tab.id (match terminalEnvTabName then title) via allTerminalTabs; handler
  showSurvey(frame.survey, slot). frame.tabName read camelCase per the contract.
- BubbleOverlay.svelte: parameterized by `tabId` (slot). null = window-wide
  centered modal (fallback); a tab id = per-terminal, anchored over the
  terminal (.per-terminal absolute, z 24000). Card-scoped keydown (1..N/F)
  instead of window-global so each survey owns its keys.
- TerminalTab.svelte: mounts <BubbleOverlay tabId={tab.id}/> over the visible
  terminal (like RichPrompt). App.svelte keeps the App-root <BubbleOverlay/>
  (default tabId null) as the window-wide fallback.
- Tests rewritten: survey store (per-terminal independence), BubbleOverlay
  (window-wide vs .per-terminal render), TerminalTab (inverted the old "no
  per-terminal overlay" pin -> now asserts the per-terminal mount).

Casing: @@LaneD pinned #[serde(rename="tabName")] (camelCase) on OpenSurvey -
matches my frame.tabName read exactly. No drift.

Files (blob fingerprints):
  web/src/state/survey.svelte.ts          014e830
  web/src/state/store.svelte.ts           0a2d96d  (R2-3 lines 60, 703-706,
        952-963, 1031-1040 ONLY; B9+B4 already committed in round-1)
  web/src/components/BubbleOverlay.svelte  b223b18
  web/src/components/TerminalTab.svelte    14d89d7
  web/src/App.svelte                       f9221a2 (comment only)
  web/src/state/survey.svelte.test.ts      4e773ba
  web/src/components/BubbleOverlay.test.ts 0225112
  web/src/components/TerminalTab.test.ts   6814564

Own-gate GREEN: npm test (1663), svelte-check 0 errors (8 full-tree errors are
other lanes' WIP - @@LaneA B5 dialog TeamDialogConfig.mcpEnv fixtures + a
SearchPanel union; my files clean), npm run build OK.

Empirical END-TO-END (Chrome, fresh /tmp/chan-laneb-r23 with @@LaneD's REAL
transport - provenance: tabName in binary): 2 panes/2 terminals;
`cs terminal survey --tab-name=Terminal-1` AND `--tab-name=Terminal-2` raised
in parallel -> Survey A anchored over Terminal-1 (LEFT), Survey B over
Terminal-2 (RIGHT), BOTH coexisting, NOT a window-wide modal; answered Survey A
-> A dismissed, B INTACT (independence); answered B; replies round-tripped
(A-one, B-two) + both blocked CLIs unblocked. Torn down (server by PID, chan
remove, rm temp). This is @@Alex's exact ask + image-3.

R2-3 is my last round-2 item. All my phase-17 work (B8, B1, B12, cs mcp_env,
B4, R2-3) landed + gate-green + empirically verified.
