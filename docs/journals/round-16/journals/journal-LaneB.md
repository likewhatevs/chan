# journal-LaneB

## 2026-06-02 - task-LaneA-LaneB-1: chan-desktop launcher redesign DESIGN doc

Self-identified as @@LaneB from $CHAN_TAB_NAME. Poked by @@LaneA to own
the design doc (design-first, @@Alex approved). Read:

- task: new-team-1/tasks/task-LaneA-LaneB-1.md
- draft + 3 screenshots: docs/journals/phase-16/desktop-redesign-draft/
- current launcher: desktop/src/index.html, main.js (~1400 lines),
  styles.css
- Tauri side: src-tauri/src/main.rs (window machinery), capabilities/*,
  permissions/app.toml, tauri.conf.json
- the model to mirror: web/src/components/TeamDialog.svelte
- sibling tasks (to shape PENDING placeholders): task-LaneA-LaneC-1.md,
  task-LaneA-LaneD-1.md

Peer inputs NOT yet on disk: launcher-inventory-LaneC.md,
spa-settings-gap-LaneD.md. Per task, started on the C/D-independent parts
and grounded the rest in my own source read so the doc is buildable now;
marked the two integration points PENDING-C / PENDING-D.

Key findings (grounded):

- frontendDist = "../src" (tauri.conf.json): the LAUNCHER front-end is
  desktop/src/* loaded directly, NOT the rust-embed web bundle. Launcher
  edits need only a Tauri rebuild, no web/ npm build. No JS bundler or
  test harness in desktop/ (plain vanilla JS): launcher correctness is
  app-smoke only, and chan-desktop is WKWebView so Chrome MCP cannot
  reach it.

- Capability model decides modal-vs-window: default.json binds the
  `main-window` permission set + dialog:allow-open to windows ["main",
  "main-*"]. A MODAL in the existing main window already has every command
  it needs (add_workspace, add_outbound_workspace, tunnel_start/status/
  stop, compute_workspace_preflight) + the folder picker. A separate
  window needs a new HTML entry + a capability binding for zero functional
  gain. -> Recommend in-launcher MODAL. (C confirms the machinery; my
  recommendation does not depend on new window code.)

- Team Work model = an in-app modal overlay with a segmented "real estate"
  toggle (Tabs vs Split) that swaps the body layout per mode
  (TeamDialog.svelte:519-598). That IS the "each of the 3 choices shows a
  different layout" interaction @@Alex wants for [New].

- Draft's ESC-on-Team-Work bug appears ALREADY FIXED: capture-phase
  Escape handler landed 2026-06-01 (6100ec84, TeamDialog.svelte:274-289).
  Will flag to @@LaneA as verify-not-fix.

Wrote new-team-1/desktop-redesign-design.md (v1). Holding the two
integration sections for @@LaneA's re-poke with C + D before cutting the
final completion task. Sent a lean status poke to @@LaneA pointing at the
v1 draft for early review.

### Re-poke: C+D recon IN; finalized

@@LaneA appended C+D findings + decisions to my task file and re-poked.
Read both deliverables:
- launcher-inventory-LaneC.md: confirms my independent source read end to
  end. Key: a REAL second window IS cheap (open_new_launcher_window
  main.rs:1947, `main-N` inherits default.json capability, no new
  capability file). C assigned me the add-time-toggle consistency call.
- spa-settings-gap-LaneD.md: NO GAP. bge + reports both live in the SPA
  Dashboard (SearchSlotConfig.svelte:287, WorkspaceSlotConfig.svelte:38).
  Gear removal strands nothing; only out-of-workspace bulk toggling goes.

DECISION FLIP (grounded in C): v1 leaned modal ONLY because of a presumed
capability cost. C confirmed there is none, and the draft explicitly says
"open a new window." So FINAL recommends a real NEW WINDOW (singleton
`main-new` label, inherits default capability, dedup+self-close), with the
modal documented as the lighter fallback (D2 for @@Alex).

OTHER DECISIONS in the final doc:
- D1 (mine, C-assigned): KEEP the add-time bge/reports toggles in the
  Local choice (creation-time selection, load-bearing for first-index;
  distinct from the removed ongoing gear). Tradeoff stated.
- Gear removal: RESOLVED safe by D; folded in with the bulk-toggle nuance.
- D3 remote ON cell (dot vs badge), D4 tagline: minor, recommended.
- ESC bug: already-fixed heads-up (6100ec84), out of scope.

Rewrote desktop-redesign-design.md as FINAL (status flipped, PENDING-C/D
markers resolved, files-changed + lane split for the window approach, all
cites file:line). Cut completion task-LaneB-LaneA-1.md + poked @@LaneA.

### Re-poke: @@Alex signed off MODAL path; BUILD (task-LaneA-LaneB-2)

@@Alex locked: D2=MODAL (not the window), D3=connection dot, D4=drop
tagline, D1=keep add-time toggles. I own desktop/src/* only; NO Rust, NO
new files. @@LaneC deletes the Rust get/set_workspace_features in parallel
(disjoint: I removed the JS callers).

Built (modal path), desktop/src/ only:
- index.html: header now enso + "Workspaces" + single [New] + theme
  toggle. Dropped #open-workspace, #tunnel-btn, #tunnel-panel-slot, and
  the .brand-tagline em (D4).
- main.js: full rewrite of the changed surface. Added showNewWorkspace-
  Dialog (overlay modeled on showPreflightDialog + a 3-button segmented
  switch like TeamDialog's real-estate toggle; ESC/backdrop/[X] dismiss;
  dismiss never calls tunnel_stop). Local choice = picker -> in-body
  preflight scan + the 2 add-time toggles (D1) -> add_workspace. Outbound
  = URL+Name -> add_outbound_workspace. Inbound = port form -> tunnel_start
  -> listening state (Local|Tunnel seg + snippets + Stop/Done). Deleted the
  inline tunnel panel (5 fns) + the gear (5 fns) + cssEscape. Rows ->
  ON|WHERE: renderWhere(d) + new ic-outbound/ic-inbound glyphs; remote ON
  cell = connection dot (D3), dropped url/tunnel text tags; thead "Where".
  Empty-state + boot first-run -> showNewWorkspaceDialog('local').
- styles.css: added .nw-* (clones preflight overlay + team-realestate
  toggle), .conn-dot, .where-cell/.where-dir, rehomed .seg-toggle/.snippet;
  removed .brand-tagline, .tag*, .tunnel-panel*, .features-*.

Verification:
- node --check main.js: syntax OK. CSS braces balanced 113/113. Grep
  audit: no leftover removed-symbol refs (only a doc comment naming the
  pattern it copies).
- `cd desktop && make build`: GREEN (exit 0), built Chan.app + the DMG.
  No warnings attributable to desktop/src (frontend is loaded directly,
  not compiled). Re-built green on final source after restoring path-cell
  on remote cells (ellipsis for long URLs; the icon still carries
  direction).
- Cannot WKWebView click-through (Chrome MCP = Blink); @@LaneD + @@Alex
  drive the final smoke.

CROSS-LANE FLAG (not mine to fix): build showed a dead_code warning
`method live_workspace is never used` (embedded.rs:77) - src-tauri is
@@LaneC's lane, and it is now orphaned (only def, no callers) because C's
deletion of set_workspace_features removed its only caller
(resolve_workspace_for_features). `-D warnings` (CI/pre-push) will fail on
it, so C must also delete live_workspace. Flagged to @@LaneA; did NOT
touch src-tauri.

Did not commit (per task; @@LaneA commits after verify). Cut
task-LaneB-LaneA-2.md + poked @@LaneA.

### ACCEPTED + HOLD

@@LaneA accepted task-2 (modal frontend done, make build green). My
live_workspace cross-lane flag was ALREADY RESOLVED: @@LaneC deleted
EmbeddedServer::live_workspace in its dead-code cascade; @@LaneA verified
embedded.rs clean + clippy -D warnings green (tree is 7 modified files now,
C's embedded.rs edit landed after my 6-file snapshot). No action for me.

Done for this round. HOLDING. Will not commit (lead commits after verify).
@@LaneA may re-poke if @@Alex's smoke turns up a frontend fix in
desktop/src/*.
