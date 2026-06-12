# Phase 23 - the tidy-up: archaeology scrub, hygiene, docs currency

Status: closed (round 1; integrated gate green at every checkpoint,
@@Alex desktop smoke passed).
Span: 2026-06-12.
Tags: #hygiene #docs #refactor #bugfix #desktop #team

The first round run on the `cs terminal team` tooling end-to-end: a
4-member team (@@Lead + @@Chan, @@ChanDesktop, @@ChanGateway, one per
build surface) in a single shared worktree, coordinated through
generated bootstrap, task files, append-only journals, and one-line
pokes. @@Alex set scope conversationally; everything else ran on the
bus.

## Roadmap (the asks)

1. **Archaeology scrub** - code comments and docs referencing phases,
   agent handles, and work-item ticket codes are artifacts of how the
   system evolved, meaningless to any other reader. Everything except
   CHANGELOG.md (and the phase reports themselves) must describe the
   current snapshot, not narrate history.
2. **Hygiene per lane** - compiler/checker warnings to zero;
   functions with >5-6 params get config structs; duplicated code
   deduplicated; ad-hoc fixups normalised (named ask: shortcuts
   declared outside the main chord store).
3. **Docs dedup** - README content that duplicates the manual becomes
   pointers into it.
4. **Design docs current** - every design.md rewritten against a
   fresh source read.

Mid-round additions from @@Alex:

5. **File-drop takeover bug** (severity high) - a Finder image drop
   on any non-editor surface navigated the WKWebView into bare image
   view with no way back. Expected: terminal prints the dropped
   path(s) at the cursor like macOS Terminal; editor keeps its embed;
   every other widget is inert.
6. **chanwriter purge** - the dead chan-writer org name goes,
   including the macOS bundle identifier.

## What shipped (32 commits on main, all local; no push)

- **Archaeology scrub**: ~330+ hits cleared across crates/, web/,
  desktop/, docs/, .agents/ - phase numbers, agent handles, ticket
  codes (`systacean-N`, `fullstack-x`, `GI-N`, `B-slice` vocab),
  round/wave markers, work-item codes in USER-VISIBLE `--help` text,
  internal handles in CLI help examples and test fixtures
  (neutralized to @@Alice/@@Bob), and test FILENAMES. Comments that
  carried real constraints were rewritten to state the constraint;
  pure narration was deleted. Three comments and several docs were
  found to be factually FALSE against current code and corrected
  (a help text claiming reports default off when new workspaces
  default on; docs citing env vars, routes, and commands that do not
  exist; a README whose dev-run instructions would bail at boot).
- **Hygiene**: warnings to zero on every surface (Rust kept at zero;
  web went from several to zero with two documented, narrow
  suppressions). Param-struct refactors: `cmd_serve`'s 15-arg tail →
  `ServeArgs`; `control_socket` start/handle_request →
  `ControlSocketCtx`; desktop's 9-param `build_workspace_window` →
  `WindowSpec`; gateway's 7-arg `ApiTokenService::create` →
  `NewToken` + shared `RequestMeta`. Dedup: desktop's triplicated
  spawn preamble → `unbury_or_restore`; gateway throttle defaults
  single-sourced into gateway-common. Shortcuts: the chord store was
  already authoritative except three ad-hoc bindings (Rich Prompt
  toggle, empty-pane Mod+W, terminal find) - all three are registry
  entries now, and a menu label that lied on Linux renders from the
  registry.
