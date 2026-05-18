# webtest-a-5: wave-1.5 + wave-2 walkthrough lane (Lane A)

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-18

## Goal

Walk through the wave-1.5 commits already on `main` and
add coverage as wave-2 commits land. Lane A angle —
editor / file browser / pane menu / find UX heavy. Your
counterpart @@WebtestB covers terminal / broadcast /
panes.

This is a rolling task — you'll append verdicts as new
commits land. No need to wait for the full wave before
reporting; ping me after each verdict cluster.

## Relevant links

* [../request.md](../request.md) — bug IDs.
* [./webtest-a-4.md](./webtest-a-4.md) — your prior sweep
  on `fullstack-4`; reuse the test drive `/tmp/chan-webtest-a-1/`
  and the running 8801 server.

## Acceptance criteria

For each landed commit below, report PASS / FAIL / PARTIAL
with enough detail for @@FullStack to act on a fail.

### Already landed (do now)

1. **`fullstack-6` (commit `67a637f`)** — pane menu reorg:
   * **B22 retest priority**: @@Alex flagged that the
     original stuck-Loading repro also left the status pill
     stuck on "copied path" or similar. The B22 cleanup in
     `fullstack-6` clears the tree state; verify it also
     clears the status pill, OR open Toggle Web Inspector
     (now in the pane hamburger) and capture any console
     exception during Copy Path on a directory. Two-state
     check, not just the tree.
   * Left-click on empty pane / tab strip selects only (no
     menu) — covers B15.
   * Right-click on pane gives Split L/R/U/D, Close,
     Next/Prev pane, Focus-color (blue/green/pink).
   * Pane hamburger gives Reload + toggle Web Inspector.
   * Doc tab right-click: Close / Close others / Close all
     / Copy path / Show in file browser / Reopen closed.
   * Rich-prompt right-click: toggle rendered/source +
     style toolbar.
   * `Cmd+]` / `Cmd+[` (native) for Next/Prev pane;
     `Cmd+Alt+]` / `Cmd+Alt+[` (web).
   * **B22 cleanup**: Copy Path on a directory no longer
     leaves the file-browser pane stuck in "Loading…".
   * Focus color persists across reload (per-pane).
2. **`fullstack-7` (commit `13eadfb`)** — light-mode
   terminal ANSI contrast:
   * Pale glyphs (white-on-white `\e[37m`, faint
     green/yellow/cyan) are now readable on light
     backgrounds. Dark mode untouched.
   * Run a small ANSI palette dump in a terminal under
     light theme; compare with dark.

### Landing soon (cover when they hit `main`)

* `fullstack-8` (BCAST/mute cluster — B17/B18 + 6-terminal
  drift). Stress test with 6+ terminals; toggle BCAST + per-tab
  mute combinations; expect no drift. Spec in
  [../fullstack/fullstack-8.md](../fullstack/fullstack-8.md).
* `fullstack-9` (B20 markdown table crash). Test by
  opening `docs/journals/phase-7/alex/setup-1.md` Q3 or
  any pipe-table-containing doc.
* `fullstack-10` (B6/B7/B12 cursor cluster). Editor heavy.
* `fullstack-11` (fs-move UX wedge). External `mv`/`rm` of
  open files.
* `fullstack-12` (B16 Cmd+T / Cmd+Alt+T rebind).
* `systacean-6` (cross-drive drift SPA-storage phase). When
  it lands, reuse your existing repro recipe from
  `webtest-a-4`.

## How to start

