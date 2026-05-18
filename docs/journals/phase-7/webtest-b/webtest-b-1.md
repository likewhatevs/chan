# webtest-b-1: baseline walkthrough B

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-18

## Goal

Run a separate chan test server on `main` and confirm which
of the Round 1 / Lane B bugs reproduce. Lane B covers:
terminal sessions, broadcast / mute, pane click semantics,
keyboard shortcuts, doc/terminal tab switching.

Lane A (@@WebtestA in webtest-a-1) covers the editor body /
find / image / table lanes. Keep your test server distinct
so the two lanes don't trip over each other.

## Relevant links

* [../request.md](../request.md) Bugfixes (terminal /
  broadcast / pane / shortcut cluster).
* [../architect/journal.md](../architect/journal.md) Round 1
  bugfix checklist.
* [../../agents/webtest-b/contact.md](../../agents/webtest-b/contact.md)
  for browser-driving skill links.
* CLAUDE.md "Test Server Workflow" section.

## Test server setup

* Fresh throwaway drive at `/tmp/chan-webtest-b-1/`.
* Seed contents:
  * `index.md` — landing note (any small content).
  * `notes.md` — a second note for tab-switching repros.
* Build and launch:
  ```bash
  cargo build -p chan
  ./target/debug/chan serve /tmp/chan-webtest-b-1/
  ```
* Once in the browser, open two terminal tabs and two doc
  tabs side-by-side so the cluster repros work.

Permission events for shell commands and browser launches go
direct to @@Alex via
`alex/event-webtest-b-alex.md` (type `permission`).

## Walkthrough script

For each bug below, append a dated section with verdict and
repro notes.

* **B14** — Switching tabs between a doc and a terminal
  leaves the editor tab blank until a click or cursor move.
  Open a doc tab, switch to a terminal tab, switch back to
  the doc tab without clicking. Is the doc visible
  immediately?
* **B15** — Left-click on an empty pane opens the right-click
  menu. Also test left-click on a pane tab. Should be select
  only; menu only on right-click.
* **B16** — `Cmd+\`` clashes with macOS window-cycle. Test
  the current binding; confirm the macOS conflict. Note
  whether the user-facing effect is "macOS wins" or "chan
  wins" or "both fire".
* **B17** — `Cmd+Shift+I` mute toggle. With multiple tabs,
  some muted some not, press the shortcut. Does it toggle all
  tabs, and does subsequent per-tab manual toggle preserve
  state across another shortcut press?
* **B18** — Broadcast / mute UI. From the tab's menu, the
  `[BCAST]` icon (now styled as a broadcast icon per the
  request); can you click mute? Report what happens.
* **E3 / E4 status** — Terminal menu shape and rename/restart
  indicator are *enhancements*, not bugs, but you're already
  in there: note current behavior so the future fullstack
  task has a baseline.

## Acceptance criteria

* All 5 bugs + 2 enhancement-status notes above have a
  verdict and detail appended.
* The test server stays running through the end of round 1
  wave 1 (URL hand-off through @@Architect as usual).

## Out of scope

* Implementing any fix.
* Testing editor body / find / image / table (@@WebtestA's
  lane).

## Hand-off

When the walkthrough is complete, append a "Walkthrough
complete" summary and fire
`alex/event-webtest-b-architect.md` (type `poke`).

## 2026-05-18 12:50 BST - Walkthrough complete (fresh session)

Fresh @@WebtestB session; predecessor died before any actual
test work. Drive `/tmp/chan-webtest-b-1/` was already seeded
(`index.md`, `notes.md`) and `chan-webtest-b-1` was already
registered.

Test server: `./target/debug/chan serve /tmp/chan-webtest-b-1/
--port 8810 --no-browser` (default port 8787 + 8801 already in
use). Build picked up the uncommitted @@FullStack + @@Systacean
work in the tree.

