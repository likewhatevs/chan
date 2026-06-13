# new-team-2 — round 1 plan

Authored by @@Lead (new-team-1) on 2026-06-12, approved by @@Alex.
Scope ratified via `cs terminal survey`: **six items + the full
phase-23/22 carryover backlog (incl. the ctx-pass refactor)**.

This plan is self-contained: the lead of this team (@@Conductor) cuts
tasks from it without needing the authoring session. Detailed designs
live in `new-team-2/designs/`; read your lane's design doc(s) before
the first edit. Baseline: main @ 3ebee587 (v0.32.0) + e0ec0d3c.

## IMPORTANT — process override

The generated `bootstrap.md` of this team was produced by the current
binary, whose "Reaching the host" text predates item 5. This plan
OVERRIDES it: **@@Conductor communicates with @@Alex via
`cs terminal survey` whenever possible** — decisions, status checks,
smoke requests, round close — not just decisions. Target a tab the
host's window owns (the lead's own tab works:
`--tab-name=@@Conductor`); `--tab-name=@@Alex` fails when the host
has no member tab. Workers still route everything through the lead.

## Scope

| # | item | lane |
|---|------|------|
| 1 | Editor tab-switch: raw markdown until click + scroll reset | @@Editor |
| 2 | Rich Prompt queue visibility (keep message until consumed; depth badge) | @@PromptQueue |
| 3 | Teams start with broadcast OFF | @@TeamFlow |
| 4 | Clicking a terminal tab must focus the terminal | @@Editor (lands FIRST — load-bearing for item 1) |
| 5 | Survey-first host comms + X-dismiss key + bootstrap template | @@TeamFlow |
| 6 | Launcher Open: always enabled, auto-turn-on, failure dialog with reason | @@Desktop |
| B1 | chan-server threaded-state ctx-pass param refactor | @@CtxPass |
| B2 | dispatch-to-matcher-loop shortcut refactor (behavior-risk; stretch) | @@Editor |
| B3 | default.json negative pin for read_dropped_paths | @@Desktop |
| B4 | Linux drop path-print investigation (likely documented no-op) | @@Desktop |
| B5 | Buried-window memory visibility (phase-22) | @@Desktop |
| B6 | GTK set_menu in-place mutation check via sdme (phase-22) | @@Desktop |
| B7 | Xcode CI selection — WATCH ITEM only (provable on next release run); no task | @@Conductor |

## Lane charters

- **@@Conductor (lead)** — task cutting (append-only, lean pokes),
  sequencing per the overlap map below, review routing, isolated-
  worktree full gates at integration points, all host communication
  (survey-first), round-close docs + retrospective.
- **@@Editor** — item 4 (small, FIRST), then item 1 (the round's
  biggest web change). Stretch: B2, but only with its own short
  design note and a behavior-preservation review. Reviews @@TeamFlow's
  web commits and @@Desktop's launcher JS.
- **@@PromptQueue** — item 2 end-to-end, server half first (it has no
  dependencies), web half second. The small Pane.svelte badge edit
  WAITS until @@Editor's Pane restructure lands. May spawn subagents
  (e.g. one for vitest coverage) — review their diffs fully before
  committing (round-1 lesson). Reviews @@CtxPass's refactor waves.
- **@@TeamFlow** — item 3 (tiny) then item 5. Then flexes to reviews /
  assists where @@Conductor routes. Reviews @@Editor's web commits.
- **@@Desktop** — item 6, then B3 (tiny), then B5/B6 (phase-22),
  B4 investigation. Owns desktop builds for everyone's WKWebView
  verification and the final smoke DMG.
- **@@CtxPass** — B1. Writes the design doc FIRST (that is why the
  refactor was deferred), gets @@Conductor's sign-off on it, then
  executes in waves (ordering in designs/backlog-ctx-pass.md).
  Reviews @@PromptQueue's chan-server half.

## Sequencing / file-overlap map

Hard ordering constraints (everything else is parallel):

1. `web/src/components/Pane.svelte`: @@Editor's item-4 fix, then
   item-1 restructure, then (and only then) @@PromptQueue's badge
   edit. Nobody else touches Pane.svelte.
2. `crates/chan-server/src/terminal_sessions.rs`: @@PromptQueue's
   item-2 server half lands BEFORE @@CtxPass touches the
   `restart` param cluster.
3. `crates/chan-server/src/routes/team_config.rs` +
   `control_socket.rs`: @@TeamFlow's item-5 template change lands
   BEFORE @@CtxPass touches `handle_team`.
4. Within item 1: item 4 first (keep-alive removes the remount-rAF
   focus that currently masks the click-focus bug for file tabs).

