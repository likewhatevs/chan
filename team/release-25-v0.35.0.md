# Phase 25 - chan-desktop acts as `chan`: one binary, empty-pane cleanup, a folded-in rich-prompt round

Status: closed - SHIPPED as `v0.35.0` (tag `1f15f8ec`, cut 2026-06-15; Alex hand-smoke passed after a show-stopper Cmd+R regression was fixed; publish run `27524598146` built/signed/published).
Span: 2026-06-14 → 2026-06-15.
Tags: #cli #desktop #packaging #upgrade #refactor #session #window #terminal #rich-prompt #cleanup #team

Round on the new-team-3 four-member team (Lead + LaneA, LaneB, LaneC), the third full round on the `cs terminal team` tooling. Scope of record was `dev/phase-25/plan.md` (steps 1-5), ratified with Alex before the team woke; the dispatch was `new-team-3/round-plan.md`. The plan's headline ask — **a chan-desktop install also gives you `chan`/`cs` on PATH, so there's nothing extra to download** — landed as a clean lib-extract + Personality fork. The release version was bumped mid-stream (`19d0c028`), and then a substantial second wave of work was folded into the same `v0.35.0` tag: standalone-window `cs terminal new`, the empty-pane window-save cleanup (step 5), and a self-contained rich-prompt enqueue/recall/reload round that grew out of an investigation done off the main scope. The tag points at the LAST of those commits, not the version-bump commit.

> **Repo note (2026-06-15):** the round's raw coordination dir `new-team-3/` (round-plan, per-lane tasks, append-only journals, the empirical checklist, the poke-path regression harness, the retest doc) is present on disk locally but is NOT git-tracked (`git ls-files new-team-3/` is empty), exactly as happened for `new-team-1/`/`new-team-2/` in phases 23-24. Paths under `new-team-3/...` referenced below do not resolve in-tree. This retrospective is the canonical in-repo record; the per-member feedback is reconstructed from those local journals + the shipped commits and is kept lean rather than fabricated.

## Roadmap (the asks)

From `dev/phase-25/plan.md`:

1. **Lib-extract the `chan` CLI** (step 1) — move the CLI surface out of `crates/chan/src/main.rs` into `crates/chan/src/lib.rs` as `pub async fn run(args, Personality) -> Result<()>`; the binary `main.rs` becomes a thin shim. `enum Personality { Standalone, Desktop }`. Pure refactor first.
2. **chan-desktop dispatches `chan`** (step 2) — `invoked_as_chan(argv0)` in chan-shell; chan-desktop, after the `cs` branch and before GUI init, runs `chan::run(args, Personality::Desktop)` when invoked as `chan`.
3. **Personality behaviour** (step 3) — `Standalone` `serve` is always browser (drop the desktop handoff AND the now-redundant `--standalone` flag); `Desktop` `serve` always desktop-integrates (hand off to a running desktop, else launch the GUI, never fall back to browser); `chan upgrade` on the Desktop binary delegates to `tauri-plugin-updater` instead of a CLI tarball replace.
4. **chan-desktop owns the `{chan,cs}` bin shims on boot** (step 4) — generalize `cs_install.rs` into a bin-shim installer owning `~/.local/bin/{chan,cs}` (symlink/wrapper per package kind, self-heal on boot), idempotent + marker + no-clobber; suppress the `csOffer` Preflight bubble under the desktop host.
5. **Don't save empty-pane windows** (step 5, independent of 1-4) — prevent (frontend deletes the session instead of writing a `null`/`treeExpanded`-only blob) + prune existing phantoms (backend treats an empty session blob as absent).

Lane map (`new-team-3/round-plan.md`): LaneA owned steps 1→2→3 (critical path, sequential — `crates/chan/**`, `crates/chan-shell/**`, `desktop/src-tauri/src/main.rs` + updater wiring); LaneB owned step 4 (`cs_install.rs`, `PreflightOverlay.svelte`); LaneC owned step 5 (fully independent — web store/client + `chan-server` routes + `chan-workspace`). The plan flagged that the macOS-signed-`.app`-as-CLI test, GUI-launch-from-CLI, AppImage shims, and the `tauri-plugin-updater` install **cannot run headless** — agents code + statically/unit gate; Alex runs the macOS/Linux empirical validation and cuts the release.

