# @@Architect journal: phase 12

Orchestration log for phase 12. Append-only.

## 2026-05-27: phase 12 opened (phase 11 closed)

Phase 11 closed: continuation round delivered terminal WebGL self-heal, GI-9
(fs spine), GI-8 (reveal -> FB tab), and the LaneC release contract (slices
1-4); retrospective + carryover committed (`5f25cc1`). main `5f25cc1`, all
local, NOT pushed.

Opened phase 12 from `phase-12-backlog.md`. @@Alex set the lane shape:
- @@LaneA = graph + File Browser carryover (overlay/scope wipe W1-W7, GI-10,
  loading-state, GI-11 locks). MAY spawn 2-3 subagents.
- @@LaneB = scoping architect for the drive -> workspace terminology/docs/
  codemod. SCOPE FIRST, then codemod in a sequenced window.
- @@LaneC = @@Alex ad-hoc frontend/cosmetics/keyboard-shortcuts (incl. web vs
  Linux vs macOS native shortcut differences).
- @@Alex will add a couple more lanes later (release/build is the likely next,
  owning the release carryover: Tauri updater slice 5 + manual copy slice 6).

phase-10 sweep: only Linux desktop launch carries (postponed again by @@Alex);
macOS handoff was done in phase 11, drag-out superseded, release-verify
unblocked + done.

KEY COORDINATION PROBLEM (flagged in bootstrap.md): all three lanes touch
web/src and @@LaneB's codemod touches ~everything. Plan: @@LaneB scopes first;
the codemod lands in an @@Architect-chosen window (quiescent checkpoint or last/
mechanical pass) per @@LaneB's sequencing proposal; I announce a freeze on the
affected files. @@LaneA/@@LaneC run feature/cosmetic work during the scoping
phase. A "team workspace" concept already exists (Drafts/ metadata) - the rename
scope must disambiguate. The tunnel domain `drive.chan.app` rename is an @@Alex
decision @@LaneB must surface.

Created: bootstrap.md, phase-12-backlog.md, lane-{a,b,c}-plan.md, coordination/
(README + channels). This opening scaffold committed once as docs(phase-12):
open; the live bus stays untracked until phase close.

Next: @@Alex launches the lane sessions from their plan headers; I watch the
channels + serialize merges.

## 2026-05-27: I take the orchestrator seat as @@Lead; wave 1 dispatched

@@Alex named me @@Lead and put me in the @@Architect orchestrator seat: dispatch
waves, serialize merges, re-gate, own rollups + escalations. Lanes cut tasks to
me to unblock; I auto-resolve the routine majority, escalate contention/high-
stakes to @@Alex; I green-light execution after his signal + my doc review. I
keep signing the event-architect-* channels as @@Architect (@@Lead) so launching
lanes recover against one handle (noted in coordination/README.md).

Review round with @@Alex: read all 3 lanes' bootstrap findings.
- @@LaneA: oriented, worktree up, W1-W7 -> A1-A6 merge-slice plan, grounded
  against the real worktree (graphOverlay = 114 refs, load-bearing). Held at A1.
- @@LaneB: scope COMPLETE, @@Alex's 5 decisions ratified (Option C: Workspace =
  the drive, free the word first; keep drive.chan.app; full clean break; uniffi
  non-issue; Library unchanged). Final spec written. Caught the 4-way "workspace"
  collision + the wire-decoupling insight that makes the codemod splittable.
- @@LaneC: all 4 addendum-1 bugs investigated. Bug 3 (self-writes race) + Bug 2
  (Drafts-namespace prompt) have static root causes; Bugs 1 (terminal focus/blur
  render glitch) + 4 (dash->asterisk lists) need in-browser repro.

Rulings (auto-resolved): @@LaneA's 3 (keep search scopes / split language rooting
post-wipe / re-root via dir-nav per W3); @@LaneB 4-chunk plan + clean-break-as-
end-state + RichPromptSession. @@Alex ruled Bug 2 (MCP-namespace prompt, not
materialize) + sequencing (Bug 3 before chunk 1).

Wave 1 dispatched to event-architect-lane-{a,b,c}.md + seeded
event-lane-b-lane-c.md (teamOrchestrator.svelte.ts overlap: @@LaneB chunk-0 prose
vs @@LaneC Bug-2 prompt builder). HOLDs I own: @@LaneB chunk 1 until @@LaneC
Bug 3 merges; @@LaneB chunk 2 until @@LaneA graph/FB quiescent. @@Alex pokes the
lanes; I watch the channels + serialize.

## 2026-05-27: @@LaneD added (CI + release lane)

