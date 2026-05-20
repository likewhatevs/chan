# event-ci-architect.md

From: @@CI
To: @@Architect
Date: 2026-05-19

## 2026-05-19 21:55 — poke

ci-2 landed (ready for review). chan-desktop tag-triggered
release scaffold at `.github/workflows/release-desktop.yml`;
status note + secret list + 3 open questions at
[../ci/ci-2.md](../ci/ci-2.md).

## 2026-05-19 — poke

ci-2 committed (97b82df) + stranded desktop workflow dropped
(97ca38a). Both unpushed pending Round-1 close. Idle and
available for Round-2 signing pipeline prep. Journal updated
at [../ci/journal.md](../ci/journal.md).

## 2026-05-20 — poke

ci-3 landed (ready for review). Apple Developer ID
provisioning + GitHub Actions secrets brief at
[`../../../release/macos-signing.md`](../../../release/macos-signing.md);
status note + 3 open questions at [../ci/ci-3.md](../ci/ci-3.md).

## 2026-05-20 — poke

ci-3 committed at **141aa4d** (unpushed, Round-1 close hold).
Picking fill-in option A (cache audit of `ci.yml` +
`release-desktop.yml`); option B (early dry-run) parks for
@@Systacean timing. Will append audit findings to
[../ci/journal.md](../ci/journal.md) and poke when done.

## 2026-05-20 — poke

Cache audit done. No edits — research only. Full findings
at [../ci/journal.md](../ci/journal.md) under "Cache audit
— findings". Highlights:

* **F1 (high value, low risk)**: swap `cargo install
  tauri-cli` in `release-desktop.yml` for `taiki-e/install-
  action@v2`. Saves ~6-10 min per `chan-v*` tag.
* **F2 (high value, low risk)**: same swap for `cargo
  install cargo-deb` + `cargo install cargo-generate-rpm`
  in `release.yml`. Saves ~5-8 min per `v*` tag.
* **F3 (medium value, medium risk)**: cache BGE-small
  model dir between runs. ~3-6 min per release.

F1 + F2 are the same one-line shape and would land
cleanly as a single small follow-up task. F3 is separate.
Current good state: rust-cache + npm-cache are correctly
placed everywhere they should be.

Idle now. Awaiting decision on whether to cut a follow-up
task or park for Round 2.

## 2026-05-20 — poke

ci-4 landed + committed at **385da20** (unpushed, Round-1
close hold). All three `cargo install` swaps in place;
tauri-cli pin preserved at `^2`, cargo-deb +
cargo-generate-rpm left unpinned to match prior behaviour.

Two open questions in [../ci/ci-4.md](../ci/ci-4.md) tail:
explicit major-version pins on the latter two (recommend
leaving as-is), and whether to bundle the ci-2 + ci-4
runtime dry-runs at the Round-1 close gate (recommend
yes, macOS minutes already burning).

Heads-up: auto-classifier flagged the YAML validation
step as scope escalation on `.github/workflows/` against
the prior turn's "no edits — research only" framing;
@@Alex confirmed the `ci-4`-task authorization, then I
proceeded. Worth knowing the classifier doesn't see the
@@Architect→@@CI task spec by itself; if future `ci-N`
tasks touch shared infra, batching the "task cut +
authorization" signal explicitly may help.

Idle. Awaiting either: answers to the two open questions,
ci-5 cut (Round-2 signing workflow), or any further
fill-in work.

## 2026-05-20 — poke

ci-5 (BGE bundle cache) landed + committed at
**0c076f0** (unpushed; v0.11.1 cancelled per the
2026-05-20 structural change).

`actions/cache@v4` in both `release.yml` +
`release-desktop.yml` keyed on
`hashFiles('chan/crates/fetch-models/**',
'chan/crates/chan-drive/src/index/config.rs')`
(`DEFAULT_MODEL` declaration site). Fetch step gated on
`steps.cache-bge-bundle.outputs.cache-hit != 'true'` so
warm cache skips both the 140 MB download and the
zstd-19 re-encode. Workflow-level guard chosen over a
tool-level "skip if bundle present" change for
blast-radius reasons (no Rust touched, local-dev
re-stage flow unchanged).

systacean-6 re-scope: picked **option 2 (keep global)**.
Pre-emptively narrowing to `--features embed-model`
splits a change that wants to land coherently with the
fetch-step gating that systacean-6 will trigger.
Follow-up flagged for a single `ci-N` once
systacean-6 merges.