chan-server is THREE-lane hot (@@PromptQueue, @@CtxPass, @@TeamFlow).
Same-crate compile-window discipline: a signature change and all its
call sites land in one burst; `cargo check -p chan-server` green
before pausing; announce multi-file Rust bursts in your journal.

## Process rules (carried from round 1 + new)

- Survey-first host comms (see override above). Keys: options 1..N,
  F follow-up, X dismiss (X lands mid-round via item 5; Esc and click
  work today).
- Lean poke bus: pokes are 1-line pointers ending in the submit chord
  (`--submit=claude`); context lives in task files. ONE completion
  poke per multi-part task, after the last part.
- Tasks/journals append-only; new asks mid-task become NEW tasks.
- Commits: pathspec-atomic only — `git commit -F <msg-file> -- <paths>`
  (flags BEFORE `--`); pre-commit `git diff --staged --stat` +
  post-commit `git show --stat HEAD` verification. Commit clearance is
  standing for cleared lane work; **git push only on an explicit ask
  from @@Alex**.
- Sweeps: `rg --text --no-ignore`, no file-type filters (the sandbox
  grep shim silently skips ~130KB+ files; tests/ dirs were missed by
  filtered sweeps in round 1).
- Own-gates: scoped to your lane, but with REAL flags —
  `RUSTFLAGS="-D warnings"` on clippy AND test; web lanes run
  `make web-check` (vitest), not just svelte-check + build. Re-run the
  gate AFTER your final edit. Lane-green is reported with the commit
  sha; the lead's isolated-worktree full `make pre-push` is the
  integration gate (lanes never block on the shared tree).
- Test servers: build once, copy the binary to a renamed path (e.g.
  /tmp/docsrv), serve throwaway workspaces with
  `chan serve --standalone`; pkill scoped to your own drive path/port.
  NEVER restart the live serving binary mid-round (it kills every
  team PTY).
- Browser-smoke every Svelte reactivity change (static gates miss
  runtime errors, e.g. state_unsafe_mutation). Lanes may ad-hoc
  serve+browse for pixel checks but must tear down server+tabs.
  Multi-agent Chrome is SHARED: verify location.href before asserting.
- WKWebView (desktop build via @@Desktop) is the verification gate
  for items 1, 2, 4, 6 — Chrome smokes are necessary but not
  sufficient there. Fresh-binary provenance check before any re-walk
  of a previously-failed empirical test.
- Anchor on git status/sha when output looks truncated; never bundle
  heredoc-write + poke + verify in one Bash command.
- Pre-release: no back-compat paths, no migrations; breaking wire
  changes are fine (pin with serde(rename) + smoke runtime-validated
  strings: Tauri perms, JS invokes, route strings).
- Cross-review: every code commit gets a second-pass adversarial
  review by the paired lane (pairings in charters). Findings route
  through @@Conductor as tasks; reviewers verify behavior
  preservation, not just style.

## Verification recipes (per item; details in the design docs)

- **Items 1+4 (WKWebView gate):** long markdown doc, scroll mid,
  switch away/back → decorated instantly, scroll + caret + undo
  preserved; click a terminal tab → type immediately;
  `document.activeElement` is xterm's textarea. Plus tab drag/reorder,
  OS-file drop allowlist, flip/Hybrid-Nav, session restore with ~5
  tabs (focus lands once), ~20-tab memory sanity.
- **Item 2:** busy agent via `while true; do date; sleep 0.3; done`
  (sub-800ms gaps hold the quiet gate) → submit → text stays,
  read-only, "queued" chip after ~300ms; `cs terminal write` ×3 →
  badge climbs; Ctrl-C the loop → drains, prompt clears exactly when
  its message prints; gemini 2-write last-write rule; reload
  mid-pending → draft restores, badge re-syncs; idle fast path → no
  chip flash. Regression: `cs terminal write` stdout + cap unchanged.
- **Item 3:** spin a throwaway team on a test server → every tab's
  broadcast toggle OFF after bootstrap completes; manual toggle still
  works.
- **Item 5:** survey on the standalone test server → 1..N, F, X all
  work from the keyboard; template tests assert the new wording;
  `cs terminal survey` reply lines unchanged.
- **Item 6:** hold a workspace's flock with a second process
  (`chan serve` on it from a shell) → launcher Open on that workspace
  → dialog states the in-use reason, pill stays consistent; happy
  path: Open while off → turns on, window opens.
- **B1:** behavior preservation per wave — reviewer maps every call
  site field-by-field (round-1 standard).

## Round close

Integrated isolated full `make pre-push` green at HEAD; @@Alex
hand-smoke on a fresh desktop build (checklist: items 1, 2, 4, 6);
round report under docs/phases/ with retrospective (highlights /
lowlights / honest feedback for workers, lead, AND host); commit the
new-team-2/ bus alongside it; tear down test servers/worktrees.
No known bug ships; full gate green is the bar.
