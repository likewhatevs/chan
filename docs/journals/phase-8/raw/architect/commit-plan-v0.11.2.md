# Commit plan — chan-v0.11.2 mini-wave

Author: @@Architect
Date: 2026-05-21

Status: **active**. Tag-cut target after the 9 task commits land
+ ride alongside the already-landed Round-2 Wave-1 work. Post-tag
sequence is the `ci-8` DMG dry-run + session recycle.

## Why a patch

Round-2 Wave-1 is the signed-DMG north star (target v0.12.0,
end of Round 2 — weeks away). Between v0.11.1 cut + the eventual
v0.12.0, @@Alex dogfooded v0.11.1 and surfaced a cluster of UX
bugs that erode daily-driver feel:

* **CRITICAL**: editor falsely surfaces "File moved or deleted"
  during active writing. Interrupts concentration on every
  occurrence.
* **DEV META-BLOCKER**: chan-desktop's tab right-click menu's
  Reload + Open Inspector entries no-op. Without devtools,
  @@Alex can't inspect / debug the other desktop-native bugs.
* Multiple smaller paper-cuts (notification UX, FB state, list
  handling) that accumulate.

A small patch wave with these fixes — same shape as the
rich-prompt mini-wave that produced v0.11.1 — ships the fixes
in days instead of weeks. @@Alex approved 2026-05-21 (A.5
"yes"); also asked to **maximally pack** anything well-defined
into the patch given the working agents have been mostly idle
this session.

## Scope — 10 fixes across 9 tasks

Maximally packed per @@Alex's directive. All items have clear
answers + small-to-medium scope. Anything needing significant
design choice (survey-reply broadcast 3 options unresolved;
Cmd+O rebind + open-file dialog with terminal-selection parser;
markmap library integration) stays for Round-2 wave-2.

### @@FullStackA — 6 tasks

| Task   | One-liner                                                              | Severity     |
|--------|------------------------------------------------------------------------|--------------|
| `-a-36` | Tab right-click Reload + Open Inspector (SPA dispatch + runtime detect) | DEV META-BLOCKER (paired with `-b-17`) |
| `-a-37` | File moved or deleted false-positive (stop + fix Re-open + Find-suggest) | CRITICAL                              |
| `-a-38` | Notification surface polish (spinner 0:00 gating + Copied path auto-dismiss) | Medium                                |
| `-a-39` | FB tab state polish (expand persistence + spawn-new chord behaviour)   | Medium                                |
| `-a-40` | Wysiwyg outline-style dotted numbering (CSS counters per A.7 option a) | Feature                               |
| `-a-41` | Source-mode editor list intervention (strip list keymaps from source mode) | Paper-cut                             |

### @@FullStackB — 3 tasks

| Task   | One-liner                                                              | Severity     |
|--------|------------------------------------------------------------------------|--------------|
| `-b-17` | Tab right-click Reload + Open Inspector (Tauri IPC + accelerator bindings) | DEV META-BLOCKER (paired with `-a-36`) |
| `-b-18` | Submit-mode persistence on reload + shell-mode tooltip copy fix        | Medium                                |
| `-b-19` | chan-desktop browser-style zoom (Cmd + / - / 0 + persist in WindowConfig) | Feature                               |

### Pre-landed Round-2 Wave-1 work that rides v0.11.2

Already committed locally (uncommitted-but-cleared at the time
of this plan; agents poke to commit on next inbound poll):

* `fullstack-b-15` — `bundled_chan_path()` + exact-match version
  probe in chan-desktop.
* `fullstack-b-16` — `resolve_chan_binary()` PATH-first probe
  (consumes -b-15).
* `ci-7` — tag-triggered signed + notarized chan-desktop workflow
  YAML.
* `systacean-11` — JSON rotation for `tauri.conf.json` to the
  release Developer ID identity (`Developer ID Application:
  Alexandre Fiori (W73XV5CK3N)`).
* `systacean-12` — `tauri-plugin-updater` cross-platform
  verification.
* `systacean-13` — Keychain-driven `make app-notarized`
  (split build from notarize per tauri-bundler 2.8.1
  constraint).

All ride the v0.11.2 tag-cut bundle. Total commit count for
the patch: ~9 new + ~6 pre-landed + various docs commits =
**~18-20 commits in the v0.11.2 set**.

### Wave-1 work that does NOT ride v0.11.2

* `ci-8` — DMG-on-tag dry-run with real Apple Developer ID keys.
  Fires AFTER v0.11.2 tag against the NEXT tag (or workflow_dispatch
  manually). Validates the signed-DMG pipeline end-to-end. Gated
  on @@Alex completing B.2 (six GH Secrets populated per
  [`populate-apple-secrets.sh`](../../../release/populate-apple-secrets.sh)).

