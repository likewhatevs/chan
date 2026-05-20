# Bugs
- File Browser: tab keeps name of removed file
  - open file browser, create file: foo.md
  - the file is selected, the tab's name follow
  - delete the file; tab name is now the name of the removed file
  - what i want: from now on, the file browser's tab name will be always a directory, with trailing slash in the name
    - if the selected item is a file, we use the parent dir's name
  - the name of the root dir, or the drive, is the name of the drive itself
  - the default drive name should be derived from the path, e.g. ~/dev/foo/bar = bar
- Clicking the notification status opens the settings overlay
  - This is wrong, let's remove all click events to the status bar, except for the closing of the notification expand/collapse
- the cmdline:
  - chan list is not very scriptable, it should be; we shoud include --json
  - chan remove requires a path, which is fine (are the drive names unique? if they are, we should accept --name)
- Pressing cmd+k prints "pane mode" at the status bar.. should say: Hybrid ☯ Enter commit, Esc discard, H help
  - And let's remove the flashing H from middle of the screen
- Rich prompt: opening rich prompt should bring the cursor over to the rich prompt
  - And after pressing cmd+enter the cursor should remain in the rich prompt area
- add cmd+t for new terminal
- for commands 1, 2, 3, cmd+k commits immediately
- The graph shows links to files that it says are not in the repo: ![](./attachments/image-1.png#w=250) 
  - Can repro seeding the drive with chan's own source code and journals
- Closing the last tab of the Hybrid should not close the Hybrid itself.. the space should stay
- Clicking 'spawn agent' from rich prompt just turns the screen into an overlay, not showing dialog: ![](./attachments/image-2.png#w=250)
- Inserting / pasting image at the end of the document pushes the cursor out of the screen and it does not roll over until the user types the next character; it should detect new images and roll if the last line has approched the end of visible area of the document
- terminal line adjustment still buggy:
  - iterm: ![](./attachments/image-3.png#w=250)
  - chan's term:  ![](./attachments/image-4.png#w=250)
- Native window state is not persisted: closing the (last) chan-desktop window resets the next open to a blank New state
  - want: stackable record of window configs; closing the last window remembers its layout (panes, tabs, selections, hash state) so the next open restores it
  - keep up to 20 window configs for now; LRU eviction beyond that
  - applies to the native shell (chan-desktop / Tauri); browser tabs are out of scope
- Rich-prompt watcher hung on first try (2026-05-19 v0.11.0)
  - vague repro: first time using the watcher on this build, hang observed before any test event landed
  - need a reliable repro before triage; flagging now so we don't lose the signal
- Watcher dir picker is over-restricted by the drive sandbox
  - Pointing the rich-prompt watcher at `/tmp/...` (outside the drive root) is rejected; user has to put the watcher dir inside the drive
  - But event files are infra traffic, not user content (per phase-7 architecture: bypasses chan-drive, written via tokio::fs in the event-reply endpoint)
  - Want: the watcher dialog accepts arbitrary filesystem paths (with a clear "outside drive root" hint if useful), not gated by the drive's editable-text sandbox
- Watcher dialog "create dir" flow is wrong
  - When the path doesn't exist: error instead of silent create
  - When the path exists: warns it will "overwrite", which is not what attaching a watcher does
  - Want: if missing → create silently (or with a single confirm); if exists → just attach the watcher, no overwrite warning ever (we never overwrite a dir when attaching)
- Terminal scrollback truncated / lost too aggressively
  - repro: Alex lost earlier prompt context in an active terminal session; the lines were no longer reachable by scrolling
  - want: a generous scrollback buffer (10k+ lines), and no resets on focus/theme/pane changes
  - audit xterm.js config + any custom buffer trims we added during BCAST / mute reworks
- Survey bubble keeps re-popping after a reply has been sent
  - confirmed repro (smoke test 2026-05-19 v0.11.0): drop a `survey` event, reply via the bubble, reply lands as `event-reply-<id>.md`, bubble re-appears
  - root cause: `web/src/state/watcherEvents.ts::readWatcherEvents` lists the dir and returns all `event-*.json|md` it finds, including surveys that have already been replied to. The reply file is a sibling, not a tombstone over the original.
  - fix options (phase-8 fullstack call):
    a) producer (or chan-server reply endpoint) deletes / archives the original survey JSON once a `survey-reply` with the same id lands
    b) SPA pairs survey + survey-reply by id and filters answered surveys out of the bubble queue
    c) reply endpoint atomically renames the survey JSON to a `.replied` sibling (audit trail preserved, listing filter is trivial)
  - the chan-server watcher dedup (`SeenEventIds`) is fine; this is purely a SPA bubble-queue bug. Don't touch the server dedup
- Rich prompt overlay obscures the bottom of the terminal
  - want: when the rich prompt opens, push the terminal up (or resize cleanly) so the last rendered terminal line stays visible above the prompt
  - resize is acceptable; today's overlay simply paints over the bottom
- Rich prompt cursor focus on open
  - if no notifications: cursor lands in the prompt input (as today, but explicitly)
  - if notifications/bubbles present: cursor lands in the survey area (numbered keystroke replies the survey)
  - on dismiss of all bubbles: cursor returns to the prompt input
- Notification flash colour is wrong
  - today: flashing blue
  - want: flashing yellow (closer to a "needs your attention" cue; blue reads as info/idle)
