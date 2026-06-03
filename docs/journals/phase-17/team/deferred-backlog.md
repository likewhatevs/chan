# Phase-17 round-1 deferred backlog

Items intentionally deferred this round (architect calls + survey outcomes).
Compile into the round-close retrospective. None are release-blockers for r1.

| id  | item                              | why deferred                         | owner (future) |
|-----|-----------------------------------|--------------------------------------|----------------|
| F1  | Rich-prompt loader/cancel +       | B1's data-loss is fixed (reap-only-  | B + D (chan-   |
|     | server prompt-ack                 | on-delivery); a TRUE confirm needs a | server prompt  |
|     |                                   | chan-server prompt-ack frame, shared | handler)       |
|     |                                   | w/ D's crate. Additive robustness.   |                |
| F2  | chan serve async watch() setup    | B10 fixed the SILENCE (heads-up +    | D (chan-       |
|     | (eliminate the ~13s pre-URL stall)| progress). Eliminating the 13s =     | workspace      |
|     |                                   | async watch w/ an event-loss window; | watcher)       |
|     |                                   | risky under release pressure.        |                |
| F3  | BM25-index sniffed text (B11      | DEFERRED per @@Alex survey           | D              |
|     | "searchable")                     | 2026-06-02. ~15-site gate change +   |                |
|     |                                   | config/asm/Kconfig BM25 noise = r2.  |                |
| F4  | Search-overlay prioritized/live   | @@Alex scoped round-1 = autocomplete | A + D          |
|     | leaf index (scan path on no-hit)  | only; live/prioritized index = r2.   |                |

## Hand-smoke pending (@@Alex, on return - WKWebView, agents can't drive it)

- S1/S2/S3 launcher (desktop/src): header order [icon][New]; remote-outbound
  code blocks; remote-inbound copy + code block. `cd desktop && make build`,
  open Chan.app.
- B12 native dashboard chord: Cmd+Shift+D (mac) / Ctrl+Shift+D (linux) opens the
  Dashboard in the desktop app. (Web Alt+Shift+D already Chrome-verified.) If
  WKWebView swallows it pre-JS, add it to chan-desktop KEY_BRIDGE_JS (the bridge
  already has case "app.dashboard.open").
- B11 visual: a .zshrc / *.service opens in the editor + shows as text in the
  file browser (backend API-verified; Chrome-doable by a lane in the
  consolidation webtest, or @@Alex confirms in desktop).
- Team MCP toggle (B5): the team-dialog MCP-env toggle defaults off + round-trips
  (I'll Chrome-smoke the SPA half; desktop is @@Alex's).
- D1 desktop.md: the chan-desktop New -> Remote -> Outbound / Inbound click-paths
  (WKWebView; @@LaneD marked them PENDING in the doc, not claimed-verified).

## D1 publish + review items (@@Alex, on return)

- D1 docs are DRAFTED + gate-green but PUBLISH waits on @@Alex verifying the
  live commands @@LaneD could not run end-to-end: `curl -fsSL chan.app/install.sh
  | sh` (needs live site), `git clone github.com/fiorix/chan` (repo private until
  public launch), `ssh -L` remote, `chan serve --tunnel-url` E2E (needs a
  deployed gateway). @@LaneD CAN do a real lima E2E tunnel run on request.
- R2-1 link choice: @@Alex's report listed mermaid as `mermaid-cjv.pages.dev`
  (a mirror); @@LaneD used the canonical `mermaid.js.org` + pinned a no-leak
  test. Confirm canonical is what you want (almost certainly yes).
- About-slide (R2-1) visual smoke: static content, low risk - folding into the
  consolidation joint frontend smoke.

## @@LaneA round-1 SPA browser-smoke (PUSHED gated-green, smoke unverified)

Chrome automation was permission-denied while @@Alex was away, so these shipped
in 03bb91f8 gated-green (svelte-check 0 err + vitest + build) but NOT
interactively browser-smoked. Hand-smoke (or a webtest pass) on return:
- B3: team-load dir field (Cmd+P) autocompletes a bare prefix as `foo/` (no
  leading "/" needed).
- MCP toggle: the team setup dialog's "expose MCP env vars" checkbox (default
  off) round-trips; the spawn path is e2e-tested (@@LaneD).
- E1: the spawn dialog's auto-assign button distributes robots across the
  layout cells (logic vitest-tested).
- Search: Cmd+S, type a path like `./sub/` -> filesystem suggestions; a file
  opens, a directory drills. The only one with no test of its own.
- R2-2 (@@LaneC, list paste/outdent): 30s confirm - paste a copied link into a
  nested list (no extra indent) + Shift-Tab a top-level bullet (stays a bullet).
  Deterministic CM6 transforms, verified via real-EditorView + unit tests +
  turndown probe; Chrome smoke was denied. Will ship in the round-2 commit.
