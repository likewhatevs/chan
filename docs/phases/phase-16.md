# Phase 16 - lead tooling, host feature stream, desktop launcher redesign

Status: closed
Span: 2026-06-01 to 2026-06-02 (round-1 opened the evening of 06-01 and cut
      v0.24.0 mid-day 06-02; the launcher-redesign carryover landed the
      evening of 06-02). Based on git author dates and dated journal headers.
Versions: v0.24.0 (cut 2026-06-02, continuing phase-15's v0.23.0)
Tags: #features #bugfixes #cli #terminal #graph #desktop #docs #ci #release

## Roadmap (the asks)

Phase 16 ran one structured core round followed by a long live feature
stream, both from @@Alex (@@Host). The asks grouped into five areas.

**CLI / lead tooling.** Add `cs terminal scrollback` (read a tab's
scrollback by name, no group support) and `cs pane` (query windows/panes/
layout/selected pane; set focus; split left/bottom; close tab/all-tabs/pane;
`--force` to kill draft+terminal tabs that otherwise block). Make the @@Lead
orchestration process self-hosting on these. Fix `cs terminal team load` to
resolve paths cwd-relative and actually spawn the team, not just summarize.

**Graph.** When plotting from a lens (lang=x, hashtags, mentions), draw the
directory spine all the way back to the workspace root so no file node is
edgeless. Mirror the dashboard/search graph spine. Also: gradually load
nodes/edges on large workspaces (e.g. a shallow Linux-kernel clone) instead
of one big up-front plot, starting from the spine and loading directories on
demand.

**Pre-flight and onboarding.** Check the `cs` symlink exists in `$PATH` at
workspace boot/pre-flight (both chan and desktop), offer to create/fix it,
and continue without blocking if it cannot be created. Show a first-load
onboarding card after opening a workspace.

**Editor / UX fixes (context-menu review).** Review every hamburger and
context menu; make right-click contextual (text-selected -> Copy/Cut/Paste +
contextual entries, not the full tab menu); add an external-link "open"
affordance and internal markdown-link previews; inspector section separators;
move the terminal "Broadcast input" menu section to the top.

**Tunnel / gateway docs reframe.** Correct the website/README messaging that
sold the online service as the feature. The tunnel is a CORE chan capability;
the online service (the `gateway/` server-side counterpart for
`--tunnel-url` / `--tunnel-token`) is experimental, off by default, and meant
as a self-hosted "your own Google Drive" you run in your own infra. Document
the admin tools, OAuth/user enrollment, and a DNS-wildcard + Let's Encrypt
self-host guide (generic, "choose your provider", lima-vm + sdme for Mac).

**CI date-bound bump.** Node-20 GitHub Actions deprecate after 2026-06-16;
bump the affected actions before then.

**Desktop launcher redesign (carryover).** Merge the separate
[Open workspace] and [Attach] header buttons into one [New] modal with three
choices (Local directory / Remote outbound / Remote inbound). This carried
over and finished after the v0.24.0 cut.

## Rounds and waves

### Round 1: core dispatch + host feature stream (v0.24.0)

The round opened with a structured wave plan, then expanded into a live
@@Host-driven feature stream that became the bulk of the work.

- **Track-0 (independent, shipped first):** @@LaneE landed the tunnel/gateway
  messaging reframe (D1) plus doc fixes (D2/D3) with no code overlap, which
  unblocked the rest.
- **Wave 1 (lead tooling + small wins, parallel):** @@LaneA built C2
  (`cs terminal scrollback`) then C3 (`cs pane`, including a new bidirectional
  control-socket channel, since the socket was one-way push only) then S1
  (SPA-visible CLI team spawn), with C1 (`cs terminal team load` fix)
  alongside. @@LaneB landed G1 (graph dir-spine on the lang/tag/mention
  lenses). @@LaneC did P1 (the non-blocking `cs`-symlink pre-flight check).
  @@LaneD landed the F-series small wins (F1/F2/F3/F6 menu/inspector/theme
  fixes, TW3/TW4). @@LaneE did B1 (the Node-20 CI bump).
- **Wave 2:** F4 (context-menu overhaul, design-first), P2 + DT1/DT2 (the
  coupled onboarding-card-into-SPA and launcher settings move), TW1 (the Team
  Work load dialog, mirroring @@LaneA's C1 contract), and a window-id fix
  (agent terminals carried `CHAN_TAB_NAME` but not `CHAN_WINDOW_ID`, so
  `cs pane`/`cs open`/`cs survey` could not target a window from an agent
  context).
- **Host feature stream (post wave-3, the bulk):** a long live stream of
  features and polish, all converging on one architectural principle (every
  terminal/agent input flows through a single serialized per-session queue).
  See "What shipped" for the concrete list. The round cut v0.24.0 mid-day on
  06-02 after a unified version bump, a full pre-push gate across all
  workspaces, and a publish=false signing dry-run.

About 26 core slices merged before the feature stream, then the feature
stream stacked on top. @@Lead serialized every merge with tight pathspecs and
re-gated through an isolated worktree after each.

### Closing round: desktop launcher redesign (after the v0.24.0 cut)

The launcher redesign (DT1/DT2 in the original plan) carried over and was
finished the evening of 06-02 by a fresh four-lane team (@@LaneA as lead +
@@LaneB/C/D), after v0.24.0 had already been cut. It explicitly belongs to
phase 16. The locked design (decisions D1-D4):

- **D2 = MODAL:** [New] opens an in-launcher overlay (not a new OS window)
  with a Team-Work-style segmented switch of three bodies: Local directory /
  Remote outbound / Remote inbound. ESC / backdrop / [X] dismiss; dismiss
  never stops a live inbound listener.
- **D3 = connection dot:** remote (URL) rows show a static dot in the On cell
  (green when connected, grey otherwise); the url/tunnel text tags were
  dropped and inbound-vs-outbound direction moved to a Where-column icon.
- **D4 = drop the tagline:** the italic "what are we working on today?" header
  line and its CSS were removed; header is now enso + "Workspaces" + [New] +
  theme toggle.
- **D1 = keep add-time toggles:** Semantic search + Reports toggles stay in
  the Local choice (creation-time selection avoids a wasteful re-index); the
  per-row settings gear was removed (its settings already live in the SPA, per
  a gap analysis that found no missing surface).

The redesign was built and committed (commit fd27d29d). @@Alex hand-smoked it
in the macOS WKWebView build and left three change requests on the smoke
checklist (swap the header icon/[New] order; add a code-block example to the
Remote-outbound body; rewrite the Remote-inbound copy). Those three follow-ups
were carried into phase 17 round-1, not redone in this round.

## Team and coordination

This phase introduced a DEDICATED architect/lead that writes no product code.
See ../agents/README.md for the roster; the lane handles were positional.

```
handle    role this phase                              card
--------  -------------------------------------------  -----------
@@Lead    architect; plan, dispatch, merge/gate        architect.md
          serialization, agent lifecycle via cs
          terminal, surveys to @@Host, retrospective
@@LaneA   CLI/terminal + lead tooling (C2/C3/S1/C1,    (no card)
          cs-write queue, window-id fix)
@@LaneB   Graph spine (G1); Rich Prompt frontend;      (no card)
          Team Work decouple/delete
@@LaneC   pre-flight + desktop (P1/P2/DT design);      (no card)
          graph inspector + About-slide
@@LaneD   frontend/UX + Team Work GUI (F-series,       (no card)
          TW1, mermaid, dashboard, blocklist UI)
@@LaneE   docs + CI + bootstrap (D1 reframe, B1 CI     (no card)
          bump, gateway guide, terminal manual page)
@@Alex    human owner (@@Host); drives the feature     (human owner)
          stream, answers surveys, hand-smokes desktop
```

Coordination scheme this phase: on-disk task/lane files plus append-only
directional event channels (`event-lane-<x>.md`, `event-lead.md`), all in the
main checkout, with the team itself running as a `cs terminal` Team Work
group. @@Lead managed agent lifecycle with `cs terminal` (add / poke / `/clear`
recycle / restart) and was the ONLY one to run `cs terminal survey` to
@@Host; other lanes wrote questions to their event files for the lead to
consolidate. Every merge went back to round-1 main by pathspec and was
re-gated.

The headline scheme refinement was the ISOLATED GATE: @@Lead ran the full
`make pre-push` gate in a detached worktree (`/tmp/chan-gate-r1`) with a
dedicated `CARGO_TARGET_DIR`, gating only the COMMITTED tree so concurrent
lane WIP could not contaminate or false-red the gate. Lanes reported a scoped
own-gate-green plus a pathspec sha; the isolate-gate was the authoritative
confirm. The full gateway-build + `--no-default-features` pass was reserved
for the single push at @@Host's explicit go.

A bootstrapping caveat shaped wave ordering: @@Lead's own tooling
(`cs terminal scrollback` = C2, `cs pane` = C3) was being BUILT by @@LaneA in
wave 1, so until those merged the lead read agent state via SPA tabs and the
event files. C2 was gated first precisely so the lead process became
self-hosting sooner.

Pokes were 1-line pointers to on-disk files (the lean poke bus), each ending
with the CK-SUBMIT chord so it auto-submitted rather than parking in the
target's compose box. Context lived in the task/design files, not the pokes.

## What shipped, tried, and undone

**Shipped (v0.24.0):**

- CLI lead tooling: `cs terminal scrollback`, `cs pane` (with a new
  bidirectional control-socket channel), S1 SPA-visible team spawn, the
  `cs terminal team load` cwd-resolve + spawn fix, and the agent-session
  window-id binding fix (plus a `cs pane --tab-name` selector for the
  no-window-id case).
- The cs-write QUEUE: an always-on per-session FIFO with idle-drain that
  serializes ALL terminal/agent input (control-socket writes, Rich Prompt,
  Team Work). This became the unifying architecture of the round.
- Graph dir-spine (G1) on the lang/tag/mention lenses, so lens plots draw the
  full spine back to the root and leave no edgeless file node.
- Pre-flight `cs`-symlink check (P1, non-blocking) and the P2 first-load
  onboarding nudge card (a thin nudge pointing at Settings, chosen over
  duplicating the Settings toggles).
- Mermaid stream: cursor-based render (no flip button), horizontal flip,
  up/down step-in, reverse-flip symmetry, visible selection inside code
  blocks, and error line/col locatability.
- Image-viewer prev/next; image-drag source-row indicator.
- Reports on by default + an actionable onboarding card; preflight OK button.
- Dashboard: carousel navigator, real-engine screensaver preview, carousel-nav
  centering, carousel moved into the OK footer row, screensaver preview shown
  inside the Screen-lock box and only when locked.
- Per-workspace directory blocklist: backend (global baseline + per-workspace
  additions, union filter, off-loop re-walk) plus a file-browser settings UI.
- Rich Prompt (a returning feature): a floating Cmd+Shift+P bubble with
  markdown-list continuation, then re-architected to be Drafts-backed with
  editor-style image paste.
- Team Work decoupled from regular terminals, then the in-terminal bubble
  deleted entirely (the lead becomes a normal terminal; identity flows through
  the queue).
- Tunnel/gateway docs reframe (D1), a gateway self-host guide-v2, the
  terminal manual page (`terminal.md`: the `cs` family, pokes, survey, MCP),
  graph Drafts-node inspector fix, path-COPY switched to the code-block icon,
  About-slide motif/pitch removal, and the Node-20 CI bump (B1 plus the
  date-bound deploy-pages v5 / import-codesign-certs v7 follow-up).

**Shipped (after the v0.24.0 cut, closing round):** the desktop launcher
redesign (commit fd27d29d) merging Open+Attach into the [New] modal.

**Tried then corrected:**

- The Team Work scope was reframed roughly four times live (whole-GUI-delete
  -> bubble-only -> preserve+decouple+tie-to-lead -> full-delete + lead-is-
  normal-terminal). No code was wasted because @@LaneB classified the surface
  read-only before any deletion, but it cost real dispatch turbulence.
- Rich Prompt's image paste started as a base64/per-agent-path approach, found
  to be not agent-consumable, and was reframed to Drafts-backed real files any
  agent reads via MCP/disk.

**Deliberately not done / deferred:**

- "Survey through the queue" was investigated and closed as a no-op (the
  survey channel was already isolated), avoiding a survey-breaking redesign.
- G2 (incremental large-workspace graph load) was deferred to a later round.
- I1 (magic file-type detect + pending-index state) and I2 (Metal GPU hang)
  were deferred as Mac + large-workspace bound.
- A chan-desktop Linux bug was filed and deferred: the released AppImage
  renders plain white on some Linux GPU/driver/compositor combinations with
  `EGL_BAD_PARAMETER` (WebKitGTK DMABUF renderer init failure). The likely fix
  is a conditional `WEBKIT_DISABLE_DMABUF_RENDERER=1`, not a blanket disable;
  acceptance needs a real Linux GPU desktop (headless aarch64 sdme containers
  cannot reproduce it).
- The launcher redesign's three @@Alex change requests carried into phase 17.

## Retrospective

This is the high-value payload, taken from the round-1 retrospective.

**Highlights:**

- One unifying principle emerged and held: EVERY input to a terminal/agent
  (`cs terminal write`, Rich Prompt, Team Work, and the question of survey
  replies) flows through ONE serialized per-session queue. The sprawling
  feature set converged on a clean architecture instead of a pile of one-offs.
- The Drafts-backed Rich Prompt reframe was the best design win of the round.
  It dissolved the image-paste problem: instead of base64/per-agent-path
  hacks, pasted images are real files in a Drafts folder any agent reads via
  MCP/disk. It came from grounding the premise, not from building it.
- Design-first repeatedly prevented bad builds: @@LaneA's survey finding
  (already isolated -> no-op), @@LaneB's image finding (base64 not agent-
  consumable -> the Drafts reframe), and the read-only Team Work
  classification that absorbed every re-scope without wasting code.
- Gate hygiene caught real stale tests (blocklist + carousel source-pin tests)
  via the isolate-gate (committed-state, immune to peer WIP) plus the full
  vitest run.
- An md5-anchored freshness check on the rebuilt :8787 test server caught a
  false "stale bundle" alarm (a pipe truncation on a 1.5MB stream),
  preventing a confabulated bug report. The lesson: anchor freshness on md5,
  not on a piped grep.

**Lowlights / contention:**

- The Team Work scope churned about four times. It cost real cycles and
  several @@LaneB holds. No wasted code (read-only classify first), but a lot
  of dispatch turbulence that a model-lock before dispatch would have avoided.
- @@LaneD's own-gate skipped vitest twice (blocklist + carousel), both caught
  by the isolate-gate. Own-gates must run the full `make web-check`, not a
  subset.
- A leftover lane test server on a stray port caused @@Host a false "unknown
  variant prompt" bug; multiple concurrent test servers created avoidable
  confusion. The metadata-id in the terminal path is the tell for which
  server/workspace is in front of you.
- The feature-stream-vs-structured-dispatch tension: the structured wave plan
  opened cleanly, but the long live host feature stream that followed was
  where the bulk of the work and most of the churn lived. Real-time
  exploration is productive, but it strains the wave model.

**Constructive feedback / lessons that generalize:**

- Design-first for anything with a premise risk was the round's MVP. Keep it
  the default for "this should work seamlessly / everywhere" asks; the premise
  is often where the bug is.
- The isolate-gate (committed-state, peer-WIP-immune) is non-negotiable with
  multiple same-area lanes. Pair it with a standing rule that own-gates run
  the full web-check.
- When the host's model is still moving, lock it with one explicit
  restatement + confirm before dispatching, rather than re-scoping reactively
  message by message. The architect over-committed to each interpretation of
  Team Work instead of surfacing the instability early and asking one crisp
  either/or.
- "Disjoint files" does NOT isolate the build when lanes share a CRATE: a
  half-applied signature change in one lane breaks the whole crate's
  `cargo check` for its same-crate peers. Make the signature plus all call
  sites in one burst, or serialize compile-breaking same-crate edits, or use
  isolated dev worktrees.
- The gate -> rebuild -> md5-verify pipeline held, and the lean-poke-bus plus
  append-only event channels kept five lanes collision-free across a very
  long, mutating feature stream. A recurring operational slip to kill:
  launching watchers/gates with `&`/disown out of habit instead of the
  required backgrounded runner, which leaves them untracked.

## Notes

Terminology drift, for mapping old names to current ones:

- "Rich Prompt" was the floating Cmd+Shift+P compose bubble over the terminal;
  by the end of the round its Team Work counterpart (the in-terminal lead
  bubble) was deleted and the surface settled toward what later phases call
  Team Work. A new reader will see both names in the source for the same
  family of ideas.
- "drive" / "workspace" both appear; this phase used "workspace" as the
  settled term for the chan root directory on disk. "folder" in launcher copy
  maps to "directory" elsewhere.
- The `cs` CLI and its wire/control-socket types live in the `chan-shell`
  crate (referenced in lane files as `crates/chan-shell/src/{cli,wire,lib}.rs`).
- "gateway" / "online service" is the experimental self-hosted server-side
  counterpart in `gateway/`, distinct from the always-core tunnel transport.

The raw working material (per-lane files, the append-only event channels, the
round-1 plan / requirements / status / host-decisions docs, the design briefs
for each slice, the launcher-redesign design and smoke checklist with @@Alex's
inline change requests, and the round-1 retrospective) is preserved in git
history under docs/journals/phase-16/ and docs/journals/round-16/; that tree
was removed from the working tree in the docs cleanup.
