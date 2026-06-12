# task-ChanGateway-Lead-4 — tasks 6+7 complete (batched): web commits + drop IPC + web guard

From: @@ChanGateway. To: @@Lead. Re: task-Lead-ChanGateway-6 + -7.
Review-only. Verdict: **ACCEPT all five commits.** Two minor findings
(both doc/help-text accuracy, route to @@Chan) + one smoke
recommendation.

## task-6 — @@Chan's web commits

- **51664864** (web scrub, 60 files): PASS. Sampled 5 pin re-anchors
  (reportsToggleClient, graphChipCountSemantics,
  graphParentEdgeInvariant incl. the POSITIONAL one, TerminalTab,
  graphFileBucketChips): all equal-or-stronger — the positional
  BFS-before-pull anchors are unique as regexed (the shorter
  "Forward-only BFS" prefix appears twice in GraphPanel but the full
  anchored phrase only once; 1168 < 1187 ordering holds). The EXT_RE
  identifier rename is complete (rg: zero FA57 survivors) with pins
  updated. All 3 corrected-FALSE comments verified TRUE against code:
  onSetAsScope is passed at 4 GraphPanel sites with exactly the
  documented graphFromHere / rescopeFromHere split; date.ts's popover
  is real (openDatePopover imported + wired); the display-only-row
  rewrite matches the lens re-scope wiring.
- **a9daa17b** (3 shortcut registry entries): PASS with finding F-W1.
  app.pane.closeEmpty dispatch is exactly the documented conditional
  (meta = metaKey||ctrlKey; preventDefault ONLY when
  closeActiveEmptyPane() actually closed an empty pane, else
  fall-through to browser/native close). terminal.richPrompt
  dispatch requires physical Cmd (metaKey && !ctrlKey) matching the
  registry's literal-Cmd entry; TerminalTab's menu label now reads
  chordFor("terminal.richPrompt") — drift-proof. SERVE_LONG_ABOUT +3
  rows match shortcuts.ts.
  - **F-W1 (minor, help text):** the new "Show/Hide Rich Prompt —
    Cmd+Shift+P" row in SERVE_LONG_ABOUT carries no note, so under
    the table header "(Cmd = Ctrl on Linux / Windows)" a Linux user
    reads Ctrl+Shift+P — which the handler deliberately ignores.
    The closeEmpty row proves the generator renders `note:` fields;
    add one to the terminal.richPrompt registry entry (e.g.
    "physical Cmd on every platform") and regenerate. Same mislabel
    class this commit fixed in the right-click menu.
- **c92e4d14** (warnings to zero): PASS. chunkSizeWarningLimit KEPT
  at 1600 (ceiling, not disabled); onwarn drops exactly ONE code
  (INEFFECTIVE_DYNAMIC_IMPORT), everything else still warns — not
  module-scoped, but the message never claimed that, and with
  splitting a documented non-goal the advisory carries no signal
  (observation, not a finding). RichPrompt svelte-ignore is
  element-scoped, one added rule.
- **e60ab688** (stragglers): PASS. Comment-only (zero non-comment
  changed lines by extraction); residual rg sweep of web/src +
  chan-server: clean (only "Phase 1/Phase 2" UX-flow prose remains —
  legitimate, not archaeology).

## task-6 — @@ChanDesktop's drop IPC (79de0e95)

PASS — contract-conformant on every point, both amendments included:

- ACL (amendment 1): allow-read-dropped-paths in its own
  capabilities/local-drop.json, windows ["workspace-*","terminal-*"];
  the serve.rs test pins the POSITIVE grants AND the negatives
  (tunnel-*/outbound-*/main excluded from the capability; the
  permission absent from BOTH broad surfaces tunnel/outbound receive
  — workspace.json and the workspace-window app.toml set). That's
  the leak-proof shape you asked for, not file-existence pinning.
  Defense I verified beyond the asks: the capability's remote.urls
  is loopback-only, so even a workspace-* webview navigated off
  localhost loses the grant; the label exclusion is what holds the
  tunnel-window boundary (their URLs are loopback too) and the
  labels are exactly what the test pins.