- Closing the last tab on the back of a Hybrid auto-flips back to front
  - repro from Alex (2026-05-19): Hybrid with a terminal on the front, some tabs on the back; close the last tab on the back; the Hybrid flips back to the front side
  - expected: the back stays as-is, empty (renders the empty-pane landing), no auto-flip
  - related to but distinct from the `fullstack-a-5` fix (which dropped `collapseEmptyPane` on last-tab close to preserve the pane). That fix preserved the PANE; this bug is about the per-Hybrid flip state — the empty back should stay shown, not auto-pivot to the front
  - keep the rule consistent: closing the last tab does NOT change Hybrid orientation. The user explicitly flips with Cmd+. Tab (or whatever the binding is); never as a side effect
  - repro from Alex (2026-05-19): in Hybrid NAV mode, pressing `[` / `]` to resize the divider often produces the opposite of expected motion, but not always
  - hypothesis: divider axis (horizontal vs vertical split) flips the key→direction mapping inconsistently; OR the orientation of the focused split differs from the user's mental model
  - needs investigation; could be the binding really is inverted, could be the orientation of the resize is ambiguous and we need an explicit convention
  - keep the convention sane: `[` grows left/up (or shrinks right/down) consistently; `]` grows right/down (or shrinks left/up). Pick one and stick to it; document in PaneModeHelp
- Tab name abbreviation looks ugly — copy Chrome's fade-out style
  - reference screenshot from Alex (2026-05-19) — Chrome **fades** the tab name into transparency at the right edge of the tab, NO trailing ellipsis character. Clean, no glyph clutter
  - implementation: `mask-image: linear-gradient(to right, black <N>%, transparent)` (or `-webkit-mask-image` for parity) on the tab-name span. The actual text overflow stays hidden via the tab's clip
  - applies to all tab strips: document tabs, terminal tabs, browser tabs (FB/Graph/Search)
  - hover tooltip on file tabs MUST show the full file path (not just the basename). For non-file tabs (terminal, Graph, Search) hover tooltip is nice-to-have, not required
  - file browser tree items also get the full-path hover tooltip: hovering an entry inside the FB tree (file OR directory row) shows the absolute path the entry resolves to
- Docked file browser flickers / reloads on any drive activity, even when the activity is outside the FB's visible scope
  - corrected repro from Alex (2026-05-19): the drive IS active (other agents writing in `crates/`, `web/`, etc.); the FB has only `tasks/` (a single subtree) expanded but is reloading because the watcher fires on every change anywhere in the drive
  - same bug as phase-7 next-phase-backlog item 9 ("Scope FB watcher to current dir or parent of selected file"); promoting it from Round 2 → Round 1 because the pain is current and disrupts navigation in any non-trivial session
  - fix direction (from item 9): per-tab watcher scoped to the selected dir (or parent of selected file if a file is selected); detach on tab close / scope change. Watcher API on chan-drive may need a subscribe-by-prefix extension; audit before designing
- Dark/light theme flip leaves half the Hybrid in the wrong palette
  - repro from Alex (2026-05-19): app in dark mode globally; back of a Hybrid pane renders in light mode → white-on-white editor surfaces, broken contrast
  - the per-Hybrid theme override (`ht`/`hb` from phase-7 `fullstack-59`) propagates to xterm.js + GraphCanvas (phase-7 `fullstack-78`) but NOT to the editor surfaces / section chrome on the back side
  - per-pane front/back have independent state (phase-7 `fullstack-70` preserved back-side state across splitPane); the theme propagation needs to apply to BOTH sides of the binary-tree, not just the front
  - acceptance: every surface inside the Hybrid pane on both front and back (editor area, sidebar chrome, section blocks, find/cmd overlays, etc.) honours the per-Hybrid theme override; flipping is fully consistent
- CSS wobble effect missing from Hybrid and right-click menus
  - never asked to remove it; it regressed and needs to come back
  - reference: still applied on the OverlayShell for Search and Settings
  - restore wobble for: Hybrid NAV entry overlay, pane right-click menus, tab right-click menus, FB / Graph right-click menus
- Switch Hybrid NAV binding from Cmd+K to Cmd+.
  - Cmd+, becomes Settings (matches macOS convention)
  - Cmd+. becomes Hybrid NAV (replaces Cmd+K)
  - hard switch: drop Cmd+K binding, don't leave both active
  - update the status-bar label from `Hybrid ☯ Enter commit, Esc discard, H help` to use Cmd+. + ensure all references in PaneModeHelp, copy, and screenshots match
- Cmd+K F (enter search overlay) does not focus the cursor in the search input
  - want: opening the search overlay via Cmd+K F lands the caret in the search field; typing immediately searches
  - same family as the rich-prompt cursor-focus bug (consistent UX rule: open an overlay → caret in the primary input)
- Index chart in the carousel is trimmed and not pannable
  - the indexing-graph slide in the carousel clips at the viewport edges
  - cannot be moved / panned around to see what's off-screen
  - want: use the same pan/zoom settings as the regular Graph view (drag to pan, wheel to zoom, recenter affordance)
- chan-desktop: external `http`/`https` links do not open at all
  - flagged 2026-05-20 by Alex: in the current Tauri-built Chan.app, clicking an external `http://...` link inside the webview does nothing, no navigation, no external browser launch
  - tested by clicking the round-1 test-server URL (`http://127.0.0.1:8787/?t=...`); click is a complete no-op
  - want: external `http`/`https` links handed off to the OS default browser via Tauri's `shell.open` (or equivalent host bridge); internal app routes stay inside the webview
  - lives in chan-desktop / `desktop/src-tauri/` link/url handling, not the web SPA
  - dispatched as `fullstack-b-7`
- Graph inspector falls back to "not in current file listing" even when server-side missing=false (SPA second-ghost on lazy-tree path)
  - flagged 2026-05-20 by @@WebtestA via Round-1 sweep: 5 plain non-markdown files (LICENSE, desktop/LICENSE, two `crates/chan-drive/src/*.rs`, a docs shell script) flagged with the warning despite being on disk; server side now reports `missing: false` for them post-`systacean-2`
  - SPA `GraphPanel.svelte::isFileGhost` derives a second-ghost state from `tree.entries` (the lazy file-browser tree); for an unexpanded subtree the lazy tree has no record and the inspector tips into ghost mode despite the server flag
  - want: drop the lazy-tree branch (server flag is source of truth) OR gate it on `tree.loadedDirs` covering `dirname(path)`
  - dispatched as `fullstack-a-12`
