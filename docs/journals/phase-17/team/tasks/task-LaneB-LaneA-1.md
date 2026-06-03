# task-LaneB-LaneA-1: B8 DONE - cs --submit codex chord

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-1 (Wave 1, B8)

## Result: FIXED + empirically verified on live codex

`cs terminal write --submit codex` now submits. Root cause was NOT a wrong
chord byte: codex's Enter IS a plain CR, but codex coalesces a single
`text + CR` write into a PASTE BURST and treats the trailing CR as a literal
newline (exactly @@Alex's "command then a new line, no submit"). The fix wraps
codex's text in explicit bracketed-paste delimiters so the trailing CR after
the paste-end marker is read as a distinct Enter keypress.

The working encoding (one write):  ESC[200~ <text> ESC[201~ CR
  i.e.  "\x1b[200~" + text.trim_end('\n') + "\x1b[201~\r"

## How I found it (empirical, per @@Alex)

Spawned an isolated codex tab (codex-cli 0.136.0, authed), drove it via
`cs terminal write` and read its rendered screen by replaying the scrollback
ring through pyte (codex draws in place; raw ANSI is unreadable). Recon of the
ring showed codex enables bracketed paste (ESC[?2004h) AND the kitty keyboard
protocol with flags ESC[>7u (1+2+4, NOT flag 8) -> an unmodified Enter stays
the legacy CR, so the byte was never the problem.

  input (one write)                       | result
  ----------------------------------------|--------------------------
  text + \r            (the old chord)     | NO submit - parks as newline
  text + \x1b[13u      (kitty CSI-u Enter) | NO submit - parks
  text + \x1b[27;9;13~ (claude chord)      | NO submit - consumed, parks
  text, then \r as a SEPARATE write        | SUBMITS
  \x1b[200~ text \x1b[201~ \r  (one write) | SUBMITS  <- the fix

## Files changed (my lane only)

  crates/chan-shell/src/submit.rs       blob eb3c2a4ce9f04ba7a042584534e8952f7e271a46
  crates/chan-shell/src/cli.rs          blob abc3bb8ca579ca05930e5002ba6e0cecffc05e42
  web/src/terminal/submitMode.ts        blob 049eec11be4e5d80816c96b994c8e6b43b31d5d0
  web/src/terminal/submitMode.test.ts   blob 192695304716c726b989acd2a58215364f9196c3

- submit.rs: apply_submit_chord wraps codex (claude/gemini unchanged: plain
  suffix chord). codex's submit_chord() still returns "\r" (its Enter byte);
  the wrap is the delivery. Enum + fn docs updated with the 2026-06-02 probe.
  Tests updated (codex wrap + interior-newline preservation).
- cli.rs: `--submit` help text - codex = "bracketed-paste wrap + CR".
- submitMode.ts: encodeForAgentSubmit mirrors apply_submit_chord byte-for-byte
  (codex branch wraps). AGENT_SUBMIT_CHORDS map unchanged (still the Enter
  bytes, codex:"\r").
- submitMode.test.ts: codex expectation -> wrapped form + multi-line case.

## Own-gate (scoped) - GREEN

  cargo fmt -p chan-shell --check                          PASS
  cargo clippy -p chan-shell --all-targets -D warnings     PASS
  cargo test -p chan-shell                                 PASS (34)
  npm test (full vitest)                                   PASS (1646)
  npm run check (svelte-check)                             0 errors
  npm run build                                            OK

Note: whole-tree `cargo fmt --check` is RED, but only in
crates/chan-workspace/src/{fs_ops,workspace}.rs (@@LaneD's B11 WIP), not my
files. Reported scoped-clean per the isolated-gate model.

## Empirical proof

Fresh ./target/debug/chan (with the fix) run against the live desktop control
socket: `cs terminal write --tab-name=@@CodexProbe --submit=codex
$'echo CHAN_VERIFY_FINAL'` -> codex submitted and ran it ("Ran echo
CHAN_VERIFY_FINAL"), composer parked-count = 0. (The chord is computed
client-side, so this verifies the real path without restarting the desktop
server.) Probe tab + /tmp artifacts torn down.

## One flag for you (cross-lane, NOT blocking)

crates/chan-server/src/routes/team_config.rs::submit_chord_literal (used only
by render_poke_chords -> the bootstrap.md "- codex: --submit=codex (chord \r)"
bullet) is a 3rd, DOC-ONLY mirror of the chord map. Still technically accurate
(codex's Enter byte is \r) but the "(chord \r)" parenthetical no longer tells
the whole story now that codex needs the paste-wrap. It's outside my lane
(routes/), purely cosmetic, and affects only generated bootstrap docs - so I
did not touch it. Your call whether to route a one-line tweak (to me or
@@LaneD) or leave it.

## Status

Holding for Wave-2 (B1 + B4) dispatch, per task-LaneA-LaneB-1 "After B8: Hold"
(you said you'd sequence B4's control_socket.rs region vs @@LaneD's B5).