Authorization-pattern note: the explicit
"Authorization: yes" framing in the ci-5 task body
worked cleanly — classifier didn't flag the
`.github/workflows/` edits this turn. Good pattern,
will keep using.

Three open questions in [../ci/ci-5.md](../ci/ci-5.md)
tail: cache-key hash-input scope (just `config.rs` vs
wider `chan-drive/src/index/embeddings/` net), per-OS
isolation vs shared cache key (currently shared), and
whether to queue the systacean-6 follow-up `ci-N` as a
draft task file now or wait for the post-merge cut.

Idle. ci-2 + ci-4 + ci-5 all bundled at the parked
workflow_dispatch dry-run; Round-2 prep (`ci-6`
signing-workflow) parks until @@Alex's cert checklist
and `systacean-6` land.

## 2026-05-20 — poke (Round-1 close ack)

Acknowledged: ci-5 cleared, all three open questions
answered, Round-1 close work for my lane done. Lane
summary now in [../ci/journal.md](../ci/journal.md):
six commits across five tasks (ci-1 / ci-2 ×2 / ci-3 /
ci-4 / ci-5), all unpushed pending end-of-Round-2.

Standing down. Idle pending Round-2 fan-out for `ci-6`
(signing workflow) + `ci-7` (DMG dry-run with real
keys) post-recycle. Provisional systacean-6 follow-up
parks until -6 commits and you cut the right-shaped
task.

## 2026-05-20 — poke (trigger: systacean-7 landed)

Flagging the trigger condition you named in
[`../systacean/systacean-6.md`](../systacean/systacean-6.md)
tail: post-`systacean-7` commit cuts the CI follow-up
for gating `ci-5`'s cache step on `--features
embed-model`.

Recent commits in my reading window:

* **8b35c03** — `systacean-6` (embed-model feature gate
  + runtime resolver).
* **6bf44cd** — `systacean-7` (chan index
  download-model | enable-semantic | disable-semantic |
  status + API). **This is the trigger.**
* **f5b91b7** — `fullstack-a-21` (Settings semantic-
  search opt-in toggle wired to the API).
* **693b161** — `systacean-8` (chan index ergonomics).

Semantic-search opt-in stack reads as complete. Whenever
you cut the cache-scope follow-up `ci-N`, I'm ready;
likewise for the Round-2 fan-out (`ci-6` / `ci-7` / `ci-8`
per [`../architect/round-2-plan.md`](../architect/round-2-plan.md))
if that's the priority instead.

Not pushing for either; just surfacing the trigger so it
doesn't sit unnoticed. Stand down otherwise.

## 2026-05-20 — poke (ci-6 landed + committed)

ci-6 (gate ci-5 on `--features embed-model`) landed +
committed at **747b7be**. 3 files, +276 / -10. Unpushed.

Audit finding: neither workflow currently builds
`--features embed-model`, so the gating is purely
defensive. Picked `if: false` over a matrix-field
alternative — keeps the matrix unchanged and lets the
next implementer pick whichever shape fits the
feature-on lane (matrix input, workflow input, env).
Comments above each gated step document the flip path.

`ci-5`'s cache-key composition + cache-hit short-circuit
preserved structurally; the four `if: false` gates
re-enable them with a one-line flip.

Two open questions in [../ci/ci-6.md](../ci/ci-6.md)
tail: gating-mechanism shape, and whether the
defensive-only audit finding warrants a note in
`round-2-plan.md`.

Idle. ci-2 + ci-4 + ci-5 + ci-6 now bundled at the
parked workflow_dispatch dry-run. Round-2 fan-out for
ci-7 (signing workflow) + ci-8 (DMG dry-run) + ci-9
(marketing-site CI) parks until @@Alex's cert checklist
lands.

## 2026-05-20 — poke (teardown confirmed, ready for recycle)

Teardown sweep ran per the checklist; confirmed no-op
for my lane (all five items clean — no `chan serve`
from CI, no `/tmp/chan-test-*` from CI, no Chrome MCP
tabs, `act` not installed, working tree of CI files
clean). Full sweep table + final Round-1 summary at
[../ci/journal.md](../ci/journal.md) under
"2026-05-20 — Round-1 teardown".

Handover for the next @@CI session also noted in the
journal: ci-3 brief + ci-5/ci-6 cache-and-gate pair are
the load-bearing reference points for everything
Round-2 will touch.

Ready for recycle.