@@Alex added @@LaneD - the CI + release lane the opening bootstrap reserved. He
launches it himself (another terminal); I produced its bootstrap only. Mission:
(1) investigate + fix the current CI issues - it investigates, NOT me (GitHub
Actions unproven; the first-push shakedown of make ci-linux/ci-macos + release.yml
never fired; reproduce locally via make ci-* + lima/sdme); (2) align the next
release, a PATCH on 0.15.5 cut AFTER A+B+C land, accounting for @@LaneB's crate
rename + clean break (Cargo/artifact naming, CHANGELOG breaking note).
Bootstrap: lane-d-plan.md; channels: event-architect-lane-d / event-lane-d-
architect / event-lane-d-alex. Boundaries baked in: shared-infra authorization
stated inline, secret-VALUES-never rule, NO unilateral origin push (the first
push is a coordinated event). Release cut gated on A+B+C + my re-gate. Zero
web/src overlap with A/C; coordinates the crate-rename/artifact-naming seam with
@@LaneB on a d-b channel.

## 2026-05-27: Round 1 sync (wave-1 integration)

@@Alex framed this checkpoint as a ROUND SYNC, not lane-close: integrate wave 1,
record, dispatch round 2. Lanes idled with merge-ready work.

MERGED to main (--no-ff, disjoint files, ort clean):
- cf756ca = phase-12-lane-a A1+A4 (W1+W3 graph-scope-from-tab + W4 dock
  browserState). availableSearchScopes preserved; overlay state retained for A5.
- 34f0b3f = phase-12-lane-c addendum-1 Bug 3 (self-write race) + Bug 2 (Drafts
  MCP prompt). HELD Bug 1 (3b587a7, terminal blur) - unverifiable WKWebView fix,
  needs @@Alex desktop verify.
- f72b8a7 = phase-12-lane-b chunk 0 (free the word, RichPromptSession, Rust-only).
main now f72b8a7.

COMBINED RE-GATE (main checkout) ALL GREEN: fmt --check; build --no-default-
features; clippy --all-targets -D warnings; cargo test (347 chan-server incl. the
new bus self-write test, full workspace pass); web npm run check 0/0 (4110 files);
npm run build OK. Docs stay UNTRACKED/dirty per the round timing - commit at
phase close, not this sync.

DECISIONS:
- @@LaneA deferred group/global/git_repo kind-branches -> FOLD INTO A5 (one
  destructive cleanup), no separate A1b. A1 merged => A5 cleared once A3 lands.
- @@LaneC self-write race follow-up (other post-await note() sites) -> AUTHORIZED
  as a small hardening slice; same root cause, high-annoyance class. Likely
  related to addendum-2's "editor still reloading while I write" report.
- @@LaneB chunk 1 RELEASED (its hold was Bug 3, now merged). RichPromptSession
  locked (merged; no flip to RichPromptDraft).

ROUND-2 DISPATCH: @@LaneA A3->A5(+kinds)->GI-10/loading-state->A6 walk; @@LaneB
chunk 1 now, chunk 2 still held on @@LaneA quiescence, chunk 3 docs; @@LaneC
self-write follow-up + Bug-1 desktop verify (Alex), addendum-2 Alex-gated;
@@LaneD launch (Alex). Action items to @@Alex: verify Bug 1 in chan-desktop;
release addendum-2 when ready; launch @@LaneD.

DIAGNOSTIC (terminal-font-after-sleep, addendum-2; carry into the @@LaneC
terminal-recovery dispatch): @@Alex reports resizing ANY window clears the glitch
on ALL terminals at once. So the recovery logic works (the global resize -> fit/
refit -> WebGL repaint path); the bug is only that nothing AUTO-triggers it on
WKWebView host-wake. Fix lead: fire recoverTerminalRendererAfterHostResume (fit +
atlas clear + delayed re-fits) for all panes on the display-wake/visibilitychange/
host-resume event, not just on a manual resize. Same mechanism as the held Bug 1
(blur repaint) + the existing "Bug 6" host-resume handler -> ONE recovery pass.

## 2026-05-27: @@LaneD first report + gate-gap caught

