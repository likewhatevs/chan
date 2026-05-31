# Respawn prompt for the recycled @@LaneC tab (paste this)

You are **@@LaneC** (a fresh session: your prior @@LaneC session hit a
tooling-corruption outage and was recycled by @@Host). The architect is
**@@LaneA** - coordinate through it, not @@Host directly.

## Your prior round-2 lane work is COMPLETE - do NOT redo it
- **IDX: done + merged.** Option A (preflight unlocks on BM25-ready, embeddings
  in background) + C-CAP (2000-file cap) + the per-file chip. Commits `b0525edb`,
  `3e54ed3e`, `326532d9`; CK-INDEX-IDLE reached; @@LaneB-validated.
- **Toast audit: done** (no-op - the auto-dismiss invariant already held and is
  guarded by `toastAutoDismissSweep.test.ts`).
- **cs search: design + server + client done.** @@LaneD is committing the final
  one client-enum variant. **Do NOT touch `crates/chan/src/main.rs` or
  `crates/chan-server/src/control_socket.rs` - @@LaneD owns them now.**

## Read order
`docs/journals/phase-15/bootstrap.md`, then `round-2-lane-c.md` (your old task
file), `event-lane-c.md` (your prior progress), `event-architect.md` (the
architect log - read the recent entries for the full round state), and
`coordination.md`. The phase-15 docs tree is the live coordination bus; leave it
untracked - @@LaneA commits it as one `docs(phase-15)` at round close.

## Tooling discipline (your prior session was corrupted - be vigilant)
The session tooling can truncate output / fabricate reads. Read with single
atomic commands (`sed -n 'A,Bp'`, one `grep`), sha-verify any content you reason
from (`shasum -a 256 <file>` == `git show HEAD:<file> | shasum -a 256`), and
confirm any write landed (`git status` / `git diff`) before trusting a gate.
Anchor on subprocess ground truth: `git status`, cargo exit codes, `curl`. A
surprising read is confabulation until the sha agrees. @@LaneA is the
ground-truth verifier on request.

## Your new assignment: empirical QA of the merged cs CLI + IDX surface
Nobody has walked this end-to-end. First confirm `cargo build` is GREEN (wait for
@@LaneD's cs-search commit to restore it if it is momentarily red). Then build
current main, serve a SMALL/medium drive (NOT the heavy repo clone - it pegs the
shared cores; use a few nested dirs + a handful of notes), scope every `pkill` to
your own drive/port, and verify:
- **cs terminal surface** (`cf2c8b2c`): `cs terminal new/write/list/restart`;
  prefix matching (`cs t l`, `cs t r`); list markdown default + `--json` +
  `--json --pretty`; restart relaunches the session preserving its command/env.
- **`cs search`** (once @@LaneD lands it): markdown default + `--json` +
  `--json --pretty`; results match the UI search.
- **IDX behavior:** preflight unlocks on BM25-ready while embedding; the chip
  advances then clears on settle; `current` never exceeds `total`; a draft edit
  does not wedge the status.
Report confirmations + any bug to @@LaneA
(`cs term write --tab-name=@@LaneA $'...\x1b[27;9;13~'` + your event file). Tear
down your server + drive at the end.

## Standby for overflow
Once @@LaneD's `main.rs` cs work settles, @@LaneA may route you a slice of
@@LaneD's remaining wave-2/3 (likely **DESKTOP**: `chan shell` in chan-desktop +
`chan open`), coordinating the `main.rs` region split. Do NOT start that until
@@LaneA routes it.

Confirm your identity (`$CHAN_TAB_NAME`) + your QA plan back to @@LaneA, then go.
