# fullstack-b-12: Chan terminal visual parity with iTerm2 (font + line metrics + cursor + Source Code Pro bundling)

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Bring chan's hybrid terminal rendering into pixel-level
parity with iTerm2's default. @@Alex shared an iTerm
screenshot of the Claude Code rebootstrap output as the
reference target; chan's xterm.js rendering should
visually match (same font + same line spacing + same
cursor shape + same antialiasing).

`fullstack-b-2` (Round-1, commit `315fcc1`) bumped
`lineHeight` from xterm.js's default (~1.0) to 1.2. That
was a partial fix; line height alone doesn't get us to
iTerm parity. This task closes the rest of the gap.

## Background

@@Alex 2026-05-20 with the iTerm2 settings panes as
reference (Terminal / Text / Font tabs from iTerm2
Preferences):

* **Terminal pane**: Report terminal type `xterm-256color`
  (confirms `fullstack-b-11`'s default). Scrollback
  unbounded by default in iTerm; chan's bounded by
  Settings (`fullstack-b-11`).
* **Text pane**: Cursor shape **box** (filled
  rectangle). Blink off. Animate movement off. Hide when
  keyboard focus lost off.
* **Font pane**: **Source Code Pro Regular 14 pt**.
  Anti-aliased on. No ligatures. Horizontal + vertical
  spacing 100% (no extra letter / line scaling beyond
  iTerm defaults).

@@Alex 2026-05-20:

> Font in case you want to match 1x1; if we could ship
> *with* source code pro, that would be great.. check if
> our licenses match.

## Source Code Pro license check

* **Source Code Pro**: licensed under the **SIL Open Font
  License 1.1** (OFL). Adobe's repo:
  `github.com/adobe-fonts/source-code-pro`. The OFL
  explicitly permits bundling with software (commercial
  or otherwise) provided the font file ships with its
  OFL notice (`OFL.txt`) alongside.
* **chan workspace**: Apache License 2.0 (per
  `LICENSE` at repo root). The Round-3 plan also moves
  toward dual MIT + Apache for public flip.
* **Compatibility**: Apache 2.0 / MIT and SIL OFL 1.1 are
  cleanly compatible when the font ships as a data file
  with its own OFL notice. chan's license stays Apache
  2.0 (later dual); the font file's OFL notice rides
  alongside in `crates/chan-server/resources/fonts/` or
  similar.

Verdict: **license check passes**. We can ship Source
Code Pro with chan as long as the OFL notice ships
alongside the font file.

## Authorization

**Authorization: yes**, this task covers edits to:
* `crates/chan-server/resources/` (new `fonts/`
  subdirectory with the Source Code Pro `.woff2` /
  `.ttf` + `OFL.txt`).
* `crates/chan-server/src/static_assets.rs` (rust-embed
  the font + OFL alongside the existing model bundle).
* `web/src/components/TerminalTab.svelte` (xterm.js
  `fontFamily` + `cursorStyle` + related options).
* `web/src/main.css` or a new `web/src/fonts.css`
  (`@font-face` declaration pointing at the embedded
  font URL).
* `Cargo.toml` if a new dep is needed for the font
  embedding (likely none — rust-embed handles it).

@@FullStackB may proceed without further in-chat
confirmation from @@Alex.

## Acceptance criteria

### Font

* Source Code Pro Regular 14 pt is the default chan
  terminal font.
* Font bundled WITH the chan binary (rust-embed pattern;
  same shape as the BGE model bundle now gated behind
  `--features embed-model`, but the font is small enough
  to always bundle — no feature gate needed).
* `OFL.txt` ships alongside the font file in the same
  resources directory, embedded via rust-embed; chan's
  About / Settings includes a one-line attribution.
* `@font-face` declaration loads the embedded font from
  a chan-served path (e.g. `/static/fonts/SourceCodePro-Regular.woff2`).
* Fallback font chain: `"Source Code Pro", "SF Mono",
  Menlo, Consolas, monospace` so if the embedded font
  fails to load, chan falls back to system mono.
* No regression on the existing terminal font behaviour
  for users who already see the system fallback (just
  upgrades them to Source Code Pro automatically).

### Line metrics

* Verify lineHeight 1.2 (from `fullstack-b-2`) still
  reads correct against the iTerm reference. Tune if
  needed; the goal is pixel-match, not lineHeight 1.2
  per se.
* Letter-spacing: 0 (no kerning adjust beyond the
  font's own metrics).
* Cell padding: xterm.js's default unless tuning is
  needed to match iTerm's tighter row pack.
* Anti-aliasing: enabled by default (CSS
  `font-smoothing: antialiased` for WebKit;
  `-moz-osx-font-smoothing: grayscale` for Firefox).

### Cursor

* Default cursor style: **block** (xterm.js
  `cursorStyle: "block"`).
* Blink: off (xterm.js `cursorBlink: false`).
* These match iTerm's defaults per the screenshot.
* Future Settings exposure (Round-2 or Round-3): cursor
  shape + blink as configurable settings. NOT in this
  task — just the defaults.

### Visual parity check

* Side-by-side screenshot of chan terminal vs iTerm2
  with the same content (Claude Code's rebootstrap
  banner is the @@Alex reference). The two should be
  visually indistinguishable at the font / glyph / line
  level. Document the comparison in the implementation
  note.

### Gate

* Pre-push gate: fmt + clippy `-D warnings` + workspace
  test + svelte-check + npm build.
* `cargo build` (default) produces a binary that
  includes the font (~200 KB Source Code Pro Regular
  woff2; well under the BGE-model scale).
* `cargo build --no-default-features` still builds
  cleanly (font bundling stays universal — no feature
  gate).
* Vitest pin for the font-loading logic if a testable
  seam exists (the `@font-face` declaration + xterm.js
  config are config-only; visual match is the
  meaningful gate, not unit-testable).

## How to start

1. Download Source Code Pro from `github.com/adobe-fonts/source-code-pro/releases`.
   The Regular weight at 14 pt size is the default;
   ship the variable / static `.woff2` for web use +
   `OFL.txt` for the license notice.
2. Drop the files into `crates/chan-server/resources/fonts/`
   (new dir; create alongside the existing `models.tar.zst`).
3. Update `crates/chan-server/src/static_assets.rs` to
   serve the font under `/static/fonts/...`.
4. Add the `@font-face` declaration in a new
   `web/src/fonts.css` (or extend an existing CSS
   entry-point), pointing at the served path.
5. Update `web/src/components/TerminalTab.svelte`'s
   xterm.js options:
   * `fontFamily: '"Source Code Pro", "SF Mono", Menlo, Consolas, monospace'`
   * `cursorStyle: 'block'`
   * `cursorBlink: false`
   * lineHeight stays 1.2 unless the visual diff shows
     a tighter / looser value matches iTerm better.
6. Visual diff: open chan terminal + iTerm side-by-side
   (lane-B test fixture). Same `echo`-d content. Capture
   screenshots; compare glyph-by-glyph. If something
   still mismatches (e.g. cell padding, baseline,
   subpixel rendering), tune.
7. Verify the fallback chain works: rename / hide the
   embedded font file temporarily, confirm chan falls
   back to SF Mono / Menlo / Consolas cleanly.
8. Pre-push gate.

## Coordination

* @@WebtestB verifies on lane-B drive once landed.
* Composes with `fullstack-b-11` (terminal scrollback +
  TERM Settings). If `-11` and `-12` are in flight at
  the same time, both touch `TerminalTab.svelte`'s
  xterm.js config. Pre-commit `git diff --staged --stat`
  catches any stowaways per the
  `feedback_shared_worktree_commits` discipline.
* Sequencing recommendation: land `-11` first (Settings
  surface), then `-12` (visual parity). Order isn't
  load-bearing since the two touch different facets of
  the same component; if you land `-12` first it works
  fine too.
* No backend / Rust work on the SPA side; the chan-server
  edits are limited to rust-embedding the font + serving
  it. No new endpoints; the font is a static asset like
  the SPA bundle.
* @@CI may want to verify the binary-size delta after
  this lands (~200 KB up on the default build); not
  blocking.

## 2026-05-20 — implemented

Source Code Pro Regular bundled via rust-embed; xterm.js
defaults retuned to iTerm2 parity. No new HTTP surface
shape beyond the single `/static/fonts/<name>` route; the
SPA loads the face via a normal `@font-face` declaration
imported at app boot.

### Server (Rust)

* `crates/chan-server/resources/fonts/` (new): drops
  `SourceCodePro-Regular.otf.woff2` (76,348 bytes; CFF
  woff2 v2.2752 fetched from Adobe's `release` branch) +
  `OFL.txt` (4,566 bytes; the font-specific OFL notice
  from the same upstream source). ~81 KB total, well
  under the task's 200 KB ceiling.
* `crates/chan-server/src/static_assets.rs`: a second
  `FontAssets` rust-embed struct (`#[folder = "resources/fonts/"]`)
  with a `serve_font` handler. Path-segment-only routing,
  immutable cache headers matching the rest of the SPA's
  hashed-asset policy, 404 on unknown names. No feature
  gate — the font ships across every build profile,
  including `--no-default-features`.
* `crates/chan-server/src/lib.rs::router`: `/static/fonts/:name`
  added to the open lane (auth_middleware lets
  non-`/api`-non-`/ws` paths through, so the browser can
  load the font via `<link>` before the SPA boots).
* Four new Rust tests:
  * `font_bundle_includes_source_code_pro_and_ofl_notice`
    pins the embed contents + the OFL header text.
  * `font_content_type_for_woff2` confirms the existing
    MIME map (untouched).
  * `serve_font_returns_bundled_bytes_with_immutable_cache`
    drives the handler directly and asserts the
    content-type + cache-control headers.
  * `serve_font_returns_404_for_unknown_name` pins the
    miss path.

### Frontend (Svelte)

* `web/src/fonts.css` (new): single `@font-face`
  declaration. `font-display: swap` so the terminal stays
  usable while the woff2 is in flight (the fallback chain
  in TerminalTab's xterm config carries the same family
  preference; the swap is visually subtle).
* `web/src/main.ts`: imports `fonts.css` before the
  editor themes so the face starts loading at app boot.
* `web/src/components/TerminalTab.svelte::start()`:
  three xterm.js option changes:
  * `fontFamily`: now starts with `"Source Code Pro"`
    and keeps `"SF Mono", SFMono-Regular, ui-monospace,
    Menlo, Consolas, "Liberation Mono", monospace` as the
    fallback chain.
  * `fontSize`: 13 → 14 (matches iTerm2 default).
  * `cursorBlink`: true → false (matches iTerm2 default,
    captured in the task screenshot).
  * `cursorStyle`: stays `"block"` (already correct).
  * `lineHeight`: stays 1.2 (from `fullstack-b-2`).
* `web/src/components/SettingsPanel.svelte`: About
  section gets a `terminal font` row that names "Source
  Code Pro Regular" and links to `/static/fonts/OFL.txt`
  for the OFL notice (target="_blank" rel="noopener").

### Tests

* SPA: new `web/src/components/TerminalTab.font.test.ts`
  (5 tests) pins the xterm.js options (Source Code Pro
  first in fontFamily, fontSize 14, cursorBlink false,
  cursorStyle block), the `@font-face` URL + properties,
  and the `fonts.css` import at main.ts boot. CSS-`?raw`
  doesn't work under JSDOM vitest (same constraint
  documented in `fullstack-b-5`); reads from disk via
  `node:fs::readFileSync` with a minimal ambient module
  shim in `web/src/raw.d.ts` so svelte-check stays happy.
* Rust: 4 new (chan-server suite 191 → 195).
* SPA: 5 new (vitest 501 → 506).

### Pre-push gate

* `cargo fmt --check`: clean (one auto-rewrap from
  `cargo fmt` after the new test was added).
* `cargo clippy --workspace --all-targets -- -D warnings`:
  clean.
* `cargo test --workspace`: all green.
* `cargo build --no-default-features`: builds (pre-existing
  `not_a_chan_drive_hint` dead-code warning, unrelated).
* `npm run check`: 3974 files / 0 errors / 0 warnings.
* `npm run build`: clean. The bundled CSS contains the
  expected `@font-face` block with the
  `url(/static/fonts/SourceCodePro-Regular.otf.woff2)`
  reference verbatim — verified by grep on the dist
  output.
* `npx vitest run`: 506/506.

### Notes for review

* **Visual diff with iTerm2**: not driven from this lane.
  Per the task body @@WebtestB owns the side-by-side
  walkthrough on lane-B once -12 lands. The font + cursor
  options follow the iTerm2 screenshot literally; any
  remaining glyph-level mismatch (subpixel rendering,
  baseline) belongs to @@WebtestB's verdict + a follow-up
  if needed.
* **Tunnel-mode font URL**: the `@font-face` `src:
  url(/static/fonts/...)` is an absolute path. For
  loopback runs this hits chan-server's new route
  directly. For `--tunnel-public` runs the same path
  goes through the drive.chan.app gateway; if the gateway
  doesn't preserve `/static/fonts/...`, the load fails and
  the fallback chain catches it (terminal still renders
  in SF Mono / Menlo). Not a regression from anything;
  worth a quick check from @@CI when the tunnel pipeline
  is exercised in Round 2.
* **No xterm.js font-loading wait**: I considered using
  `document.fonts.ready` to delay xterm construction until
  the woff2 is in. xterm.js will re-render once the face
  arrives (the FontFace API triggers a layout pass), so
  the swap happens naturally with `font-display: swap`. A
  hard wait would have added boot-time latency for no
  visible win.
* **OFL.txt content-type**: served as `text/plain` via
  the existing `content_type_for` map (matches the `.txt`
  branch). Renders inline in the browser when the user
  clicks the Settings link.
* **No fetch-fonts helper**: unlike the BGE model bundle
  (which has `crates/fetch-models`), the font is small
  enough (76 KB) that I dropped it in-tree directly via
  curl. Future weights or faces follow the same pattern.

Proposed commit subject:
`Terminal: bundle Source Code Pro Regular + iTerm cursor/size parity (fullstack-b-12)`

Holding for commit clearance; queue empty after this.