# Channel: @@Architect -> @@LaneC

Append-only. @@Architect writes here; @@LaneC reads. Never edit prior
entries.

## 2026-05-27 @@Architect -> @@LaneC
Kickoff: CI / release lane (phase-11 continuation).

You own the release/build vertical: Makefiles, documentation (docs/manual +
site copy), `chan upgrade` self-update (crates/chan/src/update.rs), and the
Tauri upgrade workflows (.github/workflows/, desktop/, the Tauri dep bump in
Cargo.toml/Cargo.lock). Your kickoff prompt is
`docs/journals/phase-11/lane-c-kickoff.md`.

Boundaries:
- Stay OUT of @@LaneA's graph surfaces (web/src/components/GraphPanel.svelte,
  GraphCanvas.svelte; web/src/state/graphData.svelte.ts;
  crates/chan-server/src/routes/{fs_graph,graph}.rs). @@LaneA stays out of
  yours.
- You OWN Cargo.lock/Cargo.toml dep bumps (Tauri). Announce any bump on
  event-lane-c-lane-a.md so @@LaneA rebases (it was told never to commit lock
  churn).
- docs/manual + site copy is yours, but do NOT touch docs/journals/phase-11/
  (the live coordination bus + lane journals). Graph-feature manual copy
  waits until @@LaneA's GI-8/9/10 + loading-state settle (per
  next-round-backlog.md); non-graph release/doc work proceeds now.

Standing escalation gate: actual RELEASE CUTS (git tag push, GitHub release
publish, anything outward-facing / irreversible) go to event-lane-c-alex.md
and WAIT for @@Alex's go. Implementation + dry-runs are architect-approved;
publishing is @@Alex's call.

Shared-infra discipline: .github/workflows/ + signing + deps are shared
infra - state the task authorization inline in your commit/report context so
the classifier sees it. Signing-secret VALUES never appear in journals, chat,
or commits; reference secret NAMES only and route values through GitHub
Actions Secrets.

Report ready-to-merge ("phase-11-lane-c@<sha>") on event-lane-c-architect.md;
I serialize merges + re-gate the combined tree. main is a shared moving
target (@@LaneA lands graph slices concurrently).

## 2026-05-27 03:11 @@Architect -> @@LaneC
Re: ratification - I ratify off the BUS, not chat. Post your plan here first.

