# task-LaneD-LaneA-3: D1 + R2-1 DRAFTED (Wave-3 + round-2)

From: @@LaneD  To: @@LaneA  Re: task-LaneA-LaneD-3

Drafted + verifiable-audit done; the unverifiable bits are flagged for @@Alex,
not claimed working. Pathspec sha256 = 756fa643ccedd385. Files:
- README.md, web-marketing/src/pages/home.html
- docs/manual/{index,desktop,gateway}.md  (desktop.md + gateway.md are NEW)
- web/src/components/EmptyPaneCarousel.svelte + dashboardTabAndCarousel.test.ts (R2-1)

## KEY FINDING: the About page is IN-APP, not web-marketing
R2-1's "About page bottom section" is the in-app About widget
(EmptyPaneCarousel.svelte slide 0, the `about-licenses` block) - NOT
web-marketing (story.html is a narrative). So R2-1 edits a frontend component
outside my platform/docs lane. It is uncontended (no peer WIP on it, not named
in any task file) and authorized by the R2-1 assignment; flagging for
transparency.

## D1 - README + website reframe
- README.md: new "## Quickstart" that OPENS the usage flow (curl|bash install
  -> git clone -> `chan serve ./chan` -> IDE in browser) + the in-browser vs.
  chan-desktop note.
- web-marketing home.html: a "quickstart" section right after the hero with the
  same example, so the site opens with it.
- /dl links: ALREADY present (install.html links cli-linux-x64/arm64 +
  macos-arm64 tarballs + the desktop cards; home links the desktop packages) -
  verified, left as-is.
- NEW docs/manual/desktop.md: install, local workspaces, remote attach
  (outbound HTTP/2 + inbound reverse tunnel), ssh -L forward, lima note. The
  WKWebView New->Remote->Outbound/Inbound click-paths are marked PENDING a
  maintainer hand-smoke (agents cannot drive WKWebView).
- NEW docs/manual/gateway.md: services table (identity / workspace-proxy /
  profile / admin), how `chan serve --tunnel-url` uses it, self-deploy notes;
  cross-links github.com/fiorix/chan/tree/main/gateway. Grounded in
  gateway/README.md.
- docs/manual/index.md: nav links to both new pages.
- Gate: web-marketing `npm run build` + `npm run check` GREEN (renders both new
  manual pages, local-link check passes, install.sh `sh -n`).

### Command audit (record)
- VERIFIED by me: `chan serve` (+ `--port/--no-browser/--standalone`,
  extensively in B10/B11); all cited flags exist in `chan serve --help`
  (`--tunnel-url`, `--tunnel-token`, `--port`, plus `CHAN_TUNNEL_TOKEN`);
  install.sh syntax-checked by the web-marketing gate; web-marketing render +
  local links.
- DESCRIBED, accurate, NOT run end-to-end here (flagged in the pages):
  - `curl -fsSL https://chan.app/install.sh | sh` (needs the live site)
  - `git clone https://github.com/fiorix/chan` (repo is PRIVATE pre-release;
    works at public launch)
  - `ssh user@host -L 8787:localhost:8787` (no remote host in sandbox)
  - lima remote `chan serve` (I have lima+sdme; can do a real run on request)
  - `chan serve --tunnel-url .../v1/tunnel` E2E (needs a deployed gateway)
- @@ALEX HAND-SMOKE (WKWebView, cannot automate): New -> Remote ->
  Outbound/Inbound click-paths. Marked PENDING in desktop.md; do NOT publish
  those as verified.

## R2-1 - open-source attribution (in-app About widget)
Added an `about-credits` block after the licenses block: the tagline "Built on
a strong open-source foundation. Chan is free and open-source software." plus
the load-bearing stack grouped browser vs. server, each linked:
- browser: Svelte, xterm.js, CodeMirror, Mermaid, Cytoscape + d3-force, KaTeX,
  Lucide; desktop: Tauri.
- server: axum on Tokio, Tantivy + Candle (BGE-small) for search, notify,
  rust-embed, portable-pty, yamux over h2 for the tunnel.
Grounded in web/package.json + the crate Cargo.tomls (credited what's actually
used). FLAG: @@Alex's report listed mermaid as `mermaid-cjv.pages.dev` (a
non-canonical mirror); I used the official `mermaid.js.org` and pinned a test
asserting the mirror does NOT leak. New pinning test added; vitest 47/47; vite
build compiles; my-file svelte-check clean.

## FLAG: your B5 TeamDialog WIP breaks the whole-tree svelte-check
Not my files - heads-up so your full-tree gate isn't a surprise:
`TeamDialogConfig.mcpEnv` is now required, but these fixtures lack it ->
5 svelte-check ERRORS: teamOrchestrator.test.ts (83 type-mismatch optional,
320, 352), teamBootstrapOrchestrator.test.ts (70), teamLeadRestart.test.ts (55).
Add `mcpEnv` to those drafts (and make the type field match optional/required
consistently). My EmptyPaneCarousel + the 3 B11 TS files are clean.

## Pending / asks
- @@Alex: WKWebView New->Remote hand-smoke (desktop.md flags it).
- A visual browser-smoke of the About slide (static content, low runtime risk)
  - fold into the joint frontend smoke, or I can serve + check.
- Optional: a real lima E2E tunnel run for deeper verification - say the word.
- Publishing/committing waits for @@Alex's command verification per your task;
  the draft + verifiable audit are done.
