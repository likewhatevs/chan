# v0.61.0

The v0.61.0 release report, reconstructed after the fact from the commit range `v0.60.0..v0.61.0` (primary) and the round's `dev/v0.61.0/` coordination tree (secondary), so it is thinner than a report written in-round. Theme: interactive Excalidraw whiteboard tabs and markdown slide preview in the workspace app, plus the desktop-PWA and leader/follower session integration for the launcher and multi-window sessions. Cut GA `v0.61.0` on 2026-07-02 (bump `26b9ea54`).

## Work streams

The release folded two independent threads that had accumulated since v0.60.0:

- [x] Frontend-only workspace demo on the `workspace-demo` branch: an in-memory git snapshot behind an injectable transport seam in workspace-app, language nodes and edges in the demo graph, chan-reports data, in-memory uploads with metadata import/export, and a demo About page. The marketing launcher embed opens the full demo, then is unwired again before GA (`d922da92`, `2c75b858`) so the shipped marketing dist carries no workspace-demo chunk.
- [x] Desktop-PWA, session, Excalidraw, and slides, run as a delivery-team round (`dev/v0.61.0/delivery-team`, lead plus lanes for server, PWA frontend, and Excalidraw). Plans: `dev/v0.61.0/desktop-pwa-session-plan.md`, `dev/v0.61.0/excalidraw-feasibility-analysis.md`, `dev/v0.61.0/round-plan.md`.

## What shipped

Grouped as the CHANGELOG `[v0.61.0]` section records them:

- **Interactive Excalidraw whiteboard tabs.** `.excalidraw` opens as an editable board alongside the markdown, JSON, and CSV renderers; autosaves like any tab; Mod+E flips board and raw scene JSON; session restore, the 409 conflict dialog, and the changed-on-disk banner all apply; Excalidraw and its React runtime are dynamic-imported so the board stays out of the eager editor bundle; the write gate accepts `.excalidraw` as editable text. The `mermaid-to-excalidraw` fence renderer now self-hosts its label fonts (no esm.sh at render time); CJK boards still fall back to the CDN (the 12.7 MB family is excluded).
- **Markdown slide preview.** A `chan:` frontmatter `kind: slides` file presents as slides, split on `@pagebreak`, with `aspect_ratio` and `zoom_factor` tuning; preview and present flows render each page theme-aware with keyboard navigation and media alignment; Mermaid and Excalidraw diagrams render inside slides; current slide and mode persist per tab; the outline groups headings by slide page.
- **Installable launcher PWA.** A web app manifest at `/manifest.webmanifest` (root scope) with maskable icons and a themed titlebar, so the launcher installs from the fixed-port devserver loopback and the https gateway origin. No service worker; the workspace-app shell carries no manifest link, so an installed app captures the launcher, not a single workspace.
- **Leader/follower session windows.** A self-managed launcher opens its own in-app browser windows and gates window creation on per-tenant leadership; the leader manages a workspace's windows, a follower sees the controls disabled; the status bar shows the window's session role when more than one window shares a session; leader close or hide shows the follower a "closed/hidden by the leader" overlay. A follower no longer deletes the session's persisted layout blob, which belongs to the leader.
- **Desktop "Open in Browser".** A Window-menu item opens the focused workspace window in the system browser through a browser-affinity record, so chan-desktop never opens a native twin for it.
- Supporting server and launcher changes: launcher capabilities split by serving surface (a `chan-launcher-surface` descriptor replaces the single read-only boolean); `/ws` sends a session roster snapshot on connect; window mint, close, and visibility are leader-gated per tenant (honest-client enforcement, not a security boundary); browser-minted windows stay in the browser.

## The rc window

`v0.61.0-rc1` was a pin state, never a tag: the pins were bumped to `0.61.0-rc1` in `ec437854`, feature work layered on top, and a `publish=false` dispatch of `release.yml` ran on the `workspace-demo` head `128b3198` (run 28595428661, derived tag `v0.61.0-rc1`). Nothing was published, so `chan upgrade` never offered the rc and there was no chan.app or GitHub Release for it; the host installed from the run artifacts (`dev/v0.61.0/host-smoke.md`). The Excalidraw tab was the hi-pri surface, smoked fully on chan-desktop; the PWA and leader/follower items were lower priority, with individual bugs postponable case by case. GA `v0.61.0` (`26b9ea54`) then stripped the rc pins and renamed the CHANGELOG `[Unreleased]` section to `[v0.61.0]`.

## Accepted gaps (from the smoke, deferred by design)

Recorded in `dev/v0.61.0/host-smoke.md`: never-drawn `.excalidraw` auto-discards on close (seeding-on-create declined for rc1); the WCO titlebar drag strip descoped; the launcher leader-side close/hide button deferred (drive via `cs window discard/hide`); "Open in Browser" acts on the focused window, not a per-window list; leader gates are honest-client enforcement, not a security boundary; `design.md` was not updated for the new surfaces (GA follow-up).

## Carryover to v0.62.0

The v0.62.0 "polish and cleanup" round picked up several of these threads: one alert surface and one connecting surface (both theme-aware), launcher parity on web and gateway, the launcher theme driving local standalone terminals, and the wysiwyg list-typing regression fix. See `team/release/release-v0.62.0.md`.