A second wave, not in the original five steps, was authorized mid-round and folded into the same release: standalone-window `cs terminal new`, and a self-contained **rich-prompt** round (enqueue recall + reload-survival) that an off-scope investigation (`new-team-3/rich-prompt-enqueue-findings.md`) recommended as a dedicated follow-up rather than bolting onto the load-bearing poke FIFO.

## What shipped

All commits below are on `main`, in `v0.34.0..v0.35.0` (v0.34.0 = `1ec3bada`, v0.35.0 = `1f15f8ec`).

**Desktop-as-`chan` (steps 1-4, LaneA + LaneB), the pre-bump wave:**

- **Lib-extract** (`e2377b24`, LaneA): the entire CLI surface moved `crates/chan/src/main.rs` → `crates/chan/src/lib.rs` (+3252/-3251, a near-pure move). `pub async fn run(args, Personality)` + `pub enum Personality { Standalone, Desktop }` are the new public seam (verified in the shipped `lib.rs`: `run` at the documented signature, the enum with both variants). The binary's `main.rs` is now a thin shim.
- **chan-desktop dispatches `chan` in-process** (`6133e17d`, LaneA): `invoked_as_chan(argv0)` added to chan-shell (file-stem == "chan", same rule as `invoked_as_cs`); chan-desktop gains a `chan` dependency and, after the `cs` branch and before GUI init, calls `chan::run(.., Personality::Desktop)`.
- **AppImage argv0 dispatch** (`4fa5df70`, LaneA): the cs/chan stem detection prefers `$ARGV0` over `argv[0]` so a type-2 AppImage `exec -a chan/cs $APPIMAGE` recovers the intended name (the AppImage runtime overrides `argv[0]` to the mount path). chan-shell's `lib.rs` carries the shared file-stem helper.
- **Desktop `serve` launches the GUI when none is running** (`54fd52e2`, LaneA): `crates/chan/src/lib.rs` (+177/-20) — the `Desktop` personality hands off to a running desktop, else launches the GUI and opens the workspace, never falling back to browser. (The matching handoff plumbing also touched `crates/chan-server/src/handoff.rs`, +415/-92, in `0da8734b`.)
- **Desktop `chan upgrade` drives `tauri-plugin-updater`** (`0da8734b`, LaneA, step 3b): the thin updater trigger is wired so the Desktop binary's `upgrade` delegates to the running desktop's updater (check/download/install) instead of a CLI tarball replace; `desktop/src-tauri/src/main.rs` + `cs_install.rs` + `handoff.rs` updated.
- **chan-desktop owns `~/.local/bin/{chan,cs}` on boot** (`4535955c`, LaneB): `cs_install.rs` (+361/-94) generalized from an AppImage-only `cs` wrapper into a `{chan,cs}` bin-shim installer (symlink per `.app`/deb/rpm, wrapper per AppImage, self-heal on boot, marker-guarded, no-clobber).
- **Suppress the `csOffer` Preflight bubble under the desktop host** (`7bcf2430`, LaneB): `PreflightOverlay.svelte` no longer offers to install `cs` when the desktop already owns the shims (+ a new vitest).
- **Docs**: `0ef06dc4`, `023712b9`, `da711e77` (desktop `design.md` §1/§9 "desktop IS `chan`" + the 3b upgrade narrative; `docs/contributing/linux-and-macos.md` "cs_install owns the {chan,cs} bin shims").

**Empty-pane cleanup (step 5, LaneC), the post-bump wave:**