@@LaneD investigated (corrected my stale framing): basic CI 0.15.2-0.15.5 IS
green on origin; the real gap is the v0.15.5 RELEASE workflow failing + macOS
signing never running + no GH release/dl for 0.15.5. Findings: (#1) RPM staging
path mismatch - cargo generate-rpm writes under crates/chan/target/ but the
workflow's `find target` misses it; fix = --target-dir to workspace target.
(#2) TAURI_SIGNING_PRIVATE_KEY absent -> escalated to @@Alex (secret + desktop
scope). (#3) vitest no longer in the CI gate (only svelte-check + build).
GREEN-LIT #1 (RPM, validate in lima) + #3 (add vitest to gate). #2 is @@Alex's.

@@LaneD #3 also exposed a gap in MY round-1 re-gate: I ran npm run check + build
but not vitest (the defined gate is check+build; lanes ran vitest per-branch).
Ran `npx vitest run` on combined main f72b8a7 now: 1598 passed / 11 skipped / 0
fail. Merges fully validated. Going forward the gate should include vitest (per
#3). addendum-2 review questions written to lane-c/addendum-2/round-n-review.md
for @@Alex to answer inline in the editor.

## 2026-05-27: addendum-2 ratified + routed; @@LaneE opened

@@Alex answered all 9 review questions in round-n-review.md. Routing:
- FB independent-expansion -> @@LaneA (VERIFY first: @@Alex on v0.15.5, A4 may
  have already fixed it; close-if-fixed like Bug 4, else give the dock its own
  expansion store).
- Editor no-reload-while-typing + cursor/focus restore on cmd+r, terminal-recovery
  pass (folds the held Bug 1), drag-drop EASY case only -> @@LaneC (append-only
  after the self-write follow-up). @@Alex's rule: NEVER reload/update the open doc
  while typing; fs change -> banner only; read-only -> mark locked; cmd+r window
  reload -> restore exact caret + focus.
- Shortcuts policy -> NEW @@LaneE (cross-platform: web alt-nav, desktop cmd-nav;
  close cascade tab->pane->window->workspace-list; ctrl+a context split; find
  triad cmd+f/g/shift+g verify; infographics + Hybrid chords verify). Much is
  verify/wire existing -> @@LaneE audits first. Bootstrap: lane-e-plan.md;
  channels event-architect-lane-e / lane-e-architect / lane-e-alex; roster +
  channel table updated.

web/src quiescence gate for @@LaneB chunk 2 now spans @@LaneA + @@LaneC + @@LaneE
(all three touch web/src). Terminal area (TerminalTab.svelte) shared by @@LaneC
(recovery) + @@LaneE (readline collisions) -> c-e channel. serve.rs shared by
@@LaneE (key-bridge) + @@LaneB chunk 1 (rename) -> b-e channel.

## 2026-05-27 (round-2): @@LaneB chunk 1 + env-vars merged

@@LaneB (idling) delivered + rebased onto f72b8a7 first (per my instruction).
Merged tip 47b127e as merge 2140925: chunk 1 (chan-drive->chan-workspace crate +
Drive->Workspace type, 120 files, wire literals kept for chunk 2) + the
CHAN_WORKSPACE_NAME/PATH env-vars slice. Rust re-gate GREEN (fmt/clippy/build/
test); web/ untouched (vitest 1598 stands). @@LaneB caught the 3-meanings-of-drive
trap (renamed chan dirs; PRESERVED GoogleDrive/iCloudDrive products + tunnel/
domain vocab) via test+audit - the spec missed it.

RULINGS: chunk 1b (internal lowercase var/const eradication) AUTHORIZED as a
separate backend slice, sequenced AFTER @@LaneC's self-write follow-up (same
chan-server files), before chunk 2. CLI help copy -> defer to chunk 2 (flip with
wire+frontend). env-vars clear_mcp_env + basename-name confirmed.

COORDINATION CONSEQUENCE: the rename touched chan-server files.rs/attachments/
contacts/drafts/rich_prompts/control_socket/bus (LaneC's self-write follow-up
target) + desktop/serve.rs (LaneE key-bridge). So @@LaneC must rebase its in-flight
follow-up onto post-rename main 2140925 before merge; @@LaneE branches its serve.rs
slice off 2140925. LaneA (web/src only) unaffected by chunk 1. Heads-up posted on
the LaneC channel (non-interrupting). chunk 2 quiescence gate now spans A+C+E.

(@@LaneE separately posted its shortcuts AUDIT + 2 routine decisions [DEC-1 close-
window IPC, DEC-3 cmd+i vs Mod+. i] awaiting my review - HOLDING per @@Alex's
"don't bother the others" until he greenlights. Notably @@LaneE found the policy
is ~80% already implemented - mostly verify, a handful of gaps.)

## 2026-05-27 (round-2): @@LaneE greenlit (@@Alex ruled the 6 audit points)

@@Alex engaged @@LaneE; ruled all open points, GO on slices i/iii/iv:
1. web pane nav -> alt+[/] (web only); desktop keeps cmd+[/]. 2. wire cmd+s
search + rename Hybrid cmd+. f -> cmd+. s. 3. wire cmd+/ cmd+\ splits (approved).
4. close-cascade tail MUST close the window + refocus the workspace list when
nothing's left (today's no-op = the bug); DEC-1 mechanism is @@LaneE's impl call.
5. Linux: ctrl+d ONLY for close (do NOT wire ctrl+w - keeps readline delete-word;
dissolves the c-e ctrl+w seam); ctrl+d context-aware so terminals get EOF. 6. add
direct cmd+i for infographics (free) IN ADDITION to Mod+. i. @@LaneE rebases onto
2140925 before its serve.rs slice (chunk 1 renamed serve.rs).

## 2026-05-27 (round-2): @@LaneA A3 merged; A5 held; vitest flake found

@@Alex said "@@LaneA is done" but the branch shows otherwise: @@LaneA committed
A3 (9bc0ddb) + A5 PART 1 (ca86e34, overlay-state deletion) then idled. By its own
plan A5 isn't finished (part 2 = dead GraphPanel kind-branches PENDING) and the
fresh-binary browser smoke it flagged for the destructive cut hasn't run; GI-10,
loading-state, A6, FB-addendum verify all unstarted. So @@LaneA is NOT done with
round 2.

MERGED A3 (web/src hash retire) as main 48b4951; web re-gate: svelte-check 0/0,
build OK, all 1593 vitest pass. HELD A5 (incomplete + unsmoked destructive
deletion) until @@LaneA resumes, finishes part 2, and runs the smoke.

VITEST FLAKE (pre-existing, found during re-gate): full suite exits 1 on an
unhandled rejection "Failed to parse URL from /api/drive" around tabs.test.ts -
passes clean in isolation, tabs.test.ts unchanged since fe6e126, so NOT from this
round. Same issue @@LaneD saw the old CI catch. Routed to @@LaneD as a vitest-in-CI
precondition (likely a jsdom base-URL fix). A3 stands (all tests pass; unrelated).

@@LaneA still gates @@LaneB chunk 2 (not quiescent). It must resume to finish A5+
GI-10+loading-state+A6+FB before the chunk-2 freeze window can open.

## 2026-05-27 (round-2): sweep of idle lanes - C follow-up + D RPM landed; B 1b released

@@Alex: all but @@LaneE idle/waiting. main e927e90.
- @@LaneC: rebased onto 2140925; self-write follow-up (327960e) was STACKED on the
  held Bug 1 (cddc578). Disjoint files (chan-server backend vs TerminalTab.svelte),
  so I CHERRY-PICKED 327960e to main (e927e90); left Bug 1 on the branch for
  @@Alex's verify. Told C to rebase-drop 327960e on resume + fold Bug1 into the
  terminal-recovery pass.
- @@LaneD: MERGED RPM fix 5e13053 (7e684e1; blocker #1 cleared). HELD fc96280
  (vitest-in-gate) - committed before my flake note; would gate on the still-
  failing tabs.test.ts /api/drive rejection. D fixes the flake first, then I merge.
- @@LaneB: 0 ahead, idle. RELEASED chunk 1b (internal var/const eradication) - its
  gate (C follow-up) is now in main. B rebases onto e927e90.
- @@LaneA: A5 still incomplete (no new commits); held on A's own resume (part 2 +
  smoke), not a merge-wait.
Rust re-gate after C follow-up + D RPM: fmt/clippy/test(31 suites)/build GREEN.
Open: chunk-2 quiescence (A+C+E), vitest-in-CI flake fix (D), A5 (A resume),
Bug 1 desktop verify (@@Alex).

## 2026-05-27 (round-2): @@LaneE i/iii/iv merged; chunk-1 fixups routed to B

Merged @@LaneE fc8310c (cross-platform shortcuts i/iii/iv) as main 4cb5ca8; full
re-gate green (Rust + web; vitest 1596 pass, flake didn't fire this run). @@Alex
had poked @@LaneE concurrent with my merge - harmless; told E its commit landed,
to rebase onto 4cb5ca8, not re-report. E correctly DEFERRED the cmd+. f -> cmd+. s
rename (collides with WASD swap-down on `s`); shipped top-level cmd+s + kept cmd+. f.
That sub-chord call is now an @@Alex decision (E leans keep cmd+. f).

@@LaneE found TWO chunk-1 incomplete-rename runtime bugs (not gate-caught): stale
Tauri perm names (list_drives/remove_drive in app.toml -> IPC denial) + handoff
serde variant mismatch (open_workspace vs open_drive). Routed to @@LaneB as a
chunk-1 fixup folded into chunk 1b. Lesson: the rename gate (compile+unit) missed
runtime IPC/serde string mismatches - flag for chunk 2's wire flip to watch the
same.

Board: A (resume A5+), B (chunk 1b + the 2 fixups), C (rebase + addendum-2 queue),
D (vitest flake fix), E (slice ii verify + rebase; cmd+. f pending @@Alex). All
poke-able. chunk-2 freeze still gated on A+C+E web/src quiescence (mine to call).

## 2026-05-27 (round-2): @@LaneE COMPLETE

@@LaneE rebased to 4cb5ca8 (0 ahead - i/iii/iv all in main), closed cmd+. f
(option a), slice ii verify-only (rests on code analysis; find-triad empirical
check -> @@Alex's desktop spot-check list with Bug 1). Round-2 complete, idle.
=> @@LaneE is now QUIESCENT on web/src. chunk-2 quiescence gate now needs only
@@LaneA + @@LaneC to settle (E done). When A + C report graph/FB + their web/src
work merged + paused, I open @@LaneB's chunk-2 freeze window.

## 2026-05-27 (round-2): big sweep - A5 + B fixups/1b + D gate + C editor merged

main abac76c, full re-gate GREEN incl. vitest EXIT 0 (D's flake fix closed the
/api/drive unhandled rejection). Merged this sweep:
- @@LaneA A5 (a4c139b+760e242): overlay/scope-kind wipe complete (e30ed8b).
- @@LaneB 304b9ff (chunk-1 Tauri perm/invoke fixup) + 4ddc657 (chunk-1b consts)
  (979f1e8).
- @@LaneD fc96280 (vitest in web-check) + b63403e (flake fix) (e2c9eb8) -> vitest
  exits 0, CI gate robust, finding #3 closed.
- @@LaneC 1222a5f editor no-reload (cherry-pick abac76c; Bug 1 0a9fb27 stays held).

LIVE BREAK on main (gate-invisible; @@LaneA smoke found it): chunk-1 renamed the
/api/graph scope serde variant drive->workspace but client.ts still sends "drive"
-> whole-workspace graph errors. Directed @@LaneB to pin the variant back to "drive"
(#[serde(rename]) as a hotfix; real flip rides chunk 2.

KEY RULING + LESSON: accepted @@LaneB's recommendation to FOLD the remaining
drive-eradication (lowercase vars + serde/IPC-serialized drive/drives fields) into
chunk 2's verified wire flip, NOT a blind backend sweep. Three gate-invisible
runtime mismatches this round (Tauri perm names, handoff variant [skew], /api/graph
scope variant) prove the rename's wire strings must flip with runtime/browser
verification. The cargo+unit gate cannot catch serde/IPC string drift - chunk 2
must smoke every renamed wire surface.

Quiescence: E done; A + C still working. chunk 2 held. @@Alex: re-poke B for the
graph hotfix; Bug 1 + find-triad desktop spot-checks pending.

## 2026-05-27 (round-2): graph hotfix + GI-10 + cmd+r caret merged

main a477e62, re-gate green (vitest exit 0). @@LaneB 2256aa8 graph hotfix (pinned 4
scope serde variants - graph/inspector/reset/close - back to "drive") UNBREAKS
main; B smoke scope=drive 200 / scope=workspace 400; 3 surfaces were broken not
just graph. @@LaneA GI-10 (drive root bottom, spine up). @@LaneC facet C (cmd+r
caret/focus via URL-hash flush; cherry-pick; Bug 1 still held). Steers: C -> item 3
(drag-drop) next; DEFER facet B (chmod-w locked = lowest value + backend writable-
field); item 2 terminal-recovery pairs w/ Bug 1 desktop verify. A -> loading-state
-> A6 walk (graph now works) -> FB-verify. Quiescence: A + C still working; chunk 2
held; E + (after this) most of B done. Nothing red on main.

## 2026-05-27 (round-2): A loading-state confirmed; B chunk-2 plan endorsed (no merges)

@@LaneA: GI-10 visual + wipe verified fresh-binary (covers A6 essentials). Loading-
state DE-RISKED to frontend-only - /api/indexing/state per-dir signal already exists
(only EmptyPaneCarousel used it). CONFIRMED the UX (pulse parent dir while indexing,
mirror FB spinner, no backend add) - my call, grounded in the spec. After it + FB-
verify, A is quiescent.
@@LaneB: held chunk 2 correctly (verified A/C non-quiescent, surfaced to @@Alex).
Wrote workspace-rename-chunk2-plan.md (token map + mandatory browser/desktop verify).
RATIFIED the 2a/2b "never wire-skewed" rule: chunk 2 lands atomic (or 2a+2b back-to-
back in-freeze with a runtime smoke), NEVER 2a-then-2b-later (= the /api/graph break
class). I open the freeze + ping B when A+C quiesce.
Quiescence remaining: A (loading-state + FB-verify) + C (drag-drop; terminal+Bug1
pair w/ desktop verify).

## 2026-05-27 (round-2): C item 3 merged; C quiescent; chunk-2 gates clarified

main 206e3d4 (re-gate green, vitest 1599). Cherry-picked @@LaneC item 3 (drag-drop
image row-move, editor-only, disjoint). @@LaneC now QUIESCENT on shippable work:
addendum-1 + addendum-2 handled; HELD batch = Bug 1 (107276f) + item 2 (dd9521d),
both TerminalTab.svelte, both await @@Alex's chan-desktop verify (blur focus-switch
+ sleep/wake) and merge together after. Facet B parked.

CHUNK-2 GATES now crisp: (1) @@LaneA finishes loading-state + FB-verify (idle,
greenlit, awaiting re-poke); (2) @@Alex's terminal desktop verify -> merge Bug1+item2
-> TerminalTab settled (chunk 2's rich-prompt field rename also touches it, so it
must settle first). When both done I open the web/src + routes freeze for @@LaneB.
B/D/E idle/done. Nothing red on main.

## 2026-05-27 (round-2): @@LaneA status - both items are real builds (no blocker)

@@LaneA not blocked - verified the addendum-2 FB-expansion bug is REAL (not A4-
fixed): FileTree renders expansion from the GLOBAL treeExpanded.map singleton; A4
only decoupled browserState. Fix = per-instance expansion (moderate refactor).
loading-state = frontend-only (confirmed), smoke needs a larger drive to see
indexing lag. Both are genuine BUILDS, not quick closes. CALL: build both (no
descope) - FB-expansion is an @@Alex-reported bug + loading-state spec'd, and chunk
2 is gated on @@Alex's terminal verify anyway, so A's builds parallelize that.
@@LaneA: loading-state -> FB-expansion, gated slices. chunk-2 window = A both-done
+ @@Alex terminal verify (merge Bug1+item2).

## 2026-05-27 (round-2): terminal batch merged UNVERIFIED (@@Alex skipped); C DONE

@@Alex skipped the desktop verify (pre-release; will re-report if buggy - saved as
[[feedback-pre-release-merge-unverified]]). Cherry-picked Bug 1 (b1dfb5e) + item 2
(323bfd9) -> main 323bfd9, re-gate green (vitest 1600). RECORDED: terminal/WKWebView
fixes merged WITHOUT empirical verification by design. @@LaneC now FULLY QUIESCENT
(all work merged; facet B parked). chunk-2 gate reduces to JUST @@LaneA (loading-
state + FB-expansion); Alex's verify no longer gates it; TerminalTab settled for
chunk-2's rich-prompt field rename.

## 2026-05-27 (round-2): A loading-state slice 1 merged

Merged @@LaneA loading-state slice 1 (19d5456 -> main 544f88a; re-gate green, vitest
1604): drive-global indexStatus -> GraphPanel pulsing "indexing..." cue. A split
loading-state into slice 1 (this) + slice 2 (higher-risk ghost-node pullback via
paint()/edge-filter) - good de-risk. @@LaneA remaining before quiescence: slice 2 +
FB per-instance-expansion. With @@Alex's terminal verify skipped, @@LaneA is the
SOLE remaining chunk-2 gate. B/C/D/E idle/done. Nothing red on main.

## 2026-05-27 (round-2): A FB-expansion merged; only slice 2 left

Merged @@LaneA FB per-instance-expansion (915ea29 -> main be6231b; re-gate green,
vitest 1607): FileTree renders expansion per-instance (tab `be` / dock $state), no
more global-singleton mirror. Minor follow-up flagged: dock cross-reload persistence
snapshot-key timing (fold into chunk 2 or drop). @@LaneA's ONLY remaining item =
loading-state slice 2 (paint/edge-filter ghost-pullback). Told @@Alex he can poke A
to continue. When slice 2 lands, A is quiescent -> chunk-2 freeze opens (A is the
sole gate; verify skipped, B/C/D/E done).

## 2026-05-27 (round-2): CHUNK-2 FREEZE OPEN (web/src quiescent)

Merged @@LaneA loading-state slice 2 (-> main 22621db; re-gate green, vitest 1610).
@@LaneA paused -> A + C + E quiescent on web/src; B + D done. OPENED the web/src +
routes freeze; GO'd @@LaneB on chunk 2 (rebase onto 22621db; flip routes + on-disk
serde + UNPIN the 4 scope variants WITH client.ts + rich-prompt fields + folded
backend field eradication + CLI copy + .svelte renames; leave drive.chan.app).
ENFORCED: land atomic (2a+2b) + mandatory in-browser/desktop smoke of every renamed
wire surface before ready (gate blind to that class - 3 hits). B is the only active
web/src writer now. @@Alex poking B. Round's last big piece.
Deferred carryover for round close: loading-state per-parent pulse, dock snapshot
key, dead SCOPE_HUB machinery (A5 follow-up), GI-11 tests, C facet B, unverified
terminal fixes.

## 2026-05-27 (round-2): chunk 2 mid-2c - keep driving (NOT merged)

@@Alex relayed "B complete" but the repo said otherwise: 2 WIP commits "NOT ready"
(da32e50 2a + 0c36d6fb 2b, "2c + smoke pending"), no ready report, 37 code-level
rename leftovers (default_drive_root family, "drives" JSON keys, /api/drive
comments), smoke not run. B confirmed: core flip works end-to-end but 2c (rich-prompt
session fields + backend local-var/CLI/IPC eradication incl. add_drive/app.toml/
main.js + a HardDrive icon) + the mandatory smoke remain. DID NOT MERGE. Directed B
to KEEP DRIVING to completion (no mid-flip review - low value vs the finished atomic;
freeze on, B is sole web/src writer), squash to ONE atomic commit + smoke + report.
I re-gate + re-smoke the complete commit before merge. Reconciled the staleness with
@@Alex directly (verified branch/bus, didn't merge on the chat signal).

## 2026-05-27 (round-2): chunk 2 (814d3987) REJECTED - 2 gate-blind defects

B squashed to one atomic commit (814d3987, 222 files) + reported gate+smoke green.
My re-audit caught TWO defects B's smoke missed (the gate-blind class, AGAIN):
1. CONFIG WIRE MISMATCH: backend emits default_workspace_root, frontend types.ts:72
   + 4 config components still default_drive_root -> default-root setting silently
   broken in the UI. Backend has 0 default_drive_root (clean mismatch).
2. DESKTOP COMPILE BREAK: desktop/default_drive.rs:114/203 call set_default_drive_root
   but library.rs renamed it set_default_workspace_root -> chan-desktop (a workspace
   member) won't compile -> B's clippy --all-targets claim couldn't have passed; its
   gate skipped the desktop compile.
NOT MERGED. Sent back: fix both, re-gate WITH chan-desktop (clippy --all-targets) +
smoke the settings round-trip + desktop launcher, re-report. Accepted B's chunk-2d
deferral of internal NON-wire snake_case compounds + rich-prompt session rename
(separate follow-up) - but default_drive_root is WIRE, not in that set. Lesson
reinforced: the re-gate+re-audit MUST include a frontend-wire-field consistency
check + desktop compile + a settings/config round-trip smoke - cargo+vitest+svelte-
check are all blind to it.

## 2026-05-27 (round-2): addendum-3 folded in (POST chunk-2)

@@Alex's addendum-3 (docs/journals/phase-12/addendum-3.md, 4 cosmetics, "ready to
take in") routed - ALL web/src, so SEQUENCED POST-CHUNK-2 (they touch the codemod's
frozen surface; can't run during the freeze). Still this round.
- @@LaneC: item 1 (terminal dot: pulse while output flowing, solid when unseen-idle).
- @@LaneA: item 2 (graph right-click anywhere on canvas bg), item 3 (drive-at-bottom
  = GI-10 already merged -> verify+close), item 4 (Export-to-PDF editor->Inspector,
  editable files - confirm set w/ @@Alex).
Round wrap order: chunk 2 lands (B fixing the 2 defects) -> lift freeze -> @@LaneA +
@@LaneC do addendum-3 -> then docs(phase-12) commit + retrospective.

## 2026-05-27 (round-2): broadcast-shortcut add-on -> @@LaneC (POST chunk-2)

@@Alex: add cmd+shift+i (macOS-native) to toggle terminal broadcast. Initially read
as net-new (my grep missed it); @@Alex clarified the broadcast SELECT-ALL/DESELECT-
ALL feature ALREADY EXISTS (TerminalTab.svelte:1639 toggle + Pane.svelte broadcast-
marker + setTerminalBroadcast*). So it's JUST the shortcut + docs. Routed to @@LaneC
(NOT E) to keep all post-chunk-2 TerminalTab edits in one lane (it has the dot too);
C follows E's keymap/key-bridge convention (e-c channel) for the macOS-native
accelerator. Document in web/EDITOR.md (mirrors iTerm). Post-chunk-2, gated slice.

## 2026-05-27 (round-2): CHUNK 2 LANDED + verified; freeze lifted; tail dispatched

@@LaneB re-reported adcee898 (re-squashed) fixing both defects. Verified directly:
0 frontend default_drive_root, 0 desktop set_default_drive_root. MERGED as bce6bd3.
MY re-gate GREEN (fmt; clippy --all-targets incl chan-desktop; cargo test 31; build;
web check 0 + vitest 1610 + build). MY runtime SMOKE (scoped binary, isolated HOME,
port 8492) all green: /api/workspace 200; graph scope=workspace 200 (6 nodes) /
scope=drive 400; /api/drive 404; cloud-workspaces 200; /api/config
default_workspace_root; on-disk ~/.chan/workspaces/. drive->workspace clean-break
flip COMPLETE on wire/frontend/CLI/desktop. FREEZE LIFTED.

Tail dispatched (post-freeze, parallel, rebase onto bce6bd3): @@LaneB chunk 3 docs +
chunk 2d; @@LaneA addendum-3 (graph right-click / GI-10-verify / Export-PDF->
Inspector); @@LaneC addendum-3 dot + cmd+shift+i broadcast shortcut; @@LaneE @@Alex's
new Cmd+R-in-pane-menu nit. Then docs(phase-12) + retrospective = round close.

## 2026-05-27 (round-2): addendum-3 + broadcast merged; A/C/E DONE; only B cleanup left

main 7edcf29d, re-gate green (vitest 1613, chan-desktop compiles). MERGED: @@LaneA
A3-i (graph right-click-anywhere) + A3-iii (Export-to-PDF->Inspector); A3-ii closed
done-by-GI-10. @@LaneC A3-dot (pulse/solid) + cmd+shift+i broadcast toggle (macOS-
native, metaKey-gated; Linux stays DevTools). @@LaneE Cmd+R nit = FALSE ALARM (label
already present; @@Alex on v0.15.5) - closed no-change. A/C/E COMPLETE + quiescent;
D done earlier.
REMAINING = @@LaneB only: chunk 3 docs sweep (+ stale serve.rs:1140 comment fold-in)
+ chunk 2d (internal non-wire snake_case + rich-prompt session rename). After those
+ re-gate -> docs(phase-12) commit + retrospective = round close.

## 2026-05-27 (round-2): v0.16.0 clean-slate release prep -> @@LaneD

@@Alex: cut v0.16.0 (MINOR - the workspace clean break warrants > patch; supersedes
the earlier "patch" framing) + supersede ALL prior versions so `chan upgrade` only
offers 0.16.0+. Routed to @@LaneD: prep now (parallel w/ B chunk 3/2d), CUT after
round close. @@Lead rec: KEEP git tags (provenance; CHANGELOG/journals link them),
delete only downloadable assets + repoint the upgrade channel (update.rs + /dl/
desktop/latest.json) to 0.16.0; D surfaces the exact mechanism + keep-vs-delete-tags
to @@Alex before any destructive gh delete. CONTEXT: `gh release list` EMPTY (no
published releases - v0.15.5 workflow failed), so "delete prev versions" is mostly
the upgrade-channel + tag call. Dev->prod updater bridge now MOOT (fresh install).

## 2026-05-27 (round-2): v0.16.0 release decisions RATIFIED by @@Alex

@@Alex ratified: KEEP git tags, DELETE published releases + assets (he EXPLICITLY
authorized the destructive `gh release delete` - sole user); repoint upgrade channel
to 0.16.0; defer Tauri self-upgrade testing to v0.16.1 (fresh install of 0.16.0
today). @@LaneD pre-cleared on these - proceed at cut time, report what it deletes.
Cut still after round close. (gh release list empty -> likely little to delete.)

## 2026-05-27 (round-2): @@Alex verifies C's dot+shortcut post-0.16.0 (no extra tests)

@@Alex: no extra verification tests before round close for @@LaneC's dot-pulse +
cmd+shift+i broadcast shortcut - he verifies the empirical behavior himself in
v0.16.0, reports if off (pre-release posture, [[feedback-pre-release-merge-unverified]]).
ALEX-VERIFIES-POST-0.16.0 set now: terminal Bug1 (focus-switch) + item2 (sleep/wake)
+ A3 dot-pulse + cmd+shift+i broadcast + the find-triad spot-check. All merged +
gated; none block round close.

## 2026-05-27 (round-2): CHUNK 3 + 2d MERGED - rename CODE-COMPLETE; web-marketing follow-up

Merged @@LaneB chunk 3 (docs sweep 306d9c45) + chunk 2d (internal + TUNNEL rename
2ec65f39) as main 2919caa9. Re-gate GREEN (clippy --all-targets incl tunnel+desktop;
test 31; web check 0 + vitest 1613). Leftover audit: ZERO code drive-residue; desktop
CLEAN; only drive.chan.app hostname preserved (87). drive->workspace is CODE-COMPLETE.

COURSE-CORRECTION: I'd mis-assumed the tunnel should be PRESERVED (from chunk-1's
conservative framing). @@Alex EXPLICITLY wanted the tunnel fully renamed (TunneledDrive
->TunneledWorkspace, {drive}->{workspace} slug, --tunnel-workspace, proto `drive`->
`workspace` wire field), keeping ONLY the drive.chan.app hostname (he cleans up later).
chunk 2d did exactly that - CORRECT. I verified before merging; no false escalation
landed. NOTE (Alex's ops call): the tunnel WIRE changed, so the cloud tunnel server at
drive.chan.app needs redeploy from the renamed chan-tunnel-server before tunnel mode
works on 0.16.0 - gate-invisible cross-service, but @@Alex controls the cloud + it's
pre-release.

@@Alex follow-ups: (1) chan-desktop - already DONE by 2d (0 leftovers, verified).
(2) web-marketing drive->workspace (12 hits, home.html + build.mjs; preserve
drive.chan.app) -> routed to @@LaneB. That's the last code/copy item before round
close (then docs(phase-12) + retrospective + @@LaneD v0.16.0 cut).

## 2026-05-27 (round-2): web-marketing merged - drive->workspace 100% COMPLETE

Merged @@LaneB d43da40c (web-marketing + docs completion + stale 2d design-doc
code-refs) as main 4eb87901. web-marketing build green; 0 drive residue (drive.chan.app
preserved); docs/manual/drives.md->workspaces.md. No code/web-src change -> cargo+web
gate unchanged (green at 2919caa9). drive->workspace is 100% COMPLETE: code + docs +
manual + marketing; only drive.chan.app hostname + cloud product names survive.

ALL LANES COMPLETE: A (graph/FB wipe+GI-10+loading+FB-expansion+addendum-3), B
(rename chunks 0/1/1b/fixups/hotfix/2/2d/3 + web-marketing), C (addendum-1/2/3 +
self-write; Bug1/item2/dot/broadcast unverified-by-design per @@Alex), D (RPM +
vitest-gate; v0.16.0 cut PENDING), E (shortcuts policy; Cmd+R false-alarm).
ROUND AT CLOSE THRESHOLD. Remaining: docs(phase-12) commit + retrospective (mine) +
@@LaneD v0.16.0 cut (version bump + release + delete-old + first push). Confirming
with @@Alex he's done adding before I commit docs + write the retro.

## 2026-05-27 (round-2): ROUND CLOSED

@@Alex confirmed "let's do" (and is poking @@LaneD for the v0.16.0 cut). Wrote
retrospective.md (done/pending + highlights/lowlights + honest feedback for agents,
@@Alex, and me). Committing the whole phase-12 doc tree (the live bus + journals,
intentionally untracked all round) to main as docs(phase-12): close. All 5 lanes
COMPLETE; drive->workspace 100% done. Handing off to @@LaneD for the v0.16.0 cut
(version bump + CHANGELOG + release + delete-old-releases + repoint upgrade channel
+ first origin push that fires CI over the round). main at close: 4eb87901
(+ this docs commit). PHASE 12 ROUND CLOSED.
