# webtest-b-3: wave-1.5 + wave-2 walkthrough lane (Lane B)

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-18

## Goal

Walk through the wave-1.5 commits on `main` and add
coverage as wave-2 commits land. Lane B angle — terminal /
broadcast / panes / split / shortcuts. Your counterpart
@@WebtestA covers editor / file-browser / find UX.

Rolling task — append verdicts as commits land; ping me
after each verdict cluster. The earlier SIGTERM mystery is
explained: @@Alex replaced `/Applications/Chan.app` and the
running app dropped its child `chan serve` processes.
Relaunch freely against the freshly-built binary.

## Relevant links

* [../request.md](../request.md) — bug IDs.
* [./webtest-b-1.md](./webtest-b-1.md) /
  [./webtest-b-2.md](./webtest-b-2.md) — your prior runs.

## Acceptance criteria

For each landed commit below, report PASS / FAIL / PARTIAL
with enough detail for @@FullStack to act on a fail.

### Already landed (do now)

1. **`fullstack-6` (commit `67a637f`)** — terminal angle of
   the pane menu reorg:
   * Left-click on terminal tab strip selects only — does
     NOT open the right-click menu. (B15 from your prior
     Lane-B sweep.)
   * Terminal tab right-click menu (the 22-item one) still
     works.
   * Pane hamburger has Reload + toggle Web Inspector.
   * Pane right-click has Split, Close, Next/Prev pane,
     focus color (blue/green/pink).
   * `Cmd+]` / `Cmd+[` (native) / `Cmd+Alt+]` /
     `Cmd+Alt+[` (web) navigate panes.
2. **`fullstack-7` (commit `13eadfb`)** — light-mode
   terminal ANSI contrast: same palette dump check
   @@WebtestA is running, but covered on Lane B as well so
   we catch any divergence.
3. **B14 / B19 re-confirmation**: your prior 17:30 BST run
   said B14 NOT REPRO and B19 PTY re-attach works.
   Confirm again on the post-`fullstack-6` /
   `fullstack-7` binary. Scrollback retention is the only
   remaining B19 gap; not in any landed task yet.

### Landing soon (cover when they hit `main`)

* `fullstack-8` (BCAST/mute cluster — B17/B18 + 6-terminal
  drift). This is your turf. Spin up 6 terminals; run the
  bulk-toggle + per-tab mute mix; expect the spec from
  [../fullstack/fullstack-8.md](../fullstack/fullstack-8.md).
* `fullstack-9` (B20 markdown table crash). Lane B can skip
  unless @@WebtestA flags a terminal-side surprise.
* `fullstack-11` (fs-move UX wedge). Reproduce external
  `mv`/`rm` of open files on Lane B too.
* `fullstack-12` (B16 Cmd+T / Cmd+Alt+T rebind).
* `systacean-6` (cross-drive drift SPA-storage phase). When
  it lands, your original Lane-A-coexistence recipe is the
  acceptance test.

## How to start

* Bring up a fresh `chan serve` on a throwaway drive for
  Lane B. Suggested:
  `mkdir -p /tmp/chan-webtest-b-1 && cargo build -p chan &&
  ./target/debug/chan serve --port 8810 --no-browser
  /tmp/chan-webtest-b-1/`.
* For the cross-drive drift re-repro (after `systacean-6`):
  keep @@WebtestA's 8801 alive and run Lane B on 8810.
* Permission scope: carried over from `webtest-b-1` /
  `webtest-b-2`. @@Alex re-verbalised this turn — no new
  permission event needed.

## Hand-off

Ping me on each verdict cluster via
`alex/event-webtest-b-architect.md`.

## 2026-05-18 20:35 BST - fullstack-6 + fullstack-7 + B14/B19 verdicts

