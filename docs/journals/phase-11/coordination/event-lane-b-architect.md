# Channel: @@LaneB -> @@Architect

Append-only. @@LaneB writes progress reports here; @@Architect reads.
Never edit prior entries. Curated highlights/lowlights/contention; link
your journal for detail.

## 2026-05-26 @@LaneB -> @@Architect
Kickoff: worktree up, scope read, first work dispatched.

Highlights:
- Worktree `../chan-lane-b` @ `phase-11-lane-b` created off main (198beb9).
- Journal open at `docs/journals/phase-11/lane-b/journal.md`.
- Read plan, CLAUDE.md, round-1, round-2, coordination README, and the
  three channels addressed to me. No cross-lane messages from @@LaneA yet.
- Confirmed Linux desktop launch (item 9) is DEFERRED; not starting it.
- Active scope locked: webdev track (bugs 1,4,5,10 -> image-drag feature
  -> bug 6) and rustacean track (bug 2 -> bug 8 -> binary-size audit ->
  macOS CLI-to-desktop handoff with the one @@Alex design-note gate).

Contention / note:
- This environment exposes no Task/Agent subagent-spawn tool (not in the
  deferred-tool list, ToolSearch finds none). I cannot fork separate
  `webdev`/`rustacean` subagent processes. I am proceeding by loading the
  `webdev`/`rustacean`/`architect` skills in-session and doing the
  implementation directly, keeping the two tracks logically separate and
  merging in small slices. Flagging in case you expected forked agents;
  the work and the gates are unchanged. Tell me if you want a different
  arrangement.

Lowlights: none yet.

Next: bug 1 (list input regression) on the webdev track and bug 2 (drag
removal + native download indicator) on the rustacean track, in that
priority order, landing in small slices through the full gate.

## 2026-05-26 @@LaneB -> @@Architect
Adopted report-and-you-merge; rebased onto main; bug 4 ready; bug 1 a no-op.

Acks:
- Read your direction. Adopting the report-and-I-merge flow: I have NOT
  self-merged anything. Work stays on phase-11-lane-b, no push to remote.
- Rebased phase-11-lane-b onto main @ 3d42b09 (@@LaneA Slice B bootstrap
  spine). Clean rebase, no conflicts (my files are disjoint). Build
  compiles. Will hold for your ping before touching the shared structural
  files when @@LaneA's Slice A lands.

READY TO MERGE: phase-11-lane-b @ 330bda1
- One commit: "fix(editor): accept trailing-slash path as a directory in
  New File/Dir dialog" (bug 4).
- Files (all lane-B-owned, NO shared structural files):
  web/src/state/pathValidate.ts, .../pathValidate.test.ts,
  web/src/components/PathPromptModal.svelte.
- Rebase surface for you: none beyond the clean rebase already done; no
  router/state/store/tabs/App touch.
- Full gate green: fmt, clippy -D warnings, cargo test, build
  --no-default-features, npm build, svelte-check, full vitest (1482 pass).
- Verified end-to-end on a fresh binary across all 3 create paths:
  New Directory `myfolder/` -> dir; New File `foo/` -> still rejected
  (guard); New File or Directory `subdir/` -> dir on disk. Caught and
  fixed a self-introduced either-flow regression mid-slice (the store
  dispatches dir-vs-file on the trailing slash; I now preserve it in the
  submit value and normalize only for internal reasoning). Detail in
  journal.

Highlights:
- Bug 1 (list input regression): NOT reproducible at HEAD; already fixed.
  Empirical re-walk on a fresh binary + DOM inspection: `-`/`*` render
  bullets, `1.`/`1)` render numbers, Enter-continuation keeps the type;
  48 list unit tests pass. Fix landed earlier in e2a58bb + 72bff66,
  predating the round-1 tracker commit. No code change. Flagging so it
  isn't expected as a separate slice. (Minor unrelated quirk noted in
  journal: lezer groups loosely-separated ordered lists; not the reported
  bug, not in scope.)

Lowlights: none.

Next: bug 5 (image paste at cursor), then bug 10 (Cmd+N cursor, will flag
App.svelte on the cross-lane channel before editing). Rustacean track
(bug 2 drag removal) starts in parallel after I open it. Holding desktop
init-path re-validation for your later-slice ping.

