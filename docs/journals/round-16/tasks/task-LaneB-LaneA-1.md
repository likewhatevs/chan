# task-LaneB-LaneA-1: launcher redesign DESIGN doc - COMPLETE

From: @@LaneB  To: @@LaneA (lead)  Type: completion

## Deliverable

new-team-1/desktop-redesign-design.md - FINAL, buildable, source-grounded.
C + D recon folded in; no open dependency remains. Covers all four
required parts: header redesign, row redesign (ON|WHERE + icons + gear
removal), the [New] surface (window vs modal + the Team-Work-style 3-choice
interaction, concretely), and the implementation plan (files, new Tauri
command, build + WKWebView-only smoke plan, lane split).

## Key calls (for your aggregation to @@Alex)

- [New] = a REAL second window (recommended). v1 leaned modal only on a
  presumed capability cost; C confirmed there is none (`main-new` inherits
  default.json), and the draft says "open a new window." Modal documented
  as the lighter fallback (decision D2).
- Gear removal: SAFE per D (no gap; both toggles live in the SPA
  Dashboard). Folded in, incl. the "out-of-workspace bulk toggling goes
  away" nuance.
- Add-time bge/reports toggles: KEEP them in the Local choice (decision
  D1, the call you routed to me from C). They are creation-time selection,
  load-bearing for the first index - distinct from the removed ongoing
  gear. Tradeoff stated if @@Alex wants them gone too.

## Open items = @@Alex design decisions only (section 6)

- D1: keep add-time toggles (recommended) vs drop for strict consistency.
- D2: confirm window (recommended) vs the modal fallback.
- D3: remote ON cell - connection dot (recommended) vs a text badge.
- D4: keep the header tagline? (minor)
- Heads-up: the draft's ESC-on-Team-Work bug appears ALREADY FIXED
  (6100ec84, TeamDialog.svelte:274-289) - verify, not re-fix; SPA not
  launcher, out of scope here.

## On your dispatch of the build

Proposed lane split (section 5): one Frontend-Lane owns ALL of
desktop/src/* (index.html, new.html, new.js, launcher-common.js, main.js,
styles.css) - single owner because main.js is one file and the tunnel/
outbound logic MOVES into new.js (splitting collides); one parallel
Rust-Lane owns src-tauri only (the open_new_workspace_window command +
delete the two gear commands), joined by a 3-line naming contract; then a
Verify-Lane does macOS app smoke (WKWebView, not Chrome-automatable).

Build should NOT start until @@Alex signs off D1+D2 (they change the file
set: window adds new.html/new.js/launcher-common.js + a Rust command;
modal does not).

## Journal

new-team-1/journals/journal-LaneB.md (full context + the decision-flip
rationale).
