# Phase-16 round-1 status (live, @@Lead-owned)

The live "where are we". @@Lead updates this as lanes report. Lane detail
lives in each `event-lane-<x>.md`; this is the consolidated board.

Legend: IDLE (spawned, not dispatched) | DISPATCHED | WORKING | GATED
(make pre-push green, sha posted) | MERGED | BLOCKED.

## Board (current)

```
Lane | owner   | state                                              | last sha
-----+---------+----------------------------------------------------+---------
A    | @@LaneA | round-1 COMPLETE (cs lead-tooling + cs-write queue)| 3d6d144e
B    | @@LaneB | Rich Prompt MERGED; carousel-row nit QUEUED        | 6d5c95eb
C    | @@LaneC | round-1 COMPLETE (onboarding+blocklist+About-slide)| a4ea43cd
D    | @@LaneD | round-1 COMPLETE (blocklist FB UI merged)          | b6b33938
E    | @@LaneE | terminal.md MERGED; verify-last doc sections PEND  | 0970c902
```

ALL in-flight product builds MERGED + on the fresh :8787 (see the REBUILD note
at the tail). Remaining round-1 work: (1) @@LaneB carousel-row nit (queued,
separate commit), (2) @@LaneE terminal-page verify-last sections (Rich Prompt +
queue, after @@Host validates), (3) @@Host validation on :8787, (4) RESPAWN,
(5) round-close.

POST-WAVE-3 @@HOST FEATURE STREAM (latest first; all MERGED unless noted):
- Rich Prompt (NEW, returning feature): floating inset rounded markdown bubble
  over the terminal bottom; Cmd+Enter -> the cs-write queue; Cmd+Shift+P toggle
  + terminal right-click entry. Brief: round-1-rich-prompt.md. @@LaneB builds
  the frontend; @@LaneA adds the WS `prompt` producer to the queue. DESIGN-
  FIRST: both post impl briefs before coding. E2E lights up when the queue lands.
- cs-write queue (@@LaneA, BUILDING): always-on per-session FIFO; ONE signal
  (agent-generating); serializes deliveries so queued msgs auto-submit after
  each other; two producers (control socket + Rich Prompt WS). Design:
  cs-write-queue-design.md (decisions from @@Host 10:40). RELEASED to build.
- blocklist (@@LaneC backend b7b1d2e2 MERGED; @@LaneD FB-settings UI BUILDING):
  per-workspace dir blocklist = global baseline + per-workspace additions;
  GET/PUT /api/index/excluded-dirs; off-loop re-walk. UI = autocomplete names.
- carousel-nav centering 6664e696 MERGED (DB1 follow-up).
- mermaid stream (@@LaneD) MERGED: cursor-based render, horizontal flip, up/down
  step-in, reverse-flip symmetry, visible selection in code blocks, error
  line/col locatability. + image-viewer prev/next 73bc874a.
- reports-on-by-default + actionable onboarding card 5172b4dd MERGED; preflight
  OK button fdd696e5. Manual @today/@date+mermaid aed5d5c8 + gateway guide-v2
  d2430270 MERGED. image-drag source-row indicator 9407070b MERGED.

REMAINING round-1 product builds: (1) @@LaneD blocklist FB UI, (2) @@LaneA
cs-write queue, (3) @@LaneA+@@LaneB Rich Prompt. Then a fresh :8787 rebuild for
@@Host validation, the RESPAWN, and round-close.

RESPAWN still PENDING @@Host's trigger (rebuild-team-server.sh; SPA-respawn per
window-id caveat; bootstrap resume-aware DONE). Best done after the in-flight
blocklist UI + queue + Rich Prompt land so the new binary has everything.

## 26 MERGED (wave-3 + post-close @@Host features) (round-1 main; tree clean, all lanes quiesced)
+ S1 4c7371bd + P2-card 35aa69e8. ALL LANES COMPLETE:
  A = C2/C3a/C3b/C1/S1/window-id (full cs lead-tooling channel)
  B = G1
  C = P1/P2-server/P2-card/P2+DT-design
  D = F1/F2/F3/F6/TW3/TW4/TW1/F4(1-5)/cross-window-DnD-fix
  E = D1/B1/D2/D3/2 story edits/node20-sweep(item6+gh-release)/item-1 guide