- **Docs**: README deduped into the manual (serve flags →
  workspaces.md, tunnel walkthrough → tunnel.md, both pages enriched
  with the content README dropped); design.md (root) regenerated
  against source (crate map incl. chan-shell, full module/route
  inventories, current subcommand surface, dead sibling-repo path
  fixed, contradictory "desktop is parked" paragraph removed); ALL
  15 design.md files rewritten from fresh source reads;
  coordination.md now describes the cs-terminal team process;
  config-reference.md trued against every struct (a registry table
  documenting three phantom fields, five missing IndexConfig fields,
  the unified ~/.chan/workspaces/<key>/ layout); macos-signing.md
  reframed from pre-shipping brief to credentials runbook;
  .agents/ roster reframed as the historical handle index. Stale
  point-in-time docs deleted outright (release-review.md, the
  manual's audit-meta sections); updater-bridge.md shrunk to its
  durable halves.
- **File-drop fix** (joint @@ChanDesktop + @@Chan, design frozen as a
  written contract): wry-source verification proved the native
  drag-drop handler swallows ALL drags on macOS (in-page DnD
  included), so the handler stays disabled and the fix is a
  DOM-level default-deny guard (acts only on Files-bearing drags;
  allowlisted zones: editors, Rich Prompt, terminal panes) plus a
  `read_dropped_paths` IPC reading the macOS drag pasteboard at
  drop time. Terminal drops insert POSIX-escaped paths through the
  normal typed-input path. The IPC is ACL-scoped to locally-served
  windows only (workspace-*/terminal-*) because the drag pasteboard
  is system-wide - remote-served SPAs (tunnel/outbound) cannot read
  it, contract-pinned with negative assertions on both broad
  capability surfaces. Linux path-print is a documented no-op (no
  persistent drag pasteboard); the takeover guard protects all
  platforms.
- **SVG embed fix** (found by @@Alex's smoke, pre-existing since
  before v0.31.1): Image-class reads through `/api/files` let the
  editable-text content sniff run first, and SVG — the one image
  format that is valid UTF-8 text — passed it and shipped as the
  editor's JSON envelope instead of raw bytes; `<img>` rejected it
  (browsers never content-sniff SVG). The route now classifies
  FIRST: `FileClass::Image | Pdf` → raw bytes + correct MIME, pinned
  by a unit test beside the existing binary/text pins. Deliberately
  fixed at the route layer only, so MCP `read_file` keeps serving
  .svg sources as text to agents.
- **chanwriter purge**: bundle identifier `com.chanwriter.desktop` →
  `app.chan.desktop` (bundle-verified: Info.plist + codesign
  designated requirement); systemd unit Documentation URLs; crate
  description; sibling-repo framings in design docs. The only
  surviving occurrences are the marketing-build guards that FORBID
  the dead org in public copy, and exempt history.
- **Cleanups authorized mid-round**: desktop's write-only
  workspace-features mirror plumbing deleted (-186 lines, resolved
  the config-reference drift finding).

## Verification

- Scoped own-gates per lane (clippy/test/fmt + make web-check),
  re-run after last edits; an isolated-worktree full `make pre-push`
  by @@Lead at integration points - run 1 caught a REAL gate-red
  (see retrospective), runs 2 and 3 green end-to-end.
- Every code commit second-pass reviewed by a peer lane
  (@@ChanGateway reviewed 11 commits adversarially: zero code
  defects, two help-text findings, both fixed). Review depth
  included field-by-field call-site mapping for every param-struct
  refactor, ACL negative-pin verification, and adversarial
  newline-filename probes of the shell escaping.
- Browser smokes for the guard (Chrome); vitest 1719 tests green;
  the WKWebView drop arc hand-smoked by @@Alex on a fresh
  stale-dist-verified build: PASSED (terminal path-print, editor
  embed, inert widgets, tunnel-window isolation, tab drags).
- The smoke surfaced one adjacent bug, fixed before close: SVG
  editor-embeds rendered the broken-image box while PNGs worked
  (see the file-drop section's sibling fix below).

## Retrospective

**Highlights:**
- The grep-shim discovery (@@Chan): the sandbox grep silently skips
  large files - a 130KB component returned phantom "no match",
  hiding ~45 hits. Ratified mid-round: every sweep moved to
  `rg --text` with no file-type filters; the re-sweeps caught real
  misses in three other lanes' "clean" surfaces (shell scripts,
  systemd units, a tests/ directory).
- The isolated gate caught what lane gates couldn't: a deprecation
  that bare clippy reads as advisory is a hard error under
  pre-push's `RUSTFLAGS=-D warnings`. Own-gate flags now match the
  real gate by rule.
- wry-source-level design verification (@@ChanDesktop) killed two
  plausible-but-broken designs BEFORE implementation; the contract
  + amendments process (security ACL scoping caught at review)
  produced a fix that survived adversarial review unchanged.
- Worker-initiated spec corrections: a hole in the guard spec
  wording (read-only editors) and an obsolete contract assumption
  (no FB drop zone exists) were caught and fixed by the implementing
  lane, with pins.
- Stalled-subagent handling model case (@@Chan): watchdog-killed
  agent's work was diff-reviewed in full and re-gated before commit,
  not trusted and not redone.

**Lowlights / lessons:**
- Five poke crossings cost a verification round-trip each. Lesson:
  one completion poke after the last part of multi-part work.
- @@Lead's recon was wrong twice: a 13-call-site warning was a
  generic-name collision (handoff.rs has an unrelated
  `handle_request`), and the initial grep patterns under-counted
  the handle vocabulary. Qualified greps + lane re-verification
  caught both.
- Own-gate green ≠ integrated green (the -D warnings incident);
  and a lane's own "blast radius = two files" claim was
  shim-phantom-clean (it was four). Independent re-verification by
  a second party earned its cost repeatedly.
- web-marketing/ was in nobody's lane table - an unowned-surface
  gap found only when a sweep correction pointed there. Lane tables
  should enumerate the whole tree, including "nobody needs to touch
  this" entries.

**Honest feedback, per member:**

- @@Chan: the strongest single-lane output of the round — biggest
  surface, ten commits, and the two best discoveries (the grep-shim
  skip, the SVG root cause). Two growth edges: the scoped-vitest
  miss their own lowlight names (a pinned-source edit needs full
  vitest, not a curated list), and the one help-text claim their
  scrub introduced (the Rich Prompt Linux row) shows new text needs
  the same verification as corrected text.
- @@ChanDesktop: best design-verification instinct on the team —
  going to wry source instead of hand-testing killed two broken
  designs before a line was written, and the bundle-verify went to
  the codesign designated requirement. The gate-red was theirs: a
  bare-clippy own-gate passed what -D warnings rejects. Adopted the
  flags rule immediately and named the root cause without
  defensiveness.
- @@ChanGateway: the review franchise. Eleven commits adversarially
  reviewed with receipts (call-site mapping, negative-pin checks,
  newline probes), zero false acceptances, and the two real findings
  were both findings. Their own sweep had two gaps (a tests/ dir, the
  packaged .service files) — both instances of the round's central
  lesson that sweeps fail at their FILTERS, not their patterns.
- @@Lead: recon was wrong twice (the 13-call-site name collision;
  the under-counted handle vocabulary) and the original task specs
  carried three obsolete assumptions lanes had to correct (the FB
  drop zone, desktop-as-separate-workspace, the guard wording hole).
  The process held because verification was redundant, not because
  the briefs were right. Also: five poke crossings burned verify
  round-trips — the lead should have set the one-completion-poke
  convention at kickoff, not discovered it in the retro.
- @@Alex: the conversational mid-round additions (drop bug,
  chanwriter) were well-timed and scoped; the smoke checklist
  execution caught a real pre-existing bug the team's browser
  testing could not (the SVG embed). One ask: the original scope's
  "duplicated code" and "non-idiomatic" items are judgment-heavy —
  next time a worked example or two of what offends would sharpen
  the lanes' first pass.

**Carryover:**
- Threaded-state param clusters deferred for a designed ctx pass:
  chan-server graph.rs merge_* family (11/9/9/8), handle_team (11),
  indexer spawn family, terminal_sessions::restart (8),
  fs_graph/survey/drafts/contacts entries (full inventory in the
  round bus, task-Chan-Lead-1.md).
- Linux terminal path-print no-op (no drag-pasteboard equivalent).
- Dispatch-to-matcher-loop shortcut refactor (behavior-risk,
  deliberately not taken this round).
- gateway/package.json version pin (0.0.0) bumps in lockstep at the
  next release cut; chan-llm README dependency example likewise.
- Optional: default.json negative pin for read_dropped_paths (belt
  symmetry; launcher is locally-served, outside the threat model).
- Buried-window/GTK/Xcode carryovers from phase 22 remain untouched
  (out of this round's scope).

## Notes

- The round's coordination bus (new-team-1/: plan, tasks, journals)
  is committed alongside this report at close.
- Versions: the round rides after v0.31.1; no release was cut within
  the round. The bundle-id rename means the next release's desktop
  artifact changes app identity (one-time keychain/TCC prompts;
  documented in the bus).