- Editor image-insert snaps viewport to top + subsequent typing does not roll the view
  - flagged 2026-05-20 by @@WebtestA: inserting `![](./test-image.png)` at end of long doc (README.md on lane-A drive) throws the cursor ~3.2k px off-screen and typing more characters does not scroll the view back; even worse than the original bug description
  - want: cursor stays in view after image insert, OR view auto-scrolls to cursor within one paint; subsequent typing rolls the view as normal
  - dispatched as `fullstack-a-13`
- Rich prompt re-open with bubble present focuses prompt input (should focus survey area)
  - flagged 2026-05-20 by @@WebtestA: cold-open path obeys the `fullstack-a-4` rule but re-open-while-bubble-present still focuses the prompt input
  - root-cause hypothesis: focus-effect grabs prompt input before BubbleOverlay's bubbleCount catches up (re-open path doesn't read the count synchronously)
  - want: re-open path obeys the same "bubbles present → caret in survey area" rule as cold-open
  - dispatched as `fullstack-a-14`
- Cmd+Enter from rich prompt drops first character into terminal
  - flagged 2026-05-20 by @@WebtestA: `echo hello` arrives at the focused terminal as `cho hello`; intermittent but reproducible
  - likely a timing/focus race: the dispatch writes to the terminal before focus transfers, or xterm.js drops the first byte during a focus-in animation frame
  - want: full text reaches the focused terminal every time
  - dispatched as `fullstack-b-8`
- Cmd+T new terminal blocked on web (Chrome reserves chord)
  - flagged 2026-05-20 by @@WebtestA: Cmd+T works on native chan-desktop (post-`fullstack-b-2`) but Chrome reserves Cmd+T as new-tab so the chord is unreachable in the SPA browser path; verdict on the original bug is "partial"
  - want: web users have a way to open a new terminal; native users keep Cmd+T
  - resolution options in the task body (pick alternate chord, Hybrid NAV `t` recommended)
  - dispatched as `fullstack-b-9`
- "New file" dialog appends `.md` even if typed name already ends in `.md`
  - side observation from @@WebtestA Round-1 sweep on 2026-05-20: typing `foo.md` in the "New file" dialog creates `foo.md.md` on disk
  - want: only append the extension if the typed name does not already end in `.md` (or strictly: any markdown-recognised extension)
  - dispatched as `fullstack-a-15`
- Hybrid NAV help overlay labels 1/2/3 as "Stage:" but runtime is immediate-commit
  - side observation from @@WebtestA Round-1 sweep on 2026-05-20: `fullstack-a-3` made 1/2/3 immediate-commit but the help overlay still uses "Stage:" copy implying a two-step commit
  - want: help copy updated to match the immediate-commit behaviour
  - dispatched as `fullstack-a-16`
- Cmd+K → p (spawn terminal) steals rich-prompt input focus to xterm-helper-textarea
  - side observation from @@WebtestA Round-1 sweep on 2026-05-20: after Cmd+K p, the newly-spawned terminal's `xterm-helper-textarea` grabs focus from the rich prompt input
  - want: rich prompt keeps focus after the spawn (consistent with the `fullstack-a-4` open-time rules)
  - dispatched as `fullstack-a-17`
- Graph route emits 3 directory link targets as `kind: file` ghost nodes
  - side observation from @@WebtestA Round-1 sweep on 2026-05-20: in addition to the 5 plain-file false-positives in bug 8 (handled by `systacean-2` + `fullstack-a-12`), 3 directories appear in the missing-nodes list with `kind: file` despite being directories on disk
  - @@Systacean's root-cause audit (2026-05-20): NOT an indexer typing leak. The graph indexer never inserts directory paths into the nodes table. The leak lives in `crates/chan-server/src/routes/graph.rs::api_graph`'s ghost path — markdown links to directories (e.g. `[label](../alex/)`) hit `ghost_set` because `disk_files` filters `!e.is_dir`, so the dst falls through to a `File { missing: true }` emission with `kind: file`
  - want: directories never appear as `kind: file` nodes in the graph
  - dispatched as `systacean-4` (option A approved by @@Architect: drop directory dsts from ghost emission AND drop the edge — smallest patch, no SPA work, no schema growth)
- chan-server event_watcher emits "Is a directory" error on freshly-created watch root
  - flagged 2026-05-20 by @@WebtestB during fullstack-b-3 wave-1 verification: attaching the watcher to a freshly-created empty directory produces `failed to read event file <path>: Is a directory (os error 21)` on the server side; surfaces as a red toast top-right
  - case a (fresh outside-drive dir): toast surfaces; case b (fresh in-drive dir): quieter, no toast; case c (existing dir with files): no error
  - likely the watcher polls the watch root as if it were an event-file journal — needs to filter the root itself out of the read-event-file enumeration
  - dispatched as `systacean-5`
- Watcher dialog still shows "overwrites existing directory" warning for in-drive dirs (call site not switched after fullstack-b-3 fix)
  - flagged 2026-05-20 by @@WebtestB during fullstack-b-3 wave-1 verification: backend `resolve_watcher_dir` works correctly, but `TerminalRichPrompt.svelte:197` still passes `mode: "move"` to `uiPathPrompt`. The new `PathPromptMode = "attach"` branches in `PathPromptModal.svelte` are live but never reached
  - want: flip call site to `mode: "attach"` + update hint copy from "moves to X/" to "attach watcher to X/"
  - dispatched as `fullstack-b-10`
- Wysiwyg-mode Cmd+Enter from rich prompt silently consumed (no dispatch)
  - caught 2026-05-20 by @@FullStackB during fullstack-b-8 root-cause investigation: `TerminalRichPrompt` doesn't thread `onSubmit` into the `<Wysiwyg>` child; the Wysiwyg keymap's `Mod-Enter` handler calls `onSubmit?.()` against undefined and returns true, eating the chord
  - source mode works only because Source has no Mod-Enter binding and the event bubbles to the wrapper
  - want: parity with source mode; Cmd+Enter dispatches from both modes
  - dispatched as `fullstack-a-18`