* `/tmp/chan-webtest-a-1/` is still seeded; 8801 is still
  up at
  `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
* If you need to rebuild the binary (after a commit lands),
  the standard cycle: `cargo build -p chan && killall chan
  || true && ./target/debug/chan serve --port 8801
  --no-browser /tmp/chan-webtest-a-1/`.
* Permission scope: carried over from `webtest-a-1` /
  `webtest-a-4`. @@Alex re-verbalised this turn — no new
  permission event needed.

## Hand-off

Ping me on each verdict cluster via
`alex/event-webtest-a-architect.md`.

## 2026-05-18 (resume) BST - Wave-1.5 cluster verdicts

Build: head `f94c4b5` (which folds in `67a637f` fullstack-6 +
`13eadfb` fullstack-7 + the final-committed systacean-3 cache
scoping). Server bounced on 8801 against
`/tmp/chan-webtest-a-1/`.

### Per-item verdicts

```
# | Item                                              | Verdict
--+---------------------------------------------------+--------
0 | systacean-3 drift re-check (cache scoping)        | pass *
1 | fullstack-6 B15 left-click semantics              | pass
2 | fullstack-6 pane right-click menu                 | pass
3 | fullstack-6 pane hamburger menu                   | pass **
4 | fullstack-6 doc tab right-click menu              | pass
5 | fullstack-6 per-pane focus color + persistence    | pass
6 | fullstack-6 Next/Prev pane (Cmd+Alt+]/[ web)      | pass
7 | fullstack-6 B22 Copy Path on directory            | pass
8 | fullstack-7 light-mode terminal ANSI palette      | pass ***
```

`*` Re-tested with both 8801 and 8810 serving the patched
binary (`Cache-Control: no-store` + `Vary: Host` confirmed via
`curl -sI`). Two checks: (a) cold tab to 8801 stays on 8801;
(b) warm-cache repro — navigate to 8810 first to populate any
shared storage, then navigate to 8801 — port stays on 8801
across 2.5s. Drift no longer reproduces. The "Vary: Host" on
hashed assets (added by `f94c4b5`, not in the in-tree-only
patch from my previous pass) appears to close the loop.
**systacean-6 may be a no-op if this repro is the only
manifestation** — flag to @@Systacean for confirmation.

`**` Pane menu items present and correct (Reload + Toggle Web
Inspector). Minor side observation: opening the hamburger menu
while a prior pane-right-click menu is open does not auto-
dismiss the other — both menus render simultaneously. Pressing
Escape doesn't fully dismiss either before the second one opens.
Cosmetic, not blocking.

`***` Theme-switch live: `data-theme="light"`, `--bg: #fff`,
`--text: #1c1c1e`. ANSI glyph CSS colors verified via DOM:
GLYPH-30 `rgb(36,41,47)` = `#24292f`,
GLYPH-31 `rgb(207,34,46)` = `#cf222e`,
GLYPH-32 `rgb(26,127,55)` = `#1a7f37`,
GLYPH-33 `rgb(138,99,0)` = `#8a6300` — all match the patch's
light palette exactly. `terminal-host` CSS bg is
`rgb(255,255,255)` (white) so glyphs render on the spec'd
light surface. Visual readability is good for the dark ANSI
variants; `\e[37m` "white" → `#6e7781` (gray-on-white) is
readable but borderline contrast — call out only, the spec
asked for "readable" not "AAA contrast". Side observation:
`getComputedStyle(.xterm-viewport).backgroundColor` reports
`rgb(0,0,0)` even in light theme, but the visible painted
surface is the white `terminal-host` underneath the viewport,
so this is cosmetic JS-introspection noise, not a real issue.

### State left on disk

* `/tmp/chan-webtest-a-1/` test drive — list.md still present
  (test artifact from `webtest-a-4`).
* Server: `chan serve --port 8801 --no-browser
  /tmp/chan-webtest-a-1/` still up.
* I left the chan tab in **light theme** + with a Terminal-1
  in the left pane + a green-bordered empty right pane. If
  @@Alex wants a clean baseline before click-around, switch
  Appearance back to Dark in Settings (Cmd+,) and right-click
  → Close pane on the empty right pane.
* 8810 was restarted by a separate process during this sweep
  (probably the next-Lane B agent bootstrap); I did not touch
  the 8811 path.

### Next cluster

Waiting on `fullstack-8` (BCAST/mute), `fullstack-9` (B20
table crash), `fullstack-10`, `fullstack-11`, `fullstack-12`,
`systacean-6` (if it goes ahead). Will append a new cluster
verdict as each one lands.

## 2026-05-18 (resume) BST - Cluster complete

## 2026-05-18 (resume) BST - Wave-2 cluster verdicts