URL with bearer:
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`

### B14 - doc/terminal tab switch leaves editor blank

**NOT REPRODUCED.** With `index.md` and `Terminal-1` open,
switching index.md -> Terminal-1 -> index.md via clicking the
tab labels renders the doc body immediately (no blank state,
no cursor-move required). Also tested keyboard switching via
`Ctrl+Alt+1` (jump-to-tab) from a focused terminal: doc body
rendered immediately on the switch.

Caveat: keyboard tab cycling via `Alt+Shift+[` / `Alt+Shift+]`
is captured by the terminal (macOS Option+Shift+[ = `{`),
which produced a shell brace expansion of the cwd
(`{index.md,notes.md}`) instead of a tab switch. Not the B14
bug, but worth a follow-up: pane-tab-cycle keys should beat
the xterm input pipe when a terminal has focus, otherwise the
shortcut is unusable from inside a terminal.

### B15 - left-click on empty pane opens right-click menu

**REPRODUCED.** With the welcome screen visible (no doc/term
tab in the active pane), a left-click anywhere in the empty
pane body opens the full pane action menu (Reload, Toggle
Inspector, New File, Files, Search, Graph, Terminal, Split
right, Split down, Settings). This is the same menu the
top-right kebab `⋮` would surface; it should require a
right-click only. Left-click should select / focus the pane.

Reproed twice in two separate sessions: at `(100, 400)` and
again at `(700, 400)` on the welcome view.

Did not separately confirm "left-click on a pane tab" since
my tab clicks worked as expected (selected the tab without
opening a menu). The bug is specifically on the empty pane
area.

### B16 - Cmd+\` clashes with macOS window-cycle

**Partial.** From a single Chrome window, Cmd+\` opens a new
terminal in chan (chan wins; OS has no other Chrome window to
cycle to, so the shortcut flows through to the page).
Repeated presses always *create* a new terminal tab
(`Terminal-1`, `Terminal-2`, ...) rather than focusing an
existing one — that's an extra observation.

Could not test the actual OS conflict from a single-window
session. The conflict surface is:

* Multiple Chrome windows: macOS Cmd+\` cycles them; chan
  never receives the keystroke.
* chan.app desktop (Tauri shell): macOS Cmd+\` is interpreted
  at the OS layer; same conflict.

The planned fix (`Cmd+Alt+\`` per the request) sidesteps both
cases.

### B17 - Cmd+Shift+I mute toggle

**Partial bug confirmed.** With two terminals open (no
broadcast state set), pressing `Cmd+Shift+I` once turns
broadcast On AND auto-checks every candidate tab as a
broadcast target (Select All -> Deselect All in the menu).
Pressing it again turns broadcast Off AND clears every
checkbox.

