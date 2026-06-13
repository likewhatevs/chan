# task-Conductor-TeamFlow-3 — item 3 (broadcast OFF) then item 5 (survey-first + X dismiss)

From: @@Conductor. To: @@TeamFlow. Cut: 2026-06-12.

## Scope, in order

1. Item 3 — teams start with broadcast OFF. Tiny: delete the
   lead-enable + worker-target loop in teamOrchestrator.svelte.ts
   (KEEP the clear-all sweep), re-pin
   teamBootstrapOrchestrator.test.ts.
   Design: new-team-2/designs/item-3-broadcast-default-off.md.
2. Item 5 — Part A: X-key dismiss in BubbleOverlay.svelte + button
   label + source-pin test. Part B: rewrite the "Reaching the host"
   section of generate_bootstrap_md() in
   crates/chan-server/src/routes/team_config.rs (survey-first
   "whenever possible", 1..N/F/X key docs, --tab-name guidance);
   extend the template tests; ASCII-only (test asserts).
   Design: new-team-2/designs/item-5-survey-first-x-dismiss.md.

Line numbers from main @ 3ebee587 — verify before editing.

## Sequencing (binding)

- Item 5 Part B (team_config.rs) lands BEFORE @@CtxPass touches
  control_socket.rs handle_team — land it promptly and poke me the
  sha (1 line; deliberate milestone poke, releases their wave 4b).
- chan-server is three-lane hot: `cargo check -p chan-server` green
  before pausing any multi-file Rust burst.
- Do NOT restart the live serving binary (kills every team PTY,
  including ours). Verify on a throwaway `chan serve --standalone`
  workspace: run a survey, exercise 1..N / F / X from the keyboard,
  regenerate a bootstrap and read the new text.

## Gate

- Web: `make web-check` (vitest) + svelte-check + build. Rust: scoped
  clippy + test with RUSTFLAGS="-D warnings". Re-run after the FINAL
  edit.
- Commits pathspec-atomic: `git commit -F <msg-file> -- <paths>`;
  staged-stat before, show-stat after.
- Sweeps with `rg --text --no-ignore`.

## Review pairing

- Your web commits → adversarial review by @@Editor (I route).
- You review @@Editor's web commits when I route them (item 1 is the
  round's biggest web change — expect a meaty review).
- After items 3+5: hold; I route reviews/assists to you.

## Completion

Milestone poke when item-5 Part B lands (sha). ONE completion poke
after item 5 is done, with new-team-2/tasks/task-TeamFlow-Conductor-<n>.md:
shas, gate results, standalone-server verification evidence.
Journal: journals/journal-TeamFlow.md, append-only.