- **Don't save empty-pane windows; prune existing phantoms** (`9862dfa1`): frontend (`store.svelte.ts`, `client.ts`) DELETEs an empty window's session instead of writing a `null`/`treeExpanded`-only blob; backend (`chan-workspace/src/workspace.rs`, +102) treats an empty blob as absent and GCs existing phantoms on open.
- **Treat terminal-only windows as ephemeral (not saved)** (`aca8ca03`): the prune was widened to the terminal-only case — a window whose only content is terminals has no durable content (the PTY dies on restart; a saved `tsid` just respawns a fresh shell), so it is not `saved`. `workspace.rs` `prune_empty_sessions` GCs both pre-fix phantoms and the terminal-only case (verified in the shipped `workspace.rs`, the documented `prune_empty_sessions` + the "phantom saved window with nothing in it" GC comment).
- **Auto-remove an explicitly-deleted terminal's tab** (`4ee85e1f`): deleting a terminal removes its tab live; `TerminalTab.svelte` + `tabs.svelte.ts`.

**Standalone-window `cs terminal new` (post-bump):**

- **`cs terminal new` works on standalone terminal windows** (`33fbb31f`): `crates/chan-server/src/control_socket.rs` (+119/-12) — a launcher (non-workspace) terminal window now accepts `cs t n` (previously refused); `--path` without a workspace root is cleanly rejected.

**Rich-prompt enqueue / recall / reload round (post-bump, LaneA server + LaneC frontend):**

- **Server: cancel-prompt + `queued_prompt_ids`** (`dd138efb`, LaneA): `Session::cancel_prompt(id)` = `write_queue.retain(prompt_id != id)` under the queue lock (cancel-vs-drain resolved atomically), `queued_prompt_ids()` for the reload contract, a `ClientFrame::CancelPrompt` arm + serde-pinned ack; +192/-1, additive, the shared poke FIFO contract untouched, 3 new in-file cancel-atomicity tests.
- **Frontend: recall a queued message + survive a window reload** (`729d9df1`, LaneC): ArrowUp-at-doc-start recalls a queued message to edit (via the existing draft, no server text-peek); a reloaded window re-shows the locked queued state with its position. `RichPrompt.svelte` + `TerminalTab.svelte` + `tabs.svelte.ts`.
- **Queued state reads as a read-only card** (`1f15f8ec`, LaneC, the tag commit): a queued message presents as a read-only card (`queued (#N) · ↑ edit · esc cancel`, caret hidden), ↑ recalls from anywhere, Esc cancels (dequeue + drop), both through the one `cancel-prompt` wire.
- **Survey overlay: wider 5% border + honor body newlines** (`aca4c39d`, LaneC): `BubbleOverlay.svelte` + `markdown.ts`.

**Cmd+R reload regression fixes (post-bump — the show-stopper arc, see Verification):**

