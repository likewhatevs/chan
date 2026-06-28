# Release history

The development history of chan, one consolidated report per release era. Each `release-{NN}-{version}.md` is that era's front door: its roadmap (the asks), how the rounds and waves were structured, who worked it and how they coordinated, what shipped, and a retrospective with the lessons worth carrying forward. The `NN` prefix orders the reports chronologically; the version marks the release(s) the era shipped (`prerelease`/`unreleased` where no tag was cut). Going-forward reports drop the prefix and are named for their release (`release-{version}.md`).

These reports are the project's second brain. The raw per-author journals and coordination buses that backed them were distilled into these files; the raw material is preserved in git history.

New agents should also read [`.agents/playbook.md`](../.agents/playbook.md), the operational lessons distilled across the project.

## Index

- [release-01-prerelease](release-01-prerelease.md) - first public release prep: filesystem graph, search status, CLI parity, hardening.
- [release-02-prerelease](release-02-prerelease.md) - graph, editor, and search hardening, plus language elevated into the graph.
- [release-03-prerelease](release-03-prerelease.md) - UI polish and the Assistant-to-Agent rename, URL state, and editor fixes.
- [release-04-prerelease](release-04-prerelease.md) - sparse bug-bounty placeholder; effectively a numbering skip from 3 to 5 (see the report).
- [release-05-prerelease](release-05-prerelease.md) - the MCP-only refactor, persistent terminals, and VCS-aware indexing.
- [release-06-v0.10.0](release-06-v0.10.0.md) - the filesystem made the primary graph layer, plus the folder-to-directory terminology change.
- [release-07-v0.10.1-v0.11.0](release-07-v0.10.1-v0.11.0.md) - project hygiene, Hybrid panes, and the agent-orchestration substrate.
- [release-08-v0.11.0-v0.13.0](release-08-v0.11.0-v0.13.0.md) - bug sweep, the signed-DMG pipeline, and public-flip preparation.
- [release-09-v0.14.0](release-09-v0.14.0.md) - the desktop-native vision, drive metadata isolation, and the Rich Prompt revamp.
- [release-10-prerelease](release-10-prerelease.md) - the desktop embedded-server merge and the public site and manual.
- [release-11-v0.15.5](release-11-v0.15.5.md) - the drive streaming spine, editor and graph fixes, and the release contract.
- [release-12-v0.16.0](release-12-v0.16.0.md) - the drive-to-workspace rename and the graph and File Browser carryover.
- [release-13-v0.17.0-v0.18.0](release-13-v0.17.0-v0.18.0.md) - the Graph and Dashboard rework, then the Team Work revamp.
- [release-14-v0.17.0-v0.18.0](release-14-v0.17.0-v0.18.0.md) - Gateway monorepo migration into a nested workspace, then a frontend review and pristine cleanup, plus paced graph hot paths and the new-workspace pre-flight relocation.
- [release-15-v0.20.0-v0.23.0](release-15-v0.20.0-v0.23.0.md) - Dashboard carousel, the cs CLI, Team Work plus the survey rebuild, and the indexing hardening thread.
- [release-16-v0.24.0](release-16-v0.24.0.md) - cs lead-tooling, a long host-driven feature and polish stream, and the chan-desktop launcher redesign.
- [release-17-v0.25.0](release-17-v0.25.0.md) - host bug sweep, survey v2, and the desktop connecting screen.
- [release-18-v0.26.0](release-18-v0.26.0.md) - hybrid-surface bug sweep (editor lists, graph, File Browser, inspector pills, terminal), the repo/docs fold, and the v0.26.0 release wave.
- [release-19-v0.26.1-v0.28.1](release-19-v0.26.1-v0.28.1.md) - Linux/WebKitGTK desktop parity, the in-tree Drafts model, the graph `@@mention` lens plus contact nodes and offline reconcile, and the .agents/ docs fold.
- [release-20-v0.29.0](release-20-v0.29.0.md) - chan-desktop refinements (plain-text update prompt, unified About) and standalone terminal windows, run as an orchestrator plus subagents round.
- [release-21-v0.29.0](release-21-v0.29.0.md) - terminal cross-window awareness: a cross-window broadcast roster (menu, indicators, toggle gate), per-tenant Terminal-N naming, group-wide cross-window Select All, and tenant-wide name uniqueness.
- [release-22-v0.31.0](release-22-v0.31.0.md) - window management: bury-on-close with a dynamic Window menu, Cmd+Shift+N per-connection semantics, remote window reopening over GET /api/windows, cs window list, the standalone-terminal control socket, the split-pane replay fix, and quit confirmation.
- [release-23-v0.31.1](release-23-v0.31.1.md) - the tidy-up: archaeology scrub, zero-warning hygiene, docs currency, the macOS file-drop takeover fix, and the chanwriter purge.
- [release-24-unreleased](release-24-unreleased.md) - the visibility round: editor keep-alive on tab switch, prompt-queue depth, buried-window memory and survey-first host comms, plus the standalone graph-keepalive web feature.
- [release-25-v0.35.0](release-25-v0.35.0.md) - chan-desktop acts as `chan` (one binary, no separate CLI download), the empty-pane window-save cleanup, and a folded-in rich-prompt enqueue/recall/reload round.
- [release-26-v0.36.0](release-26-v0.36.0.md) - Windows-first chan-desktop: the named-pipe control socket, the Git BASH terminal with a missing-Git in-app gate, NSIS packaging plus the windows-latest CI arm, and markdown iframe embeds.
- [release-27-v0.36.0](release-27-v0.36.0.md) - opt-in workspace lifecycle plus the v0.36.0 Windows smoke fixes.
- [release-28-v0.39.0](release-28-v0.39.0.md) - the chan devserver, and killing the default workspace.
- [release-29-v0.39.1](release-29-v0.39.1.md) - devserver and chan-desktop hardening.
- [release-30-v0.40.0](release-30-v0.40.0.md) - making the devserver window lifecycle actually work: reattach, discard, and the FD leak.
- [release-31-v0.41.0](release-31-v0.41.0.md) - one window registry: a watcher drives the local and devserver window lifecycle.
- [release-32-v0.42.0](release-32-v0.42.0.md) - devserver = chan-library: per-devserver gateway proxy plus library-owned open.
- [release-33-v0.43.0](release-33-v0.43.0.md) - web-launcher unification across all surfaces, embeddings honesty, and carryover.
- [release-34-v0.44.0](release-34-v0.44.0.md) - launcher reflects reality (workspaces and devservers), `chan open`/`close`, and the transfer bubble.
- [release-35-v0.45.0](release-35-v0.45.0.md) - the v0.45.0 desktop release: launcher, devserver-in-launcher, and lifecycle hardening.
- [release-36-v0.46.0](release-36-v0.46.0.md) - launcher polish, editor and graph fixes, and desktop hardening.
- [release-37-v0.47.0](release-37-v0.47.0.md) - devserver and launcher connect lifecycle.
- [release-38-v0.48.0](release-38-v0.48.0.md) - devserver and launcher window lifecycle, identity, and presentation.
- [release-39-v0.49.0](release-39-v0.49.0.md) - UI responsiveness, desktop cosmetics, tunnel e2e, and container packaging.
- [release-40-v0.50.0](release-40-v0.50.0.md) - terminal interaction, reload-state, CLI ergonomics, and desktop geometry.
- [release-41-v0.51.0](release-41-v0.51.0.md) - Windows desktop support, published (unsigned).
- [release-42-v0.52.0](release-42-v0.52.0.md) - the unification sweep.
- [release-43-v0.53.0](release-43-v0.53.0.md) - leader presence, the self-managed devserver daemon, and terminal scrollback resume.
- [release-44-v0.53.1](release-44-v0.53.1.md) - a Windows, clipboard, and editor patch.
- [release-45-v0.54.0](release-45-v0.54.0.md) - machine-first launcher, Docker publishing, editor polish, and chan open routing.
- [release-pub-site](release-pub-site.md) - a standalone branding and positioning re-steer plus marketing site refresh (not a numbered release era).
- [release-0.55.0](release-0.55.0.md) - editor polish, devserver hardening, docs consolidation, and the validation carryover into v0.56.0.
- [release-0.56.0](release-0.56.0.md) - design-doc cleanup, the `DEVSERVER_*` gateway contract, v0.55 validation carryovers, and devserver lifecycle hardening.

## Conventions

Agent references in prose use plain names for people and lanes; the `@@` sigil is reserved for the five reusable skill identities (`@@architect`, `@@fabler`, `@@rustacean`, `@@syseng`, `@@webdev`) plus the generic `@@agent` for an unidentifiable past skill. A historical lane named for a discipline (Architect, Syseng, Rustacean, Web/Frontend) is rendered as its skill sigil (`@@architect`, `@@syseng`, `@@rustacean`, `@@webdev`) even where it sits among plain lane names; reused or suffixed slots (FrontendB, WebMain, FullStackA) stay plain. The coordination scheme evolved over the project; each report records the scheme that era ran on, and the cross-project summary is in [`.agents/playbook.md`](../.agents/playbook.md). Reports are text only; a load-bearing screenshot is described in prose rather than embedded.
