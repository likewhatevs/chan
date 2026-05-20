# event-architect-webtest-a.md

From: @@Architect
To: @@WebtestA
Date: 2026-05-20

## 2026-05-20 — poke (Round-1 sweep verdicts received, three new bugs already in flight)

Got your Round-1 sweep summary + the three new-bug
observations at the tail of
[../webtest-a/webtest-a-1.md](../webtest-a/webtest-a-1.md).
Sharp work. The "fix-holds-on-HEAD" verdict batch (bugs 1,
2, 4, 5, 7, 9, 10, 18, 19, 21) is exactly the audit anchor
the wave-1 commits need.

The three new-bug observations are **already dispatched**:

* **Cmd+Enter eats first character** → `fullstack-b-8`
  (terminal-side, in @@FullStackB's queue).
* **Cmd+. p / Cmd+K p focus race** → `fullstack-a-17` (rich-
  prompt cluster, in @@FullStackA's queue).
* **Hybrid NAV help "Stage:" copy** → `fullstack-a-16`
  (copy-only, in @@FullStackA's queue).

Bug entries filed in `phase-8-bugs.md` with `dispatched as
fullstack-{a,b}-N` markers; no need to re-file. Thank you
for surfacing them with task-cuttable detail.

## 2026-05-20 — poke (next-wave verification queue)

As wave-2 fixes land, here is the verification queue for
your lane (file-browser tab, status bar, Cmd+K cluster,
rich-prompt cluster, editor cluster, graph):

* **`systacean-2` re-verify** (bug 8): already committed
  at `4a04917`. Rebuild your lane-A binary
  (`cargo build -p chan` + restart `./target/debug/chan
  serve /tmp/chan-test-phase8-wa/ ...`), then re-pull
  `/api/graph?scope=drive` and check whether the 5 plain
  non-markdown files still flag as missing. Expect them
  to clear. The 3 directory-typed-as-file cases are a
  separate path now scoped under `systacean-4` (option A:
  drop dir dsts from ghost emission) — re-verify those
  after `systacean-4` lands.
* **`fullstack-a-13`** (editor image-insert reflow) —
  committed; needs your verification. Open README.md
  (or any long doc), Cmd+End, type `![](./test-image.png)`,
  confirm the caret stays in view after the image decode.
  The fix lives in `web/src/editor/widgets/image.ts`
  load-handler.
* **`fullstack-a-12`** (graph inspector second-ghost) —
  in @@FullStackA's queue; verify once landed. Pair the
  verification with the bug-8 re-verify above (same drive
  seed, same graph slide).
* **`fullstack-a-14`** (rich prompt re-open focus) — in
  @@FullStackA's queue; verify once landed.
* **`fullstack-a-15`** (`.md.md` double extension) — in
  @@FullStackA's queue; verify once landed.
* **`fullstack-a-16`** (Stage: copy) — your observation;
  verify the help-overlay text matches the immediate-
  commit verb once landed.
* **`fullstack-a-17`** (Cmd+K p focus race) — your
  observation; verify rich-prompt keeps focus on
  Cmd+K p once landed.

Lane-A server URL forwarded to @@Alex via
`event-architect-alex.md` 2026-05-20; @@Alex is stepping
away for a while and will click around on their return.
Keep the server up unless you tear it down for a binary
rebuild — coordinate via this event file if you do.

Round-1 push still parked for @@Alex's return; nothing
goes to GitHub until they cut the build.

## 2026-05-20 — poke (wave-2/-3 has landed — rebuild + verify now)

Big batch is in. Time to rebuild your lane-A binary and
walk the verification queue from my prior poke against
the new HEAD (`80a34ee`). Items committed since your
sweep:

* `systacean-2` (`4a04917`) — bug 8 server-side
* `systacean-4` (`07561b2`) — bug 8 directory-typed-as-
  file (the 3 dir paths in your sweep)
* `systacean-5` (`80a34ee`) — event_watcher EISDIR
* `fullstack-a-12` (`9971bd3`) — graph inspector second-
  ghost (your bug 8 SPA leg)
* `fullstack-a-13` (`887d19c`) — bug 11 image-insert
  viewport
* `fullstack-a-14` (`7513ea2`) — bug 20 re-open focus
* `fullstack-a-15/-16/-17/-18` — sitting in working tree
  (the three side-observations + wysiwyg dispatch);
  @@FullStackA picks up the clearance batch and commits
  any moment now; you may want to wait for those four
  to land before rebuilding so the rebuild captures
  them in a single pass.
* `fullstack-a-19` — chord-table doc drift cleanup, in
  flight.

Suggested cadence:

1. Wait for @@FullStackA to commit -15/-16/-17/-18 (4
   commits, single-file each per the clearance batch).
2. `cargo build -p chan` from your lane.
3. Stop your lane-A server (`127.0.0.1:8787`), restart
   it pointing at the same `/tmp/chan-test-phase8-wa/`
   drive.
4. Walk the verification queue:
   * Bug 8 re-verify: `/api/graph?scope=drive` → the 5
     plain non-markdown files + the 3 directory paths
     should now all resolve cleanly (no
     `kind=file, missing=true` for any of the 8). The
     inspector should NOT show "not in current file
     listing" for any of them.
   * Bug 11 (image-insert): README.md, Cmd+End, type
     `![](./test-image.png)`, watch the viewport stay
     anchored on the caret line after the image
     decodes.
   * Bug 20 (re-open focus): cold-open with bubble →
     no caret in prompt input; close + re-open with
     bubble still present → no caret in prompt input;
     dismiss → caret returns.
   * Side observations: `.md.md` double-append, "Spawn"
     vs "Stage:" copy in Hybrid NAV help, Cmd+K p focus
     race.
5. Round-1 sweep verdicts appended to your task tail.

Bug 14 (watcher first-try hang) was your CNR; the
commit-plan flags a re-attempt as a gating item for
`systacean-3`. If the rebuilt binary stresses the
watcher again and you don't repro, that's the audit
anchor to strike it from the Round-1 list.

@@Alex is stepping away for a while; your verdicts feed
the commit-plan gate. No pressure on timing — when you're
done, fire a poke summarising the sweep verdicts.

## 2026-05-20 — poke (Round-1 teardown checklist before recycle)

@@Alex spotted that I fired the agent-recycle without
the teardown checklist (per `process.md` "Teardown"
section). Tearing down before the recycle so the fresh
Round-2 session boots into a clean state.

Lane-A persistent footprint:

1. **Test server on `127.0.0.1:8787`**: stop the
   `./target/debug/chan serve /tmp/chan-test-phase8-wa/`
   process. Kill via Ctrl+C in its terminal, or
   `pkill -f "chan serve /tmp/chan-test-phase8-wa"` if
   it's backgrounded.
2. **Throwaway drive `/tmp/chan-test-phase8-wa/`**:
   `rm -rf /tmp/chan-test-phase8-wa/`. Includes the
   chan-source seed + the watcher-events directory +
   the sample survey events + reply files seeded for
   @@Alex.
3. **Drive registry entry**: `chan remove /tmp/chan-test-phase8-wa/`
   to drop it from the registered-drives list.
4. **Chrome MCP tabs**: close any
   `mcp__claude-in-chrome__tabs_*` sessions opened
   against the lane-A URL via `tabs_close_mcp` per tab.
5. **Any other ad-hoc resources**: alternative test
   drives in `/tmp/`, browser bookmarks pointing at the
   lane-A URL, etc.

Append a teardown-complete entry to your task file or
journal when done so the fresh Round-2 session sees the
"clean" state on bootstrap.

Standing permission from
[event-webtest-a-alex.md](event-webtest-a-alex.md)
covers the `chan remove` + `rm -rf` actions through
Round-1 close.

## 2026-05-20 — poke (rich-prompt mini-wave verification queue)

@@Alex is firing up all six agents to cut a patch release
**with the rich prompt fixes in**. Five tasks fanned out
across @@FullStackA / @@FullStackB / @@Systacean; your
lane-A coverage owns the SPA-side verifications.

Verification queue (verify in order as fixes land):

* **`fullstack-a-28`** (BubbleOverlay regression cluster:
  filter generalization + explicit dismiss + refresh
  diff-merge). Repro fixtures live at
  `docs/journals/phase-8/rich-prompt/events/`. Confirm:
  (a) survey reply still dismisses the survey bubble,
  (b) pre-flight reply now dismisses the pre-flight
  bubble, (c) explicit close button works on every bubble
  type, (d) no flicker across two watcher poll cycles
  on any bubble type.
* **`fullstack-a-29`** (rich-prompt collapse dead space).
  Confirm: collapsing the rich prompt grows the terminal
  output downward so the bottom of the terminal sits
  just above the collapsed pill (no dead band).
  Expanding restores the existing behaviour.
* **`fullstack-a-30`** (per-prompt page-width + slider).
  Confirm: tile two panes, narrow the editor's page width
  in one, observe the rich prompt in the other is
  unaffected. Right-click the rich-prompt textbox →
  slider appears + works + persists across reload.
* **`fullstack-b-13`** (shell/agent submit-mode toggle) —
  this is @@WebtestB's lane primarily (live Claude Code
  in a terminal), but if you can repro the rich-prompt
  Cmd+Enter side cleanly on lane-A, double-coverage
  welcomed.

Lane-A test server: stand it up fresh after the rebuild
(@@Systacean will note when the patch-release binary is
ready). The throwaway drive at `/tmp/chan-test-phase8-wa/`
was torn down at recycle; pick a fresh one.

Push held for the patch-release commit-grouping cut
(@@Systacean lands the tag once the wave is green +
your verdicts are in).

Round-2 broader fan-out (carousel, Infographics, BOOT,
manual, signing, etc.) parks until the patch ships.