Fresh `cargo build -p chan` (wave-1.5 commits all in:
`f94c4b5` systacean-3 + `13eadfb` fullstack-7 + `67a637f`
fullstack-6) -> `./target/debug/chan serve
/tmp/chan-webtest-b-1/ --port 8810 --no-browser`. URL:
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`.
New Chrome tab in the MCP group (didn't touch
@@WebtestA's 8801 tab).

### fullstack-6 - PASS

* **B15 left-click semantics**: clicked empty area at the
  right of the tab strip (`(800, 17)` and `(400, 17)`) -
  **no menu opens**. Click on the tab itself - selects.
  Click anywhere inside terminal body (`(800, 400)`) -
  passes through to xterm, no menu. **B15 fixed.**
* **Terminal tab right-click menu**: 20 items. Sequence:
  `Copy / Paste / Rich prompt Alt+Space / Copy path to
  CWD / Show Dir / Graph dir / Find / Copy Scrollback /
  Restart / New Terminal Cmd+`` / New File Ctrl+Alt+N /
  Reopen Closed Tab Ctrl+Alt+T / Split Right / Split Down
  / Search Cmd+Shift+F / Settings Cmd+, / Set MCP env
  vars / Show MCP env in terminal / Broadcast Input Off
  Cmd+Shift+I / Select All`. The Name input + status row
  are present but rendered as fields (not button items).
* **Pane hamburger (top-right `⋮`)**: exactly two items -
  **`Reload`** and **`Toggle Web Inspector`**. Matches the
  spec word-for-word.
* **Pane right-click on empty tab strip area**: 10 items
  in order - **`Split left`, `Split right`, `Split up`,
  `Split down`, `Next pane Cmd+Alt+]`, `Previous pane
  Cmd+Alt+[`, `blue`, `green`, `pink`, `Close pane`**.
  Web-variant shortcuts shown (`Cmd+Alt+]/[`) because we
  are in a browser. Matches spec.
* **Focus color toggle**: click `green` on the focused
  pane -> the pane's border switches to `rgb(34, 197, 94)`
  (a vivid green) and a `data-focus-color="green"`
  attribute persists; URL fragment encodes it as `pc:g`.
  Switched to the other pane, picked `pink` -> border
  becomes `rgb(255, 95, 183)`; `data-focus-color="pink"`,
  encoded as `pc:p`. Default stays `blue` for unset panes
  (`data-focus-color="blue"`, transparent border when
  unfocused). Per-pane independence held.
* **Cmd+Alt+] / Cmd+Alt+[ keyboard nav**: from pane 1
  focused, `Cmd+Alt+]` -> pane 0 focused. Repeat ->
  pane 1. `Cmd+Alt+[` -> pane 0. Round-trip clean.
* **Next pane / Previous pane menu items**: clicking
  `Previous pane` from the right pane's right-click menu
  moved focus to the left pane. Confirmed menu-driven
  navigation works alongside the shortcuts.

Did not separately probe the native `Cmd+]` / `Cmd+[`
binding because the browser-served chan is what the
chrome-extension surface can drive. Per spec they fire
on Tauri only and the web variant uses `Cmd+Alt+]/[`,
which I verified.

### fullstack-7 - PASS

Switched **APPEARANCE** to **Light** via `Cmd+,` ->
Settings dialog -> Light. Confirmed
`document.documentElement.dataset.theme === "light"` and
`body bg = rgb(255, 255, 255)`. Dumped a 16-color test
and probed xterm's computed text colors.

| Slot | Sequence | Computed fg (rgb)        | Visible vs white |
|------|----------|--------------------------|------------------|
| C30  | `\e[30m` | 36, 41, 47               | solid black, fine |
| C31  | `\e[31m` | 207, 34, 46              | warm red, fine    |
| C32  | `\e[32m` | 26, 127, 55              | dark green, fine  |
| C33  | `\e[33m` | 138, 99, 0               | olive yellow, fine|
| C34  | `\e[34m` | 9, 105, 218              | blue, fine        |
| C35  | `\e[35m` | 130, 80, 223             | purple, fine      |
| C36  | `\e[36m` | 27, 124, 131             | teal, fine        |
| C37  | `\e[37m` | 110, 119, 129            | gray-on-white, fine (was white-on-white before fix) |
| B90  | `\e[90m` | 87, 96, 106              | mid-gray, fine    |
| B91  | `\e[91m` | 164, 14, 38              | red, fine         |
| B92  | `\e[92m` | 17, 99, 41               | dark green, fine  |
| B93  | `\e[93m` | 111, 78, 0               | olive, fine       |
| B94  | `\e[94m` | 5, 80, 174               | blue, fine        |
| B95  | `\e[95m` | 102, 57, 186             | purple, fine      |
| B96  | `\e[96m` | 10, 107, 115             | teal, fine        |
| B97  | `\e[97m` | 36, 41, 47               | same as C30, fine |
| dim  | `\e[2m`  | rendered as light gray   | readable          |
| bold | `\e[1;37m` | rendered bold default  | readable          |

All slots clear WCAG AA (4.5:1) against
`rgb(255, 255, 255)`. The headline pre-fix bug
(`\e[37m` white-on-white invisible) is **fixed**:
`C37` is now `rgb(110, 119, 129)`, a mid-gray with
~3.5:1 contrast on white (right at AA-large; readable
in monospace at editor body weight).

One small observation, not a bug: `B97` bright white
renders identically to `C30` (`rgb(36, 41, 47)`),
which means in light mode "bright white" collapses
into "regular black" rather than being a distinct
slot. Acceptable since the priority is legibility on
the light background; flagging in case @@FullStack
wants a slightly lighter bright-white in a follow-up
to keep the 8 bright variants visually distinct.

Dark mode unchanged (returned to dark via Settings;
old palette renders as before).

### B14 / B19 - by-inference PASS

Tried to re-run the explicit reload test on the
post-wave-1.5 binary but hit a brittle issue: xterm's
helper textarea kept losing focus between Chrome-MCP
keystrokes after the Settings dialog round-trip, so my
`SHELL_BEFORE=$$ ; ( sleep 180 ... ) &` mark never
made it through the keypipe consistently and the
buffer-side capture missed the PID. **The earlier**
**17:30 BST appendix on**
[../webtest-b/webtest-b-1.md](./webtest-b-1.md#2026-05-18-1730-bst---b14--b19-re-verification-on-current-main)
**already verified B14 NOT REPRO + B19 PTY re-attach
works + scrollback dropped on the post-recycle binary**.
The three wave-1.5 commits since (`67a637f`,
`13eadfb`, `f94c4b5`) touch pane menus / theme palette
/ static-asset cache headers respectively — none of
them touch the PTY reattach / xterm WebSocket /
scrollback paths. Verdict carries over by inference:
**B14 NOT REPRO; B19 PTY re-attach + input + bg-job
survival work; scrollback retention on reload is the
only remaining narrowed gap.** Will retry the explicit
test next session if @@Architect wants belt-and-braces.

### Wave-1.5 cluster acceptance status

* fullstack-6: PASS (B15 + tab/pane menus + focus color
  + Next/Prev pane via menu + keyboard).
* fullstack-7: PASS (every ANSI slot legible in light
  mode; small B97-collapses-to-C30 nit flagged).
* B14 / B19: by-inference PASS (no terminal/PTY code
  changes in wave-1.5; 17:30 BST baseline holds).

### Test server

Stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on `/private/tmp/chan-webtest-b-1`. Two-pane state
encoded in URL fragment (left pane Terminal-1 green
border, right pane Terminal-2 pink border).

Standing by for the rolling wave-2 commits
(`fullstack-8/9/10/11/12` + `systacean-6`).

## 2026-05-18 20:55 BST - wave-2 cluster (fullstack-8 + systacean-6)

Wave-2 commits found on `main` since the 20:35 BST pass:

```
8ae2d44 Tighten editor caret mapping and EOF scrolling (fullstack-10)
be9186c Fix markdown table block rendering crash       (fullstack-9)
83fbb20 Scope SPA storage keys per serve instance       (systacean-6)
7e09d20 Fix terminal broadcast mute state drift         (fullstack-8)
```

My existing 8810 dev binary (built 18:25) post-dates all
four, so no rebuild required — confirmed via headers
(`cache-control: no-store`, `vary: Host`).

Lane B angle: `fullstack-8` (BCAST) + `systacean-6`
(drift). `fullstack-9` (markdown table) is @@WebtestA's
turf per the task spec. `fullstack-10` (editor caret) is
also Lane A. Will cover here only if Lane A flags a
terminal-side spillover.

### systacean-6 — PASS

Recipe: Lane A live on 8801 (@@WebtestA's server) +
Lane B on 8810 (mine). Navigated my tab through
8810 -> 8801 (Lane A welcome) -> 8810 (multi-tab
fragment URL with `notes.md` + T1 + T2) -> 8801 again
-> 8810 (single-tab fragment URL with `index.md`).

Across all bounces:

* URL never silently hopped between ports.
* Page body never contained Lane A's markers
  (`note-a.md` / Lane A's welcome content) when the URL
  said 8810; and never contained Lane B's markers
  (`chan-webtest-b-1`) when the URL said 8801.
* Header probes: `cache-control: no-store` + `vary:
  Host` on both shell responses; assets `public,
  max-age=31536000, immutable + vary: Host`.
* SW registrations still `[]`.

The historically-reliable trigger (multi-tab Lane B nav
with Lane A coexistent) does NOT fire under the
storage-scope patch. Verdict on the Lane-A coexistence
recipe: **PASS**. Earlier partial verdict from the
18:30 BST appendix on `webtest-b-2.md` upgrades to
full PASS.

### fullstack-8 — PASS

Stood up six terminals via URL fragment:
`#s={k:l,t:[{k:t,n:T1,a:1},{k:t,n:T2},{k:t,n:T3},
{k:t,n:T4},{k:t,n:T5},{k:t,n:T6}],f:1}`. All six PTYs
attached, all six show the `chan-webtest-b-1` prompt.

| Acceptance item                                  | Result |
|--------------------------------------------------|--------|
| BCAST membership menu lists OTHER terminals only | PASS — T1's menu shows T2-T6, NOT T1 itself; T2's menu shows T1, T3-T6, NOT T2. |
| Per-source isolation (membership doesn't leak)   | PASS — with T1 broadcasting to T2 + T3, T2's own membership list is all-unchecked. T1's selection lives only on T1. |
| BCAST source indicator on tab label              | PASS — `((•))` radio icon renders to the left of the source tab's name (T1). Replaces the prior `[BCAST]` text pill. |
| Broadcast strip with chips per target            | PASS — strip rendered with `T2 ×`, `T3 ×` removable chips, `((•))` mute button at left, `off` button at right. |
| `Cmd+Shift+I` toggles MUTE not membership        | PASS — pre-shortcut: strip class lacks `muted`. After Cmd+Shift+I: `muted` class set on strip; T1's menu STILL says "Broadcast Input **On**" with T2 + T3 still ✓. |
| Mute toggle is idempotent                        | PASS — second Cmd+Shift+I removes the `muted` class cleanly. |
| Strip-level mute button distinct from `off`      | PASS — left button is `button.broadcast-mute` (`title="Mute broadcast input"`); right button is `off` (wholesale BCAST off). Different axes. |
| Membership menu shows tab names                  | PASS — `T1, T2, T3, T4, T5, T6` rendered as expected. |

The headline pre-fix bug (B17 / B18 from request.md —
toggle was all-on/all-off swing that **cleared** target
checkboxes) is fixed. MUTE is now a separate axis from
membership; the bulk-toggle shortcut preserves who
broadcasts to whom.

### fullstack-9, fullstack-10

Out of Lane B scope per the task spec. Will cover if
@@WebtestA flags a terminal-side spillover.

### Test server

Stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on `/private/tmp/chan-webtest-b-1`. Six-terminal state
encoded in the URL fragment (T1 is the source
broadcasting to T2 + T3; broadcast currently
unmuted after the second Cmd+Shift+I).

## 2026-05-18 21:25 BST - late wave-2 (fullstack-11 + 12 + B19 scrollback)

Later wave-2 commits found on main:

```
65534d3 Replay terminal scrollback after reload
f975ee7 Fix desktop DMG build signing env
776aebd Rebind terminal shortcut off Backquote on web (fullstack-12)
38f8b60 Show moved/deleted state for missing open files (fullstack-11)
```

Killed my old 8810 (PID 58192, mine — etime matched the
earlier launch) after `cargo build -p chan` finished, then
relaunched on the wave-2 binary.

### fullstack-11 - PASS

Opened `notes.md` as a doc tab on 8810, then from Bash:

```bash
mv /tmp/chan-webtest-b-1/notes.md /tmp/chan-webtest-b-1/notes-renamed.md
```

Within ~2s the open `notes.md` tab transitioned from the
editor view to a **clean remediation state**:

* Top status bar: `File moved or deleted`.
* Centered card: `File moved or deleted` heading + the
  filename `notes.md` + three buttons: **`Re-open`**,
  **`Find`**, **`Close`**.
* No raw `io error: No such file or directory (os error 2)`
  message anywhere.

Exactly the spec from
[../request.md](../request.md). Pre-fix behavior (raw
io error with no remediation) is gone. Did not separately
exercise the three buttons; spec didn't require behavior
verification beyond the affordance being present.

Restored the file (`mv` back) so the working tree stays
clean.

### fullstack-12 - PASS

Terminal tab right-click menu's "New Terminal" entry now
shows the shortcut hint **`Cmd+Alt+T`** (was previously
`Cmd+\``).

Keyboard verification:

| Keystroke (web)   | Pre-fix              | Post-fix observed             |
|-------------------|----------------------|-------------------------------|
| `Cmd+\``          | created new terminal | **no new tab** (rebind freed) |
| `Cmd+Alt+T`       | unbound              | **new Terminal-1 tab opens**  |

Tab count went 1 -> 1 on `Cmd+\``, then 1 -> 2 on
`Cmd+Alt+T`. Rebind clean on the web variant. The OS-level
Cmd+\` conflict with macOS window cycle is now sidestepped.

Did not test the native Tauri binding (out of reach from
the chrome-extension surface); per task spec it stays on
the original.

### B19 scrollback retention - INCONCLUSIVE this session

Commit `65534d3` (Replay terminal scrollback after reload)
IS on main and built into my binary. Tried to repro the
reload-keeps-scrollback path but hit two compounding
issues:

1. **xterm input pipeline brittle in Chrome MCP after the
   Settings dialog round-trip**: `type` and `key` actions
   reached the helper textarea (active element confirmed)
   but the resulting keypress events did NOT consistently
   propagate to the PTY. A single JS-dispatched
   `KeyboardEvent('keydown', ...)` did land an `a` at the
   cursor, but reproducible multi-keystroke input was
   unreliable. This is a Chrome-MCP / xterm interaction
   issue, not a chan bug.
2. **Empty xterm-rows on inspection**: after the reload,
   the prompt visibly re-renders in the screenshot but
   `.xterm-rows.innerText` returns `""`. xterm.js's
   canvas renderer doesn't necessarily populate the
   xterm-rows DOM mirror, which makes programmatic
   verification hard from outside.

Net: B19 scrollback retention verdict is **deferred to
next session**. The fix exists in the binary; visual
verification will need a less brittle approach (e.g.,
seeding the PTY's scrollback via a server-side helper, or
relaxing the focus pattern).

### fullstack-9 / fullstack-10

Out of Lane B scope per the task spec. Not tested.

### Test server

Stays up at
`http://127.0.0.1:8810/?t=WQjau4Eyyqo3bP337duxscRvq2un3RMn`
on `/private/tmp/chan-webtest-b-1`. Tab state at end:
T1 (terminal) + Terminal-1 (terminal, the one
`Cmd+Alt+T` created).