D2 reply to @@LaneA posted on event-lane-b-lane-a.md (concur with the
split: scoped fs frame for the FB tree + keep the global watch frame for
the editor's single-file external-edit toast).

## 2026-05-26 @@LaneB -> @@Architect
Bug 5 ready to merge; reproduced + fixed empirically.

READY TO MERGE: phase-11-lane-b @ 9773f44 (stacks on bug 4 @ 330bda1)
- One commit: "fix(editor): paste image at end of doc when editor is
  unfocused" (bug 5).
- Files (lane-B-owned, NO shared structural files):
  web/src/editor/bubbles/image_drop.ts (+ new image_drop.test.ts).
- Rebase surface for you: none beyond the bug-4 rebase already done.
- Full gate green (fmt, clippy -D warnings, cargo test, build
  --no-default-features, npm build, svelte-check, full vitest 1485 pass).

What it fixes: a screenshot pasted into a freshly-opened note landed at
offset 0 (above the title) because the paste handler trusted
selection.main.head, which is 0 before the user places a caret.
Reproduced empirically (synthetic image paste on a fresh-open doc landed
at row 1). Fix gates the caret on view.hasFocus: focused -> caret
(unchanged); unfocused -> append at end of doc.

Note: both bug 4 (330bda1) and bug 5 (9773f44) are stacked on the branch.
Merge order is bug 4 then bug 5; neither touches shared structural files,
so you can fast-forward the branch or cherry-pick in order.

Next: bug 10 (Cmd+N cursor). It touches App.svelte (the two-sided merge
point). I will post the exact hunk to event-lane-b-lane-a.md BEFORE I
commit so @@LaneA can reconcile, then report ready-to-merge here. After
that, the rustacean track (bug 2: drag removal + native download
indicator).

## 2026-05-26 @@LaneB -> @@Architect
Bug 10 ready; webdev quick-wins (1,4,5,10) DONE; NO App.svelte touch.

READY TO MERGE: phase-11-lane-b @ af783d8 (stacked: bug4 330bda1 ->
bug5 9773f44 -> bug10 af783d8, all on main@3d42b09)
- bug 10 commit: "fix(editor): focus the editor after a fresh open so
  Cmd+N is typeable".
- Files: web/src/editor/Wysiwyg.svelte + Source.svelte. NOTE: I did NOT
  touch App.svelte. The bug was the editor's focus-vs-content race, not
  the Cmd+N chord handler, so the fix stays in my owned editor surface
  and AVOIDS the App.svelte two-sided merge point entirely. Better for
  @@LaneA coordination than the plan's tentative App.svelte scoping; I've
  told @@LaneA no reconciliation is needed there.
- Rebase surface for you: none beyond the bug-4 rebase. No shared
  structural files touched by any of my three slices.
- Full gate green (fmt, clippy -D warnings, cargo test, build
  --no-default-features, npm build, svelte-check, full vitest 1485 pass).

Root cause + fix: mount-time view.focus() runs on an empty doc; content
loads async so focus falls back to <body> by the time it lands.
maybeRestoreCaret now re-asserts focus (rAF-deferred, gated on autoFocus)
at the point the caret is placed. A synchronous re-focus did NOT stick
(verified empirically); the rAF defer was required. Verified: Cmd+N then
immediate typing replaces the draft title (was: keystrokes lost).

Status of the webdev track:
- Bug 1 (lists): no-op, already fixed at HEAD (reported earlier).
- Bug 4 (trailing-slash dir): DONE, 330bda1.
- Bug 5 (paste at cursor): DONE, 9773f44.
- Bug 10 (Cmd+N focus): DONE, af783d8.
All four quick-wins cleared.

Next on the webdev track: the image-drag-across-rows FEATURE (same file
as bug 5, image_drop.ts), then bug 6 (terminal FitAddon timing). Then I
open the rustacean track (bug 2: remove FB native drag in/out + native
download indicator). Still holding desktop init-path re-validation for
your later-slice ping, and the CLI-handoff design note for after the
desktop bugs.

## 2026-05-26 @@LaneB -> @@Architect
Resumed fresh, rebased, image-drag feature done. READY TO MERGE.

- Rebase: phase-11-lane-b rebased onto main @ d70f585 (your merge of bugs
  4/5/10). Clean; my 3 commits folded into main via the merge; branch is
  now the main tip + one new commit.
- READY TO MERGE: phase-11-lane-b@b70f4ac -- image-drag-across-rows
  feature.
  - Files (all lane-B-owned editor surface, NO shared structural file):
    web/src/editor/widgets/image.ts,
    web/src/editor/bubbles/image_drop.ts,
    web/src/editor/bubbles/image_drop.test.ts,
    web/src/editor/Wysiwyg.svelte.
  - Rebase surface: NONE of store.svelte.ts / tabs.svelte.ts / App.svelte
    / api/client.ts / lib.rs::router() / state.rs touched. Disjoint from
    @@LaneA's queued Slice A (store/tabs) and Slice C (state.rs). Should
    merge with no conflict on top of either.
  - Full gate green: fmt, clippy -D warnings, cargo test, build
    --no-default-features, npm build, svelte-check (0 err), full vitest
    1490 pass / 11 skip / 0 fail (+5 new moveImageSource tests).
- HIGHLIGHT: verified end-to-end on a fresh binary -- drop moves the
  `![](src#w=N)` markdown to the target row ON DISK with width/alignment
  preserved; plain click still selects without edit-mode.
- LOWLIGHT (resolved): two non-obvious CM6 gotchas cost time -- (1) CM
  resets `draggable` on widget-root DOM, so the drag source had to move to
  the inner <img>; (2) a `readonly editable` widget field collided with
  WidgetType's getter-only `editable` and CRASHED the decoration plugin
  (image rendered as raw source) -- renamed to `writable`. Both caught and
  fixed via fresh-binary re-walks + console inspection. Detail in journal.
- Next: bug 6 (terminal FitAddon timing, TerminalTab.svelte), then I open
  the rustacean track with the RESHAPED bug 2 (remove FB native drag in/
  out + deliver the download-with-progress capability for @@LaneA to wire;
  interface to be posted on event-lane-b-lane-a.md). Still holding the
  CLI-handoff design note for after the desktop bugs, and desktop init-
  path re-validation for your later-slice ping.

## 2026-05-26 @@LaneB -> @@Architect
Bug 6 done. READY TO MERGE. Webdev track complete; opening rustacean next.

- READY TO MERGE: phase-11-lane-b@0a8e0ae -- bug 6 (idle terminal garbled
  until clicked/resized).
  - File: web/src/components/TerminalTab.svelte (lane-B-owned; NO shared
    structural file). Stacks on top of b70f4ac (image-drag, also queued).
  - Rebase surface: none of the shared files. Both my queued commits
    (b70f4ac image-drag, 0a8e0ae bug 6) are disjoint from @@LaneA's Slice
    A/C surfaces -- merge either order, no conflict expected.
  - Full gate green: fmt, clippy -D warnings, build --no-default-features,
    npm build, svelte-check (0 err), full vitest 1490/11/0 (cargo test
    unaffected, no Rust touched).
- Root cause (HIGHLIGHT): the tab uses visibility:hidden (not
  display:none) while inactive, so xterm/WebGL can paint at a stale size
  while hidden; nothing re-fit + repainted when a tab became ACTIVE
  without also becoming FOCUSED (focus effect + ResizeObserver both miss
  the pure visibility flip). New $effect keyed on `active` runs the
  existing host-resume recovery (fit + texture-atlas clear + delayed
  re-fit). Verified clean on a fresh binary via tab-switch re-walk.
- STATUS: webdev track COMPLETE (bugs 1/4/5/10 + image-drag + bug 6).
  Now opening the RUSTACEAN track with reshaped bug 2: (a) remove FB
  native drag in/out (drag_out.rs + FileTree.svelte JS wiring), (b)
  deliver the download-with-progress capability (Tauri command + progress
  events + api/client.ts wrapper/store) for @@LaneA to wire into the
  inspector; I'll post the interface on event-lane-b-lane-a.md. Then bug 8
  + binary-size audit, then the CLI-handoff design note (@@Alex gate).

## 2026-05-26 @@LaneB -> @@Architect
Bug 2a done (native drag in/out removed). READY TO MERGE + one scope flag.

- READY TO MERGE: phase-11-lane-b@3fec962 -- remove FB native drag in/out.
  - Files (7, all lane-B-owned; NO shared structural file): deleted
    desktop/src-tauri/src/drag_out.rs; edited desktop/src-tauri/src/
    {main.rs,serve.rs}, desktop/src-tauri/permissions/app.toml,
    web/src/components/FileTree.svelte, + 2 inverted tests
    (fileTreeDragOut.test.ts, fileBrowserUploadDrop.test.ts).
  - Stacks on 0a8e0ae (bug 6) + b70f4ac (image-drag). All three disjoint
    from @@LaneA's Slice A/C surfaces. Note: this touches FileTree.svelte
    -- @@LaneA's Slice A reshapes store.svelte.ts (FbTreeInstance) but NOT
    FileTree.svelte per their note, so no conflict expected; flag me if
    their later FB slice touches FileTree.
  - Full gate green: fmt, clippy --all-targets -D warnings (incl
    chan-desktop), cargo test (all workspace), build --no-default-features,
    npm build, svelte-check (0 err), full vitest 1490/11/0.
- SCOPE FLAG (proceeded with the defensible reading; correct me if wrong):
  "remove drag in and out entirely" = the OS<->app interchange. I removed
  drag-OUT (to Finder, incl the crashing native Tauri command) and drag-IN
  (external OS file drop -> upload), and KEPT the app-internal drag:
  tree-move (relocate within the tree) + file-into-editor-pane open. Those
  never cross the OS boundary, never hit the crash, and have no Upload/
  Download-button equivalent. @@Alex's verbatim text + the "operate via
  Upload/Download buttons" framing both point at OS interchange. If @@Alex
  meant ALL File Browser drag (including internal reorg), it's a one-line
  follow-up to strip the row draggable + the tree-move branch. Verified
  empirically: internal tree-move still moves files on disk; external
  Files dragover rejected; drag-out payloads gone.
- Next: bug 2 part (b) -- desktop download-with-progress capability for
  @@LaneA, interface to land on event-lane-b-lane-a.md.

## 2026-05-26 @@LaneB -> @@Architect
Bug 2b done (download-with-progress capability). READY TO MERGE. Bug 2 complete.

- READY TO MERGE: phase-11-lane-b@66dec92 -- desktop-native download flow
  for the inspector (the reshaped bug 2's part b).
  - Files (8, all lane-B-owned; NO shared client.ts/store.svelte.ts):
    NEW desktop/src-tauri/src/download.rs, edited main.rs + serve.rs +
    permissions/app.toml; web/src/api/desktop.ts (runDesktopDownload),
    NEW web/src/state/downloadTransfer.svelte.ts (the rune store), + 2
    NEW tests.
  - Deliberately routed through api/desktop.ts + a new lane-B store
    module so it touches ZERO shared structural files -- avoids any
    collision with @@LaneA's Slice A store.svelte.ts reshape.
  - Stacks on 3fec962/0a8e0ae/b70f4ac. All four disjoint from @@LaneA's
    surfaces.
  - Full gate green: fmt, clippy --all-targets -D warnings (incl
    chan-desktop), cargo test (workspace, incl 3 new download.rs unit
    tests + the ACL test), build --no-default-features, npm build,
    svelte-check (0 err), full vitest 1498/11/0 (+8 new).
- DESIGN: SPA fetches via XHR (download progress -> in-app indicator),
  hands bytes to a save_file_to_downloads Tauri command that writes to the
  OS Downloads folder with browser-style "file (1).ext" dedupe + filename
  sanitization. Byte transfer stays in JS (reuses the upload-progress
  pattern, avoids a second loopback fetch from Rust); notes-scale content
  so in-memory buffering is fine. Interface (runDesktopDownload + the
  downloadTransfer store) posted on event-lane-b-lane-a.md for @@LaneA.
- BUG 2 COMPLETE (a: native drag in/out removed; b: download capability).
- Next: bug 8 (desktop auto-reload/hang during editing) + the binary-size
  audit (.github/workflows edit authorized; I'll state it inline), then
  the macOS CLI-to-desktop handoff DESIGN NOTE (my one @@Alex gate ->
  post to event-lane-b-alex.md, wait for ratification).

## 2026-05-26 @@LaneB -> @@Architect
ready to merge: phase-11-lane-b@48b6446 (bug 8). Rebased onto ebcabad
first (my 4 commits dropped via your merge; branch was clean at ebcabad).

WHAT: desktop auto-reload + hang-on-loading during editing.

FINDINGS (full reload/watch-path trace, Rust + SPA):
- No app-logic auto-reload exists for editor (drive) windows. The only
  reload is reload_window (Cmd+R / pane menu / reloadWindow IPC), all
  user-driven; drive windows listen to ZERO Tauri events; the SPA's only
  location.reload() are Cmd+R and InfographicsTab's post-import reload.
- The "auto-reload during editing" is the macOS WKWebView web-content
  process being recycled under memory/fd pressure (same root as round-1
  line 12: "Too Many Open Files -> failing autosave -> hanging server").
  WKWebView reloads the page when its content process dies.
- The "hang on loading" is bootstrap() single-shot api.drive() with no
  retry: a reload racing the embedded server's recovery sticks on
  "loading..." forever. cmd+ (zoom, NOT reload in chan-desktop) just
  forces a re-composite.

FIX (two contributing defects):
1. watcher.rs (lane-B): watched ~/.chan/ non-recursively but emitted
   registry-changed on ANY dir event -- incl preferences.toml /
   server.toml atomic-write siblings re-saved during editing (pane drag
   -> preferences save). Stormed the launcher list_drives refresh +
   added fd/event churn feeding the WKWebView pressure. Now filters
   debounced events to file_name=="config.toml" (the design.md
   contract). EMPIRICALLY VALIDATED on real macOS FSEvents via a
   throwaway notify probe: config.toml rename forwards; preferences.toml
   sibling + watched-dir-self events suppressed. 4 unit tests.
2. store.svelte.ts (SHARED, additive, announced to @@LaneA on
   event-lane-b-lane-a.md): bounded retry around the first api.drive()
   (5 attempts, 250ms linear backoff) on TRANSIENT failures only
   (conn-refused/timeout/5xx); 401/4xx unchanged so missing-token overlay
   path holds. New lane-B test file bootstrapRetry.test.ts (6 tests).

DID NOT touch main.rs reload_window / embedded.rs -- both correct; the
bug is the watcher's path-blind emit + bootstrap's missing retry. The
deeper fd-exhaustion root (line 12) has existing infra (chan-drive
fd_budget.rs, terminal EMFILE guard) and reads as a separate/other-lane
concern; my fixes cut the trigger frequency + make the symptom
self-healing.

FILES: desktop/src-tauri/src/watcher.rs (lane-B),
web/src/state/store.svelte.ts (SHARED, additive),
web/src/state/bootstrapRetry.test.ts (NEW lane-B).

REBASE SURFACE: store.svelte.ts is @@LaneA's hot file -- my only edit
inside bootstrap() is the one `api.drive()` -> `driveWithRetry()` line;
the helpers + __test export are additive. Announced to @@LaneA; trivial
reconcile.

GATE (green): cargo fmt --check (root+desktop); clippy --all-targets -D
warnings (root+desktop); cargo test (332 chan-server + workspace, 0 fail)
+ desktop cargo test --all-targets (7 + 4 new watcher); build
--no-default-features; svelte-check 0 err; full vitest 1514/11/0 (+6);
npm run build OK. chan-desktop rebuilt + smoke-launched on macOS (boots
clean), torn down by scoped pid kill.

NEXT (continuing rustacean track, no hand-back yet): binary-size audit
(.github/workflows edit authorized -- I'll state it inline), then the
macOS CLI-to-desktop handoff DESIGN NOTE -> event-lane-b-alex.md (the
@@Alex gate; I hand back after posting it).

## 2026-05-26 @@LaneB -> @@Architect
ready to merge: phase-11-lane-b@dfdc012 (bug 3, binary-size audit).
Stacked on the bug-8 commit (48b6446).

FINDINGS: both shipped CI binaries are ALREADY lean / SPA-only with no
model baked.
- release.yml (chan CLI): `cargo build --release -p chan`, no
  embed-model; the BGE-bundle cache + fetch-models steps are hardcoded
  `if: false` (systacean-6). .deb/.rpm wrap the same lean binary.
- release-desktop.yml (chan-desktop): `make build` (lean helper) + cargo
  tauri build; model steps `if: false`; chan-desktop pulls chan-server
  with `features=["embeddings"]` only + chan-drive default-features=false
  -> runtime resolver, nothing baked.
- MEASURED: lean `cargo build --release -p chan` = 28 MB (macOS aarch64,
  unstripped); embed-model documented ~89 MB (~61 MB / ~68% smaller).
  `strings` on the 28 MB binary: only curated-model PICKER metadata +
  runtime-resolver paths (`<user-config>/chan/models/`), no
  models.tar.zst bytes.

So NO .github/workflows edit was needed (the workflows are already
correct). The authorization to touch workflows for this task went unused;
flagging that here for the record per the classifier/shared-infra note.

THE ONE LEVER (applied, architect-approved, no @@Alex gate): the LOCAL
Makefile diverged -- `make install` -> `make build-release`
(--features embed-model, 89 MB) and `make rpm` passed --features
embed-model. So a contributor's local install was 89 MB vs CI's 28 MB.
Pointed both at the lean path (`make install` -> `make build`; `make rpm`
drops --features embed-model + the models prereq). `make build-release`
stays as the explicit opt-in offline-from-first-launch variant; header
doc refreshed. Makefile-only (no Cargo.toml, no code, no workflow). `make
-n install` / `make -n rpm` confirmed lean.

FILES: Makefile only.
GATE: a Makefile edit doesn't touch the Rust/web gate; the prior bug-8
slice left the tree fully green and this commit compiles nothing new.
`make build` already produced the 28 MB binary cleanly during the audit.

NEXT (final rustacean item): macOS CLI-to-desktop handoff DESIGN NOTE ->
event-lane-b-alex.md (my one @@Alex gate). I post it and WAIT for
ratification before any implementation; I hand back after posting it.

## 2026-05-26 @@LaneB -> @@Architect
CLI-to-desktop handoff DESIGN NOTE posted -> handing back.

- Posted the full design note to event-lane-b-alex.md (my one @@Alex
  gate). Covers options A/B/C, same-user UDS discovery (mcp_bridge.rs
  per-pid socket is the reuse pattern; desktop publishes a WELL-KNOWN
  per-user UDS so the CLI finds it without the pid; token over the UDS,
  never argv/env), ownership/token/lifecycle/version/capability mismatch,
  the mandatory no-desktop fallback, and standalone-forcing flags.
- RECOMMENDATION: Option B (ask desktop to open a window) as default when
  a same-user desktop is discovered + GUI session + no standalone/tunnel
  flag; fall back to Option C (own the server) otherwise. One pick from
  @@Alex unblocks implementation. NOT implementing until ratified.
- Could you flag @@Alex for ratification per the protocol?

RUSTACEAN TRACK STATUS this turn:
- bug 8 (desktop auto-reload/hang): DONE, ready @ 48b6446.
- binary-size audit (bug 3): DONE, ready @ dfdc012 (Makefile-only; CI
  already lean, no workflow edit needed).
- CLI-to-desktop handoff: design note posted, GATED on @@Alex.

Branch phase-11-lane-b tip = dfdc012 (both ready slices stacked on
ebcabad). HANDING BACK -- the only remaining lane-B item (handoff impl)
waits on @@Alex; Linux desktop launch stays DEFERRED. Two slices ready to
merge whenever you pick them up; scoped pkills only, no self-merge, no
push.

## 2026-05-26 @@LaneB -> @@Architect
Resumed, rebased, item 1 (source-mode list rule) done.

ready to merge: phase-11-lane-b@eaef1df
- files: web/src/editor/sourceModeListKeymap.test.ts (TEST-ONLY, +68)
- rebase surface: NONE. Branch rebased onto main @ 250d2f6 first (my bug 8
  + binary-size dropped into the merge; branch == 250d2f6 tip, clean). The
  one new commit touches a single lane-B-owned test file, no prod code, no
  shared file. Trivial / conflict-free for you.

Highlights:
- Item 1 (new-file-and-draft-spec: source-code mode must not run markdown
  input rules) is ALREADY CORRECT at HEAD - same shape as bug 1. Source.svelte
  runs no markdown input rules / no list keybinds and seeds the markdown
  language with addKeymap:false (5c9acca "fullstack-a-41", confirmed ancestor
  of 250d2f6). list.ts + block decorations are WYSIWYG-only; every markdown()
  call in the tree is addKeymap:false; there is no space-triggered list input
  rule anywhere.
- Empirically re-verified on a fresh binary for BOTH .md and .txt in source
  mode: typed `* hello` / `- item` stay literal, Enter inserts a bare newline
  (no auto-continue). DOM line reads confirmed.
- Delivered the regression test (the actionable part): the existing test only
  pinned the Enter path; added a typing-path helper + 4 cases (`* `/`- `/`1. `
  typed at line start + into an empty doc) so a future input-rule leak into
  source mode is caught. 9/9 in that file pass.

Lowlights: none. No prod code needed - clean confirm + test lock.

Contention: none. No shared-file touch, no cross-lane announcement needed.

Full gate green: fmt, clippy -D warnings, cargo test, build
--no-default-features, svelte-check 0/0, full vitest 1541/11/0 (+4), npm
build. Test server + tab torn down (scoped pkill on /tmp/chan-test-lane-b-
srcmode), drive removed.

State: CLI-to-desktop handoff IMPLEMENTATION still GATED on @@Alex ratifying
my design note (event-lane-b-alex.md). Linux desktop launch still DEFERRED.
Idle after this unless you send more or relay @@Alex ratification.

## 2026-05-26 @@LaneB -> @@Architect: READY TO MERGE -- CLI-to-desktop handoff (slice 1, the whole feature)

ready to merge: phase-11-lane-b@0f3d4ea (rebased onto main f088e83).

@@Alex ratification CONSUMED: Option B default (desktop opens a native
window) when a same-user desktop is discovered + GUI session + no
standalone/tunnel flag; fall back to Option C (own the server) otherwise.
Shipped the whole feature in ONE gated slice (UDS discovery + handshake +
desktop listener + cmd_serve client + all flags/fallbacks) because the
pieces are tightly coupled and the safety flags are load-bearing.

FILES (4, all lane-B-owned; NO @@LaneA shared structural file --
lib.rs::router() / state.rs / store.svelte.ts untouched):
- crates/chan-server/src/handoff.rs  NEW (625 lines) -- the shared
  protocol/listener/client module, PUBLIC so both `chan` and `chan-desktop`
  (which both already dep chan-server) consume it. No new crate, no new dep
  (reused rustix for getuid).
- crates/chan-server/src/lib.rs       +4 (one `pub mod handoff;` + doc)
- crates/chan/src/main.rs             +92 (cmd_serve handoff attempt before
  open_drive; new --standalone flag; maybe_handoff_to_desktop helper)
- desktop/src-tauri/src/main.rs       +114 (well-known-socket listener in
  setup; open_drive_from_handoff window-spawn handler)

REBASE SURFACE for the merge: handoff.rs is brand new (no collision).
lib.rs adds one mod line in the `mod ...;` block near event_watcher/host
(not the router/pub-use block @@LaneA touches). chan/main.rs touches the
Serve clap struct + cmd_serve + dispatch (CLI-only, no lane-A overlap).
desktop/main.rs adds to the setup block + a new fn (no lane-A overlap;
lane A doesn't touch the desktop crate). Should merge clean.

Highlights:
- Single-writer invariant HELD empirically: with the desktop listener up,
  `chan serve <drive>` hands off, prints "opened ... in chan-desktop", EXITS
  0, and binds NOTHING on the port (health 000). The CLI never opens the
  drive -- the handoff runs BEFORE open_drive.
- Load-bearing standalone default UNCHANGED: no-desktop, --standalone,
  CHAN_NO_DESKTOP_HANDOFF=1, SSH-headless, version-skew, stale socket, and
  garbage-handshake ALL fall through to own-the-server (health 200), each
  empirically verified.
- Version skew -> no silent IPC: "chan-desktop is version X, CLI is Y;
  cannot hand off" + standalone. Socket is 0600 owner-only (verified
  srw-------).

Lowlights:
- Could not exercise the PACKAGED chan-desktop app here (the listener only
  binds inside the real Tauri build). I drove the desktop's EXACT production
  listener code via throwaway chan-server example probes (removed after the
  walk), so the client<->protocol<->listener path is end-to-end verified;
  what remains unproven is that serve::start actually spawns the OS window
  in the packaged app. A `cargo tauri build` + launch + `chan serve` smoke
  would close that -- I can drive it next if you want it before merge.

Contention: none. No shared-file touch; no cross-lane announcement needed.

Full gate green: cargo fmt --check (root + desktop), cargo clippy
--all-targets -D warnings (root + desktop), cargo test (chan-server 340 incl
+8 handoff; all workspace 0 fail), desktop cargo test --all-targets (75 + 7),
cargo build --no-default-features, svelte-check 0/0, npm run build OK (no web
change). Test probes + servers torn down (scoped to
/tmp/chan-test-lane-b-handoff), sockets/temp cleaned, stale registry entry
`chan remove`'d.

State: handoff feature DONE pending your merge + (optional) the packaged-app
smoke. Linux desktop launch (item 9) still DEFERRED. Idle after this unless
you want the packaged-app smoke or send more.

## 2026-05-26 @@LaneB -> @@Architect: READY TO MERGE (Task 1, watcher ignore-filter)

Re-activated on the watcher-scalability spec (now owned by me). Rebased
phase-11-lane-b onto main @ 28d44c7 (my handoff folded in via the merge,
branch is the main tip, clean, no diff). Then Task 1.

ready to merge: phase-11-lane-b@c9a9aae
files: crates/chan-drive/src/watch.rs, crates/chan-drive/src/drive.rs
rebase surface: trivial. Both files are chan-drive watcher path. NO overlap
with @@LaneA's GI-3 (link resolution / graph indexer) -- I touched watch.rs
+ the watch()/watch_team() methods, not graph.rs / the link index. Declared
on event-lane-b-lane-a.md for the record; no sequencing needed.

WHAT: ignore-filter the single recursive watcher feed with the SAME unified
WalkFilter the bootstrap/index walk uses, so a node_modules/target/venv/.git
storm is dropped in the watcher worker thread BEFORE it reaches either the
chan-server broadcast bus (scopes.emit_fs) or the indexer (index_tx). That
is the earliest possible drop -- ahead of both broadcast and indexing, which
is exactly what the spec asked.

HOW: threaded Arc<WalkFilter> (already on the Drive as walk_filter) into
WatchHandle::start; is_filtered() now drops any event whose relative path
runs through an excluded-dir basename at any depth (matching
walk_drive_filtered's filter_entry). Before this it only dropped
.chan/.git/.hg. Watcher-specific deviations preserved: .git/HEAD,
.git/index, .hg/dirstate still FORWARD (indexer checkout-storm detection);
.chan/ always drops regardless of the configurable set.

TESTS: 4 in watch.rs -- full default-set VCS-noise coverage; any-depth drop
(node_modules/target/venv/__pycache__ nested + .chan); real-content
pass-through (README.md, src/main.rs, and the targeting.md / node_modules_
notes.md non-prefix-match guard); a dispatch()-level test proving an
excluded-subtree event never reaches the callback.

Full gate green: cargo fmt --check, cargo clippy --all-targets -D warnings
(workspace incl desktop), cargo test (chan-drive 533 incl +4 watch;
chan-server 340; all workspace 0 fail), cargo build --no-default-features.
No web touched -> no npm/svelte-check needed. No servers spun up for this
slice (pure unit-test slice); nothing to tear down.

NEXT: Task 2 (git-storm resilience empirical check) + Task 3 (indexing
benchmark) + Task 4 (packaged-desktop handoff smoke). Continuing.

## 2026-05-26 @@LaneB -> @@Architect: Task 2 DONE (git-storm resilience confirmed)

No code change -- this is a verification task; full details + numbers in my
journal. Bottom line: a git checkout storm on a large repo (clone of THIS
repo, ~1336 files churned per switch, 8-12 checkouts) while running 2
terminals does NOT starve the editor read path.

Two runs, scoped to /tmp/chan-test-lane-b-gitstorm (torn down):
- Baseline editor-read latency ~1ms (median).
- During the storm: median 0.8-2.0ms, p99 1.3-5.6ms, worst single read
  35.4ms (a recovery-window tail while the indexer caught up). No
  seconds-range stalls, no timeouts, no 5xx.
- The only non-200s were CORRECT 404s (README.md genuinely absent on the
  storm-root branch), all sub-2ms -- not starvation.
- Run 2 was the hard case: the initial bm25 CONTENT reindex (751 .md files)
  was STILL running AND the storm hit -- read latency stayed sub-3ms during
  the storm window. fd_budget paced the reindex; interactive reads kept
  their headroom.
- FD pressure was a non-issue: 56 open fds against a 1M soft limit.
- Task-1 interaction CONFIRMED: zero ProviderError/reindex-storm/EMFILE log
  lines from the .git storm -- the .git events were dropped at the watcher
  boundary (Task 1), so the storm never reached the indexer at all.

Meets expectations: yes, comfortably. NEXT: Task 3 (indexing benchmark).

## 2026-05-26 @@LaneB -> @@Architect
READY TO MERGE: ignore-set consistency fix (TOP PRIORITY) - phase-11-lane-b@e7b7824

Rebased onto main 6103f4d first (c9a9aae replayed as b43ddeb, kept). Then
the consistency fix on top.

ROOT CAUSE of the 60K-131K node graph: the default ignore set was ALREADY
sane (registry::DEFAULT_INDEX_EXCLUDED_DIRS has node_modules/target/venv/
.venv/.git/.hg/.svn/dist/build/...). The gap was FOUR walks that feed the
index/graph and bypassed the WalkFilter. The MAIN reindex + rebuild_graph
were already filtered; the leaks were elsewhere:
1. fs_graph.rs (the filesystem-shape graph, /api/fs-graph) - a raw read_dir
   recursion that only skipped .git/.chan at the TOP level. PRIMARY culprit.
2. graph.rs semantic-graph presence set (drive_disk_files/dirs) + the
   unified-tree layer (merge_unified_tree_layer) - used UNFILTERED
   list_tree_unified.
3. chan-report engine - its own filesystem walk for language analysis,
   surfacing target/*.rs as Rust + node_modules/*.js as JS in the graph's
   language layer.
4. (minor) the two trash remove/restore subtree walks in drive.rs.

THE FIX (all consult the per-drive WalkFilter now):
- fs_graph.rs: FsGraphWalker carries the WalkFilter; skips blocklist
  basenames + .git/.chan at ANY depth.
- graph.rs: 3 call sites swapped to NEW filtered listing helpers.
- chan-drive: new list_tree_filtered_unified / list_tree_prefix_filtered_
  unified (Drive) + list_tree_prefix_filtered + filter-aware subtree branch
  (fs_ops). RAW list_tree* stay UNFILTERED so open-inside-a-noisy-dir works
  (requirement 3). report.rs threads WalkFilter dir names as exclude_globs.
  Trash walks use walk_drive_filtered.

FILES (6): crates/chan-drive/src/{drive.rs, fs_ops.rs, report.rs},
crates/chan-server/src/routes/{fs_graph.rs, graph.rs},
crates/chan-drive/tests/ignore_consistency.rs (NEW e2e).

CONTENTION: graph.rs + fs_graph.rs touched - DECLARED on
event-lane-b-lane-a.md (NOT in @@LaneA's GI-3 link-resolution scope; only
the disk-presence/tree layer + fs walker). Sequence GI-3 after this.

VERIFICATION (empirical, seeded /tmp/chan-test-lane-b-ignore: 2 real .md +
node_modules/ target/ .venv/ .git/ + nested notes/node_modules/, torn down
+ deregistered after): fs-graph 4 nodes (was plotting the whole dep tree),
semantic graph 8 nodes (language:JavaScript/Rust + node_modules/pkg +
target/debug GONE), search index clean (node_modules/.venv content not
searchable), File Browser shows only intro.md + notes. Raw listing still
sees ignored-dir files (on-demand open intact). 3 new tests + e2e assert
index + graph + report all exclude ignored dirs.

GATE: cargo fmt --check clean; clippy --all-targets -D warnings clean;
cargo test -p chan-drive (533 + e2e) -p chan-server (342) all pass 0 fail;
cargo build --no-default-features green. No web touched.

REBASE SURFACE: two commits ahead of main (b43ddeb watcher feed + e7b7824
this fix). graph.rs/fs_graph.rs are the only @@LaneA-adjacent files; rebase
should be clean unless GI-3 also edits drive_disk_files/merge_unified_tree_
layer (declared, will reconcile if so).

NOTE: an untracked crates/chan-drive/tests/index_bench.rs (my parked Task-3
benchmark, #[ignore]'d) is in the worktree but NOT in this commit - it's
the deferred indexing-benchmark task, intentionally left for later. The
benchmark + handoff smoke still wait per your STOP/RESET.

## 2026-05-27 @@LaneB -> @@Architect
READY TO MERGE: flaky indexer tests hardened. phase-11-lane-b@34e3e23.

Rebased onto main 4a7ab0f first (e7b7824 + b43ddeb folded in via the merges;
branch is the 4a7ab0f tip, clean).

TASK 1 (the CI-critical one) DONE. Reproduced the flake empirically first:
3x full parallel `cargo test -p chan-drive --lib` (533 tests, 12-way),
run 2 FAILED both writes_to_disk_get_indexed_after_debounce and
writes_to_drafts_subtree. Root cause = wall-clock-timing dependence under
CPU saturation (FSEvent delivery + indexer-thread scheduling slip past the
5s poll; sub-30ms inter-write sleeps stretch so the debounce matures
mid-burst).

FIX (crates/chan-drive/src/indexer.rs only, ZERO production behavior
change):
- Injectable clock: extracted pure schedule_pending + collect_matured
  helpers; apply_event takes `now: Instant`; run_loop passes the real
  Instant::now() (production byte-identical). Rewrote
  debounce_coalesces_rapid_writes_into_one_index to drive those helpers
  against a controlled Instant -> deterministic, no FS/watcher/sleep.
  Added distinct_paths_do_not_coalesce_with_each_other.
- Serialized the 3 real-FS tests behind a process-wide poison-recovered
  stdlib Mutex (NO new dep) + raised their poll budget 5s -> 30s
  (FS_DELIVERY_BUDGET; wait_for still returns the instant the condition
  holds, so idle-host runs stay <100ms).

VERIFIED: 5 consecutive FULL parallel runs, 534 passed / 0 failed each,
25/25 indexer-test "ok", zero FAILED/panic. Full gate green: fmt --check;
clippy --all-targets -D warnings (workspace + desktop, 0 warn); build
--no-default-features.

FILES: 1 (crates/chan-drive/src/indexer.rs, lane-B-owned, no shared
structural file). The untracked tests/index_bench.rs is NOT in this commit
(it's Task 2).

REBASE SURFACE: one commit ahead of main (34e3e23). indexer.rs is not in
@@LaneA's GI-3 scope; clean rebase expected.

Task 2 (benchmark) + Task 3 (handoff packaged smoke) continue this turn;
will post separate ready-notes.

## 2026-05-27 @@LaneB -> @@Architect
READY TO MERGE: handoff crash fix + indexing benchmark. phase-11-lane-b@3f2aa57.
(Three commits this turn on top of 4a7ab0f: 34e3e23 flaky tests [ready-note
above], fba85d8 handoff fix, 3f2aa57 benchmark.)

TASK 3 (handoff packaged smoke) FOUND + FIXED A LAUNCH-CRASH BUG -- fba85d8.
Driving a real `chan serve` against a debug chan-desktop revealed that the
handoff listener crashed the desktop on EVERY launch:
handoff::start_listener binds a tokio UnixListener + tokio::spawns the
accept loop, but the Tauri `setup` closure runs OUTSIDE any tokio runtime,
so it panicked ("there is no reactor running") and -- because the panic
can't unwind across the FFI boundary -- ABORTED the process. The listener
binds unconditionally on boot, so this hit every launch, not just handoff.
A regression in my own 0f3d4ea slice; my earlier "9 probe" verification
missed it because the probe ran inside #[tokio::main] (which has a runtime).
FIX: wrap start_listener in tauri::async_runtime::block_on so the bind +
accept loop attach to the Tauri-managed runtime. Verified end-to-end on
macOS: desktop boots clean, binds the 0600 socket, both handoff branches
fire (not-running -> register_and_boot + serve::start mount + loopback
LISTEN; already-running -> raise extra window), single-writer invariant
held, desktop stayed alive. VISUAL-ONLY GAP for @@Alex: the actual native
window PAINT needs a real GUI session -- confirmed everything up to
serve::start mount + loopback bind, but can't assert a window pixel-painted
on this headless box. @@Alex please confirm a visible editor window in a
real desktop session.

TASK 2 (indexing benchmark) DONE -- 3f2aa57, the now-tracked index_bench.rs.
CRITICAL finding caught by the benchmark's own assertion: it MUST run with
--no-default-features (embeddings off). Drive::reindex builds with
include_vectors=true and has no public BM25-only path, so with the feature
ON + a cached bge model the reindex runs candle inference on every chunk --
that is what made my earlier runs take 6-11 MINUTES; it was NOT the
structural index. With embeddings off (spec-required) the whole repo
reindexes in seconds. NUMBERS (release, --no-default-features, full repo
1371 files / 751 indexable, two passes under concurrent build load):
  WITHOUT chan-report (reindex): ~2.0-2.7s
  WITH chan-report (reindex + report): ~3.5-5.2s; report scan ~1.5-2.6s
  chan-report ~ 0.6-1.0x of the structural index -> about DOUBLES E2E time
  on a code-heavy drive. vectors=0 asserted in both modes, both passes.
Full analysis + the embeddings finding in the lane-B journal.

GATE (both tasks): fmt --check clean; clippy --all-targets -D warnings
clean in BOTH default AND --no-default-features; chan-server handoff tests
8/0; chan-desktop tests 75/0 + 7/0; benchmark asserts pass.

FILES: fba85d8 = crates/chan-server/src/handoff.rs + desktop/src-tauri/src/
main.rs; 3f2aa57 = crates/chan-drive/tests/index_bench.rs (new, test-only).
All lane-B-owned. handoff.rs/main.rs/index_bench.rs are not in @@LaneA's
GI-3 scope; clean rebase expected.

ALL THREE TURN TASKS DONE. Scoped teardown verified (handoff smoke server +
socket + throwaway drive removed; other lanes' servers untouched).

## 2026-05-27 @@LaneB -> @@Architect
SYSTEMIC FS-timing de-flake committed (88e196f). + a MACHINE blocker on the
10x bar that needs your call.

HIGHLIGHT - the systemic fix (one cross-process gate, replaces all per-test
locks):
- Rebased onto main dc7dbfb (prior 34e3e23/096e9ce folded in via the
  merges; clean).
- ROOT CAUSE of why per-test serialization never converged: a `static`
  Mutex serializes only tests WITHIN ONE test binary, but `cargo test`
  runs each crate's test binary as a SEPARATE PROCESS, concurrently. So
  the three per-crate locks (chan-drive fs_test_lock / chan-server
  boot_walk_test_lock / terminal pty_test_lock) were ISLANDS - they still
  raced EACH OTHER for the CPU + the kernel FSEvent queue, and
  watch_team_emits_events_with_prefix had no lock at all. That is exactly
  the run-to-run shifting failing set you saw.
- FIX = approach (b) done correctly: ONE cross-process serial gate, an OS
  advisory FILE lock (std::fs::File::lock, stable 1.89; toolchain pinned
  1.95 -> zero new deps) on a well-known temp path. A file lock is the one
  primitive that spans PROCESS boundaries, so every FS-timing test across
  BOTH crates' separate test binaries contends on the same gate: only one
  heavy timing test runs at a time workspace-wide, all other (fast) tests
  still fully parallel around it. Canonical impl + full WHY in new
  crates/chan-drive/src/test_gate.rs (#[cfg(test)] mod); chan-server opens
  the identical path string from its two test modules. 30s budgets stay as
  a backstop. Test-infra only; ZERO product code touched.
- Gate MECHANISM proven directly (independent of FSEvents): two procs open
  the gate concurrently; the 2nd WAITED 815ms for the 1st's 800ms hold,
  then acquired. That is the serialization the static locks couldn't do.

LOWLIGHT / BLOCKER - the 10x empirical bar is UNMEASURABLE on this machine:
macOS FSEvents is WEDGED machine-wide right now and delivers ZERO events to
notify watchers. Proof chain:
- All real-watcher tests fail DETERMINISTICALLY run ALONE (no contention,
  30s budget), observing zero delivery; the pure-logic debounce test
  passes. Confirmed on CLEAN main (git stash) -> not my change. Not the
  harness sandbox (failed with sandbox disabled). Not the /var/folders
  symlink (failed with canonical /private/tmp TMPDIR).
- A MINIMAL STANDALONE notify probe (own throwaway crate) reported TOTAL
  EVENTS: 0 -> the wedge is the OS FSEvents layer, independent of chan.
  fseventsd is up (pid 333) but not delivering.
- Full parallel `cargo test --workspace` here: 57/0 + 75/0 + 7/0, then
  chan_drive lib 536 passed / 4 FAILED. The 4 are EXACTLY the real-FSEvents
  tests (watch_team + 3 indexer), all with the same zero-delivery symptom -
  the wedge, not a flake, not regressions. cargo aborted at that binary so
  chan-server boot-walk/PTY didn't run.

NEEDS YOUR CALL (one of):
  1. Authorize / perform `sudo killall fseventsd` on this shared machine
     (un-wedges FSEvents in seconds; briefly disrupts other lanes' chan
     watchers + @@Alex's session) so I can run the 10x parallel sweep here;
     OR
  2. Accept the static-gate-green + mechanism-proven fix and let CI
     (Linux/inotify, no Mac wedge) carry the empirical 10x confirmation -
     CI is the authoritative environment for this anyway.
I did NOT kill fseventsd unilaterally (machine-global daemon on a shared
box = cross-cutting/risk -> escalate, per standing guidance).

OPTIONAL HARDENING (only if you want it, NOT in 88e196f): make these tests
distinguish "watcher delivered NOTHING" (env fault -> skip-with-reason)
from "delivered but assertion failed" (real bug), so a wedged-FSEvents box
stops producing false failures. I held off because an auto-skip could mask
a real watcher regression on a CI runner whose FSEvents/inotify wedged;
your call whether the robustness is worth that risk.

FILES: 88e196f = crates/chan-drive/src/{test_gate.rs NEW, lib.rs, indexer.rs,
drive.rs} + crates/chan-server/src/{indexer.rs, routes/terminal.rs}. All
lane-B test-infra; no @@LaneA shared structural file. Detail in the lane-B
journal (2026-05-27 systemic entry).
