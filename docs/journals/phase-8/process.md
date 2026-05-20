# Working on chan — Phase 8 process

Inherits the phase-7 process verbatim, with three deltas:

1. **Round shape locked**: Round 1 = bug sweep + new build. Recycle
   between rounds. Round 2 = features per
   [`next-phase-backlog.md`](../phase-7/next-phase-backlog.md).
2. **@@CI is a new working agent** (6th slot) standing up in
   parallel with Round 1 to land GitHub Actions + signing
   infrastructure ahead of the phase-8 north star (notarized DMG).
3. **North star**: ship a notarized macOS `.dmg` (and signed
   Windows + Linux equivalents) that users can download and
   install without Gatekeeper / SmartScreen friction. Coordinated
   work spans @@CI + @@Systacean + the FullStack lanes for
   chan-desktop bundling.

Everything else (event protocol, recycle mechanics, permission
mechanics, survey shapes, lane boundaries, atomic-write contract,
test-server URL hand-off, commit coordination) carries forward
from [`../phase-7/process.md`](../phase-7/process.md). Read that
first if you have not.

## Roster (phase 8)

| Tag             | Profile                                                      |
|-----------------|--------------------------------------------------------------|
| @@Architect     | Plan, dispatch, decisions, phase journal.                    |
| @@FullStackA    | Backend + frontend. Axum, Svelte, editor, terminal.          |
| @@FullStackB    | Same profile as A; second lane for queue depth.              |
| @@Systacean     | Syseng + Rustacean. CLI, build, deps, toolchain, indexer.    |
| @@CI            | CI infrastructure: GitHub Actions, signing, release pipeline.|
| @@WebtestA      | Authoritative walkthrough lane A.                            |
| @@WebtestB      | Authoritative walkthrough lane B.                            |

### Lane boundaries

@@CI owns the CI plumbing (workflows, build matrix, signing,
notarization, release artifacts). @@Systacean continues to own
in-tree code quality (CLI, drive, indexer, pinned toolchain).
Boundary heuristic: if it lives in `.github/workflows/` or talks
to GitHub Actions secrets, it is @@CI. If it lives in `crates/`,
it is @@Systacean. Edges:

* Signing-key rotation that touches `desktop/src-tauri/` config
  is @@CI's call; the in-tree `desktop/CLAUDE.md` update is a
  shared edit.
* Release-tag automation (on `chan-v*` → build + notarize +
  upload) is @@CI. The release-process documentation
  (`docs/release.md` or equivalent) is @@Architect-led with @@CI
  review.

Other lane boundaries unchanged from phase 7. Test-server +
walkthrough audit trail remains the authoritative validation.

## Round 1 → Round 2 transition

When Round 1 closes:

1. @@Architect appends a Round-1 close note to
   `architect/journal.md` summarizing what landed and what was
   deferred.
2. @@Systacean cuts the patch release (likely v0.11.1) per the
   commit-coordination protocol.
3. @@Architect fires `agent-recycle` events for each working
   agent via `alex/event-architect-alex.md`, each linking to the
   handover entry in the agent's own journal.
4. @@Alex closes Round-1 sessions and opens fresh Round-2
   sessions using the bootstrap prompt
   (`docs/agents/bootstrap.md`).
5. Round 2 kicks off with @@Architect cutting fresh task files
   from the backlog.

## Skill mapping (phase 8)

Same as phase 7 plus @@CI:

| Agent       | Skills                                            |
|-------------|---------------------------------------------------|
| @@Architect | architect                                         |
| @@FullStack | webdev + rustacean + pythonic                     |
| @@Systacean | syseng + rustacean                                |
| @@CI        | syseng + rustacean (CI-focused; same blend as     |
|             | Systacean, different scope: CI infra vs app code) |
| @@WebtestA  | webdev                                            |
| @@WebtestB  | webdev                                            |

Contact cards live under `docs/agents/<tag>.md`; skill copies
under `docs/agents/<tag>/skills/`.