- **Reattach all-terminal windows on Cmd+R** (`4471a37d`, LaneC): a per-window **sessionStorage** reload snapshot (layout + tsids) so Cmd+R re-grafts the existing PTYs instead of spawning fresh ones, while the durable blob stays absent (step-5's no-phantom property holds). +90/-3 in `store.svelte.ts` + tests.
- **Stop the stray-PTY leak from the all-terminal reload snapshot** (`04363aa4`, LaneC): canonical snapshot key + skip tsid-less terminal snapshots, killing an orphan PTY found on busy-reload.
- **Don't re-answer historical queries during reattach replay (CPR leak)** (`36fcbab5`, LaneC): `TerminalTab.svelte` suppresses historical cursor-position-report replies during the reattach replay.

**Docs hygiene:**

- **Unwrap hard-wrapped prose in `.md` files** (`c2a5d10f`): ~80-col reflow across 109 files (tables/code/lists preserved), -8400 net.

**Release mechanics:**

- `19d0c028` (the version-bump commit, mid-timeline): `Cargo.toml`/`Cargo.lock`, `tauri.conf.json`, `gateway/*`, `web/package.json` → 0.35.0. `1bd3e1a5` synced `web/package-lock.json` (missed in the bump). The tag `v0.35.0` is annotated and points at `1f15f8ec`, after all the post-bump work.

## Verification

- **Gate model:** Lead owned the isolated full-tree `make pre-push` via a separate gate worktree (gates the committed state, immune to peers' WIP); lanes reported scoped own-gate-green + pathspec sha with real flags (`RUSTFLAGS="-D warnings"`, `make web-check`). LaneA's server lane: 433 chan-server tests incl. the 3 new cancel/queued tests, chan-desktop builds. Frontend lanes ended green at 1778 vitest.
- **Poke-path regression harness** (LaneB, `new-team-3/poke-path-regression.py`): a real-binary driver (the queue internals are private modules) asserting the shared `cs terminal write` poke path is unaffected by the rich-prompt `CancelPrompt` work — baseline 5/5 green, re-run with the cancel-interleave assertion against `dd138efb` unchanged. Ack is on STDERR (a harness wording fix, no code impact); headless test server = `CHAN_NO_DESKTOP_HANDOFF=1 chan serve --port P <ws>` (the removed `--standalone` no longer exists).
- **Empirical deferral, as planned:** the macOS-signed-`.app`-as-CLI exec, GUI-launch-from-CLI, AppImage shims, and the `tauri-plugin-updater` install were code-complete + gated and deferred to Alex on a real signed/notarized desktop (`new-team-3/phase-25-empirical-checklist.md`).
- **Alex hand-smoke #2 on the first signed dry-run** (`v0.35.0-retest.md`): **A. empty-pane (step 5) "it works"; B. `cs terminal new` on standalone windows "it works"; C. rich-prompt reload contract — SHOW-STOPPER REGRESSION.** "Whenever I hit Cmd+R the terminal session restarts entirely... no longer keeping up with the session." Cmd+R spawned a fresh PTY instead of reattaching — breaking the very window-state-survives-reload contract the round was meant to uphold. **The round did NOT tag here.**
- **Root cause (bisected by LaneB, NOT the rich-prompt work):** step-5's `aca8ca03` made `serializeSession()` return null for an all-terminal window → no on-disk blob → the reload tsid-graft starved → fresh PTY. The server PTY survived; `729d9df1` only EXPOSED it. LaneB had OBSERVED the symptom in a browser smoke and mis-attributed it to vite-dev timing — the round's central miss (see lowlights). Fixed by `4471a37d` (sessionStorage snapshot) + the `04363aa4` leak fix + the `36fcbab5` CPR fix, all e2e-verified by LaneB before re-dry-run; folded in for one clean build (no-known-bug bar).
- **Release:** re-pushed, fresh dry-run green, Alex re-tested the reload contract → PASS → "carry on with the release" (the verbatim go). Pushed `main` (`1ec3bada..1f15f8ec`, gate green, verified via `git ls-remote`), tagged + pushed `v0.35.0` @ `1f15f8ec`, publish run `27524598146` (full matrix → macOS sign/notarize → GitHub Release → Pages `/dl`; self-upgrade data-driven from `/dl/latest.json`).

## Retrospective

**Highlights:**
- The lib-extract was a genuinely clean seam: a ~3250-line near-pure move (`e2377b24`) gave the whole desktop-as-`chan` story one public entry point (`chan::run(args, Personality)`) with exactly one behavioural fork, gated green as a pure refactor before any dispatch landed — the plan's "land the refactor first" sequencing held.
- **Empirical testing caught what gates and browser smokes could not, AGAIN.** The show-stopper Cmd+R reattach regression was invisible to every headless surface and only surfaced on Alex's real signed desktop. The round honored its own "NOT tagging" discipline rather than shipping a known break.
- The fix converged peer-to-peer without a host design call: LaneC chose the sessionStorage snapshot over LaneB's URL-hash idea (the hash is shareable → tsid leak + live-PTY hijack), the lanes reconciled, and LaneB e2e-verified.
- The off-scope rich-prompt investigation correctly recommended a *dedicated* sub-round rather than bolting recall/cancel onto the load-bearing poke FIFO — and the server half (`dd138efb`) landed additive with the shared poke path provably untouched (harness 5/5).

**Lowlights / lessons:**
- **The central miss: a lane-smoke anomaly that rationalized away the observed symptom.** LaneB saw "Cmd+R spawned a new terminal session" in a browser smoke and filed it as vite-dev timing / "not a defect." It was a real, real-desktop-confirmed regression. Lesson: an anomaly that explains itself away needs escalation, not dismissal.
- The regression was a cross-lane coupling the lane table did not anticipate: step-5's null-serialize (LaneC's prevention half) and the reload tsid-graft were the same on-disk blob, so the "fully independent" step-5 lane silently broke a reload contract another change exercised. Independent file ownership did not isolate the behaviour.
- Release-bump-before-done: the version pins landed mid-stream (`19d0c028`), so the tag floats far above the bump commit and a reader bisecting by the release commit would miss most of what shipped. Recorded here so the tag (`1f15f8ec`), not the bump, is the reference.
- Headless can't prove rich-prompt's live ↑/Esc + read-only card rendering — MCP Chrome's no-OS-focus quirk left per-message `pending` untracked, so the recall gate correctly didn't fire under a programmatic `cm.focus()`. The visual/interaction set rode Alex's real-desktop pass, flagged up front.

**Honest feedback, per member** (reconstructed from the local `new-team-3/` journals; the raw bus is not in-tree):
- **LaneA**: the critical-path lane, executed exactly to the sequencing — pure lib-extract first, then dispatch, then Personality + the updater delegation, each gated. The server half of the rich-prompt round (`dd138efb`) was additive with a serde-pinned contract and self-reviewed for the cancel-vs-drain race under the lock; the poke FIFO it shares stayed untouched.
- **LaneB**: built the real-binary poke-path regression harness (the round's reusable safety net) and did the bisect that pinned the show-stopper to step-5, not rich-prompt. The same lane also owned the round's worst miss — observing the regression and dismissing it as dev-timing. Net: strong investigation instinct, with the lesson that a smoke anomaly is an escalation, not a footnote.
- **LaneC**: the largest surface — step-5 prevention+prune, the rich-prompt frontend, and then all three reload-regression fixes (reattach, stray-PTY leak, CPR leak) plus the survey overlay. Owned the fallout of its own step-5 null-serialize cleanly: root-caused, picked the safer snapshot mechanism, and got each fix e2e-verified before fold-in.
- **Lead**: held the isolated gate and the empirical sequencing, and crucially did NOT tag on the failed hand-smoke — ran the fix→re-push→re-dry-run→re-test loop to a clean build instead. Mis-routings were minor; the discipline that mattered (no-known-bug bar, empirical-before-tag) held.
- **Alex**: the mid-round additions (standalone `cs terminal`, the rich-prompt round) were well-scoped, and the hand-smoke caught the one regression no automated surface could. The show-stopper call was decisive and correct.

**Carryover:**
- The `tauri-plugin-updater` install/relaunch, AppImage shim self-heal, and GUI-launch-from-CLI desktop items were code-complete + gated but only partially empirically exercised at the deadline (the optional §D of `v0.35.0-retest.md`); confirm on the next desktop install cycle.
- deb/rpm desktop vs the standalone `chan` deb both wanting `/usr/bin/chan` — PATH precedence/conflict layering (`~/.local/bin` user shim vs `/usr/bin`) is documented but not stress-tested across both package kinds on one box.
- Rich-prompt v2 ideas from the investigation (durable pending ids, skip-fail-when-terminal) remain future work; the live ↑/Esc + read-only card visual rides Alex's real-desktop confirmation.
- Lane tables should flag shared-behaviour coupling (the step-5 null-serialize ↔ reload-graft blob) even when file ownership doesn't overlap.

## Notes

- v0.34.0 = `1ec3bada`; v0.35.0 = `1f15f8ec` (annotated tag). The version-bump commit `19d0c028` sits mid-timeline (work continued and folded into the same tag). There is no `CHANGELOG.md` entry for 0.35.0 — this project's changelog is the annotated tag message (`git show v0.35.0`).
- The `--standalone` `serve` flag was removed in this phase (clap now rejects it); the headless/handoff-free path is `CHAN_NO_DESKTOP_HANDOFF=1 chan serve`.
- The bundle-id and one-time keychain/TCC implications of treating the desktop binary as `chan` ride the same desktop identity from phase 23's `app.chan.desktop` rename; no new identity change here.
- The round's coordination bus (`new-team-3/`: round-plan, tasks, journals, empirical checklist, the poke-path harness, retest doc) is local-only and not committed, consistent with phases 23-24. This report is the canonical record.
