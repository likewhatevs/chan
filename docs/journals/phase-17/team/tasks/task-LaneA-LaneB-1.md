# task-LaneA-LaneB-1: B8 - cs --submit codex chord

From: @@LaneA  To: @@LaneB  Wave: 1 (isolated, start now)

## Objective

`cs terminal write --submit codex` writes the text + a newline but does NOT
submit on a live codex. @@Alex: "it writes the command followed by what looks
like a new line, and does not submit; fix it, you can test locally with naked
write and the submit sequence if any - def not the current \r". Find the chord
that actually submits codex and fix it.

## Spec source

- docs/journals/phase-17/round-1/draft.md (the B8 "cs terminal write --submit
  codex" bullet).

## Anchors (re-verify against HEAD; lines drift)

- crates/chan-shell/src/submit.rs ~54-78: the chord map; codex is "\r" today.
- web/src/terminal/submitMode.ts ~19-23: the TS mirror of the same chord map.

## Method (empirical repro FIRST, per @@Alex)

1. Build a FRESH binary before testing (rust-embed + stale-binary false
   positives have burned us): npm run build in web/, then cargo build -p chan,
   and spawn a real codex terminal to probe against. The team roster is all
   `claude`, so you must spawn codex yourself (`cs terminal new` with a codex
   startup command, or ask @@LaneA if codex is not installed on this machine).
2. Reproduce: naked `cs terminal write --tab-name=<codex-tab> $'echo hi'` (no
   --submit) parks the text; confirm it does NOT submit.
3. Probe candidate chords by writing raw bytes after the text and watching
   whether codex submits: plain CR `\r`, LF `\n`, CRLF, the xterm
   modifyOtherKeys Cmd+Enter CSI `\x1b[27;9;13~`, Ctrl+J `\n`, etc. Identify
   the ONE that submits codex hands-free.
4. Fix BOTH the Rust source (submit.rs) AND the TS mirror (submitMode.ts) in
   lockstep - they must stay byte-for-byte in sync (see bootstrap "Shared-file
   contention": submit.rs/submitMode.ts byte-for-byte). Update the inline
   live-probe comment with today's date + what you observed.

Note: bootstrap flags B8 as related to B5 (codex startup) which @@LaneD owns
(MCP env off by default - codex's MCP failures). Keep your B8 fix to the submit
chord only; coordinate notes with @@LaneD via @@LaneA if your codex probing
surfaces anything about codex startup/MCP.

## Gate (own-gate before reporting done)

- Rust: cargo fmt --check + cargo clippy -p chan-shell --all-targets
  -D warnings + cargo test -p chan-shell.
- Frontend: make web-check + svelte-check + npm run build (submitMode.ts is
  consumed by the SPA).
- Empirical: demonstrate `cs terminal write --submit codex` now submits on the
  live codex you spawned. Record the working chord + bytes in your journal.

## Report

When done, cut tasks/task-LaneB-LaneA-1.md (summary + the working chord +
own-gate-green + pathspec sha for submit.rs + submitMode.ts) and poke @@LaneA.

## After B8 (do NOT start yet)

Wave-2 = B1 (rich prompt per-terminal) + B4 (cs pane split/close). B4 touches
crates/chan-server/src/control_socket.rs (pane-exec region) which shares the
chan-server crate with @@LaneD's B5 (spawn-options region) - I will sequence
that to avoid a shared-crate compile window. I'll dispatch Wave-2 as a separate
task after you report B8. Hold.
