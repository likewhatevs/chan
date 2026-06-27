# Phase 17 - host bug sweep, survey v2, desktop connecting screen (v0.25.0)

Status: closed
Span: 2026-06-02 to 2026-06-03 (opened the evening of 06-02 after the phase-16 v0.24.0 cut; cut v0.25.0 the afternoon of 06-03). Based on git author dates and dated journal headers.
Versions: v0.25.0 (cut 2026-06-03, continuing phase-16's v0.24.0)
Tags: #bugfixes #features #terminal #editor #graph #desktop #survey #docs #ci #release

## Roadmap (the asks)

Phase 17 ran on two successive @@Alex reports against the live IDE, the first a broad bug-and-enhancement sweep, the second a short follow-up that expanded into a desktop and release wave. The launcher redesign that finished just before this phase is documented in phase-16; phase 17 only folds in its three smoke follow-ups (the launcher copy items S1/S2/S3).

**Round-1 bugs.** The Cmd+Shift+P rich-prompt chord toggled the compose bubble on EVERY terminal and stole focus, instead of acting only on the focused terminal in the focused pane (per-terminal isolation, with survey bubbles always on top). The unordered-list bullet glyphs needed the Google-Docs depth-cycle look. Loading an existing team required typing `/` to trigger path autocomplete (confusing). `cs pane split` should mirror the hamburger's RIGHT|BOTTOM options, and one-shot `cs` commands should not enter hybrid-nav transaction mode or steal focus from the sending terminal. The MCP server failed when starting codex (codex needs file config beyond env vars): never touch the user's config files for MCP, and start terminals with MCP env DISABLED by default. The image-draft save dialog lacked path autocomplete. `cs terminal write --submit codex` wrote the command plus a stray newline and did not submit. The graph from a fresh Cmd+Shift+M window could not expand directories until a "graph from here", then lost its depth slider and its non-directory layers. `chan serve` on a very large workspace (a shallow Linux-kernel clone) ran silently for a long time even with `--verbose`, with no progress signal before the URL printed.

**Round-1 enhancements + docs.** Add an auto-assign button to the Spawn-agents dialog after a layout is chosen. Open both the README and the website with a concrete `curl | bash` install plus `chan serve ./repo` usage example, and document chan-desktop (local browser, remote outbound/inbound attach, the reverse tunnel) and the `gateway/` self-hosted online-service surface. Plan the website screenshots.

**Round-2 (the second report).** Add the missing open-source attributions to the About page (svelte, tauri, mermaid, xterm.js, codemirror, d3-force, and the rest of the real stack) with a free-and-open-source tagline. Fix the editor bug where pasting a link into a list INDENTS the list and Shift-Tab makes it worse. Make surveys PER-TERMINAL, not window-wide, so each terminal's survey is independent.

**Round-2 desktop + release (expanded live).** A chan-desktop outbound remote-workspace window that cannot reach its URL paints a blank white webview; show a connecting surface immediately with a spinner, the URL, a live elapsed timer, and one timestamped row per retry until the user closes the window. Encode the workspace kind in the desktop window title (home / computer / outbound / inbound icon plus the locator). Replace the pre-flight bubble's OFF/ON-label-plus-button pair with a single checkmark toggle per row. Make the CI-built macOS DMG match the local one's layout deterministically. Then cut v0.25.0.

## Rounds and waves

### Round 1: host bug sweep + enhancements + docs

The lead split the round-1 report across four lanes by domain and dispatched in two waves: Wave-1 isolated items fired as each lane reported ready; Wave-2 shared-file items the lead sequenced (notably the chan-server window shared by B4 and B5). Concrete outcomes, by lane:

- Terminal / cs (@@LaneB): per-terminal rich prompt with the data-loss fix (reap-only-on-delivery; window-global bubble visibility was the bug); codex submit fixed by wrapping the write in bracketed paste (codex coalesces text+CR into a paste burst, so a bare CR was eaten); a direct dashboard chord; `cs pane split RIGHT|BOTTOM` with no focus-steal and no transaction mode for one-shot commands.
- Editor / graph (@@LaneC): depth-cycle bullet glyphs; lazy-tree path autocomplete (the file tree is lazy, which framed the original bug wrong); graph expand / depth-slider / layers rework.
- Platform / docs (@@LaneD): editable-by-content sniff (a `.zshrc` / `*.service` opens as text); serve-progress heads-up (the long silence is the `watch()` setup stall, not indexing); MCP env off by default with a team-config opt-in toggle and no writes to user config files; the README / home-page / desktop + gateway manuals.
- Lead lane (@@LaneA): launcher copy S1/S2/S3 (the phase-16 carryover smoke follow-ups); team-load path autocomplete (bare prefix suggests `foo/`); the MCP-toggle UI; the spawn-dialog auto-assign; and search path autocomplete.

### Round 2: second report, then a desktop + release wave (v0.25.0)

@@Alex's second report (three items) was triaged onto the existing lanes and dispatched as each finished its round-1 Wave-2 work, append-only with no mid-task interrupt:

- R2-1 in-app open-source attribution on the About page (@@LaneD; folded with the docs work). The list was later trimmed to a one-line tagline.
- R2-2 list paste-link indent plus top-level outdent (@@LaneC; the real cause was turndown emitting a stray `-   ` marker on paste).
- R2-3 per-terminal surveys (lead contract amendment, @@LaneD transport, @@LaneB SPA), a natural follow-on to B1's per-terminal pattern.

The round then expanded into a larger desktop and release wave, run as contract-first parallel splits and capped by the v0.25.0 cut:

- Survey system v2: surveys reach team-DIALOG-created terminals (those were spawned with `window_id: None` and never rebound on attach, so the survey resolver found no window), and every survey now offers options plus an F follow-up plus a Dismiss, with a distinct "dismissed" reply kind so the asking agent can tell.
- Desktop connecting / retry screen for outbound remote-workspace windows: the outbound window now loads a bundled `connecting.html` that drives a page-side retry loop over a Rust `probe_url` IPC (page-driven to avoid a lost-event race), instead of pointing the webview straight at an unreachable remote.
- Desktop window title shows the workspace kind icon plus locator; the pre-flight bubble's per-row toggle became a single checkmark.
- A submit-agent refactor (`SubmitAgent::derive(command, CHAN_AGENT)`) replaced the manual per-member agent picker and dropped the stored `agent` field; submit chords became runtime-overridable.
- Post-round hotfixes @@Alex caught after the close: a graph language-edge fix for a bare FSEvents rename, the About-page trim, the rich-prompt submit-chord plus Tab-list-indent fixes, a `cs` window-command error when no window is connected, blocking global shortcuts behind the disconnect overlay, and the mermaid right-margin alignment.
- Release: the Finder-less DMG layout (dmgbuild) so CI matches the local layout deterministically, the unified v0.25.0 version bump, and a follow-up to codesign the DMG container before notarization (the dmgbuild path left it unsigned; the release-desktop.yml dry-run caught it before the tag).

## Team and coordination

The phase ran as a four-lane cs-terminal team (`phase-17-team`) under the Team Work bus, with @@Alex as host. @@LaneA was the lead and architect; @@LaneB / @@LaneC / @@LaneD were the workers. @@Alex set the scope, then stepped away mid-session authorizing autonomous commit and push.

The coordination scheme was the per-author-journal-plus-task-file bus carried over from prior phases: the lead cut domain-scoped task files (`tasks/task-<from>-<to>-N.md`) and design briefs, pinged a lean one-line poke pointing at each, and workers wrote completion notes back into the task or `followups/` files plus their own append-only journal. Each lane gated its own slice green (cargo fmt / clippy -D warnings / test for Rust; `make web-check` for the SPA; `make -C desktop check` for the Tauri crate); the lead owned the full-tree gate, the per-lane atomic commits with verified staged stats, and the foreground push with a `git ls-remote` verify before tagging. Surveys over the `cs terminal survey` channel were the agreed way to ask the host (not the TUI, whose typing collides with the poke queue). The agent roster and contact cards are in `../agents/README.md`.

## What shipped, tried, and undone

**Shipped (v0.25.0).** The per-terminal rich prompt with the data-loss fix; codex submit via bracketed paste; the dashboard chord; `cs pane split RIGHT|BOTTOM` with no focus-steal; depth-cycle bullet glyphs; lazy-tree and search and image-draft-save path autocomplete; the graph expand / slider / layers rework; editable-by-content sniff; serve-progress heads-up; MCP env off by default with a team opt-in toggle and no user-config writes; the spawn-dialog auto-assign; team-load autocomplete; the README / home / desktop
+ gateway docs; About-page attribution (trimmed to a tagline); the list
paste-link indent and outdent fix; per-terminal surveys; survey system v2 (team-dialog reach plus F/Dismiss on every survey); the desktop connecting / retry screen for outbound windows; the desktop window-title kind icons; the pre-flight checkmark toggle; the `SubmitAgent::derive` refactor with runtime-overridable chords; the graph FSEvents language-edge fix; the `cs`-no-window error; the disconnect-overlay shortcut block; the mermaid margin fix; and the Finder-less DMG layout with a signed-then-notarized container.

**Tried then corrected.** Several tasks were framed wrong by the report and the lanes corrected to the real cause (codex paste-burst not a CR mismatch; the serve stall being `watch()` setup not indexing; the file tree being lazy; window-global bubble visibility; turndown's stray list marker). On survey v2 the lead's first attempt to drop `allowFollowup` fixed only the snake_case wire fixtures and not the camelCase dialog literals; @@LaneD's full-tree gate caught it. The DMG first shipped unsigned out of the new dmgbuild path; the release-desktop.yml dry-run caught it and the codesign-before-notarize hotfix landed before the tag.

**Deliberately not done / deferred.** F1 rich-prompt loader/cancel plus a server prompt-ack (needs a new chan-server frame; B1's reap-only fix already removed the data loss). F2 async `watch()` setup to kill the ~13s pre-URL stall (an event-loss window, risky under release pressure; B10 fixed the silence). F3 BM25-index sniffed text and F4 a prioritized live leaf index (scoped to a later round per an @@Alex survey). Several WKWebView hand-smokes (the launcher copy, the native dashboard chord, the connecting screen) stayed @@Alex's to run, since agents could not drive WKWebView on this setup. D1 publish waited on @@Alex verifying the live install / clone / tunnel commands.

## Retrospective

This is the learning payload, distilled from the round retrospective.

**Highlights.**

- Root-cause depth. The lanes dug past each task's premise to the real cause rather than patching the symptom: the codex paste burst, the serve stall being `watch()` setup, the lazy file tree, the window-global bubble, the turndown stray marker. Several tasks were framed wrong and the lanes corrected them.
- Lane-boundary discipline. Every time the real fix sat in an unlisted file, the lane STOPPED and routed instead of reaching across (B9 to GraphCanvas, B4 to applyPaneExec, R2-2 to list.ts / paste_html.ts, R2-3 recognized as a survey-contract change rather than a one-file edit). @@LaneC self-corrected a flaky-grep claim by anchoring on an atomic Read; @@LaneB caught R2-3 as a contract scope.
- The full gate earned its keep: it caught @@LaneD's B10 chan-desktop `ServeConfig.verbose` miss (a separate Cargo workspace the scoped gate is blind to) before it reached the remote.
- Clean delivery: per-lane atomic commits with verified staged stats, the isolated full gate green, foreground push with a `git ls-remote` verify, and a release-pipeline dry-run that caught the unsigned DMG before the tag.

**Lowlights / contention.**

- The bootstrap's owned-file lists under-specified the editor/state boundaries, so lanes repeatedly surfaced "the real fix is in an unlisted file" (B9, B4, R2-2, R2-3). Make the lists domain-based (a lane owns a coherent area) rather than a fixed enumeration, or scope the recon deeper before assigning.
- Two teams shared one worktree (the leftover phase-16 launcher team plus phase-17). Pokes by tab-name span groups and the prior round was never torn down, which cost early cycles and risked corruption. Tear down a finished round's team before loading the next, and scope every poke by `--tab-group` from the start.
- Chrome automation was permission-denied while @@Alex was away, blocking the interactive smoke of several round-1 SPA changes (it worked for @@LaneB's R2-3). They shipped gated-green, not interactively smoked. Pre-grant the permission, or have a non-Chrome smoke path, before an autonomous window.

**Lessons worth carrying forward.**

- Adding a required field to a shared TS type needs a grep of ALL literals (both casings) plus svelte-check; a scoped vitest strips types and passes with fixtures missing the field. This is exactly how the lead's `mcpEnv` miss slipped a scoped gate and only the full-tree gate caught it.
- The one piece with no test (the search path-detection logic) is the one that could not be smoked when Chrome was denied. Write a unit test for the load-bearing logic rather than leaning entirely on a browser smoke that may not materialize.
- Auto-deriving a follow-up file directory would write a file into the user's workspace for every deferred survey, including non-team ones. The chosen shape keeps F-with-context (opt-in via `--followup`) writing a file and a bare F a no-file deferral, distinct from a Dismiss.
- Page-driven retry beats a Rust emit loop for a webview that probes on load: a Rust-driven emit can fire before the webview attaches its listener (Tauri does not replay events to late listeners), stranding the screen. Rust owns the detection primitive (`probe_url`) and the window redirection; the page owns the loop cadence, timer, rows, and success navigation.
- dmgbuild fixes the Finder-dependent DMG layout (it writes `.DS_Store` programmatically so local equals CI), but it does NOT codesign the DMG container the way `tauri build --bundles dmg` did. A new packaging path must re-check the whole signed-then-notarized-then-stapled chain, not just the layout. The release dry-run is what makes that catchable before a tag.

**Feedback recorded for @@Alex.** The rapid requirements plus the cs-survey channel worked well once set up; the "ask me via survey, not the TUI" correction was right. The biggest friction was the leftover team in the worktree. Consider domain-based owned-file lists in the bootstrap and pre-granting Chrome for autonomous windows.

## Notes

Terminology drift, for mapping old names to current ones:

- "Rich Prompt" is the floating Cmd+Shift+P compose bubble over a terminal; this phase made it per-terminal. Its Team Work counterpart (the in-terminal lead bubble) is the same family of ideas a reader will see under the Team Work name in later phases.
- "drive" and "workspace" both appear in the source; "workspace" is the settled term for the chan root directory on disk. "folder" in launcher copy maps to "directory" elsewhere.
- The `cs` CLI and its wire / control-socket types live in the `chan-shell` crate; survey, terminal, and survey-reply routes live in `crates/chan-server/src/{survey.rs,control_socket.rs,routes/}`.
- "gateway" / "online service" is the experimental self-hosted server-side counterpart in `gateway/`, distinct from the always-core tunnel transport.
- The connecting-screen brief recorded three desktop webview flavours: `workspace-*` (local embedded server), `tunnel-*` (a remote dialing in over a local loopback listener), and `outbound-*` (attach a remote by URL); the blank-white bug was the `outbound-*` case only.

A load-bearing screenshot in the round retrospective showed the standalone connecting page in Chrome: an immediate paint (never blank white) with a spinner, a live MM:SS timer, and one red timestamped retry row per attempt accruing with no give-up, plus the green "connected (HTTP 200)" success state. The live WKWebView visual stayed an @@Alex hand-smoke.

The raw working material (per-lane journals, the task and followup files, the round-1 and round-2 drafts and plans, the design briefs for the survey system, the connecting screen, the desktop refinements, the DMG layout and the graph language-edge fix, the deferred backlog, and the round retrospective) is preserved in git history under docs/journals/phase-17/; that tree was removed from the working tree in the docs cleanup.