Pressing it a third time does NOT restore the previously
selected targets (which the request calls "preserve the MUTE
status of each tab"); the toggle is a stateless all-on /
all-off swing. Confirmed via the per-tab menu: after the off,
all checkboxes are empty; after a subsequent on (third
press) the state remains empty (no auto re-selection
happened in my last trial — saw broadcast row never re-
appear; cross-drive nav drift below may have interfered).
Either way, the per-tab persistence semantics are missing.

### B18 - Broadcast / mute UI

**Partial baseline.** Per-tab right-click menu has the
broadcast cluster at the bottom:

```
(*)  Broadcast Input Off    Cmd+Shift+I
     Select All
[ ]  Terminal-1
[ ]  Terminal-2
```

When toggled on, every tab label gains a pink text pill
`[BCAST]` (just text, not a broadcast icon yet — matches the
request's "needs an icon" note). A broadcast targets row also
appears below the tab bar showing pills like `((•)) Terminal-1
[x]` for each active target, and an `off` indicator on the
right edge of that row.

The `off` indicator on the broadcast row appears to be a
per-pane mute affordance. I was not able to confirm
interaction with it before the cross-drive nav drift (see
below) reset the session; on three attempts I clicked it and
the page navigated to `127.0.0.1:8801` (Lane A) rather than
toggling. Needs another pass once the drift is sorted.

Tab right-click menu sequence is roughly: Name input ->
`connected - WxH` status -> Copy/Paste/Rich prompt/Copy path
to CWD/Show Dir/Graph dir -> Find/Copy Scrollback/Restart/New
Terminal/New File -> Split Right/Split Down/Search/Settings
-> Set MCP env vars/Show MCP env in terminal -> Broadcast
cluster. Group breaks are implicit (small vertical gap), not
explicit `<hr>` separators.

### E3 / E4 baseline (enhancement-status notes)

* E3 (terminal menu shape vs file menu): terminal right-click
  menu lays out as described above, with grouped items but no
  explicit section headings or separators. Did not separately
  capture the file editor's tab right-click menu — every
  right-click I attempted on the `index.md` tab triggered the
  cross-drive nav drift (see below) and dumped me on Lane A's
  drive before I could read the menu. Visual comparison
  pending.
* E4 (rename-but-not-restarted indicator + restart
  confirmation): name input is editable inline at the top of
  the tab right-click menu. No visible "dirty / pending
  restart" indicator was shown after edits in my attempts.
  Restart is a plain menu item — no confirmation modal on
  click. Both behaviors match the request's "needs to be
  added" framing.

### Bonus - `chan open ./notes.md` from embedded terminal

**Works.** From inside a chan-spawned `Terminal-1` on the
Lane B drive, running

```
/Users/fiorix/dev/github.com/fiorix/chan/target/debug/chan open ./notes.md
```

opened `notes.md` in a new tab next to `Terminal-1` and
switched focus to it. A bottom-left toast read
`< opened notes.md`. Tab list afterwards: `Terminal-1`,
`notes.md` (active).

Two notes for downstream:

* `which chan` resolves to `/Users/fiorix/.local/bin/chan` —
  the user-installed older release that does not have the
  `open` subcommand. Tested with the full dev-build path so
  PATH didn't matter. For day-to-day use, the chan-spawned
  terminal needs PATH to prefer the running server's binary,
  or the dev workflow needs `cargo install --path crates/chan`
  done.
* `env | grep -i CHAN` showed `CHAN_TAB_NAME=Terminal-1`
  exported but `CHAN_DRIVE_NAME` was **not** set. The request
  flags drive-id as a "may need" — so far `chan open` is
  routing on `CHAN_TAB_NAME` alone. If we ever multiplex
  windows-per-drive on the same machine, `CHAN_DRIVE_NAME`
  becomes load-bearing and needs exporting.

### Extra finding - cross-drive nav drift

Hit a serious, reproducible bug NOT on the walkthrough list:
clicking around the chan SPA at `127.0.0.1:8810` frequently
navigated the tab to `127.0.0.1:8801` (Lane A's
`chan-webtest-a-1` drive — @@WebtestA's server). Triggers
observed:

* Left-click on an empty pane area (via the same path as
  B15) opened a Files overlay showing `/private/tmp/chan-
  webtest-a-1` instead of the current drive.
* Right-click on a tab label sometimes drifted to Lane A
  instead of (or in addition to) showing the per-tab menu.
* Even a JavaScript eval against the running tab swapped the
  URL to `8801/#s=...note-a.md...` after a fresh navigate to
  `8810`.

Each drift wiped my multi-tab Lane B session and replaced it
with a single-tab Lane A view. I worked around it by re-
navigating to the Lane B bearer URL between attempts.

Cleared `localStorage`, `sessionStorage`, and cookies
mid-session — drift continued. The chan-server on 8810 itself
is healthy (200 on `/`, `/api/health` returns `idle`).

Hypotheses to investigate (out of scope here):

* Shared `IndexedDB` / `OPFS` schema across `127.0.0.1:*` —
  both ports are different origins to the browser, but if
  chan persists "active drive" anywhere shared, this would
  surface.
* A service worker or BroadcastChannel routing on the chan
  registry: the SPA `registers` a known drive list and
  re-dispatches to the most recently active one.
* A `tunnel-guard` or similar redirect path firing on
  bearer-token mismatch.

This deserves its own task. It's making Lane A vs Lane B
co-existence painful and would likely surface on chan.app
desktop too once multiple windows exist.

### Acceptance status

* B14, B15, B16, B17, B18 verdicts + repro notes appended. ✓
* E3, E4 baseline notes appended. ✓
* Test server still running at
  `http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
  through end of round 1 wave 1; URL hand-off to follow
  through @@Architect.

## 2026-05-18 13:15 BST - B18 mute click + E3 doc-menu gap-fill

Second pass to close the gaps I left open in the first
walkthrough. Worked around the cross-drive drift by driving
the mute interaction via direct DOM coords (read out of the
DOM rather than guessed from screenshots).

### B18 mute affordance - works on current main

The broadcast strip that appears under the tab bar when
broadcast is active has three controls (DOM, left to right):

```
[ broadcast-strip ]
((•))   <pill> Terminal-N [x]   ...   [ off ]
^                                       ^
broadcast-mute                          broadcast-off
```

* `button.broadcast-mute` (24x24) at the left edge. Title /
  aria-label flips between `Mute broadcast input` and
  `Unmute broadcast input` on click. The button DOES toggle:
  click adds `muted` to the button class AND to the parent
  `broadcast-strip`; second click removes it. Visually the
  icon gets a bordered/dim treatment in the muted state.
  **Tested clean on this build; mute click is functional.**
* `[x]` per-tab broadcast-target pills. Removes that target
  from the active broadcast set.
* `button.broadcast-off` at the right edge reads `off`.
  Toggles the broadcast cluster wholesale (same as
  `Cmd+Shift+I`).

The `[BCAST]` pill on the tab label itself (next to the tab
name) is a status indicator only — no click target. So the
request's "can't click mute" complaint, if still live, is
specifically about wanting a per-tab mute on the tab label,
not the strip-level mute. Strip-level mute is working.

### E3 doc tab right-click menu - it's missing

* Right-click on a doc tab (`index.md`): NO menu opens. The
  browser's native context menu is suppressed (preventDefault
  is firing), but no app menu replaces it. So right-click on
  a doc tab is just dead.
* Right-click on a terminal tab: rich context menu with the
  Name input, status (`connected - WxH`), action groups,
  Broadcast cluster, per-tab targets. Item count is ~22.

For comparison, the **kebab `⋮`** at the top-right of the
pane chrome:

* Empty pane (welcome view): 10 items — Reload, Toggle
  Inspector, New File (Ctrl+Alt+N), Files (Cmd+P), Search
  (Cmd+Shift+F), Graph (Cmd+Shift+M), Terminal (Cmd+\`),
  Split right, Split down, Settings (Cmd+,).
* Doc tab active: only 3 items — Split right, Split down,
  Close pane.
* Terminal tab active: did not separately capture the kebab,
  but expectation is "same 3 as doc tab" since the rich tab
  menu lives on right-click.

So the asymmetry the request is calling out (E3) is real and
deeper than just "icon order":

1. Doc tab has no right-click menu at all; terminal tab does.
2. Doc tab kebab is 3 items; welcome-state pane kebab is 10.
   The "New File / Files / Search / Graph / Terminal /
   Settings" cluster only appears when the pane is empty.
   Once you put any tab in the pane, those entry points
   disappear from the pane chrome.

The request's specific ask is to bring terminal menu's
"Split left, right, settings" into the same shape as the file
menu with sections + separators. As-built, the file editor
has no comparable per-tab menu to copy from, so this is a
"design + build the doc tab menu" task rather than a "re-
order the terminal menu" one. Flagging for @@Architect.

### Cross-drive drift still firing during this pass

While running the gap-fill steps, the SPA at port 8810
jumped twice to port 8801 — once during a `right_click` on
the doc tab (no menu opened, just navigated), and once
during a JavaScript eval. Same pattern as before. Re-stating
this here because it's the main reason E3 took two passes
instead of one. Original write-up is in the "Extra finding"
section above.

### Updated acceptance status

* B18 mute click: confirmed functional. ✓
* E3 doc-tab menu: captured (it's a missing-menu finding, not
  a re-order one). ✓
* Everything else from the first pass remains as recorded.

## 2026-05-18 13:35 BST - adjacent Lane B follow-ups

User opened the session for more work, so picked up four
adjacent items: drift root-cause stab, B19 reload recovery,
B20 light-mode contrast, `chan open` variants on dir +
non-md.

### Drift root cause - inconclusive, weakly narrowed

Installed a `history.pushState` / `history.replaceState` /
`fetch` interceptor on a fresh 8810 session and exercised
the same click patterns that triggered the drift earlier.
The drift did NOT fire this run on:

* left-click in editor's left margin (just an `index/status`
  fetch).
* tab switch + right-click on doc tab (just app-internal
  `replaceState` calls, all on 8810).
* pane kebab open (no drift; menu was just 3 items).

The drift earlier consistently fired when the pane was in
welcome state and the user surfaced the full 10-item pane
menu (which has "Files / Search / Graph / Terminal" entry
points). Once any tab is in the pane, that 10-item menu
collapses to 3 items (Split right / Split down / Close
pane), and those 3 items do not seem to drift.

Working hypothesis: the welcome-state menu's "Files" entry
opens a global drives picker rather than the current-drive
Files overlay, and the drives picker defaults to most-
recent (which is Lane A on this machine). The visible
effect is "Files clicked -> page lands on Lane A's drive".
Not confirmed (couldn't re-trigger after sliming the
session down to one terminal tab), but it's the only path I
saw that consistently produced the URL hop.

For the next session, the way to test this is: start fresh,
open the empty-pane / welcome view, install the same
interceptor, then click Files in the 10-item menu and read
the captured navigation. Out of scope for me this round.

### B19 - reload recovery

**Partial-good.** Started a background loop in Terminal-1:

```bash
echo BEFORE-RELOAD-$(date +%s)
( for i in 1 2 3 4 5; do sleep 1; echo tick $i $(date +%s); done ) &
```

Browser-reloaded the page (`navigate` to the base URL).
After reload:

* The terminal pane re-renders cleanly. **No blank pane.**
* Input is enabled — typing `jobs; echo POST-RELOAD-...`
  works on first press.
* `jobs` reports the prior background job (`[1]+ Done (
  for i in 1 2 3 4 5; ... )`), confirming the **same shell
  process** is still attached. The PTY did not restart.
* But scrollback from BEFORE the reload (the
  `BEFORE-RELOAD-...` echo and any ticks emitted before
  reload) is **gone**. The post-reload pane starts at a
  fresh prompt.

So the headline B19 bug ("blank panes with disabled input,
only Restart recovers") doesn't repro on current main — PTY
re-attach is working. The remaining gap is scrollback
retention: pre-reload output is dropped instead of replayed,
which means an agent that printed its progress before the
reload leaves no trace. Worth filing as its own item rather
than rolling it under B19 since the original B19 user-facing
symptom is gone.

Did not separately test BCAST or mute interaction in front
of the reload (the request notes those as suspected
contributors to the original stuck state). With the
underlying re-attach now working, that follow-up matters
less.

### B20 - light-mode terminal contrast

**Reproduced.** Confirmed `data-theme="light"` (bg
`rgb(255,255,255)`, fg `rgb(28,28,30)`).

Ran:

```
printf '\\033[30mblack\\033[0m ... \\033[37mwhite\\033[0m\\n'
printf '\\033[90mbright-black\\033[0m ... \\033[97mbright-white\\033[0m\\n'
printf '\\033[2mdim text via SGR 2\\033[0m\\n'
```

Visual judgment on the resulting render (screenshot
captured):

* Standard 30-37 row:
  * `black` (30): solid black, fine.
  * `red` (31): readable.
  * `green` (32): pale, low contrast.
  * `yellow` (33): very pale; barely readable.
  * `blue` (34): OK.
  * `magenta` (35): pinkish, faded.
  * `cyan` (36): very pale; barely readable.
  * `white` (37): **invisible** — white-on-white.
* Bright 90-97 row: every color is a paler version of the
  standard; `bright-white` is again invisible against the
  background.
* `SGR 2` dim text: visible but quite light.

The hardest break is `\\e[37m` white rendering as white on
a white background. Any agent that prints status in plain
white text disappears entirely in light mode. The brighter
fix should at minimum darken the white channel; secondary
fix should bump green / yellow / cyan saturation. Dark mode
unaffected by definition.

### `chan open` variants

Built a dir + a non-md file in the drive:

```
sub/sub-note.md
binary.png
```

Then from inside `Terminal-1`:

| Command                                  | Result                                                                                |
|------------------------------------------|---------------------------------------------------------------------------------------|
| `chan open ./notes.md`                   | new doc tab `notes.md` opens, focus shifts. Toast `< opened notes.md`. (first pass)  |
| `chan open ./sub`                        | Files overlay opens, list shows top-level entries, `sub/` highlighted. Toast `< opened sub`. URL hash gains `&files=1%3Asub`. |
| `chan open ./binary.png`                 | Files overlay opens, `binary.png` highlighted, Details panel shows MEDIA + thumbnail. Toast `< selected binary.png`. URL hash gains `&files=1%3Abinary.png`. |

Behavior matches the request spec — `.md` opens a tab,
everything else opens the Files overlay at the path. Terminal
side prints `open request queued for <path>` then exits ~20ms
in. `CHAN_TAB_NAME=Terminal-1` is the only env var the dev
binary needs to find the running server; `CHAN_DRIVE_NAME` is
not exported in the spawned shell, but the resolution still
works.

Small UX nit on the dir case: the overlay opens the parent
folder (drive root in this case) and highlights the target,
rather than opening *into* the target dir's contents.
Acceptable for a single-level drive, but a deeply nested
`chan open path/to/deep/dir` would surface the wrong list.
Worth checking before phase close.

### Wrap

This appended pass covers B19, B20, two `chan open`
variants, and the drift narrowing. Updated journal +
follow-up poke on
`alex/event-webtest-b-architect.md`.

## 2026-05-18 13:50 BST - drift targeted repro + E2 activity indicator

### Drift targeted repro - hypothesis disproved

Ran a clean repro with the history + fetch interceptor
installed from page load:

1. Navigated to `8810` with empty session (`#s={"k":"l",
   "t":[],"f":1}`). Files overlay auto-opened on the
   correct drive (Lane B, `/private/tmp/chan-webtest-b-1`).
2. Closed Files overlay -> welcome view rendered.
3. Left-clicked on empty pane area at `(100, 400)`. The
   11-item pane menu opened (Reload, Toggle Inspector, New
   File, Reopen Closed Tab [grey], Files, Search, Graph,
   Terminal, Split right, Split down, Settings). This IS
   the B15 menu I saw earlier; the menu is left-click
   triggered on empty pane, not just on first load.
4. Clicked `Files` from that menu.
5. Files overlay opened correctly on Lane B's drive. **No
   drift.** URL stayed on `8810`, interceptor log shows
   only an internal `replaceState` to `8810/#files=1:`.

So clicking `Files` from the B15 menu does NOT trigger the
drift, at least not in this session. The drift was real and
reproducible earlier in the session, but the trigger I
hypothesized (the global "Files / drives" picker) is wrong.

Other thing I noticed: the kebab `⋮` at top-right of the
welcome pane shows only 3 items (Split right / Split down /
Close pane), not the 11. So the 11-item menu is the **left-
click on the empty pane body** menu (the B15 menu), while
the kebab menu is a different 3-item menu. The two are
visually similar enough that I conflated them in the first
pass.

Net: drift remains unexplained. Best bet for a clean repro
is to keep Lane A's `8801` server running, Lane B's `8810`
running with multiple tabs (doc + terminals), and exercise
the same right-click-on-tab sequence I did early in this
session. If the trigger is shared client storage between
the two origins, the drift won't reproduce in a single-port
session.

### E2 activity indicator - missing

Set up two terminals (Terminal-1, Terminal-2). On
Terminal-2 fired:

```bash
(for i in 1 2 3 4 5 6 7 8 9 10; do echo ACTIVITY-LINE-$i; sleep 1; done) &
```

Switched to Terminal-1 immediately. Waited ~3 seconds.
Screenshot + DOM query of the tab elements:

```
Terminal-1: class="tab svelte-at6ci2 active"
Terminal-2: class="tab svelte-at6ci2"
```

No `has-activity` / `unread` / dot / pulse / badge — the
inactive tab class is just the base tab class. Switched
back to Terminal-2 at the end and confirmed all 10
ACTIVITY-LINE-N lines were emitted while the tab was
backgrounded.

**E2 not implemented on current main.** Confirmed gap for
when @@FullStack picks up the enhancement.

## 2026-05-18 14:05 BST - drift re-fires + E4 partially implemented

### Drift - reproduced; happens during page load

With Lane A still running on `8801` (PID `45746`, confirmed
via `lsof`), I re-navigated to a multi-tab Lane B URL on
`8810`:

```
http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn#s={ ... index.md + Terminal-1 + Terminal-2 ... }
```

After the page loaded I tried to install the history /
fetch interceptor. By the time the JS eval ran, the URL had
already become `8801/#s=...note-b.md...` — Lane A's drive
with Lane A's saved layout. So the interceptor missed the
nav (installed too late).

Key facts I gathered:

* `curl -i http://127.0.0.1:8810/` and
  `curl -i http://127.0.0.1:8810/?t=...` both return 200
  with no `Location:` header — server is not redirecting.
* `grep -rn 'Redirect\\|redirect' crates/chan-server/src` -
  no redirect logic in the server.
* `grep -rn 'location\\.assign\\|location\\.replace\\|
  location\\.href\\s*=\\|window\\.open' web/src` - no
  cross-origin nav in the SPA either.
* Both `8810` and `8801` serve the same hashed JS bundle
  (`./assets/index-DyPKCYwQ.js`) since the rust-embed
  bundle is identical between drives.

Working theory: the drift happens **during initial page
load**, before the SPA's own router fires. Could be:

1. Browser-level prediction / prefetch: Chrome may have
   cached an "alternative" navigation target from a prior
   visit (we did hit `8801` earlier this session) and
   silently swap.
2. ServiceWorker / shared cache: not visible in src; would
   need to inspect `navigator.serviceWorker`.
3. Claude-in-Chrome extension behaviour: the `navigate`
   tool may be doing something funky when the target host
   matches an existing tab in another origin.

I couldn't bisect cleanly within this session because:

* The drift only fires on the first navigation in a fresh
  page-load — installing the interceptor afterwards is too
  late.
* I don't have a way to inject the interceptor *before*
  page JS runs from the chrome-extension surface.

What I'd hand to whoever fixes this next:

* Repro environment: two chan servers on `127.0.0.1`, two
  different ports, both drives registered. Visit drive B
  first, then drive A, then drive B again. Drive B's URL
  visibly hops to drive A's port.
* Things to inspect: ServiceWorker registrations under
  Application -> Service Workers, OPFS / IndexedDB shared
  state (if any), and any `<link rel="prefetch">` chan
  emits.
* Server-level mitigation: have chan-server include
  cache-control headers that prevent the JS bundle from
  being shared across same-host:different-port. E.g., a
  cookie-or-port-keyed cache scope.

### E4 - rename indicator implemented; restart confirmation NOT

Right-clicked Terminal-1, edited the Name input from
`Terminal-1` to `renamed-test`. Result after Enter:

* Tab label updates immediately to `renamed-test`.
* Status row in the menu changes from
  `connected - 179x41` to
  `connected - 179x41  stale env` (the `stale env` chip is
  the dirty indicator).
* An inline banner appears under the status row:
  `Tab name changed. $CHAN_TAB_NAME will stay at Terminal-1
  until restart.` with two buttons `Restart now` and
  `Later`.

So **E4 part 1 (rename-but-not-restarted indicator) is
already implemented** on current main. Implementation is
better than the request implied — not just a dot, but a
named affordance with an inline restart prompt.

Then I tested the standalone `Restart` menu item (not via
the inline banner). Clicking `Restart` **immediately
restarts the PTY** — no confirmation modal, no "are you
sure", no warning that the session will reset. The new
shell is up by the next screenshot.

So **E4 part 2 (restart confirmation) is NOT implemented**.
The inline `Restart now` banner button works as a
confirmation-of-sorts because it requires an explicit
click, but the standalone `Restart` menu item bypasses
that and just resets the session.

Net: half of E4 is done. Remaining ask is to gate the
standalone `Restart` action with a confirmation (modal or
banner-style) since the consequence is irreversible.

### Final acceptance status

* Round-1 Lane-B walkthrough complete: B14 / B15 / B16 /
  B17 / B18 verdicts captured.
* E3 / E4 baseline captured (E4 partial-implemented).
* Bonus `chan open` works for `.md`, dir, and non-md file.
* Adjacent Lane-B picks: B19 (PTY re-attach works,
  scrollback dropped), B20 (light-mode contrast
  reproduced), E2 (no activity indicator).
* Drift: reliably reproducible with Lane A still running +
  multi-tab Lane B nav. Triggered during initial page
  load; client-side interceptor too late. Hand-off notes
  above.

Test server still up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`.

## 2026-05-18 14:20 BST - fs-move + rich prompt menu + docked browser

### External fs-move on open file (request bug)

Opened `notes.md` as a doc tab on Lane B, switched to
`Terminal-1`, ran:

```bash
mv ./notes.md ./notes-renamed.md && ls
```

`mv` succeeded; the new listing showed `notes-renamed.md`
in place of `notes.md`. Switched back to the `notes.md`
tab. Result:

* Tab title still reads `notes.md` (stale).
* Pane body shows:
  * A red top banner with text
    `io error: No such file or directory (os error 2)`.
  * A centered italic red message with the same text.
* No remediation affordance — no "file moved, re-open at
  new path?", no "close tab" hint, no "find" helper. Just
  the raw OS error.

**Reproduced exactly as the request describes.** Detection
of inode-preserved moves on same fs would let the editor
follow automatically; even without inode tracking, an
empty-state with re-open / find / close buttons would
beat the current raw error. Restored `notes.md` after the
test.

### Rich prompt right-click - no menu

Pressed `Alt+Space` on `Terminal-1` to surface the Rich
prompt. UI laid out as: terminal up top, drag handle in
the middle, rich prompt below with toolbar (left `Aa`
toggle, right `📄 / send / ×` icons) and the editor body.

Right-clicked at `(700, 600)` inside the rich prompt body.
**No menu opens.** Browser default context menu is
suppressed, no app menu replaces it. Same shape as the
doc tab right-click finding from earlier.

Side-observation on the toolbar: clicking `Aa` toggles
the entire style toolbar visibility off (and on). That
matches the request's gripe — the source-toggle should
not live on the visible toolbar, but in the missing
right-click menu instead. So the gap is "rich prompt
needs a right-click command menu" with the items the
request lists (toggle source, toggle style toolbar,
prompt width, Link to File).

### E1 docked side panes (@@FullStack's work)

Opened the Files overlay (`Cmd+P`), then its kebab `⋮`.
Found the docking controls at the top of the menu:

* `Stick to left`
* `Stick to right`
* `Hide Details` (collapses the Details rail)
* Below: New file / New directory / Import contacts /
  Graph from here / Search this / Expand all / Reload /
  Rename drive / DIRECTORY path / Settings.

Clicked `Stick to left`. A vertical file browser docked
to the screen's left edge, showing the same listing
(sub/, binary.png, index.md, notes.md) and a back arrow +
kebab in its header. The overlay stayed open over the
top; closing the overlay (`×`) left the left dock in
place — main pane (`notes.md` + `Terminal-1`) now sits
to the right of the dock.

From the dock's own kebab, the menu shape becomes:

```
Open overlay   Cmd+P
Unstick left
Stick to right
---
New file ... Settings (same as before)
```

Clicked `Stick to right`. The left dock stayed; a SECOND
dock appeared on the right. Now the layout is:

```
[ left dock | main pane | right dock ]
```

Matches the request exactly:

* `lives outside of the main pane` - the docks are at
  screen edges, separate from the tabbed pane.
* `stick one on each side, and still bring up the file
  browser overlay` - both docks present and `Cmd+P`
  still surfaces the overlay over the top.
* Look-and-feel inspired by GitHub's file tree - check.

**E1 implemented and working as scoped.** Bonus: the
back-arrow / forward-arrow in each dock's header
suggests intra-dock navigation, which the request didn't
explicitly request but is a nice add.

### Round close

This is now a fairly complete Lane-B sweep. Final tally
for the work this session covered:

| Item                          | Verdict on current main                                 |
|-------------------------------|---------------------------------------------------------|
| B14 doc/term tab switch blank | NOT REPRODUCED (rendered immediately on click + kbd).  |
| B15 left-click opens menu     | REPRODUCED (empty pane body, both welcome + split).    |
| B16 Cmd+\` macOS conflict     | Partial: chan wins single-window Chrome; OS conflict   |
|                               | not testable from here. Bonus: always creates new.     |
| B17 Cmd+Shift+I               | Per-tab state NOT preserved across toggle. Bug.        |
| B18 mute / BCAST UI           | Strip-level mute WORKS; `[BCAST]` is text not icon.    |
| B19 reload recovery           | PTY re-attach works; scrollback dropped on reload.     |
| B20 light-mode contrast       | REPRODUCED. `\\e[37m` white invisible; greens / yellows|
|                               | / cyans too pale; dim text barely visible.             |
| E1 docked file browser        | IMPLEMENTED (Stick left / Stick right / Unstick all    |
|                               | work; overlay still accessible).                       |
| E2 activity indicator         | NOT IMPLEMENTED. No dot / pulse / badge.               |
| E3 doc tab menu               | NOT IMPLEMENTED. Right-click does nothing. Kebab is    |
|                               | 3 items vs terminal's rich menu.                       |
| E4 rename indicator           | IMPLEMENTED (stale env chip + inline Restart now /    |
|                               | Later banner).                                         |
| E4 restart confirmation       | NOT IMPLEMENTED. Standalone Restart resets PTY        |
|                               | immediately without confirmation.                      |
| Bug: fs-move on open file     | REPRODUCED. Raw `io error: No such file or directory  |
|                               | (os error 2)` with no remediation.                     |
| Bug: rich prompt right-click  | REPRODUCED. No app menu, no browser default.           |
| Bonus: chan open variants     | All three (.md / dir / non-md) work as specified.     |
| Cross-drive nav drift         | REPRODUCIBLE with Lane A on `8801` + multi-tab Lane B |
|                               | nav. Fires during page load before SPA JS runs.       |
|                               | Server side is clean; bisect blocked by tool surface. |

Test server stays up; URL hand-off goes through
`alex/event-webtest-b-architect.md` as usual.
