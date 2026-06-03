# task-LaneA-LaneD-3: D1 (README + website reframe) + R2-1 (attribution)

From: @@LaneA  To: @@LaneD  Wave: 3 (D1) + round-2 (R2-1 folds in)

B5 done - thanks. D1 now. DRAFT EARLY, VERIFY/PUBLISH LATE: draft the docs +
audit the commands you CAN verify now; FLAG the desktop/WKWebView commands for
@@Alex's hand-smoke (do not publish those unverified). @@Alex is away a few
hours; he authorized commit+push, so commit your drafted+verified docs at the
round boundary, but DO NOT assert a command works that you could not run.

## D1 - README + website reframe (bootstrap D1 + round-1/draft.md "Documentation")

1. README.md + web-marketing home OPEN with a usage example:
   curl|bash install -> `chan serve ./repo` -> IDE in the browser
   (git clone https://github.com/fiorix/chan ; chan serve ./chan).
2. web-marketing/ (NOT web/):
   - LINK the released packages: /dl/cli + /dl/desktop (from release.yml's /dl
     metadata).
   - Add a chan-desktop section: macOS .app / Linux AppImage; remote attach
     inbound + outbound; lima-vm + ssh-tunnel examples (per draft.md).
   - Add a Chan gateway section: gateway/ services (identity/profile, db + CLI
     admin, workspace-proxy for `chan serve --tunnel-url`, OAuth, self-deploy).
     Cross-link gateway/README.md.
3. AUDIT + TEST every command before it is presented as working (@@Alex's
   explicit requirement). Verify what you can: `chan serve`, the curl|bash
   install, the tunnel commands (you have the lima-vm/sdme Linux path for a real
   run). FLAG the chan-desktop / WKWebView-only steps (New->Remote->Outbound/
   Inbound click-paths) for @@Alex - mark them "pending @@Alex hand-smoke" in the
   draft, do not claim-verified.

## R2-1 - open-source attribution (round-2/draft.md + round-2/plan.md)

Add to the About page's bottom section + "built on strong open source
foundation, chan is free and open source software":
- svelte, tauri (the two @@Alex flagged as missing)
- mermaid https://mermaid-cjv.pages.dev/ , xterm.js
  https://github.com/xtermjs/xterm.js/ , codemirror https://codemirror.net/ ,
  d3-force https://d3js.org/d3-force
- check the rest of the real stack too (axum, candle/BGE embeddings, rust-embed,
  notify, yamux/h2) and credit what's actually used.
LOCATE the about page first (likely web-marketing; confirm whether there is also
an in-app one) - read it, don't guess; add to what's there.

## Gate

- Build the web-marketing site (its own build) + verify links resolve.
- Markdown lint / spellcheck if the repo has one.
- The command audit above. Record which commands you ran + their output in your
  report; mark the @@Alex-hand-smoke ones explicitly.

## Report

Cut task-LaneD-LaneA-3 (what's drafted, which commands verified-vs-flagged,
own-gate) + poke. Publishing/committing the final docs can wait for @@Alex's
command verification, but get the draft + the verifiable audit done now.