NOT pushed (full pre-push reserved for the push at @@Host's ask).

## RESPAWN (next step - @@Host triggers)
Ready: bootstrap resume-aware (DONE), rebuild-team-server.sh ready (make
install + SPA-respawn caveat). @@Host runs it from a NATIVE terminal; the
team re-spawns FROM THE SPA (not native, per window-id fix A). New @@Lead
resumes from this doc. POST-RESPAWN VERIFY (the things only-live-confirms):
- DnD fix: cross-window editor-tab drag actually moves (Tauri multi-window)
- P2 card: onboarding nudge renders + dismisses on workspace-open
- cs t sc / cs pane: @@Lead self-hosts on them (window-id now binds)
- general live smoke of F-series/TW1/graph-spine on the new binary
@@HOST OPEN (non-blocking): guide topology (kept host-binaries); gateway
default-port change for AirPlay (guide offsets dev-runner only).
WAVE-3: item6 12fadfcd + gh-release d829fb2e (CI) + window-id-fix-A 6a97cd6f +
F4-4-5 02b33639 + C3b de5dcbfd + DnD-fix 582db200 + item-1-guide dfbf3c57
(docs). @@LaneD COMPLETE (F4 + DnD bug). DnD fix = pane-id-collision (gate on
window identity) - @@Host verifies in desktop post-respawn. Item-1 guide done
(ports offset 17000+ for AirPlay, sdme-install added, container-names fixed) -
opened for @@Host final look.
ONLY 1 SLICE LEFT: S1 + attach-backfill (@@LaneA) and P2 nudge card (@@LaneC).
@@HOST OPEN: guide topology Q3 (kept host-binaries default), gateway default-
port change for AirPlay (guide offsets dev-runner only; service defaults
unchanged - separate gateway-code task if wanted). -> wave-3 close imminent.

C2 8b21edd9 | D1 78c4586b | B1 0910fed6 | D2/D3 0d6f9060 | G1 78df707d |
P1 770823fd | C1 01bf3252 | F-series 6100ec84 | TW1 e0f9755c | P2 64e4fc80 |
F4-step1 8e81146e | C3a c118bdc1 | taxes 3e94bd5b | F4-step2 3158383f |
F4-step3 682c9de1 | story-setup ba73ce5a. (16 total) NOT pushed (full pre-push reserved for the
push at @@Host's ask). Decisions doc: round-1-host-decisions.md (6 @@Alex
items; cs open blocked by no $CHAN_WINDOW_ID in agent context - @@Host opens
it). LEAD-TOOLING FINDING: agent terminals carry CHAN_TAB_NAME but NOT
CHAN_WINDOW_ID, so `cs open` / `cs survey` / likely `cs pane` can't target a
window from an agent context -> @@Lead self-hosting on cs pane is in question
(asked @@LaneA whether their e2e used a browser window vs agent terminal).

## Wave-2 in flight (now building on top)

```
slice    | lane    | files (disjoint-by-design)            | status
---------+---------+---------------------------------------+----------
C3a      | @@LaneA | wire/cli/window_bus/control_socket    | Rust found.
         |         | (own) + state.rs/lib.rs/routes/mod.rs |  going; SPA
         |         | +routes/window.rs (LaneC-seam GRANTED)|  seam pending
         |         | + store.svelte.ts/client.ts(LaneD seam)|  LaneD confirm
P2-server| @@LaneC | preflight.rs ONLY (option-1 snapshot  | greenlit,
         |         | summary); OFF the 3 plumbing files    |  parallel-safe
TW1      | @@LaneD | TeamDialog.svelte (mirrors C1 spawn)  | building
```

*BLOCK (00:09, RESOLVED 00:16): both paths worked - @@LaneC committed P2 via
pathspec (64e4fc80, preflight.rs only, verified zero @@LaneA bleed) AND
@@LaneA restored `cargo check -p chan-server` green. P2 isolate-gating now;
C3a remaining is SPA-only (no further shared-crate compile risk). Detail:
@@LaneA's C3a is half-applied -> chan-server won't
compile (E0061: control_socket::start 7-arg vs lib.rs:431 6-arg call site).
@@LaneC's P2 (code-complete in preflight.rs) shares the crate so can't gate.
LESSON: "disjoint files" does NOT isolate compile when SAME CRATE. Resolution:
(A) @@LaneC commits preflight.rs via pathspec now -> @@Lead isolate-gates the
COMMITTED state (no @@LaneA WIP present) -> unblocked; (B) @@LaneA restores
`cargo check -p chan-server` green. Recommended @@LaneC path A. For round-2:
serialize compile-breaking same-crate edits or use isolated dev worktrees.

Parallel-safety: C3a (LaneA) owns state.rs/lib.rs/routes/mod.rs; P2-server
(LaneC) is confined to preflight.rs -> disjoint files BUT same crate (compile-
coupled; see BLOCK above).
C3a SPA seam RESOLVED (00:06): @@LaneD confirmed TW1 touches neither client.ts
nor tabs.svelte.ts -> @@LaneA takes client.ts cleanly, tabs `layout` stable.
C3a fully unblocked. C3b exec ops + S1 follow C3a.

Part-2 commits (on top of the wave-1 batch), gating at HEAD 6100ec84:
- 6100ec84 F-series (@@LaneD)  F1/F2/F3/F6/TW3/TW4; browser-verified by lane
- 01bf3252 C1 (@@LaneA)        cwd-aware team load + load-spawns; live-verified
- 770823fd P1 (@@LaneC)        gate-green in isolation (770823fd run, fail=0)

C1 CONTRACT (for @@LaneD TW1): `cs terminal team load` now resolves dir cwd-
relative AND spawns (shared spawn_and_poke_team, lead-first + per-agent poke,
-N group-collision suffix). GUI load mirrors NEW's orchestration (read via
/api/team-config/read then run bootstrap), NOT read-only summary. /api/team-
config stays read-only; cwd resolution is CLI-side. -> routed to @@LaneD.