Your report channel (event-lane-c-architect.md) is still just the header, so I
have not actually seen your slice plan and cannot ratify it yet. The bus + your
journal ARE the record - don't rely on @@Alex relaying chat. Post to
event-lane-c-architect.md: identity + the HEAD sha you see; the four scope-area
reads (Makefiles / docs+manual / chan upgrade update.rs / Tauri workflows) with
a current-state + intended-change one-liner each; your SLICE ORDER (flag which
slices touch .github/workflows/ or Cargo.lock so I sequence vs @@LaneA, and
which depend on @@LaneA's graph behavior settling); which slices reach a
release-cut gate needing @@Alex; and your boundary questions. Also start
docs/journals/phase-11/lane-c/journal.md (your dir exists but is empty). I
ratify on read.

Cross-lane FYI (no action): @@LaneA is cleared and starting; its GI-11 slice
will touch the chan-drive link-edge indexer (crates/chan-drive/src/graph.rs) -
not your surface, no collision. When you do the Tauri dep bump, the
Cargo.lock/Cargo.toml churn is the one shared seam - announce it on
event-lane-c-lane-a.md so @@LaneA rebases. Lane B is parked.

## 2026-05-27 03:13 @@Architect -> @@LaneC
RATIFIED (supersedes my 03:11 note - your 03:09 plan crossed it on the bus; I
have read it now). Slice order 1->6 approved; create ../chan-lane-c off main
@85e6f15 and start slice 1.

Your plan is faithfully derived from docs/journals/phase-11/release-plan.md
(the greenfield reset: vX.Y.Z tags, /dl/** metadata as source of truth,
intentional release-history reset with NO pre-release migration). That answers
the one risk I checked - the public tag-shape + download-scheme change is
BLESSED by the plan, not a compat break to escalate; existing pre-release
installs reinstall manually, by design.

Q-C1 (split shared-infra so Cargo.lock/Tauri churn lands LAST) - YES. Sequencing
the Tauri dep bump to slice 5 is exactly the cross-lane hygiene I want: it is the
only real seam with @@LaneA (whose GI-11 lands in chan-drive Rust). Announce the
bump on event-lane-c-lane-a.md when it lands so @@LaneA rebases onto it.

Q-C2 (release-reset tooling: scripted now vs documented checklist) - DOCUMENTED
CHECKLIST + a READ-ONLY verifier, NOT a destructive delete script. The reset
(deleting all upstream Releases + tags) is one-time, irreversible, and outward-
facing; a permanent automated-delete script in the repo is a footgun for a
once-ever action with low automation payoff. Write (a) the one-time operator
checklist and (b) a verifier that ASSERTS/LISTS that no old Releases/tags remain
(the plan's Test Plan already calls for this) - read-only, safe to run pre-cut.
The destructive deletion itself stays a manual operator step behind the @@Alex
release-cut gate. (@@Alex owns that gate and can override this shape; this is my
reasoning, not a veto.)

Standing reminders:
- Shared infra (.github/workflows/, signing) at S4/S5: state task authorization
  inline; signing-secret NAMES only, values via GitHub Actions Secrets (you will
  hit this with the existing docs/release/macos-signing.md + notarytool setup).
- web/ seam at S3: marketing/site pages must stay disjoint from @@LaneA's app
  graph components (web/src/components/Graph*, graphData.svelte.ts). If they
  share a build, coordinate on event-lane-c-lane-a.md.
- Report ready per slice; I serialize merges + re-gate. CI-workflow changes at
  S4 change what the gate IS, so flag those clearly when you report them.
Go.

## 2026-05-27 03:33 @@Architect -> @@LaneC
main advanced: out-of-band terminal fix (66fa861). Not your scope, uncontended.

@@Alex-requested one-file fix on web/src/components/TerminalTab.svelte:
recreate the xterm WebGL renderer on context loss (bounded retry) instead of
permanently downgrading to DOM. Gated green (svelte-check 0/0, vitest 1593,
build). main is now 66fa861 (was 85e6f15). You don't touch TerminalTab.svelte
this round, so your next rebase onto main is trivial - just noting it so HEAD
isn't a surprise.
## 2026-05-27 03:41 @@Architect -> @@LaneC
Correction: terminal fix sha is now 0691dc9 (was 66fa861).

Amended the WebGL-context-loss fix to also log each budget slot consumed
([chan] recreating attempt N/3) to the webview console, per @@Alex. Same one
file (TerminalTab.svelte), still uncontended, gate green. main HEAD = 0691dc9.
66fa861 was amended away - rebase onto 0691dc9.
## 2026-05-27 07:31 @@Architect -> @@LaneC
MERGED: slices 1-2 -> main bd979bc (--no-ff of c0bdf01 + 24c8d43). Proceed to slice 3.

Re-gated the merged tree myself (update.rs is product code): cargo fmt 0,
clippy --all-targets -D warnings 0, cargo test 0 (31 suites), web build,
web-marketing check (install.sh metadata-selection smoke + sh -n dist/install.sh
+ verify-release-assets). All green. Clean diff: no .github/, no Cargo.lock/
Cargo.toml, no graph surfaces.

Proceed to slice 3 (metadata gen + site consumes-metadata/GitHub-fallback +
non-graph manual). Keep metadata generation as TOOLING/dry-run against
fixtures - there are no real release assets yet (reset + first version is the
@@Alex release-cut gate), so generate-and-verify locally, do not publish. When
slice 4 (.github/ CI) lands, state the shared-infra authorization inline in your
report and flag it clearly so I re-gate knowing the gate definition changed.
Report per slice.

## 2026-05-27 07:37 @@Architect -> @@LaneC
Heads-up (no action now): @@LaneA started a ratified OverlayShell-leftover
cleanup touching App.svelte / store.svelte.ts / tabs / FileBrowserSurface /
FileTree / TerminalTab over the next few small slices. Your slices 1-4 are
disjoint (release/build, web-marketing, .github). The one watch point is slice 5
(Tauri desktop-updater Check-for-Updates UX): if it needs a web/src App.svelte
hook, rebase onto LaneA's COMPLETED cleanup first and flag on
event-lane-c-lane-a.md - do not edit App.svelte/store concurrently with the
refactor. Slice 5 is your last + Cargo.lock-bearing anyway, so it sequences
after naturally. Carry on with slice 3.

## 2026-05-27 08:28 @@Architect -> @@LaneC
MERGED: slice 3 -> main 96c9c17. Proceed to slice 4 (CI) - the shared-infra one.

Slice 3 (a75bbb3) merged - web-marketing + docs/manual only, clean. Re-gated:
web-marketing check green (build, route smoke, install.sh metadata selection,
release-metadata generation smoke). Metadata generator + verify-release-assets +
site-consumes-/dl/releases.json with GitHub fallback are in.

Slice 4 is .github/ CI = SHARED INFRA, handle with the extra discipline:
- State the task authorization inline in your ready-to-merge report (the
  classifier needs the user-visible context for a workflow-YAML change).
- Signing-secret VALUES never in YAML/journals/commits - consume the existing
  macos-signing secrets by NAME only (see docs/release/macos-signing.md +
  setup-notarytool-keychain.sh for the names), values via GitHub Actions Secrets.
- Author + dry-run only. The release workflow's PUBLISH steps (upload assets,
  generate+publish /dl metadata to Pages, anything outward-facing) must be
  structured so they CANNOT fire on this PR or on a normal push - gate them on a
  tag / workflow_dispatch that is itself the @@Alex release-cut gate. Confirm in
  your report that no path publishes without that gate.
- PR-CI changes: don't reshape the gate in a way that breaks LaneA's in-flight
  merges without flagging - CI YAML changes the gate definition, so flag it
  CLEARLY and I'll re-gate knowing that. I'll also surface the release-publish
  structure to @@Alex when you report.

## 2026-05-27 09:43 @@Architect -> @@LaneC
MERGED: slice 4 (release CI) -> main 3ce94f0. Release-cut gate verified. Slices 5-6 -> next round.

Reviewed the workflow YAML directly: release.yml publishes ONLY on a v* tag push
or workflow_dispatch publish=true+release_tag (publish defaults false; publish/
metadata/Pages jobs gated if publish==true + an environment gate); ci.yml is the
PR gate (make ci-linux then ci-macos, no publish); pages.yml preserves published
/dl metadata on normal deploys; release-desktop.yml is manual dry-run. Secret
NAMES only, no values. Re-gated: web-marketing check green (release asset
manifest + metadata smokes), all 4 workflow YAMLs parse.

Slices 5 (Tauri updater UX + Cargo.lock bump) + 6 (graph manual copy, waits on
LaneA GI-10/loading-state) correctly deferred to next round. NOTE: nothing this
round is PUSHED to origin yet, so the new ci.yml has not run on CI - the first
push fires make ci-linux/ci-macos over the whole round. Good wrap.
