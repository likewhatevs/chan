# Phase-15 round-4 retrospective (v0.23.0)

Architect-written at round close. Honest, for everyone (the lanes, @@Host, and
the architect). The companion live record is `round-4-status.md`.

## What shipped in v0.23.0

- Wave 1 (pre-crash, on the v0.22.0 base): editor relative-link pills + `#`/`^`
  modes (A `99ef3c8e`), nav-help label (A `12160198`), ubuntu chan-desktop via
  sdme (B `bb1eed2f`), `cs terminal team new|load --script` (C `ede23ecf`),
  semantic hybrid wiring + phase-8 docs essence (D `0a180ffd`/`f8c8edec`).
- Wave 2: fedora+arch sdme templates (A `7a27e191`), release.yml M1 multi-arch
  desktop matrix (B `06c371a6`) + gateway-linux sdme (B `30a3347f`),
  server-side `cs terminal team new` spawn (C `626593e9`), phase-8 raw/ deletion
  (D `e747f1d2`).
- Wave 3: static musl `chan` CLI (B `101c0f66`), desktop window-close crash fix
  (A `3c437f24`), native macOS Export-to-PDF (A `173bf693`) reworked to the
  print pipeline so it paginates + honors `@pagebreak` (A `ccc117e7`).
- Wave 4: indexing spine pulse during the embed sweep (D `ce9c286e`), tokei
  log-spam filter (D `fcf06679`), `.txt` is no longer a graph document (D
  `a5c95545`).
- Plus: the orange transparent enso favicon unified across every chan site (A
  `d55439c1`).

## Done / pending

DONE + gated-green (full release gate `make pre-push` + the gateway workspace
passed at the bump). PENDING (empirically-unverified, @@Host re-reports if
broken): the desktop crash fix + the macOS PDF render + the favicon per-site
visual - all need a human on macOS/WKWebView (not browser-automatable). The
release.yml musl/zig CI path is validated only by the real v0.23.0 tag run (we
skipped the pre-tag workflow_dispatch dry-run on @@Host's "push now").

## Highlights

- Recovered cleanly from a mid-round session CRASH (the architect tab died mid
  fedora-build). All durable state was on disk + the VM, so recovery was
  empirical and complete - the doc-as-source-of-truth model earned its keep.
- B retired the two riskiest unknowns de-risk-first and locally: chan-desktop
  AppImage via sdme, and musl static linking (proved `ldd` "not a dynamic
  executable" + runs on the VM before touching CI).
- D live-browser-smoked the spine pulse (4 dirs orange during a real 359/360
  embed sweep) and the `.txt`->text mapping - the runtime proof static gates
  miss, done unprompted.
- Shared-tree pathspec-commit discipline held across ~16 commits from 4 lanes +
  the architect with zero cross-contamination.
- @@Host's rapid bug stream (crash, PDF, pagination, `.txt`, magic, favicon)
  was triaged + placed coherently: small fixes folded into a Wave-4, the
  magic-detection feature deferred to round-5.

## Lowlights / friction

- The PDF fix took TWO iterations: the first used WKWebView `createPDF` (a
  screen capture) which clips long notes and ignores `@pagebreak`; reworked to
  the print pipeline. The wrong API shipped one commit before the right one.
- arch chan-desktop AppImage could not be fully validated (linuxdeploy fails
  even with NO_STRIP=1; deb+rpm are validated) -> round-5.
- A favicon agent CONFABULATED a deletion (claimed it removed
  web-marketing/favicon.ico; it had not) - caught by verification.
- A truncated `make pre-push` command reported a false exit-0 (the gate never
  ran) - caught by reading the log, not the exit code.

## Honest feedback

- @@LaneB: exemplary. De-risk-first throughout, the `HOME=/root` rootfs catch
  the static gate missed, the M1/M2/M3 fork that surfaced the right release.yml
  decision cleanly, and a courteous stale-view flag on the PDF files. Model lane.
- @@LaneC: correct REFUSAL to half-wire the SPA `--command/--env` seam (avoided
  the gate-blind-wire trap), clean server-side spawn, and an honest
  SPA-visibility limitation flag. Did exactly the right scoping.
- @@LaneD: the standout. The `.txt` fix exceeded its triage (handled the
  `.md`->`.txt` rename graph-node eviction), and the live browser smoke of the
  spine pulse is the gold standard for runtime-reactive verification. Recycled
  into a fresh wave with a clean `/clear` + re-orient.
- @@Host (Alex): the specific, real-world bug reports (the too-new-libc/musl
  ask, the `@pagebreak` pagination, the favicon enso, `.txt` documents) drove
  genuine quality. Decisive direction + clearing blockers kept momentum high.
  One reflection: the bug stream arrived mid-cut and expanded scope several
  times; the IN/OUT calls all landed well, but an earlier "here is everything
  queued" checkpoint could have reduced architect context-thrash.
- Architect (me): the `createPDF`-vs-print-pipeline miss is the main
  self-critique - I specced the PDF fix on `createPDF` without flagging that it
  does not run the print pipeline or honor `@pagebreak`; I should have known the
  print-CSS distinction up front and saved an iteration. I also leaned heavily
  on delegated agents, which kept coordination clean but required catching two
  agent confabulations and one false exit-0 - the verify-everything discipline
  held, but at a cost. Strong points: the empirical crash recovery, the
  shared-tree commit hygiene, and stopping the arch rabbit-hole on time.

## Carryover to round-5 / phase-16

- Content "magic" file-type detection + a "pending indexing" state (deferred
  Bug 2B; hand-rolled UTF-8/NUL sniff recommended, touches the editable-text
  gate).
- arch chan-desktop AppImage linuxdeploy failure (capture the real error via a
  verbose direct run).
- cs-on-AppImage argv0 dispatch (round-3 carryover; the ARGV0/CHAN_INVOKED_AS_CS
  hook).
- SPA-visible CLI team spawn (a server->SPA attach window-command).
- The `[[` stuck-Indexing-bubble smoke (carried empirically-unverified again).
- Pre-existing broken link `skills/architect.md` in desktect.md/architect.md.
- Validate the release.yml musl/zig CI path with a workflow_dispatch dry-run
  before the next cut (skipped this round).
- CI: GitHub Actions Node 20 deprecation (found in the v0.23.0 release run).
  `actions/checkout@v4` / `setup-node@v4` / `upload-artifact@v4` run on Node 20,
  which GitHub forces to Node 24 on 2026-06-16 (removed 2026-09-16). Bump the
  action versions across `.github/workflows/*` before then. Hard date.
- Editor UI: the unordered-list bullet glyphs are TOO BIG; shrink both the
  filled (black) dot and the hollow (transparent) nested dot. They live in
  `web/src/editor/Wysiwyg.svelte` ~972-990 (`.cm-md-ul-marker`: star -> filled
  bullet at the top level, hollow bullet when nested). @@Host, 2026-06-01.
