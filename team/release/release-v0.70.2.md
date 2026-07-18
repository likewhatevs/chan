# Release v0.70.2 - the terminal-reconnect regression patch

Patch round run 2026-07-18 off the v0.70.1 tag. Six fixes: two are the same regression in v0.70.0's terminal-socket reconnect kit, the rest are editor and slides polish reported from the owner's daily use. Diagnosis and the gate were integrator-owned; the code was implemented by orchestrated implementer subagents on the `v0.70.2-dev` branch, one lane at a time into a clean tree. Shipped without an rc cycle by the owner's call. Coordination artifacts live in the untracked `dev/v0.70.2/` tree of the round host's checkout.

## What shipped

The headline is one root cause behind two very different symptoms. Commit `9511228a` (v0.70.0) gave every terminal socket a reconnect kit: a 20s heartbeat ping, a 45s read-deadline that force-closes a quiet socket, and an `onclose` auto-redial. It was applied to every terminal, and on a redial that cannot reattach the prior session the server's `get_or_create_for_ws` spawns a fresh session running the tenant's default command.

- **The devserver control terminal stopped looping its connect script.** The control tenant's default command is the connect script, so the kit re-ran it after every exit: script exits, the socket redials, the server spawns a fresh session that runs the script again. The control terminal is a local, single-shot runner owned by the desktop exit watcher, so it is now excluded from the kit end to end: no heartbeat, no read deadline, no connect deadline, and no auto-redial. The connect-deadline guard was the non-obvious part of the fix; it is armed unconditionally at dial and normally superseded by the read-deadline in onopen, so once the read-deadline became a no-op for the control terminal an unguarded connect-deadline would have force-closed a healthy control socket at 10 seconds.
- **An idle remote terminal keeps its process, and a dead program no longer leaks mouse tracking.** On a normal terminal the fresh session is a fresh shell, so a long idle or laptop sleep replaced whatever was running (an agent, an editor) and the reused xterm still held the dead program's mouse-tracking mode, printing motion reports at the prompt until a reload. Two client-side changes: the attach budget no longer discards a resumable session id on transport failures (only the server's explicit close ends a session), so a persisted remote session is reattached after an offline window instead of abandoned; and when a genuinely fresh shell does replace a session, the terminal resets its input modes (mouse, focus, alt-screen, including the urxvt 1015 encoding) first.
- **Inline markdown renders inside table cells.** Cells were inserted as plain text, so bold, italic, inline code, and links showed their literal markers; each cell now runs through the same sanitized inline-markdown pipeline the rest of the document uses.
- **Exported slide decks size embedded Excalidraw diagrams to the slide.** An Excalidraw export bakes fixed pixel dimensions that the PDF foreignObject rasterization does not constrain with a percentage max-width, so diagrams overflowed the page; the injected SVG is normalized to an absolute-pixel max-width that survives that path, matching the on-screen preview. Mermaid diagrams are untouched.
- **New slide decks seed the default zoom_factor.** The New slide deck template now writes `zoom_factor: 2` alongside `aspect_ratio`, so the default is explicit in the starter frontmatter.
- **The editor page-width scrollbar sits at the window edge.** The cap moved from the CodeMirror scroller to the content element, so the scroller stays full width; the scrollbar is at the window edge and the off-page band is scrollable.

A seventh reported item, `chan upgrade --version <x>`, was investigated and deferred by the owner: the CLI threads the version correctly, but the deploy hosts only the latest per-version metadata at `/dl/cli/vX.Y.Z.json`, so a pinned non-latest version 404s. The fix is a deploy-pipeline change owned separately.

## Team / process

Integrator-driven, orchestrator-plus-subagents. The integrator diagnosed every bug against live code, wrote a single `dev/v0.70.2/plan.md` with per-lane WHAT-briefs and house rules, and owned the gate, merges, journal, version bump, and release. Implementer subagents ran the lanes: editor rendering (bugs 2, 3, 5) and the slides seed (bug 4) in parallel on disjoint files, then the terminal socket (bugs 1 and 6, one shared file) alone. The two terminal regressions were pinned by a fresh-context archaeology agent after the integrator ruled out four other hypotheses by git history (the exit watcher, connect idempotency, the control-tenant PTY, and the launcher poll), which is what surfaced the single shared root in `9511228a`.

## Validation

Each lane own-gated (svelte-check plus the full workspace-app vitest, 2826 tests; per-crate clippy `-D warnings`, focused cargo tests, and `cargo fmt --check` after the last edit). The integrator verified every diff against live source before committing. Full `make pre-push` green twice: on the fix tip, and again on the version-bumped GA commit (a full rebuild), across both workspaces including the gateway build, web-check with the full vitest and production build, and the marketing smokes. Bug 2 carries a jsdom CodeMirror render test asserting `<strong>` inside a cell; bug 4 a byte-exact seed test; the served bundle was grep-confirmed to carry every fix (a stale-embed guard). GA validation was a `release.yml publish=false` dispatch on the GA-pinned branch; all jobs green except the Windows signed job, which failed on an SSL.com CodeSignTool download outage (their endpoint returned HTTP 500) unrelated to the change and independent of the tag.

## Retrospective

### Highlights

- The exhaustive ruling-out paid off: four plausible explanations for the control-terminal loop were each killed by git history before the real cause was found, and it turned out to unify with a second, unrelated-looking bug under one commit. Front-loading the diagnosis is what made the fix a small, surgical scope-and-restore of the reconnect kit rather than a rewrite.
- Disjoint-file lanes with own-gates kept the shared tree clean; the terminal lane ran alone because its two bugs shared a file, avoiding a merge seam.

### Lowlights

- The interactive headless-browser smoke of the visual fixes (scrollbar, excalidraw export) was blocked by an unrelated cap-std file-open quirk on the tmpfs test workspace, so those two rest on code review, unit tests, and a served-bundle freshness grep rather than a live drive.
- The GA dry run was blocked at the finish line by an SSL.com download outage, holding the tag through no fault of the release.

### Honest feedback

The reconnect kit shipped in v0.70.0 was correct for tunneled terminals but was applied to every terminal without excluding the one class it should never touch (the local single-shot control terminal) and without a reset-on-fresh-shell seam. Both gaps are the kind a review focused on the tunneled path would miss. The fixes here are narrow by design; the broader lesson is that a socket-lifecycle change should enumerate every terminal class it applies to.

## Follow-ups

- `TRACKED_PRIVATE_MODES` (`terminal_sessions.rs`) omits the urxvt 1015 mouse mode, so a live reattach would not re-assert it. Out of scope here; a small server-side follow-up.
- `chan upgrade --version <x>` needs the deploy to retain per-version `/dl/cli/vX.Y.Z.json` across deploys. Owned separately (deploy pipeline).
- The CodeSignTool download step has no retry; a bounded backoff would let a transient SSL.com 500 self-heal on future releases.