- Wysiwyg-mode Cmd+Enter double-dispatches text to the terminal (regression from `fullstack-a-18`)
  - flagged 2026-05-20 by @@Alex: type `pwd` (no Enter) in the rich prompt, press Cmd+Enter, terminal shows `pwdpwd`
  - root cause: `fullstack-a-18` threaded `onSubmit={submit}` into Wysiwyg's keymap, which now calls `submit()` once and returns true → `preventDefault` on the DOM event. But the wrapper's `onKeydown` at `TerminalRichPrompt.svelte:118-122` doesn't check `e.defaultPrevented`, so it ALSO calls `submit()` → double dispatch
  - source mode unaffected because Source has no Mod-Enter binding and the chord bubbles cleanly
  - want: wrapper's `onKeydown` returns early on `e.defaultPrevented` so children that handled the event aren't re-handled
  - dispatched as `fullstack-a-20`; hard gate before v0.11.1 (must land alongside or before `fullstack-a-18` ships)
- Hybrid NAV chord-table documentation drift in PaneModeHelp + SERVE_LONG_ABOUT
  - flagged 2026-05-20 by @@FullStackB during fullstack-b-9 work: the Hybrid NAV section still says "Pane Mode (Cmd+K)" (renamed to "Hybrid NAV (Cmd+.)" in `fullstack-a-7`), lists `s` for Search (moved to `f` in phase-7 `fullstack-74`), lists `k` for kill-pane (moved to Cmd+K Backspace in phase-7 `fullstack-77`)
  - want: PaneModeHelp + SERVE_LONG_ABOUT match the actual runtime chord set; audit the rest of the table for additional drift while in the file
  - dispatched as `fullstack-a-19`
- chan index status blocks on the drive lock while chan serve is running on the same drive
  - flagged 2026-05-20 by @@WebtestB during a proactive CLI walk on systacean-7: `chan index status --path <live-served-drive>` errors with "drive is locked by another process"; should be readable any time since status is read-only
  - want: read-only / shared lock for the status path, or skip the lock entirely
  - dispatched as `systacean-8`
- chan index status auto-registers on a non-existent path
  - flagged 2026-05-20 by @@WebtestB: `chan index status --path /tmp/nonexistent` emits "Error: registering /tmp/nonexistent" — a read-only query has a registration side-effect, and the error message leaks the implementation detail
  - want: refuse cleanly without registering; user-visible message names the problem ("not a chan drive at <path>")
  - dispatched as `systacean-8`
- chan index argument-shape asymmetry: `rebuild` takes positional `<PATH>`, others take `--path` flag
  - flagged 2026-05-20 by @@WebtestB: script-writers have to special-case `rebuild`; suggested accepting `--path` as a synonym on `rebuild` for uniform script handling
  - dispatched as `systacean-8`
- chan-desktop FB dock shows a visible vertical separator line
  - flagged 2026-05-20 by @@Alex (with screenshot): the docked file-browser separator bar between the FB tree and the editor pane is visually intrusive; user wants the element kept (drag-resize works) but the visible paint gone
  - want: idle `background: var(--separator)` paint removed; hover state + cursor stay as the discovery affordance
  - dispatched as `fullstack-a-23` (Option A locked: per-instance `idleVisible?: boolean` prop on ResizeHandle)