WAVE-1 BATCH GATE: GREEN at HEAD 78df707d (23:49). gate.sh isolated worktree
ran fmt+clippy(-D warnings)+test(--all-targets, all suites pass incl C2's 9
+ G1 invariant)+web-check+web-marketing-check -> fail=0 in ~5.5min. All five
committed slices (C2/D1/B1/D2/D3/G1) accepted into round-1 main. Gate target
dir now warm -> next gate runs incremental/fast. NOTE: NOT pushed; full
pre-push (adds gateway-build + build --no-default-features) runs only before
a push at @@Host's ask.

Batch on main (round-start bedaea64..HEAD), newest first:
- 78df707d G1 (@@LaneB)  Rust+web   -> in isolated build gate
- 0d6f9060 D2+D3 (@@LaneE) docs      -> ACCEPTED (review; link targets exist)
- 0910fed6 B1 (@@LaneE)  CI YAML     -> ACCEPTED (no half-bumps; majors API-
                                       verified: checkout v6.0.2/setup-node
                                       v6.4.0/upload v7.0.1/download v8.0.1)
- 78c4586b D1 (@@LaneE)  docs        -> ACCEPTED (reframe accurate to source,
                                       no em-dash, no marketing fluff)
- 8b21edd9 C2 (@@LaneA)  Rust        -> in isolated build gate (review clean,
                                       wire-tag pinned, lane scoped-green)

Gate harness: docs/journals/phase-16/round-1-team/gate.sh -> detached worktree
at /tmp/chan-gate-r1, dedicated CARGO_TARGET_DIR=/tmp/chan-gate-target, runs
fmt+clippy+test+web-check+web-marketing-check on the COMMITTED tree only (no
WIP contamination). gateway-build + build --no-default-features reserved for
the full pre-push before any push. Log: /tmp/chan-gate-r1.log.

C2 CONTRACT (recorded): `cs t sc --tab-name <NAME>` -> full replay ring as
raw PTY bytes to stdout, no trailing newline. Errors: 0=「no live terminal
session matched」, >1=「N live sessions match...」, missing tab-name=clap exit
2. Wire: ControlRequest::TermScrollback{tab_name} tag "term_scrollback".
Alias `cs t sc` avoids the `cs t s`/survey infer collision. Verified live by
@@LaneA; awaiting DONE/sha to gate+merge (CRITICAL PATH, gate first).

Shared-file watch: routes/mod.rs touched by @@LaneC (P1 new route). Keep
commit pathspecs tight so @@LaneA (control_socket/wire/terminal_sessions/cli)
and @@LaneC (preflight/mod.rs) splits don't cross-contaminate.

## Wave gating

- Track-0: @@LaneE D1/D2/D3 ship independently (no code overlap).
- Wave-1: A(C2->C3->S1,C1) B(G1) C(P1) D(small wins) E(B1) — all parallel now.
- Wave-2 (after wave-1 merges): F4, P2+DT1+DT2, G2(deferred->r2), TW1(needs C1), TW2.