- Amendment 2: read on the main thread (run_on_main_thread +
  oneshot), test-pinned via the include_str! source pin.
- dropped_paths.rs read whole: minimal unsafe (the two AppKit extern
  statics), every degenerate case → [] (no file type available, nil
  plist, non-array plist, non-string entries), paths never logged,
  NSFilenamesPboardType mirrors wry's own collect_paths (parity
  rationale documented), no added normalization — plist strings pass
  through NSString→String UTF-8 only, spaces/unicode intact.
- Non-blocking notes: (a) landed signature is
  Result<Vec<String>,String> vs the contract's bare Vec<String> —
  benign; the SPA must handle invoke rejection for the ACL case
  anyway, and a19d7d40's wrapper does. (b) default.json has no
  negative pin for the permission — launcher is locally-served so
  it's outside the threat model; add one only if you want belt
  symmetry.

## task-7 — @@Chan's web guard (a19d7d40)

PASS — conforms to the frozen contract + both your amendments:

- Files-discriminator: gates all three listeners (dragover capture,
  drop capture, drop bubble); BOTH directions vitest-pinned
  (['text/plain'] dragover AND drop not prevented; ['Files']
  prevented outside zones with dropEffect="none"). No
  stopPropagation anywhere — zone handlers untouched, per contract.
- Allowlist: Wysiwyg/Source/RichPrompt hosts + terminal panes +
  .cm-editor. RichPrompt as a zone is INTENDED: it's a drafts-backed
  editor with image-drop machinery; the marker preserves pre-guard
  behavior. The contract's "file-browser upload zone" entry is
  correctly obsolete: uploads are Upload-button-only and
  fileBrowserUploadDrop.test.ts pins "no longer routes external file
  drops to upload" — deviation justified AND pinned.
- The bubble-phase net (cancels drops zone handlers left unhandled,
  e.g. read-only editors) closes a gap the contract didn't even
  name. Good engineering.
- Terminal path-print: preventDefault fires SYNCHRONOUSLY before the
  async IPC await (the ordering that matters — awaiting first would
  reopen the navigation window); paths ride sendUserInput (normal
  input path, incl. broadcast-input semantics, consistent with typed
  input). Escaping pinned exactly per contract ('…', ' → '\'' ,
  space-separated, single trailing space, [] → ""). I probed
  newline-bearing filenames adversarially: a quoted \n stays inside
  the single quotes — quoting-safe, no finding.
- ACL degrade: readDroppedPaths returns [] on plain browser
  (isTauriDesktop gate) and catches invoke rejection → [] — nothing
  thrown to console.
- Svelte-5 runtime risk: LOW. Guard install is a plain onMount side
  effect; zone markers are attribute-only; the terminal handler
  mutates no $state in derived contexts.
  - **Recommendation:** the contract's verification split assigns
    @@Chan a browser smoke (drop on graph/search → nothing). Their
    commit message lists static gates only. CDP can't fully
    synthesize OS-file DataTransfer, so the honest path is folding
    three checks into @@Alex's manual desktop smoke: (1) Finder-drag
    onto Graph/Search → not-allowed cursor, no navigation; (2) drop
    on terminal → quoted paths typed, trailing space; (3) drag a
    pane tab → completely unaffected.

### Findings index

- F-W1 (minor, @@Chan): SERVE_LONG_ABOUT Rich Prompt row needs a
  literal-Cmd note (a9daa17b).
- F-W2 (nano, @@Chan): pre-existing test name in
  fileBrowserUploadDrop.test.ts:30 reads "upload progress can
  workspace status" — a drive→workspace codemod scar (was "can
  drive status…"); rename the test.
- Smoke recommendation above for the drop arc.

Journal: new-team-1/journals/journal-ChanGateway.md.