- Terminal swallows app-level chords (Option+Shift+[/] tab cycle and friends) when xterm has focus
  - flagged 2026-05-20 by @@Alex dogfooding: in the web build, `Option+Shift+[` / `Option+Shift+]` cycles tabs from any pane EXCEPT when the focused pane is a terminal — xterm.js's `attachCustomKeyEventHandler` consumes the keystroke before chan's app-level keydown sees it
  - native probably works (per @@Alex: "we switch on native, which probably already works because Cmd+Shift+[/]" — untested; needs confirmation as part of the fix verification)
  - root cause: chan's `handleTerminalKeyEvent` (the function passed to xterm's `attachCustomKeyEventHandler`) returns true (let xterm process) for the chord. xterm consumes + writes the escape sequence to the PTY; the App-level keydown never fires
  - **fix MUST be dynamic** per @@Alex — don't hardcode "intercept Option+Shift+[/] specifically." The handler should consult chan's shortcut registry and return false (don't consume in xterm; let bubble) for ANY chord that matches an app-level shortcut. If a user customises a tab-cycle chord later, the intercept follows automatically
  - scope of chords that should escape terminal focus (all the global ones):
    * Tab cycle: `Option+Shift+[` / `Option+Shift+]` (web) + `Cmd+Shift+[` / `Cmd+Shift+]` (native)
    * Hybrid NAV entry: `Cmd+.`
    * Settings: `Cmd+,`
    * New terminal: `Cmd+T` + `Cmd+Alt+T` (web)
    * File browser: `Cmd+O` + web alt (Round-2 chord migration)
    * Rich prompt: `Cmd+P` + web alt (Round-2 chord migration)
    * Graph: `Cmd+Shift+M` + universal `Mod+. v` (Round-2 chord migration)
    * Alt+Space (rich prompt open)
    * Any chord registered with the "global" / "always-on" flag in `shortcuts.ts`
  - implementation shape:
    1. Extend the shortcut registry in `web/src/state/shortcuts.ts` so each entry can carry a flag like `escapeTerminal: true` (or default-on for top-level chords; default-off for in-mode chords like Hybrid NAV interior keys)
    2. `handleTerminalKeyEvent` consults the registry: for an incoming `KeyboardEvent`, derive the chord shape, look up in the registry. If matched and `escapeTerminal` is true → return `false` (don't consume; bubble to App-level). Otherwise current behaviour
    3. Test pin: at least one test asserting `Option+Shift+]` against a terminal pane bubbles to the App-level tab-cycle handler instead of writing to the PTY
  - native confirmation: part of the fix is verifying the native path (where `Cmd+Shift+[/]` is the equivalent chord) already works — if it does, the implementation difference is the web vs native chord; if it doesn't, the same fix covers both
  - cross-lane touch: the intercept lives in `TerminalTab.svelte`'s xterm handler (@@FullStackB territory) but consumes from `shortcuts.ts` (@@FullStackA's chord-migration work). Coordinate with the Round-2 chord-migration task drafted in round-2-plan.md — pair this fix with that task so the new chord set lands escape-aware from the start
  - dispatched for Round-2 wave-1 — primary owner @@FullStackB (xterm handler), pairs with @@FullStackA's chord migration; task cut at Round-2 fan-out
- Docked file-browser rows overflow into adjacent rows when the dock is shrunk smaller than a name
  - flagged 2026-05-20 by @@Alex dogfooding (screenshot in the conversation): shrinking the docked FB to a width smaller than the longest visible path produces overlapping rows. `chan-pre-release-phase-1/`, `chan-pre-release-phase-2/`, etc. visually stack on each other instead of either wrapping to multiple lines or truncating with a fade
  - @@Alex's framing: "in my world i'd say they `\r` instead of `\n`" — the text is carriage-returning over the next row instead of newline-wrapping
  - root cause family: row CSS likely has `white-space: nowrap` (no wrap) + `overflow: visible` (bleeds past width) + fixed row height (subsequent rows positioned at fixed offset, sitting "underneath" the overflow text)
  - `fullstack-a-10` added the `title=` tooltip on FB tree rows for the full path on hover, but didn't change the row's overflow behaviour — so the visual gap is still there at narrow widths
  - **three fix-direction options**:
    1. **Truncate with the Chrome-style fade from `-a-10`** — apply the same `mask-image: linear-gradient(...)` pattern that the tab-strip uses, so long names fade to transparent at the right edge instead of bleeding. Fixed row height stays; tooltip on hover already shows the full path
    2. **Wrap to multiple lines** — `white-space: normal` + `word-break: break-word` + dynamic row height. Each row grows to fit its content. Cleaner visually but row heights vary which can feel jumpy when scrolling
    3. **Hard truncate with `text-overflow: ellipsis`** — fallback shape; loses the fade aesthetic but is the simplest CSS
  - recommendation: option 1 (Chrome-style fade) — keeps the row-height invariant for scroll predictability + matches the tab-strip pattern @@Alex picked in `-a-10`. Tooltip + full-path-on-hover already covers the "see the rest" UX
  - lives in `FileTree.svelte` row CSS (the same component `-a-10` touched for the tooltip wire-up)
  - dispatched for Round-2 wave-1 against @@FullStackA (file-browser component lane, owned `-a-10`); task cut at Round-2 fan-out
- OverlayShell doesn't claim keyboard focus on open; ESC routes to the underlying focused element
  - flagged 2026-05-20 by @@Alex dogfooding: terminal has focus → Cmd+, opens Settings (OverlayShell) → ESC. Instead of dismissing the overlay, the ESC keystroke lands on the terminal (xterm.js helper textarea still has focus); the overlay stays open.
  - root cause family: same as `fullstack-a-17` (Cmd+K p spawn) and `fullstack-a-20` (Cmd+Enter double-dispatch) — focus discipline. When the OverlayShell mounts it does NOT blur the currently-focused element + capture keyboard focus inside itself. The xterm helper textarea retains focus; subsequent keystrokes (including ESC) route to xterm, not the overlay.
  - applies to every OverlayShell consumer that doesn't already grab focus in its mount-effect: Settings (per the repro), Spawn dialog, possibly others. Search (Cmd+K F per `fullstack-a-6`) already focuses its input on open + likely works; audit confirms which other overlays need the fix.
  - want: OverlayShell **always** claims keyboard focus on mount. Either via a focused element inside the overlay (Settings panel's first input / Search's search field) OR a hidden focus trap on the overlay root. ESC routes to the overlay's dismiss handler unconditionally while the overlay is open.
  - shape of fix: similar to the `blurTerminalHelperTextarea` helper from `-b-8` but as a generic OverlayShell on-mount step. Or use a `<dialog>` element with `showModal()` which the browser handles natively. Implementer picks; both reach the same outcome.
  - dispatched for Round-2 wave-1 against @@FullStackA (overlay components + focus rules lane); task cut at Round-2 fan-out
- Terminal box-drawing characters render with broken alignment under Source Code Pro + bundled-font default
  - flagged 2026-05-20 by @@Alex dogfooding: Claude Code's bordered welcome banner renders in chan's terminal with VISIBLE misalignment on the vertical + horizontal bars + corner glyphs of the box. Same banner in iTerm with the same Source Code Pro Regular 14pt renders cleanly.
  - root cause: Source Code Pro's coverage of the **Box Drawing** unicode block (U+2500-U+257F) is incomplete. The browser falls back to the NEXT font in chan's `fontFamily` chain (SF Mono / Menlo / Consolas / monospace) for missing glyphs. The fallback font has different cell metrics; the browser does NOT scale it to match the primary cell width. iTerm hides the issue because macOS CoreText auto-falls-back AND scales to match the cell. xterm.js / browsers don't.
  - affects every TUI / borders / tables / box-drawing program: Claude Code banner, `less`, `htop`, `mc`, `gum` tables, etc.
  - **direction confirmed 2026-05-20 by @@Alex** (after the initial "Source Code Pro is optional, like BGE-small" framing): mirror the BGE-small architecture from `systacean-6` / `-7` / `fullstack-a-21`. Default build ships NO font; per-OS native mono is the default. Source Code Pro is opt-in via Settings; downloaded on demand to user-config dir; cargo feature flag keeps the embedded-shipping path for power users / offline installs.
  - **shape of the fix**:
    1. **Revert `-b-12`'s rust-embed of the font**. The bundle in `crates/chan-server/resources/fonts/` goes behind a new cargo feature `embed-font` (default off), mirroring `embed-model` from `systacean-6`. Default `cargo build` no longer ships the woff2. `cargo build --features embed-font` keeps the embed path.
    2. **Per-OS native mono is the default `fontFamily`** in xterm.js. Suggested chain (single line; browser falls back per-OS to whatever's local):
       `"SF Mono", "Cascadia Code", "DejaVu Sans Mono", "Liberation Mono", Menlo, Consolas, "Source Code Pro", monospace`
       OS-native font-fallback handles box-drawing correctly at matched cell widths (same trick CoreText does for iTerm).
    3. **New Settings dropdown**: "Terminal font: [System default] / [Source Code Pro (downloads ~76 KB on enable)] / [Custom...]". Custom... is a free-text input for any installed font.
    4. **Source Code Pro download flow** mirrors `systacean-7`'s semantic-search download shape but smaller:
       * Server endpoint `POST /api/fonts/source-code-pro/download` fetches the woff2 + OFL.txt to `<user-config>/chan/fonts/source-code-pro/`.
       * Settings UI calls the endpoint when the user picks the option.
       * Synchronous download given the tiny size (~76 KB); no progress bar needed.
       * Idempotent (skips fetch if file already present + matches content-hash).
       * `--features embed-font` builds skip the download flow (font resolved from the embedded bundle instead).
    5. **Resolver shape** parallel to `resolve_model` from `systacean-6`: `resolve_font(name)` returns the path to the local font file (embedded OR downloaded OR error if missing + opted-in).
    6. **Settings hint under the Source Code Pro option**: "May have alignment issues with TUI box-drawing characters (`htop`, `less` borders). System default uses your OS's monospace font which handles these correctly."
  - dispatched for Round-2 wave-1 against @@FullStackB + @@Systacean (terminal-font lane + cargo-feature + chan-server endpoint). Splits like the BGE work did: @@Systacean cuts the cargo feature + resolver + endpoint; @@FullStackB updates the SettingsPanel + xterm.js default. Task cut at Round-2 fan-out.
- Settings page layout polish (Semantic search adjacency + Terminal 2-column + TERM restart hint)
  - flagged 2026-05-20 by @@Alex dogfooding the new build. Three deltas:
    1. **Adjacency**: move "Semantic search" (from `fullstack-a-21`) next to "On save" (from `fullstack-a-25`). Currently the two are separated by other sections; they're both editor-preference adjacent.
    2. **Terminal section to 2-column**: the Terminal section (from `fullstack-b-11`) currently spans full-width outside the `.section-row` 2-column wrapper. Move it into the 2-column layout like every section except ABOUT. Suggested layout: `[Terminal scrollback (MB)]  [Default TERM]` side-by-side in one row.
    3. **TERM restart hint**: surface a clearer warning under the Default TERM setting that changing it requires a terminal restart (kill existing + spawn new) to take effect. The spawn-time-only semantic was documented in the hint text from -b-11 but @@Alex wants it stronger — "Changing this requires restarting your terminals to take effect." or similar.
  - all three small; single SettingsPanel.svelte commit
  - dispatched for Round-2 wave-1 against @@FullStackA (SettingsPanel ownership); task cut at Round-2 fan-out
- Left-docked file-browser resize moves the FB DETAILS inspector along with the tree
  - flagged 2026-05-20 by @@Alex dogfooding: dragging the vertical bar between the left-docked FB tree and the editor pane resizes BOTH the FB tree AND its DETAILS inspector. Should be independent — two separate widths, two separate resize handles, two persistence keys
  - both widths must persist across page reload (currently the tree width persists via `paneWidths.browser` per `FileBrowserSidePane.svelte`; inspector width likely shares the state or is computed off it)
  - audit the FB dock layout (`FileBrowserSurface.svelte` variant=dock, sibling DETAILS inspector wherever it mounts) — likely a 2-column flex with one shared width source
  - want: tree width and inspector width each have their own resize handle + their own persistence key (e.g. `paneWidths.browserTree` + `paneWidths.browserInspector`); each handle has its own drag scope
  - dispatched for Round-2 wave-1 against @@FullStackA (SPA / dock layout); task cut at Round-2 fan-out
- CLI error messages lack context (seed: `chan serve` bind-port error doesn't name the address)
  - flagged 2026-05-20 by @@Alex dogfooding: running a second `chan serve` against the default port produced `Error: running server / Caused by: 0: io: Address already in use (os error 48) / 1: Address already in use (os error 48)`. Doesn't name WHICH address or port. A new / uninformed user can't act on the message without reading source or running `chan serve --help` to discover the default.
  - want: every `chan` CLI error names the user-facing input that produced it (port, path, env var, secret name, etc.). The bind-port case should read more like `Error: cannot bind to 127.0.0.1:8787: another process is using this port. Pick a different port with --port <N> or stop the other chan serve.`
  - broader theme per @@Alex: "we need to up our cmdline game by a lot" — this seed example is one instance of a category. Audit + improve every chan / chan-server / chan-drive error path.
  - dispatched as a Round-3 Track-3 cluster (cleanup + hardening); see [`architect/round-3-plan.md`](architect/round-3-plan.md) Track 3 for the audit scope.
- Outside-drive watcher read fails with "No such file or directory"
  - flagged 2026-05-20 by @@WebtestB during a proactive lane-B walk: attaching the watcher to an absolute outside-drive path succeeds (post `fullstack-b-3` + `systacean-5`), but reading events from that path errors with `watch read failed: io error: No such file or directory (os error 2)`. The read path enforces drive-sandbox resolution; absolute outside-drive paths fail the sandbox lookup
  - want: read path applies the same in-drive-vs-outside-drive split as the attach path's resolver
  - dispatched as `systacean-9`
- Graph: "graph from here" should be default; parent inspector should render ancestor scope navigation
  - flagged 2026-05-20 by @@Alex: today's graph view requires an explicit button click to engage "graph from here" mode (scope to subtree rooted at current selection). @@Alex wants this to be the default behaviour — open graph, render scoped to the active context, no button needed. The parent / breadcrumb inspector should render the ancestor chain so the user can navigate back up to the drive root scope, clicking ancestors to re-scope to "from here" rooted at each one
  - verbatim ask: "i want the graph's parent inspector to show the graph from here, enabling to go all the way back to drive where graph from here is the default and dont need a button"
  - pairs with the chord-migration task (Cmd+Shift+M from a doc spawns graph rooted at the doc, etc.)
  - dispatched as `fullstack-a-33`
- Chord migration: Cmd+T, Cmd+O, Cmd+P, Cmd+Shift+M with context-aware spawn semantics + surface unification
  - originally drafted in [`architect/round-2-plan.md`](architect/round-2-plan.md) "Chord migration + surface unification"; pulled forward into the rich-prompt mini-wave per @@Alex 2026-05-20
  - 2026-05-20 refinement (@@Alex): "e.g. from a doc, cmd+shift+m does graph from here using the doc; or cmd+t new terminal from current cwd or doc's parent dir" — each spawn chord picks up context from the focused surface (terminal cwd / doc parent dir / drive root fallback)
  - dispatched as `fullstack-a-32`
- Terminal broadcast selector: missing self entry + confusing on/off toggle shape
  - flagged 2026-05-20 by @@Alex: the terminal's broadcast-input selector (the UI that picks which terminal tabs receive the broadcast forwarded by `broadcastTerminalInput`) does not list the current tab itself in the selectable list. Also the current on/off toggle UI for the per-tab broadcast state is confusing
  - want, three parts:
    1. **Include self in the list**: the current tab appears in the broadcast-target list alongside the others. Mark it as "self" with an icon OR place it above the others with a separator (implementer's call; both shapes are acceptable)
    2. **Checkbox shape, not toggle**: drop the on/off rocker UI; use a plain checkbox per row instead
    3. **Label**: "broadcast input on/off" (keep the label text @@Alex named) — applied to whatever container UI hosts the per-tab checkboxes
  - small UX polish; ride the rich-prompt mini-wave so it gets into the patch release
  - dispatched as `fullstack-a-31`
- Survey-reply echoes to the terminal as `poke<Enter>`; breaks agents that need `poke<Cmd+Enter>`
  - flagged 2026-05-20 by @@Alex during the broadcast smoke test follow-up: clicking a reply option on the survey bubble correctly writes `event-reply-<id>.md` AND also echoes the reply into the underlying terminal's PTY. The echo shape is literally the string `poke` followed by Enter (newline). For a shell, Enter submits the line — fine. For an agent running in the terminal (Claude Code / codex / gemini), Enter inserts a newline into the agent's input draft; only Cmd+Enter submits the message. Result: the literal word `poke` ends up wedged in the agent's input draft, never submitted
  - @@Alex's verbatim ask: "poke<cmd+enter> not poke<enter>"
  - same root family as the item C "shell vs agent submit-mode" planned for the rich-prompt session evolution (see [`architect/rich-prompt-session-evolution.md`](architect/rich-prompt-session-evolution.md)) but surfacing at a second consumer site: the survey-reply path, not the rich-prompt Cmd+Enter path. **Both consumers need the same submit-mode toggle.**
  - want: a single per-prompt (or per-tab) shell-vs-agent toggle that governs how trailing-submit gets encoded into the PTY write. Shell mode → `\n` as today. Agent mode → the agent's submit chord (encoding to be confirmed at task-cut; common shapes are xterm modifier-other-keys `\x1b[27;9;13~` or a literal `\x0d`). Both code paths (rich-prompt submit + survey-reply echo) consume the same toggle
  - dispatched for the Round-2 rich-prompt mini-wave (see below) against @@FullStackB. The PTY chord-encoding research + the toggle wiring sit in the terminal / chan-server territory @@FullStackB owns. @@FullStackA's bubble-overlay regression task consumes the toggle for the survey-reply echo path
- Poke + pre-flight bubbles flicker; survey bubble does not; non-survey replies don't dismiss source bubble
  - flagged 2026-05-20 by @@Alex during the broadcast smoke test (screenshot): dropped three test events into a watcher dir — one `survey` (multi-question, 2 questions × 2 options), one `poke` (note + topic), one `pre-flight` (note + topic)
  - **Survey path**: validated end-to-end. Bubble rendered cleanly + stable. @@Alex picked options Y + 1 → `event-reply-arch-survey-1.md` landed with `answers: [{question_index:0, key:"Y"}, {question_index:1, key:"1"}]` → bubble filtered out via the `fullstack-a-5` post-reply filter. Also confirmed: option-key keystroke did NOT leak into the prompt buffer (the `fullstack-a-14` autoFocus gate is working)
  - **Pre-flight path**: bubble flickers, AND a reply DID land (`event-reply-arch-preflight-1.md` with `type: "survey-reply"`, `answers: []` — likely the auto-appended standing "C — Check my comments first" option from `normalizeStandingOptions`), BUT the source bubble did not dismiss. **Root cause confirmed**: `BubbleOverlay.visibleEvents`'s filter from `fullstack-a-5` only matches `type === "survey"` source events with sibling `survey-reply` files. Pre-flight (and poke, when they have reply UI surfaced via standing options) doesn't get the same dismissal even after reply
  - **Poke path**: bubble flickers, "cannot dismiss." Either no reply UI was surfaced (no standing options reached the user), or @@Alex didn't try. Either way: no dismiss affordance reached the user. (Note: in screenshot @@Alex shared, the bubble has a small refresh-style icon top-right but no visible close / dismiss control)
  - **Two distinct fixes needed**:
    1. **Filter generalization** (small): `BubbleOverlay.visibleEvents` should filter any source event whose id has a sibling `*-reply-<id>.md`, not just `type === "survey"`. One-line change to the predicate. Fixes pre-flight + any poke whose user does reply via standing options
    2. **Explicit dismiss affordance for all bubbles** (medium): even bubbles with no reply path (a poke whose standing_options the user ignores; future notification types) need a way to be dismissed. Add a close button to every bubble; clicking it persists the dismissed id (session storage / SerTab dismissed-ids set) so the bubble stays gone across watcher polls. Survey reply path remains the preferred dismissal but explicit close becomes the universal escape hatch
  - **Refresh hygiene** (companion finding worth investigating): the flicker itself may be a render-churn issue independent of dismissal — the bubble overlay's refresh path may be replacing the visible-events array atomically (clear → re-populate) on each watcher poll, producing a brief "empty" frame. Switching to diff-merge (only add new ids, only remove ids whose source files disappeared from the listing) would eliminate the flicker for ALL types regardless of dismissal contract. Worth profiling at task-cut to confirm whether (1) + (2) alone resolve the flicker or whether a third change is needed
  - dispatched for Round-2 wave-1 against @@FullStackA (rich-prompt + BubbleOverlay lane). Hard gate ahead of the rich-prompt session-evolution work (history backlog + cwd preflight + team conductor), which all build on top of the bubble overlay layer
- Watcher fsnotify path parses every non-hidden file; convention is not enforced or documented
  - flagged 2026-05-20 by @@Alex during a broadcast smoke test: server-side `event_watcher::ingest_once` (lines 121-183) reads + JSON-parses every non-hidden, non-directory file in the watched dir. Parse failures bump `dropped_events` + emit `tracing::warn!` + surface as red toasts. Any non-event file in the watcher dir produces noise
  - asymmetry: the SPA `watcherEvents.ts` + the systacean-9 read endpoint both filter by the regex `^(event|pre-flight)-.+\.(md|json)$`. The fsnotify watcher path in chan-server does NOT apply this filter
  - convention check: existing event files all use `.md` extension despite content being JSON (`event-survey-bug20-v2.md`, `event-reply-<id>.md`). The convention IS `.md`-only, but neither `event_watcher.rs`'s module doc nor `phase-N/process.md` documents this explicitly. The regex filter on the SPA + read-API path tolerates both `.md` and `.json`, which understates the convention
  - want, two parts:
    1. **Server tightening**: mirror the SPA / read-endpoint regex in `event_watcher::ingest_once` — skip non-matching filenames silently (no read, no parse, no warn, no `dropped_events` bump). Symmetric with the directory-skip + hidden-skip guards already there. Tests against the systacean-5 / systacean-9 pattern
    2. **Doc tightening**: add a "Watcher event-file naming convention" section to `event_watcher.rs`'s module doc + a corresponding note in process.md / chan-drive design.md stating: filename must match `(event|pre-flight)-<id>.md`, content is JSON; anything else in the watcher dir is silently ignored
  - dispatched for Round-2 wave-1 against @@Systacean (event_watcher.rs lives in chan-server; convention + tests are theirs). Doc edits coordinate with @@Architect for process.md
- Rich prompt collapse / expand chevrons leave dead vertical space under the terminal
  - flagged 2026-05-20 by @@Alex dogfooding with screenshot: collapsing the rich prompt (`v` chevron from `fullstack-a-24`) drops the prompt to its header-only pill, but the terminal-host above it stays at its expanded-state height — leaving a tall empty band between the bottom of the terminal output and the top of the collapsed prompt pill. Expanding back (`^` chevron) works correctly because `fullstack-a-4`'s `.terminal-host` margin-bottom recompute fires on open
  - root cause hypothesis: the `.terminal-host` dynamic `margin-bottom = heightPx + 12px` from `fullstack-a-4` was wired to the prompt's open/close + height-resize transitions, but the new collapsed state from `fullstack-a-24` produces a different effective height (collapsed pill is header-only) that the margin-recompute path doesn't observe. xterm.js's ResizeObserver isn't re-firing on the collapse transition, so the terminal stays at its previous fit
  - want: `v` (collapse) reduces the reserved space to `collapsed-pill-height + 12px` and xterm re-fits to grow downward, sitting just above the collapsed pill. `^` (expand) restores the expanded behaviour as today. Both transitions trigger the same recompute path
  - related: `fullstack-a-4` (open path), `fullstack-a-24` (collapse / expand chevron introduction)
  - dispatched for Round-2 wave-1 against @@FullStackA (rich-prompt lane); small task, single-file scope likely (`TerminalRichPrompt.svelte` + `TerminalTab.svelte` margin-recompute reactor). Cut at Round-2 fan-out
- Rich prompt page-width is shared/inherited from the editor; breaks under tiling
  - flagged 2026-05-20 by @@Alex dogfooding with screenshot: in a tiled layout (multiple panes splitting the viewport horizontally), shrinking the editor's document-page width causes the rich prompt's composer to render badly at narrow widths
  - root cause hypothesis: the rich prompt's composer (Wysiwyg / Source) inherits the same CodeMirror page-width / max-content-width constraint that the markdown editor uses, OR shares a global page-width setting. The constraint is global / cross-pane; in a tiled layout the rich prompt of pane A is being squeezed by the page-width set for the editor in pane B (or a global setting)
  - want, two parts:
    1. **Each terminal's rich prompt has its own page width**, independent from the editor's and independent from sibling tiles. Persists per-rich-prompt-session (SerTab field, similar shape to the new `rpc` / `rph`)
    2. **Page-width slider lives in the rich-prompt textbox right-click context menu** in addition to the editor's existing surface — symmetric affordance so the user reaches the slider from whichever surface they're in
  - assumption to verify at task-cut: the page-width slider already exists in the markdown editor's right-click menu (per @@Alex's framing "add the slider … as well"). If it doesn't, the editor-side wire-up is part of the task scope
  - dispatched for Round-2 wave-2 against @@FullStackA (rich-prompt + editor lane); task cut at Round-2 fan-out. Couples with the rich-prompt session evolution work in [`architect/rich-prompt-session-evolution.md`](architect/rich-prompt-session-evolution.md) since both extend the rich-prompt surface in the same band of work

## Round 2 — needs deeper change

- Large markdown files block the editor with a spinner while loading
  - want: lazy / chunked loading as the user scrolls, instead of one big up-front parse
  - architectural change (virtual scroll + chunked parser), not a quick fix → carries to Round 2