## Critical-path sequencing

```
[parallel] -a-36 + -b-17 (DEV META-BLOCKER — pair)
              ↓ (unblocks DevTools on chan-desktop for everything else)
        -a-37 (CRITICAL file-moved-or-deleted)
              ↓
[parallel] -a-38, -a-39, -a-40, -a-41, -b-18, -b-19
              ↓
[Wave-1 pre-landed commits absorb into the bundle]
              ↓
@@Systacean cuts chan-v0.11.2 tag + pushes
              ↓
release-desktop.yml fires (unsigned matrix; signed lane blocked
on B.2 secrets)
              ↓
@@WebtestA + @@WebtestB walk the cut binary
              ↓
[asynchronously] @@Alex runs populate-apple-secrets.sh →
ci-8 fires manually via workflow_dispatch on the NEXT
chan-v* test tag → real signed DMG validates end-to-end
              ↓
Session recycle → Round-2 wave-2 (Hybrid back-side refactor +
rich-prompt session evolution + other accumulated bugs)
```

### Wait-pattern shapes

* `-a-36` waits on `-b-17` landing so the SPA dispatch can
  invoke the Tauri IPC commands. Both ship together as the
  paired task.
* `-a-37` waits for DevTools unblock (so @@FullStackA can
  inspect the file-watcher / self-writes paths root cause).
* `-a-38..-41` are parallelizable; commit independently as
  ready.
* `-b-18` + `-b-19` are independent of the `-b-17` pair.

## Tag-cut sequence (when ready)

1. **Gate check**: all 9 task commits in HEAD + pre-landed
   Wave-1 commits in HEAD + @@WebtestA + @@WebtestB green on
   the cut binary (or @@Alex's "cut it" signal accepts
   post-tag walkthroughs).
2. **Pre-push gate workspace-wide** per CLAUDE.md.
3. **Version bump**: `0.11.1` → `0.11.2` across the five
   manifests: workspace `Cargo.toml`, `Cargo.lock` refresh,
   `desktop/src-tauri/tauri.conf.json`, `web/package.json`,
   `web/package-lock.json`.
4. **Single release commit**: `chan v0.11.2`.
5. **Annotated tag**: `git tag -a chan-v0.11.2 -m <body>` —
   body listing the 9 task scopes + the pre-landed Wave-1
   work that rides.
6. **Push**: `git push origin main --follow-tags`.
7. **CI**: `release.yml` + `release-desktop.yml` fire on
   tag; unsigned matrix produces dogfood binaries. Signed
   lane only fires once B.2 secrets are populated.

## For future @@Architect (session-recycle hand-off)

If @@Architect's session has been recycled and a fresh one
picks up this plan, here's the at-a-glance state:

### Canonical artifacts to read on bootstrap

1. **[round-2-plan.md](round-2-plan.md)** — Round-2 plan
   with all locked decisions at the head + the new
   "Hybrid back-side revisited" section + Wave-2 dispatch
   table. The plan-level reference.
2. **[round-3-plan.md](round-3-plan.md)** — Round-3 plan
   (open-source flip + multi-model picker + polish wave).
   Includes Track 5 (per-agent submit-chord encoding map)
   as a locked carry-over from v0.11.1.
3. **[round-2-open-questions.md](round-2-open-questions.md)**
   — running index of architect-side questions + blocking
   actions. Section A is plan decisions; section B is
   hands-on @@Alex tasks. Most questions resolved 2026-05-21;
   B.2 (GH Secrets) is the load-bearing remaining gate.
4. **[journal.md](journal.md)** — canonical decisions log.
   Latest entry: 2026-05-21 Hybrid back-side revisited.
5. **[../phase-8-bugs.md](../phase-8-bugs.md)** — bug audit
   anchor. Many entries from this session: file-moved
   false-positive, FB expand state, etc. Most are
   v0.11.2-dispatched per this plan.

### Standing state at plan-write time