## Merge queue / sequencing

- @@LaneA C2 first — @@Lead self-hosting blocks on it; gate ahead of others.
- DT1<->P2 (both @@LaneC): P2 before/with DT1 so no setting vanishes.
- TW1<->C1 (@@LaneD needs @@LaneA's C1 contract from event-lane-a.md).
- F6<->D1 (@@LaneD icon vs @@LaneE copy in web-marketing/): agree file split.

## Open survey items for @@Host

- [ESCALATED-INLINE 23:31] D1 production-setup docs depth: (a) gateway/README
  DNS+LE section only / (b) full gateway/docs/production-setup.md walkthrough /
  (c) defer deep-dive, ship reframe alone. +infra specifics (DNS provider, cert
  method dns-01-vs-http-01, nginx vhost layout). Reframe ships NOW regardless.
  NOTE: `cs terminal survey` (both --tab-name=@@Lead and --tab-group=
  phase-16-r1) returned "no live terminal session matched" — the survey needs a
  connected SPA window owning the tab; none matched. `cs terminal write` works
  (PTY-only). FINDING for lead-tooling: survey channel needs a live SPA window;
  C3 `cs pane` (being built by @@LaneA) would diagnose window/pane state.
  Escalated this decision to @@Host inline instead.
- [00:02] P2 onboarding card shape (wave-2 build): thin first-run NUDGE that
  points at Settings vs full inline Semantic/Reports TOGGLE pair. @@LaneC +
  @@Lead both lean thin nudge (avoids duplicating the Settings toggles).
  Non-blocking: @@LaneC's P2 server-summary slice is card-shape-independent.
  DT1/DT2 open Qs: RESOLVED by @@LaneC (00:20) - DT is "re-host not redesign"
  (inbound/outbound already in d.kind; New window already exists main.rs:1947;
  3 modes are relocated existing handlers). DT1=directional icon over URL
  badge; DT2=interaction-match in plain JS (launcher isn't Svelte). NO @@Host
  DT survey needed; only P2 card shape remains @@Lead-endorsed. Round-2 build.

## Follow-up candidates (backlog / round-2)

- CHAN-DESKTOP BUG (filed by @@Host 2026-06-01): Linux AppImage (CachyOS,
  Chan_0.23.0_amd64) renders white windows; console = `Could not create
  default EGL display: EGL_BAD_PARAMETER. Aborting...` x3. WebKitGTK DMABUF
  renderer init failure; likely fix `WEBKIT_DISABLE_DMABUF_RENDERER=1`
  (conditional, not blanket). Needs a real Linux GPU desktop to verify (sdme
  containers can't repro). Full record + diagnosis: desktop-linux-egl-bug.md.
  Owner-to-be: chan-desktop lane. Deferred (out of round-1 scope).

- bootstrap.md + docs/agents/{bootstrap,desktect}.md cite many dead phase-8/
  raw paths (raw/ deleted e747f1d2). @@LaneE keeps D2 narrow (architect.md:15,
  desktect.md:30) + D3 paragraph this round; broader citation rot = separate
  task. (raised by @@LaneE 23:28, accepted by @@Lead)
- DATE-BOUND 2026-06-16 (Node-20 deprecation): two actions are genuinely
  node20 JS and NOT bumped by B1 (riskier, deliberately out of scope), each
  verified via action.yml `runs.using`:
    * actions/deploy-pages@v4 (pages.yml, release.yml) -> node20; latest v5
    * apple-actions/import-codesign-certs@v3 (release-desktop.yml, release.yml
      SIGNING path) -> node20; latest v7 (4-major jump, signing-sensitive)
  These must bump before 06-16 or Pages-deploy + macOS signing break.
- NOT Node-20 (corrected by @@LaneE 23:46, @@Lead re-verified action.yml):
  actions/upload-pages-artifact@v3 is `using: composite` (no Node runtime) ->
  NOT forced by the deadline. Still bump it in LOCKSTEP with deploy-pages as
  the Pages-publish pair (latest v5), under a "keep the pair aligned" reason,
  not the Node-20 one. Swatinem/rust-cache@v2 is not a Node-20 GH action;
  fine as-is.
  -> DONE this round: item6 12fadfcd MERGED the node20 pair (deploy-pages v5 +
  upload-pages-artifact v5 lockstep + import-codesign-certs v7, verified inputs
  default to v3 behavior); + action-gh-release v2->v3 folding in. SIGNING
  DRY-RUN REQUIRED pre next tag (release-desktop.yml workflow_dispatch
  publish=false; v5 dropped -A / partition-list scoping).
- R2 FOLLOW-UP: attach-backfill (server binds registry window_id when an SPA
  window attaches to a windowless-spawned session). @@LaneA deferred (fix A +
  S1 + windowed-respawn cover the real workflow; backfill is partial value,
  doesn't touch the agent $CHAN_WINDOW_ID env which is fixed at spawn). + the
  cs pane --tab-name selector already shipped in C3b for the no-window-id case.
- KNOWN 06-16 EXPOSURE (unfixable now): mlugg/setup-zig@v2 is node20 and
  upstream has NO node24 release -> the musl cross-build (release.yml) stays
  node20 past the deadline. Pre-06-16 plan needed: wait for upstream node24 /
  swap the zig-setup action / inline zig. -> surface to @@Host.

## Contracts (cross-lane, recorded)

- [23:28 @@LaneE] F6/D1 web-marketing split: @@LaneE owns src/pages/{home,
  story}.html (copy); @@LaneD owns templates/base.html theme-toggle + site.js
  + styles.css (icon). No overlap. @@LaneE already poked @@LaneD.

## FYI to @@Host (no action)

- @@LaneC P1 smoke left a stray workspace-registry entry for a now-deleted
  /private/tmp/lanec-offws (held by a chan-desktop flock). Benign, self-
  reconciles when that desktop tab closes. @@Lead call: leave it (won't force-
  kill desktop). Mentioned only for awareness.

## To review (architect)

- docs/journals/phase-16/round-1-part-c-p2-dt-design.md (@@LaneC P2/DT1/DT2
  design: open-then-configure onboarding move, P2->DT1->DT2 sequence, cs card
  reuses PreflightOverlay mount). Wave-2 build; review before that wave opens.

## Log

- round opened; all 6 tabs live in group phase-16-r1; dispatched 5 lanes.
- 23:28 @@LaneE working: D1 reframe in progress, ASK queued, CONTRACT+NOTE in.
- 23:28 @@LaneA on C2 (terminal_sessions.rs, wire.rs); @@LaneD on F1/F3/TW4
  (Wysiwyg.svelte, TerminalTab.svelte, TeamDialog.svelte).
- 23:57 @@LaneD full `make pre-push` on clean HEAD 6100ec84 returned exit 0 (independent corroboration incl gateway-build + --no-default-features); my isolated gate is the authoritative confirm. @@LaneD now on TW1 (TeamDialog.svelte, stacked on 6100ec84).
- 00:06 C3a SPA seam CLEARED: @@LaneD confirmed TW1 touches neither client.ts nor tabs.svelte.ts; @@LaneA takes client.ts cleanly, tabs layout stable. C3a fully unblocked (Rust + LaneC plumbing seam + SPA seam all clear).
- 06:04 image-drag source-row indicator 9407070b MERGED (26). @@LaneD -> mermaid flip-to-render next. @@LaneE -> gateway guide-v2 (writing).

## Consolidated :8787 REBUILD (12:45, fresh - @@Host validating)
Binary /tmp/docsrv (renamed copy, built from HEAD 6d5c95eb, 09:40). Serving
/tmp/chan-mermaid-test, --standalone --port 8787. URL:
http://127.0.0.1:8787/?t=o3QesCoHjLJiewt2Ard7sbRAZE3iJpxh
Freshness VERIFIED by md5: served assets/index-DTZRgflv.js md5 c645ac2e... ==
local web/dist; contains Rich Prompt(2) + carousel-nav + excluded-dirs. (The
first curl|grep showed 0 Rich Prompt = PIPE TRUNCATION on the 1.5MB stream, not
a stale bundle - md5 + downloaded-file grep settled it. Lesson: anchor on md5.)
Batch in this binary: Rich Prompt (656a745a+6d5c95eb) + cs-write queue
(3d6d144e) + blocklist backend+UI (b7b1d2e2/989c3ef0/b6b33938) + carousel
centering (6664e696) + About-slide removal (a4ea43cd). NOT in: carousel-row
nit (queued @@LaneB) + terminal-page verify-last doc sections.