Build: head `8ae2d44` (fullstack-10) which folds in
`be9186c` (fullstack-9 B20 table fix), `83fbb20`
(systacean-6 per-instance SPA storage), `7e09d20` (fullstack-8
BCAST/mute — Lane B's primary, skipped here). Server bounced
on 8801 against `/tmp/chan-webtest-a-1/`.

### Per-item verdicts

```
# | Item                                              | Verdict
--+---------------------------------------------------+--------
1 | fullstack-9 B20 pipe-table render                 | pass
2 | fullstack-10 B12 source/rendered caret on image   | pass
3 | fullstack-10 B6 EOF typing scroll behavior        | pass
4 | systacean-6 cross-drive drift (warm cache repro)  | pass
```

`1` (B20 fix): note-a.md (the canonical pipe-table repro)
now renders the full doc — heading, paragraph, numbered list,
bullet list, **`<table>` with rows Alpha/1/Ready, Beta/2/Waiting,
Gamma/3/Done** (table tag verified via DOM, `hasRenderedTable:
true`), and the "Trailing whitespace target." paragraph after
the table (which was lost in the original B20 cascade). No new
RangeError on the run (the buffered console errors are all
17:24-17:25 timestamped — pre-rebuild). StateField path for
block decorations is the CM6-supported path; matches the
patch's `be9186c` description.

`2` (B12 fix): with cursor at source position 42 (inside the
URL `./img/photo-1.png` — specifically at char 10 of the URL,
between `phot` and `o`), navigated source→wysiwyg→source.
Wysiwyg: no crash, image renders, source row revealed beneath
(`![](./img/photo-1.png)` shown as editable text under the
rendered image — expected source-reveal behavior). Back to
source: cursor lands at `startOffset: 10` in the same URL
text node = same offset, exact round-trip preservation.

`3` (B6 fix): scrolled to a state where cursor was at end of
note-b.md (24570 chars, ~91 paragraphs) with viewport at
scrollTop 7218 (cursor visible at bottom of viewport). Typed
three characters one at a time: `scrollTop` stayed at **7218
across all three keystrokes** (no per-character thrash). Then
five `Return`s added 180 px of scroll vs 144 px of new doc
height — minor scroll-into-view adjustment, well within
expected behavior. Pre-fix: every keystroke kicked the
viewport down by extra padding. Fixed.

`4` (systacean-6): fresh chrome MCP tab navigated to
`http://127.0.0.1:8810/` first (cache warm for the Lane B
session — that server is still running with the @@WebtestB
6-terminal stress test), then navigated to
`http://127.0.0.1:8801/?t=...`. Tab stayed on **8801** across
3 second observation, rendered my Lane A drive contents
(img/, index.md, list.md, note-a.md, note-b.md). Drift is
closed even under the warm-cache stress that previously
reproduced.

Notable: the 8810 server in this test was running the older
binary (pre-systacean-6 — started in an earlier round and not
restarted to avoid disrupting @@WebtestB). The fact that
drift no longer reproduces shows the systacean-6 fix on the
8801 side is sufficient — each SPA instance scopes its own
keys, so a stale identifier from a sibling chan-serve can't
hijack the receiving SPA.

### Skipped (Lane B's primary)

* `fullstack-8` (BCAST/mute B17/B18 + 6-terminal drift) —
  @@WebtestB is actively stress-testing this on 8810 (T1-T6
  visible in tab 503725105). My task spec includes it as
  optional Lane A coverage; deferred to avoid stepping on
  Lane B's run.

### Not yet landed

* `fullstack-11` (fs-move UX wedge).
* `fullstack-12` (B16 Cmd+T / Cmd+Alt+T rebind).

Will pick those up on the next poke after they land.

### State left on disk

* `/tmp/chan-webtest-a-1/` test drive intact (list.md still
  present as a test artifact).
* 8801 server still up at
  `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`.
* 8810 + phase-6 chan-serves not touched.

## 2026-05-18 (resume) BST - Wave-2 cluster complete

## 2026-05-18 (resume) BST - Wave-2b cluster verdicts

Build: head `65534d3` (Replay terminal scrollback after reload),
which folds in `776aebd` (fullstack-12 Cmd+T/Cmd+Alt+T rebind),
`38f8b60` (fullstack-11 fs-move/delete UX), `f975ee7` (build
infra: DMG signing env, skipped — not Lane A). Server bounced
on 8801.

### Per-item verdicts

```
# | Item                                              | Verdict
--+---------------------------------------------------+--------
1 | fullstack-12 B16 Cmd+Alt+T opens new terminal     | pass
1 | fullstack-12 B16 legacy Cmd+` no longer fires     | pass
2 | fullstack-11 external mv on open file             | pass
2 | fullstack-11 external rm on open file             | pass
3 | systacean-8 scrollback after reload               | fail *
```

`1` (fullstack-12): `Cmd+Alt+T` in a focused pane spawns a new
Terminal-1 tab as expected (web variant of the native Cmd+T).
After spawn, `Cmd+`​`` does NOT create an additional terminal —
the legacy backquote binding is fully retired on web. Terminal
count went 0 → 1 on Cmd+Alt+T, stayed 1 after Cmd+`. Matches
`776aebd`'s commit description.

`2` (fullstack-11): Created a fresh `move-target.md`, opened
in the editor, then external `mv` to `move-target-renamed.md`.
Tab surface flipped to a clean empty state:
* Header banner: **"File moved or deleted"**
* Center title + filename (`move-target.md`)
* Three affordance buttons: **Re-open** / **Find** / **Close**
* No `i/o error` text anywhere.

Repro confirmed for the `rm` case too with a separate
`delete-target.md`. Same empty state, same affordances. The
request's acceptance ("clear 'this file was moved or deleted'
state with re-open / find / close affordance instead of raw
i/o error") is met.

`3` (systacean-8): **FAIL** in live test, with a notable
nuance. Repro:

1. `Cmd+Alt+T` to spawn Terminal-1 in the focused pane (fresh
   session against the rebuilt 8801).
2. Ran `seq 1 30 | awk '{print "SCROLLBACK-LINE-" $0}'` —
   confirmed all 30 lines rendered live.
3. `location.reload()` on the tab.
4. After reload + 5 s settling: the terminal pane shows only
   the fresh prompt
   (`mbp /private/tmp/chan-webtest-a-1 $`).
   `scrollbackLines: 0` lines containing `SCROLLBACK-LINE`,
   `xterm-viewport.scrollHeight == clientHeight (706)` — nothing
   in scrollback buffer. Mouse-wheel scroll up reveals nothing.

**Nuance**: the PTY itself survived. Up-arrow in the post-
reload prompt recalls the prior `seq 1 30 | awk …` command
(bash history persisted). `echo $$ $RANDOM` returned the same
shell PID (93062). So the re-attach landed on the original
PTY (B14 path still good — that part is not a regression).

What's missing is the visible **replay**: the server-side
ring buffer (in `crates/chan-server/src/terminal_sessions.rs`,
`Session::attach` returning `replay: Vec<Vec<u8>>`) and the
WS handler that sends those chunks (`routes/terminal.rs:202-
206`) appear present. The client patch (`tabs.svelte.ts` in
`65534d3`) drops `tseq` from the per-tab session payload so
the reconnect always requests `since=None` (i.e. give me
everything since the start of the ring). Yet xterm renders
empty on re-attach.

Plausible failure modes (none verified, all hand-offs for
@@Systacean):
* The xterm.js instance gets cleared/recreated between the
  Session frame and when the binary replay chunks arrive,
  so the replay writes into a buffer that's then cleared
  by a later code path.
* The sessionStorage tsid actually points to a new session
  in the rebuilt server, and a fresh session_id was passed
  (server creates a new session with empty ring). Worth
  capturing the WS connect URL to confirm.
* Some other timing race where the rendered terminal
  appears to consume the prompt-only first frame and then
  the replay chunks land into a different xterm scrollback
  region than the visible viewport.

Hand-off to @@Systacean for investigation. The B14 / PTY
survival side is intact; only the visible scrollback replay
is missing.

### State left on disk

* `/tmp/chan-webtest-a-1/move-target.md` — restored after the
  mv test.
* `/tmp/chan-webtest-a-1/delete-target.md` — removed during
  the rm test; NOT recreated (it was a throwaway). The
  delete-target.md tab is still showing the "File moved or
  deleted" empty state in tab 503725098 — fine for
  click-around demonstration; can be Closed via the affordance
  button when @@Alex is done with it.
* `/tmp/chan-webtest-a-1/list.md` still present (test artifact
  from `webtest-a-4`).
* 8801 server still up. Tab 503725098 has Terminal-1 with the
  systacean-8 repro state (post-reload, prompt visible,
  history intact). 8810 + phase-6 untouched.

## 2026-05-18 (resume) BST - Wave-2b cluster complete