* **v0.11.1 cut + pushed** at `2c6680b` 2026-05-20.
* **Round-2 Wave-1 fanned out** 2026-05-20: signed-DMG
  pipeline track + bundled-chan-binary track. All 6 tasks
  landed locally (uncommitted in working tree pending each
  agent's commit step).
* **Round-2 decisions all locked** 2026-05-20 (sequencing,
  item-6 hosting = GitHub Pages, item-7 layout = PATH-first
  with bundled fallback + version match, PIN hash = SHA-256
  + salt, manual home = `docs/manual/`, first-release
  version = v0.12.0).
* **Hybrid back-side revisited** locked 2026-05-21 (per-type
  config surfaces; FB back = drive inspector data; Graph
  back = colour legend; theme = global per-type;
  Wave 2 = Task A architecture refactor; Wave 3 = Tasks
  B/C/D/E).
* **v0.11.2 patch wave** dispatched 2026-05-21 per this plan.

### Blocking gates still open at plan-write time

| Gate | Owner | Status |
|------|-------|--------|
| B.1 — release identity transcribed | @@Alex → @@Systacean | DONE 2026-05-21 (transcribed via [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md) "approved (transcribed by @@Architect)") |
| B.2 — GH Secrets populated | @@Alex via local script | IN PROGRESS — [`populate-apple-secrets.sh`](../../../release/populate-apple-secrets.sh) ready to run |
| B.3 — local notarization smoke test | @@Alex (optional) | OPTIONAL — does not block v0.11.2 |
| `ci-8` dry-run | @@CI | PARKED on B.2 |

### Agents at plan-write time

| Agent | State | Wave-1 queue empty after | v0.11.2 queue |
|-------|-------|--------------------------|---------------|
| @@FullStackA | Standing by | (no Wave-1 work) | 6 tasks `-a-36..-a-41` |
| @@FullStackB | -15 + -16 ready to commit | After commits | 3 tasks `-b-17..-b-19` |
| @@Systacean | -11 + -12 + -13 ready to commit | After commits | (none — Round-2 wave-2 work later) |
| @@CI | ci-7 ready to commit | After commit + B.2 + ci-8 | (none — Round-2 wave-2 work later) |
| @@WebtestA | v0.11.1 walkthrough queue | After walkthrough | v0.11.2 walkthrough on cut binary |
| @@WebtestB | v0.11.1 walkthrough queue + standing chan-desktop runtime perms | After walkthrough | v0.11.2 walkthrough + ci-8 DMG verification |

### Standing permissions (carry-over for v0.11.2)

* @@FullStackB + @@WebtestB: chan-desktop runtime
  permissions (per `ada8478` 2026-05-20). Covers
  `make run`, `npm run tauri dev`, `Chan.app` launch +
  click cycles against throwaway drives.

### What v0.11.2 wave looks like for future-me

The mini-wave dispatches just like the v0.11.1 one did:

* Architect-side: cut tasks + fire dispatch pokes (done in
  this turn).
* Working agents read inbound channels, pick up tasks,
  implement, fire ready-for-review pokes.
* Architect clears + provides commit subjects.
* Agents commit (per-file `git add` discipline per the
  multi-agent worktree).
* @@WebtestA + @@WebtestB verify on the rebuilt binary as
  each commit lands (proactive walks per the
  feedback_proactive_walks memory).
* @@Systacean cuts the tag after gate-clearance from @@Alex.

The bisect window for finding any post-tag regressions sits
inside the 9-task surface; pre-landed Wave-1 work has already
been gate-cleared earlier.

### Post-v0.11.2 session recycle

@@Alex flagged 2026-05-21 the post-v0.11.2 cadence: cut the
DMG (via ci-8 + the secrets), then recycle ALL six agents
+ architect for the longer Round-2 wave-2 coding session.
This plan-doc serves as the hand-off anchor for any recycled
architect; the bootstrap chain (architect prompt in
`docs/agents/bootstrap.md`) + this plan + round-2-plan.md
should produce a coherent picture in one bootstrap walk.

## What this plan is NOT

* A push trigger. Tag fires when ALL gates land (9 task
  commits + pre-landed Wave-1 commits + green pre-push gate +
  @@Alex's explicit "cut it" signal — per the established
  v0.11.1 pattern).
* A scope-creep gate. Bugs surfaced during v0.11.2
  walkthroughs slip to v0.11.3 (separate cut) OR roll into
  Round-2 wave-2.

## 2026-05-21 — Plan revision: v0.11.2 SHIPS SIGNED

@@CI flagged (their 2026-05-21 routing poke) that with
`ci-7` + the six GH Secrets + `systacean-11 / -13` all in
HEAD, the `chan-v0.11.2` tag will AUTO-FIRE the signed
macOS pipeline. Plan's original "v0.11.2 ships unsigned;
v0.12.0 is the first signed release" framing is no longer
accurate.

@@Architect 2026-05-21 approved option **(c)** from @@CI's
routing options: fire `ci-8` dry-run BEFORE `chan-v0.11.2`
tag cuts. Validates the sign+notarize+staple pipeline
against real keys on a pre-release test tag
(`chan-v0.11.99-dryrun.1` default) first, then the real
v0.11.2 tag cuts signed with pre-flight confidence.

### What this changes

* **v0.11.2 is the first signed release** in practice
  (not v0.12.0 as the original plan stated).
* **ci-8 dry-run** is now a HARD GATE before the v0.11.2
  tag cuts (not "fires after v0.11.2" as the original
  plan stated).
* **ci-9 patch task cut** to fix the verify-step
  regression `systacean-13` introduced (DMG-only stapling
  per Apple's canonical flow). ci-9 → ci-8 → v0.11.2
  tag is the new sequence.

### Updated critical-path sequencing

```
ci-9 (verify-step patch) lands
              ↓
ci-8 dry-run fires (chan-v0.11.99-dryrun.1) via tag push
              ↓ (if green: signed DMG produced, second-Mac
                 verified by @@WebtestB)
9 v0.11.2 task commits + pre-landed Wave-1 work all in HEAD
              ↓
@@Alex "cut it" signal
              ↓
@@Systacean cuts chan-v0.11.2 tag (signed pipeline AUTO-FIRES)
              ↓
v0.11.2 GitHub Release with signed + notarized DMG
              ↓
Session recycle (all 6 + architect)
              ↓
Round-2 wave-2 (Hybrid back-side + rich-prompt session evo +
walkthrough findings from v0.11.2 dogfooding)
```

### What v0.12.0 becomes

With v0.11.2 already signed, v0.12.0 is now "the first
Round-2-feature-track release" — bundles Hybrid back-side
refactor + rich-prompt session evolution + carousel +
Infographics + manual + BOOT + signed Linux/Windows once
those signing lanes open. Not a "first signed" milestone
anymore. v0.12.0's purpose carries forward unchanged
otherwise.

### v0.11.2 commit set updated

Added to the set:
* `ci-9` (verify-step patch) — rides v0.11.2.

The pre-landed Wave-1 work that rides the tag bundle is
unchanged otherwise. Total commit count for the patch
now: ~10 new (9 task commits + ci-9) + ~6 pre-landed +
docs commits = **~20-22 commits in the v0.11.2 set**.

## Tag draft (v0.11.2)

@@Systacean uses this at tag-cut time. Subject under 50
chars; body wrapped at 72 cols. Refine if it reads awkwardly
against the actual landed-commits list at cut time.

### Subject

```
chan v0.11.2
```

### Body

```
First signed + notarized release. UX bug-fix wave from
v0.11.1 dogfooding plus the Round-2 signing pipeline
landing.

User-visible fixes
==================

Editor + writing flow:
* "File moved or deleted" panel no longer falsely surfaces
  while the file is on disk; Re-open button restored; Find-
  suggest-reopen inline UX on legitimate moves.
* Source-mode editor stops auto-continuing lists (raw mode
  is now raw).
* Wysiwyg outline-style dotted numbering for nested
  numbered lists (1. / 1.1. / 1.1.1.).

Rich prompt + notifications:
* "Copied path" status-bar notification auto-dismisses
  after ~3 s.
* Pre-flight bubble spinner no longer stuck at 0:00 (gated
  on timing data being present).
* Submit-mode toolbar toggle now persists correctly across
  page reload (SerTab rpsm re-syncs server-side on tab
  restore).
* Shell-mode tooltip copy fixed (no longer claims to
  append a trailing newline).

File browser:
* Expand/collapse state persists across tab switches.
* Spawn chord (Cmd+O / Hybrid NAV `o`) now always creates
  a new FB tab instead of focusing an existing one.

chan-desktop (Tauri):
* Right-click tab Reload + Open Inspector now work
  (previously no-op on desktop-native). Tauri IPC
  commands + Cmd+R / Cmd+Opt+I accelerators wired.
* Browser-style zoom: Cmd++ / Cmd+- / Cmd+0 zoom in /
  out / reset. Zoom level persists per-window.

Round-2 signing pipeline (build-time / CI):
* Tag-triggered signed + notarized chan-desktop workflow
  (.github/workflows/release-desktop.yml).
* chan-desktop bundles the chan binary; launch-time
  PATH-first probe with bundled fallback + version match.
* Makefile supports notarytool Keychain profile for local
  smoke tests (Apple-blessed mechanism).
* signing identity rotated to release Developer ID.

Known limitations
=================

* tauri-plugin-updater self-update flow is registered but
  not yet user-invokable (no UI hook); planned for Round-2
  wave-2.
* macOS DMG signed + notarized; Linux .AppImage / .deb /
  .rpm + Windows MSI signing brief land in Round-2 wave-2.

Audit trail at docs/journals/phase-8/architect/commit-plan-v0.11.2.md.
```

### Tag command

```bash
git tag -a chan-v0.11.2 -F <(cat <<'EOF'
<body from above>
EOF
)
git push origin main --follow-tags
```

(Use `-F` with a tempfile if the heredoc shell-substitution
trips on embedded single quotes, per the v0.11.1 tag-cut
pattern.)
