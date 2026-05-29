# fullstack-a-42 — Settings About section build-out (post-trim) with donation QR

Owner: @@FullStackA
Goal: After the Hybrid back-side migration trims Editor /
Terminal / Search settings out of `SettingsPanel.svelte`,
build out the remaining Settings overlay's About section
into the canonical "where Chan lives + how to reach the
project" surface.

## Background

Round-2 wave-2 ships a substantial Settings-overlay
restructure (see [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited"):

* **Terminal settings** → Hybrid Terminal back (Task A
  prereq + Task B migration).
* **Editor settings** (Theme / Layout / Date Pills / On
  Save) → Hybrid Editor back (Task C migration).
* **Search settings** (semantic search toggle from
  `-a-21`, future multi-model picker from Round-3 Track
  2) → Hybrid File Browser back (Task F migration —
  locks the previously-open question on FB-back content,
  see plan §"Hybrid back-side revisited" open question
  #2; @@Alex decision 2026-05-21).

What's left in Settings after the trim: drive-level
toggles that survive the migration (e.g., any non-search
drive scope), app-level config (window state per
`-b-1`), and the About section. The About section is
the load-bearing remainder — @@Alex's framing: with the
space freed, build a proper About surface.

The donation QR is part of @@Alex's intent to give
Chan-loving users a low-friction support gesture. Same
QR also ships on the chan.app website per backlog item
6 (parallel work; not this task).

## Acceptance criteria

* `SettingsPanel.svelte`'s About section (current shape:
  ~line 770; chan version + Source Code Pro
  attribution) expands to host:
  1. **chan version** — already wired from `-b-12`'s
     build-info; preserve current shape. Show release
     vs dev hint where appropriate (e.g., "0.11.2" vs
     "0.11.2-dev-<sha>").
  2. **chan paths** — surface the canonical "where
     does Chan live" set:
     * Drive root path (currently-open drive).
     * Embedded paths: index, graph store, sessions
       store, any others surfaced by chan-drive /
       chan-server today. Walk
       `crates/chan-drive/src/lib.rs` +
       `crates/chan-server/src/state.rs` to enumerate.
     * Config path (`<config>/chan/`).
     * Log path if user-visible (else omit).
     Each path renders with a copy-to-clipboard button
     reusing whatever Copy-path component exists today
     (the bug `-a-2` flagged about persistent "Copied
     path" toast applies here — make sure the copy
     interaction lands on a transient-toast channel,
     not the persistent one; coordinate with whoever
     lands that fix if it hasn't already).
  3. **Project link** — chan's GitHub repo URL with
     both copy-to-clipboard + open-in-browser
     affordances. URL = `https://github.com/chan-writer/chan`
     (canonical per CLAUDE.md "Issue tracker: GitHub
     repo chan-writer/chan"). Use Tauri's
     `shell.open()` (or the webdev equivalent) for the
     browser-open path; this is chan-desktop-friendly.
  4. **Donation QR** — display
     `/qr-donate.png` (already in `web/public/`,
     committed alongside this task). Short copy
     explaining the gesture, written in @@Alex's voice
     not marketing tone — something like "If Chan is
     a daily driver for you, scan to send a tip.
     Optional; the project is free either way."
     (refine at implement time). Visual sizing: 160-200
     px square is a typical donation-QR scale; tune
     against the rest of the About section visually.
  5. **Existing attribution** — Source Code Pro OFL
     from `-b-12` preserved. Future attributions
     (markmap MIT when it lands) extend the same list.
* The trimmed Settings overlay overall renders
  proportionally — About is no longer a footer
  afterthought but a peer section.
* No new persistence (the About section is read-only
  display + extern actions).
* Accessibility: QR image has descriptive alt text;
  paths are selectable text (not images); links have
  visible focus styles consistent with the rest of the
  SPA.
* Dark / light theme parity — QR image background
  works against both (the asset is a black-on-white
  png; consider whether to wrap in a white plate for
  dark-theme readability, or invert. Decide at
  implementation; black-on-white on a white plate is
  the safer call).

## How to start

1. Read [`web/src/components/SettingsPanel.svelte`](../../../../web/src/components/SettingsPanel.svelte)
   — the existing About section at ~line 770 is your
   starting structure. Note the `buildInfo` field
   loaded on mount (line ~80) — that's the version
   wire.
2. Confirm prerequisites have landed:
   * Task A (back-side architecture refactor) —
     required.
   * Tasks B + C + F (Editor / Terminal / Search
     settings migrations) — required so the Settings
     overlay is actually trimmed. If any of these
     haven't landed, this task waits.
3. The QR asset is already in
   [`web/public/qr-donate.png`](../../../../web/public/qr-donate.png)
   (61 KB; black-on-white 2D code). Reference it as
   `/qr-donate.png` from the SPA — `web/public/`
   contents are served at the root by Vite + by
   chan-server's static-asset pipeline.
4. Walk [`crates/chan-server/src/routes/build_info.rs`](../../../../crates/chan-server/src/routes/build_info.rs)
   for the existing build-info surface; if path
   enumeration needs a new endpoint, add it there or
   reuse `state.rs` exports.
5. For path enumeration, prefer SERVER-SIDE
   resolution (chan-server knows its drive root +
   embedded-store paths authoritatively) over
   client-side guessing. Add a small endpoint or
   extend `/api/build-info` if needed.
6. Implement, then `npm run check` + `npm run build`
   per the standard pre-push gate. The Hybrid back-
   side migration prereqs are the load-bearing risk;
   visual polish (sizing / copy / alignment) is the
   second-pass concern.

## Coordination

* **Sequenced after Hybrid back-side wave** — this
  task's prereqs are Tasks A + B + C + F in the
  "Hybrid back-side revisited" set. Don't pick this up
  until those have landed in HEAD.
* **QR asset is pre-committed** by @@Architect
  alongside this task file (
  `web/public/qr-donate.png` ); not part of the
  implementer's diff. The implementer references the
  file but doesn't add it.
* **Companion website work** — placing the same QR on
  the chan.app website is backlog item 6 (website
  migration). Different lane (whoever owns the
  marketing-site port). Both surfaces use the same
  asset; if the QR ever changes, both surfaces need
  refresh. Worth a small note in the chan.app task at
  fan-out.
* **Bug crosstalk** — the persistent-toast bug for
  "Copied path" notifications (see `phase-8-bugs.md`
  "Copy-path notification timeout") affects the About
  section's path-copy affordance. If that fix lands
  first, this task inherits the working transient-
  toast channel. If this task lands first, ensure the
  About copy-path action uses whatever channel will
  become the transient one — coordinate with the bug-
  fix implementer.
* **Shared worktree discipline** — `web/public/qr-
  donate.png` is the architect-side stage; this
  task's commit covers `SettingsPanel.svelte` +
  whatever path-enumeration endpoint additions. Per-
  file `git add` discipline per
  [feedback_shared_worktree_commits].

## Out of scope

* Donation flow beyond displaying the QR — no in-app
  payment wire, no analytics, no tracking. Just the
  QR + short copy.
* Website QR placement (separate backlog item).
* Multi-currency / multi-platform donation aggregator
  surfaces. Phase-9+ if ever; not this task.
* About-section telemetry (anonymous "user viewed
  About" pings) — explicitly NOT shipping that.
  Chan is local-first; About is a view-only surface.
* Any changes to the build-info surface beyond what
  About-section rendering needs.

## Numbering note

Cut as `fullstack-a-42` against the highest-committed
`-a-N + 1` rule (last committed: `-a-41`). If the
Hybrid back-side wave (Tasks A-F) claims numbers
`-a-42` ... `-a-47` at fan-out, this task gets renamed
to `-a-48` at that time. Number is sequence-only, not
priority — actual sequencing is per the prereq chain
in §"Coordination".
