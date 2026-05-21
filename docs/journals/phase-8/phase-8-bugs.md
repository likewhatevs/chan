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
- File browser misses external file-creation events; scope model needs to be drive-wide + per-expanded-leaf
  - flagged 2026-05-20 by @@Alex ("bug for later"): @@Alex had an assistant (terminal agent) create files inside the drive. The file browser did NOT pick the new files up; @@Alex had to right-click → reload to see them. **Critical context (2026-05-20 follow-up)**: @@Alex was working at the drive root when this repro'd. So even the simplest case (drive-root scope) misses external file creation
  - @@Alex's verbatim refinement: "we should observe changes drive-wide and also for each leaf that is expanded" + clarification: "drive-wide i mean the first depth of drive's dir" — NOT recursive across the whole drive
  - **scoping model shift** (NOT a one-line fix to `fullstack-b-6`):
    * Today's `fullstack-b-6` shape: each FB instance has a SINGLE scope (the selected dir / parent-of-file).
    * Wanted shape: each FB instance watches multiple shallow (non-recursive, first-depth-only) scopes simultaneously — the drive root's first depth (always) PLUS each expanded subtree's first depth (one shallow scope per expanded leaf node). Events landing in any of those shallow scopes refresh the affected level of the tree
    * Rationale: a user navigating a tree expects to see external file activity at the drive root (top-level files appearing) AND inside any subtree they've explicitly expanded (they're paying attention to it). Unexpanded subtrees stay quiet (deeper levels don't churn until expanded). Non-recursive shallow watches keep the cost bounded — N expanded leaves = N watched levels, not N exploded subtrees
  - hypothesis space for the underlying machinery (still open):
    a) The single-scope filter from `fullstack-b-6` is the SPA-side bottleneck: events DO arrive but only one scope is honoured. Refactor to a per-instance scope-set.
    b) The chan-drive filesystem-watcher path may not be firing for terminal-side writes (PTY-driven `echo > file` / `touch` etc. — writes happen via the shell, not through chan-drive's `Drive::write_*` API). If the watcher relies on chan-drive-mediated writes only, external writes are invisible to the SPA
    c) `self_writes.rs` suppression may be over-eager and dropping external writes that share a temp-path or atomic-rename pattern
    d) Path-derivation mismatch: the assistant created files at a slightly different absolute path than what the FB scope's `tree.entries` keys on (canonical `/private/tmp` vs `/tmp` alias surfaced in @@WebtestB's teardown finding; symlinks; case-sensitivity)
  - first investigation step: spin up a test drive, open the FB at drive root, externally `touch /path/to/drive-root/newfile.md` from a terminal, observe whether: (i) chan-drive watcher fires (server log), (ii) the SPA receives the event, (iii) the FB scope filter passes it. Each "no" narrows to the root cause. If (i)/(ii) both fire but (iii) drops it — hypothesis (a) confirmed; refactor scope filter
  - dispatch direction: @@FullStackA first (SPA-side FB watcher + refresh path is the most likely owner — `fullstack-b-6`'s scope filter); escalate to @@Systacean if root cause is in `chan-drive/src/watch` or `chan-server/src/self_writes.rs`
  - parked for Round-2 wave-2 (or post-patch follow-up); not blocking the patch-release tag
- File rename UX: parity with terminal rename, input box positioned above the page-width-constrained content
  - flagged 2026-05-20 by @@Alex (feature, "next build"): chan already supports inline rename on terminal tabs; want the same affordance for file tabs / file rows. Verbatim ask: "same way we can rename terminal, we should be able to rename files.. place the input box above the page width"
  - read of the ask:
    1. Mirror the terminal rename UX shape — same trigger (double-click on tab? right-click → rename?), same inline input box pattern, same commit-on-Enter / cancel-on-Esc semantics. Whatever the terminal rename does today, the file rename should match.
    2. Input box positioned ABOVE the page-width-constrained content column. The editor's content respects the `--chan-page-max-width` cap (per `fullstack-a-30`); the rename input lives in a header band above that column, not constrained by the cap.
  - backend dependency: needs a filesystem rename operation through `chan-drive` (atomic + path-sandbox-safe). Verify at task-start whether `Drive::rename` exists today; if not, a small chan-drive + chan-server route addition is in scope. The atomic-write contract guarantees rollback safety on partial failure.
  - dispatched as `fullstack-a-35` — rides the patch release
- Wysiwyg paste: pasted markdown gets its special characters escaped (`*` → `\*`, etc.)
  - flagged 2026-05-20 by @@Alex: "when i copy pure markdown from xcode and paste on notes, it shows correctly.. when i paste on chan, it escapes the bolds and so on.. * -> \*"
  - context: macOS Notes accepts the pasted markdown as-is and renders bold / italic / etc. correctly. Chan's Wysiwyg paste handler escapes the markdown special characters, turning `*bold*` into the literal string `\*bold\*` instead of rendering the bold
  - root cause hypothesis: the Wysiwyg paste handler treats pasted text as "user-typed plain text" and applies the same escape-special-chars rule that keystroke input uses (to prevent unintended formatting when the user types a literal `*`). The intent differs by source: keystroke = literal character; paste = probably markdown source
  - fix direction: detect markdown-shaped pasted content (presence of paired `*..*` / `**..**` / `_.._` / `#..` heading lines / `- ` list items / etc.) and skip the escape pass for those pastes. Plain text without markdown markers continues to escape as today
  - alternative shape (simpler): always paste-as-markdown (no escape on the paste path); the source-mode toggle from -a-26 already lets the user switch to source view if they want to see / edit the raw escaped form
  - implementer picks the smarter detection or the simpler always-paste-as-markdown rule. Audit Wysiwyg's existing paste extension first (likely in `web/src/editor/Wysiwyg.svelte` or a CodeMirror extension config) before designing
  - dispatched as `fullstack-a-34` — rides the patch release
- chan-desktop Tauri window title: shows "chan drive: <name>", should be "<path>" instead
  - flagged 2026-05-20 by @@Alex: "note for next build, the tauri title: 'chan drive: <name>' should be <path> instead". The current chan-desktop window title is formed as `chan drive: <drive-name>` (e.g. "chan drive: chan"); @@Alex wants the title to be the full drive path instead (e.g. `/Users/fiorix/dev/github.com/fiorix/chan`)
  - read of the ask: replace the entire title string with the path — no "chan drive:" prefix. If implementer disagrees on dropping the prefix (e.g. for menubar discoverability), surface a scope question; otherwise default to path-only
  - lives in chan-desktop / `desktop/src-tauri/` window-creation path; per-window state is keyed by `w=<window-label>` URL parameter per CLAUDE.md, so the title swap happens at window-build-time alongside the existing label-derivation logic
  - dispatched as `fullstack-b-14` — small change, rides the patch release
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

- Rich prompt submit-mode doesn't survive page reload (server-side state desync)
  - flagged 2026-05-20 by @@Alex dogfooding v0.11.1: toggle the rich-prompt toolbar to Agent mode, reload the page, the toolbar reads "Agent" (SPA-side `SerTab.rpsm` persisted correctly per `fullstack-b-13` SPA-side) but the survey-reply echo + Cmd+Enter dispatch revert to Shell-mode `poke\n` chord. Two-half state desync between SPA-restored UI + server-side dispatch
  - root cause hypothesis: `Session.agent_mode: AtomicBool` (server-side, in `crates/chan-server/src/terminal_sessions.rs`) is set by the `PUT /api/terminal/:session/submit-mode` endpoint when the user clicks the toolbar toggle. Reload path: the SPA reconstitutes `TerminalRichPromptState.submitMode` from SerTab on remount, but does NOT replay the PUT to re-sync the server-side `agent_mode` field. If the session id changes across the reload (or chan-server restarts), the new server-side `agent_mode` defaults to false (Shell). SPA UI says Agent; server emits Shell chord
  - want: on rich-prompt state restore from SerTab, if `submitMode === "agent"`, queue a follow-up `api.setTerminalSubmitMode(sessionId, "agent")` call after the session is reachable. Single-file SPA-side fix likely in `tabs.svelte.ts` reconstitution path or `TerminalRichPrompt.svelte`'s mount effect
  - alternative shape (deeper): persist `Session.agent_mode` server-side across server restarts (per-drive config, similar to watcher attach state). More work; probably not needed if the SPA-side re-sync covers the reload case
  - dispatched for Round-2 wave-2 (or later) against @@FullStackB (the lane that owns `-b-13`'s submit-mode wire); task cut at Round-2 fan-out
- chan-desktop missing browser-style zoom (Cmd + / - / 0)
  - flagged 2026-05-20 by @@Alex: the desktop-native shell (Chan.app via Tauri) doesn't respond to the standard browser zoom chords. Cmd++ (zoom in), Cmd+- (zoom out), Cmd+0 (reset to 100 %) all no-op in the chan-desktop webview; the same chords work in a regular Chrome/Safari tab against the chan SPA
  - root cause hypothesis: Tauri 2's webview doesn't bind the OS zoom chords by default. The `core:webview:allow-set-webview-zoom` capability is already granted (enabled during `fullstack-b-7` for the opener IPC plumbing) so the underlying API is reachable; just need explicit accelerator bindings in chan-desktop
  - want, three chords + persistence:
    1. **Cmd+=** (and Cmd++, since Shift+= produces +): zoom in by 10 % (or some sensible step; match Chrome's behavior — 10 % steps)
    2. **Cmd+-**: zoom out by 10 %, with a floor (e.g. 25 % or whatever Tauri's webview accepts)
    3. **Cmd+0**: reset to 100 %
    4. **Persistence**: last zoom level survives `Chan.app` relaunch, persisted per-window. Composes with the `fullstack-b-1` LRU window-config (add a `zoom_level: f64` field to `WindowConfig`)
  - implementation shape: bind the chords in chan-desktop's accelerator config (likely `desktop/src-tauri/src/main.rs` or wherever existing shortcut bindings live, e.g. the `KEY_BRIDGE_JS` adjacency). Each handler calls `webview.set_zoom(new_level)` and writes the new level into the live `WindowConfig` for the LRU pickup. Tauri 2 API surface: `tauri::WebviewWindow::set_zoom(scale_factor: f64)`
  - cross-platform: macOS chord is `Cmd`, Linux/Windows is `Ctrl`. Bind both via Tauri's `accelerator!` macro (or whichever pattern existing shortcuts use)
  - dispatched for Round-2 wave-2 (or later) against @@FullStackB (chan-desktop lane); task cut at Round-2 fan-out. Independent of Wave-1 north-star work

- Survey-reply echo does not honour the rich-prompt broadcast target set
  - flagged 2026-05-20 by @@Alex dogfooding v0.11.1: rich prompt had broadcast input ON with all six agent terminals selected as targets + submit-mode toggle on Agent; @@Alex clicked reply option "1" on a bubble via mouse → bubble dismissed cleanly (survey-reply file landed, filter removed the bubble per `-a-28`) but NO `poke<chord>` arrived in any of the six broadcast-target terminals. Expectation: the survey-reply echo fans to the same broadcast target set as the rich-prompt submit path
  - root cause: `dispatch_agent_event` in `crates/chan-server/src/terminal_sessions.rs:502` writes the `poke + chord` bytes to a SINGLE session via `send_input` — the session that originally owned the survey event. The "rich-prompt broadcast input" feature (`broadcastTerminalInput` from `-a-31`) lives entirely on the SPA side and only applies to the rich-prompt SUBMIT path (Cmd+Enter from the rich prompt fans the typed bytes to all selected target PTYs). The survey-reply echo path bypasses the SPA's broadcast layer entirely — server writes direct to PTY, SPA never sees the bytes, so the broadcast fan-out logic never runs
  - want: when the user replies to a bubble survey AND the originating rich prompt has broadcast input ON, the echo (`poke<chord>` or whatever shape the submit-mode toggle resolves to) lands in EVERY broadcast-target PTY simultaneously, same shape as rich-prompt Cmd+Enter broadcasting
  - three fix-direction options:
    1. **Server-side fan-out**: extend `Session` (or add a sibling Registry table) to track per-rich-prompt broadcast target lists. SPA pushes the target set to the server when broadcast input toggles ON/OFF + when the row checkboxes change. `dispatch_agent_event` reads the originating session's broadcast list + fans to each target's PTY. Pros: server-authoritative, works even if the SPA tab loses focus during the reply. Cons: introduces a new piece of server state that has to track SPA-side selection changes
    2. **SPA-side intercept + fan-out**: server's `dispatch_agent_event` stops writing the echo to the PTY directly. Instead emits a WS frame to the SPA describing the intended echo. SPA receives, decides whether to fan (if broadcast is ON) or emit to the single originating PTY (broadcast OFF). Pros: leverages the existing SPA broadcast layer; one source of truth for broadcast targeting. Cons: SPA needs to handle the case where the SPA-PTY connection drops between the reply landing + the WS echo frame
    3. **Suppress server echo for surveys + SPA fires its own**: server's `dispatch_agent_event` skips the echo entirely for `type === "survey"` (and possibly other reply-driven event types). SPA's bubble-reply handler is the SOLE emitter; it fires `poke<chord>` through `broadcastTerminalInput` directly using the standard rich-prompt-submit broadcast machinery. Pros: simplest delta; reuses existing SPA broadcast plumbing exactly. Cons: shell-mode users who liked the existing server-side echo lose it (mitigation: keep server-side echo for shell mode, only intercept for agent mode — but that's a state-coupling rule that needs careful test pinning)
  - recommendation: **option 2 or 3** — both move the broadcast decision to the SPA layer where the broadcast targets actually live. Option 3 is the smallest delta + cleanest mental model ("SPA owns the echo when broadcast is involved"); option 2 is more architecturally consistent but bigger
  - lane: @@FullStackB (the lane that owns `-b-13`'s submit-mode + survey-reply echo plumbing). Composes with the related submit-mode-persistence bug filed above (both touch the same wire); could land together as one round of submit-mode wire polish
  - dispatched for Round-2 wave-2 (or later) against @@FullStackB; task cut at Round-2 fan-out

- Pre-flight bubble spinner stuck at `0:00` (never ticks, no progress)
  - flagged 2026-05-20 by @@Alex during webtest walkthrough of v0.11.1 (third sighting): pre-flight bubble in the rich-prompt notifications shows a spinner glyph adjacent to the `0:00` label, but the label never increments and the spinner doesn't visually animate. Screenshot in the conversation shows `@@Architect / ↻ 0:00 / Reply dismisses pre-flight bubble (filter...` with the standing option `1 Open the terminal` below
  - root cause hypothesis: pre-flight events in chan's event schema carry an optional duration/eta concept (the spinner + 0:00 is rendering against SOME timing field — likely `started_at` or `eta`). The architect-fired pre-flight events I dropped during the v0.11.1 dogfood (`event-arch-preflight-2.md`, `event-arch-round2-kickoff-1.md` IF that one renders pre-flight-style) carry only `topic` + `note` + `from/to/id/type`; no timing data. The BubbleOverlay renders the spinner unconditionally for `type: "pre-flight"` and reads timing from `event.started_at` (or similar) — falls through to default `0` → display `0:00` → no tick because there's no positive duration to count
  - alternative root cause: timing field IS populated by the watcher (filesystem `ctime` of the event file fed into the SPA payload) but the SPA-side tick interval never fires (Svelte 5 `$effect` not subscribing correctly to a per-second ticker). Walkthrough check at task-cut: drop a pre-flight event, screenshot at t=0s, screenshot at t=30s, observe whether the label is 0:00 in both
  - want, two acceptable shapes:
    1. **Suppress the spinner when no timing data is present**: if the pre-flight event JSON has no `eta` / `started_at` / equivalent, hide the spinner + label entirely. The bubble still renders the topic + note + standing options; just no timer chrome. Smallest delta; matches the user's mental model ("the architect dropped a notification, not an operation")
    2. **Show elapsed time since event-emit**: derive the start time from the watcher's read of the event file's mtime / ctime. Tick every second. Useful if the user wants to know how long a bubble has been sitting unanswered. Slightly more code; pairs naturally with the explicit-dismiss affordance from `-a-28` (long-sitting bubbles get a visual "stale" cue from the elapsed-time number alone)
  - recommendation: **option 1** for the v0.11.2 / Round-2 patch — smallest fix, kills the visual bug without committing to an elapsed-time UX. Option 2 can land later as a polish pass if it earns its keep
  - lane: @@FullStackA (BubbleOverlay rendering owner; same lane as `-a-28` which last touched this surface). Single-file fix likely in `web/src/components/BubbleOverlay.svelte` — find the spinner / `0:00` markup, gate on a timing-data-present check
  - dispatched for v0.11.2 mini-wave (if cut) OR Round-2 wave-2 against @@FullStackA; task cut when v0.11.2 scoping firms up

- File browser tab loses expand/collapse state across tab switches
  - flagged 2026-05-20 by @@Alex dogfooding v0.11.1: open an FB tab, expand a few directories down a few levels, switch to another tab (terminal / editor / graph / search), switch back to the FB → the tree resets to the default expansion (drive root only or whatever the default initial-expand state is). Every previously-expanded dir is collapsed again
  - root cause hypothesis: the FB tab's expanded-dirs set lives in component-local state (`Set<string>` keyed by directory path, scoped to the `FileBrowserSurface.svelte` / `FileTree.svelte` component instance). Svelte tab-switch pattern unmounts the inactive tab's component subtree; on re-activation the component remounts with a fresh state object. Persistence to SerTab was never wired
  - want: persist the per-FB-tab expanded-dirs set across tab activation. Same SerTab shape as the other state-restoration work this phase (`rpsm` / `rpc` / `dbi` / `rppw` etc. — short-form conditional-spread field, restore on deserialize, persist on serialize). Multiple FB tabs each keep their own expansion state (keyed at SerTab level since each tab has its own SerTab payload)
  - fix shape: new `SerTab` field — suggest `fbe?: string[]` (FB Expanded; array of absolute / drive-relative paths). Conditional spread on serialize (`fbe.length > 0`); range-guarded on deserialize; `FileBrowserSurface.svelte` (or wherever the `expandedDirs` Set lives) reads from + writes to `tab.<expandedDirsField>` instead of component-local state
  - related work that informs the shape:
    * `-a-28`'s `dbi?: string[]` (BubbleOverlay dismissed-ids) is the closest precedent — same shape (array of string ids in SerTab), same conditional-spread persistence pattern. Use that file as the reference template
    * `-b-6` (FB watcher scope) is per-tab too; the scope already persists per-tab so the SerTab plumbing for FB tabs exists. The new field rides alongside the scope field
  - acceptance: open FB at drive root, expand 3 directories, switch to a terminal tab, switch back — all three remain expanded. Add a vitest pin in `tabs.test.ts` for SerTab round-trip of the new field
  - lane: @@FullStackA (SPA + FileBrowserSurface + tabs.svelte.ts). Small task, single-PR shape; rides the same band of "remember-state-across-tab-switch" work that already covers rich-prompt height / collapse / page-width
  - dispatched for v0.11.2 patch (if cut) OR Round-2 wave-2 against @@FullStackA; task cut when scoping firms up

- Cmd+O semantics: rebind FB to Cmd+Shift+E, make Cmd+O a context-aware "Open file" dialog
  - flagged 2026-05-20 by @@Alex dogfooding v0.11.1 + the `fullstack-a-32` chord migration that just landed. Three coupled pieces:
    1. **Chord rebind**: today Cmd+O opens the File Browser (per `-a-32`'s new chord set). Move FB to `Cmd+Shift+E` (matches VSCode's "Focus on Files Explorer" mental model). Free Cmd+O for the new behaviour
    2. **New "Open file" dialog**: Cmd+O surfaces a modal/dialog similar in shape to the existing "New file" dialog (built on `PathPromptModal.svelte`). The dialog accepts a path, validates it against chan-drive's editable-text gate + sandbox boundary, opens the file in the editor on confirm
    3. **Context-aware pre-population**: the dialog opens with the path field pre-filled based on the focused surface:
       * **Focused FB**: pre-fill with the path of the currently-selected node IF it's a regular file we can open in the editor (drop directory selections, drop binary / non-editable selections — chan-drive's editable-text gate is the rule)
       * **Focused terminal**: read the xterm.js selection. If the selected text parses as a path (relative or absolute) that resolves to an editable regular file inside the drive sandbox, pre-fill that. Example user flow: `echo ./docs/something-that-exists.md` in the terminal → user mouse-selects the path text in the terminal output → `Cmd+O` → dialog opens with `docs/something-that-exists.md` pre-filled → Enter opens the file
       * **No actionable focused surface**: dialog opens with the field empty (drive-root-relative); user types the path manually
  - composes with `-a-32`'s `resolveSpawnContext()` helper — the context-resolution logic for "what's the focused surface and its context?" is exactly what this enhancement extends. New helper `resolveOpenFileCandidate()` (or similar) that returns `{ path: string, source: "fb" | "terminal" | "none" }` based on the focused surface + the editable-text gate
  - Hybrid NAV mnemonic stability: `Mod+. o` stays as FB spawn (the universal in-mode mnemonic from `-a-32` lives alongside the new top-level chord). Mnemonic reads as "o for **o**pen browser" inside NAV mode; top-level `Cmd+O` reads as "Open file" outside NAV mode. The mnemonic divergence is acceptable since NAV mode is opt-in
  - cheatsheet + chan-desktop accelerator updates (PaneModeHelp + SERVE_LONG_ABOUT + Tauri `KEY_BRIDGE_JS`): same shape as the `-a-32` resync. Single commit covers all surfaces
  - cross-impact: this is a partial revert / refinement of `-a-32`'s chord set. Audit-trail readable as "the chord migration landed first; Cmd+O for FB was the right v1 shape; @@Alex dogfooded + flagged that Cmd+O wants the open-file semantic the rest of the editor world uses". Not a regression
  - terminal-selection path-parsing edge cases (worth investigating at task-cut):
    * Quoted paths (`"foo bar.md"` with spaces; backtick-wrapped paths)
    * Trailing whitespace / line-noise (selection grabbed `./foo.md\n  $`)
    * Relative-to-cwd-of-terminal vs relative-to-drive-root (if the terminal's cwd differs from the drive root, resolve via the terminal's last-known cwd from `-a-32`'s context resolution)
    * URLs that look path-shaped (`https://example.com/foo.md` — should NOT match; gate on file-existence check)
    * Symlinks pointing outside the drive sandbox (chan-drive's existing path-sandbox refusal handles cleanly)
  - lane: @@FullStackA (chord migration + dialog component + context resolution all SPA-side). Coordinates with the `-a-32` `resolveSpawnContext()` shape so both helpers compose cleanly. No chan-server / chan-drive work expected (existing `/api/files/{*path}` GET handles the actual file open via the editor's existing load path)
  - sequencing thoughts: small-to-medium task, mostly SPA. Could land in v0.11.2 patch (if cut) but the terminal-selection path-parser is the novel piece + worth a careful walkthrough. Recommending Round-2 wave-2 against @@FullStackA with the rich-prompt session-evolution stack (similar lane, similar surface)
  - dispatched for Round-2 wave-2 against @@FullStackA; task cut at Round-2 wave-2 fan-out

- **CRITICAL UX**: Editor falsely flips to "File moved or deleted" while file is still on disk (repeated; interrupts writing)
  - flagged 2026-05-20 by @@Alex with screenshot, third+ occurrence: editing `docs/journals/phase-8/alex/hybrid-revisited.md`, editor surface flips to a centered "File moved or deleted" panel with Re-open / Find / Close buttons. File is NOT actually moved or deleted — the docked FB on the left still shows it, double-clicking the FB entry reopens normally. Re-open button on the panel routes to the FB with nothing selected (broken path; should restore the same file in place since it IS still at the recorded path)
  - **impact**: breaks concentration during active writing. @@Alex's framing: "we don't want users to have this kind of experience". Hard UX regression on a daily-driver flow
  - root cause hypothesis space (implementer narrows during repro):
    a) **chan-drive atomic-write race**: chan-drive's `write_text` uses temp + rename. If the editor's path-existence watcher catches the brief unlink window during the rename, it fires "file moved" falsely. Should be microseconds but inotify / FSEvents can fire spuriously
    b) **`self_writes.rs` suppression miss**: chan-server's self-writes suppression de-noises the watcher for chan's own writes. If the suppression key doesn't match (e.g., path canonicalisation `/tmp` ↔ `/private/tmp` on macOS, or pathbuf vs string mismatch, or normalisation around `.` segments), the editor sees a phantom delete for chan's own save
    c) **Concurrent-write events from sibling files in the same directory**: this session's @@Architect terminal does lots of `Edit`-tool writes to siblings (`event-architect-*.md` in the same `phase-8/alex/` dir). If the editor's watcher is directory-scoped and not file-scoped, sibling-file write events could trigger a spurious "did my file change?" check that returns the wrong answer briefly
    d) **`fullstack-b-6` FB watcher scope leak into the editor's path check**: -b-6 scoped FB watcher to selection, but the editor's "is my file still there?" check may share infrastructure with the FB watcher + inherit the scope filtering wrongly
    e) **Editor's mtime / stat cache going stale**: editor reads file mtime + size at open; on watcher event, compares to cached. If chan-drive's rewrite produces a smaller file or same mtime (clock skew), the check may interpret "file changed" as "file gone"
  - want, three pieces:
    1. **Stop the false detection** — root-cause the spurious "moved or deleted" trigger and fix it. Should be impossible to surface this panel while the file is on disk at the recorded path. Add a recovery check: when the panel is about to fire, run `stat` on the recorded path with a 100-200ms debounce; if the file is back, dismiss the panel without UI flash
    2. **Fix the Re-open button** — currently routes to FB with nothing selected. Should restore the same file in place (re-read content + reset cursor / scroll state). This is broken even when the panel IS legitimately surfacing for a real moved-or-deleted file
    3. **Improve the "file moved" UX (the Find suggestion @@Alex proposed)** — when the panel surfaces, run a backend search by basename (and optionally content-fingerprint of the cached file contents) across the drive. If a unique match is found at a different path, present inline: "File seems to have moved to `docs/elsewhere/hybrid-revisited.md` — Reopen there?" with a one-click reopen. Currently the Find button takes the user out of context entirely; the inline suggestion keeps the writing flow intact
  - lane: primary @@FullStackA (editor + file-tab + "moved or deleted" panel UI). Secondary investigation by @@FullStackB / @@Systacean if root cause is in chan-server `self_writes.rs` or chan-drive's `write_text` atomic boundary (need to coordinate at task-cut once the root cause narrows)
  - escalation: **v0.11.2 patch candidate** — recommend cutting a small patch wave if root cause is contained + fix is low-risk. The interruption-during-writing impact is severe; this is the kind of bug that erodes user trust quickly
  - dispatched for v0.11.2 patch (if cut) OR Round-2 wave-2 hard-front against @@FullStackA; task cut once @@Architect confirms with @@Alex whether v0.11.2 patching is in scope

- **Enhancement (companion to the critical bug above)**: usability-test coverage for daily-driver writing workflows
  - flagged 2026-05-20 by @@Alex on the same turn as the critical bug above: "I want to include some usability tests that try out workflows like this"
  - target workflows to cover end-to-end (each test exercises the full editor + FB + watcher + indexer + Find chain):
    1. **Edit + image + table workflow**: open a file → edit a paragraph → add an image (paste or drag) → copy-paste a markdown table from another source → save. Assertion: file content persists correctly, no spurious "moved or deleted" panel, image atom inserts cleanly, table renders without escaping (`-a-34` paste-unescape regression check)
    2. **FB-driven rename workflow**: open a file → edit a bit → rename via FB context menu → continue editing the renamed file in the same tab. Assertion: tab title updates, file content persists, no panel surfaces, FB tree reflects the new name, watcher state migrates cleanly
    3. **Shell-`mv` workflow**: open a file → edit a bit → open a terminal → `mv` the file to a new path → return to the editor. Assertion: editor detects the move (legitimate this time), surfaces the panel with the "file seems to have moved to {newpath} — reopen?" suggestion (the new UX from the bug above), one-click reopen restores the editor at the new path
    4. **Concurrent-write workflow**: open file A in pane 1 → open file B (sibling of A in the same dir) in pane 2 → edit both → save both. Assertion: neither editor flips to "moved or deleted" from the other's save events; both saves complete; both files contain the typed content
    5. **Find-after-rename workflow**: rename a file via FB or shell → open the search overlay → type the original basename. Assertion: Find returns the renamed file (chan's indexer picks up the rename + re-indexes cleanly under the new path)
  - implementation shape options:
    * Vitest end-to-end harness against a synthetic drive (small fixture set; runs in CI as part of the existing `web/ vitest run` gate). Pros: fast, reproducible. Cons: mocks the FS layer, may miss real-watcher bugs
    * Playwright or similar against a real chan server backed by a real drive directory. Pros: catches actual watcher / indexer races. Cons: slower, flakier in CI, needs orchestration
  - recommendation: hybrid — Vitest for the deterministic shape (DOM + state assertions); a separate Playwright suite that runs less often (release-gate only?) for the watcher / race-sensitive paths. The webtest lanes already do this manually; codifying it catches regressions before they reach @@Alex
  - lane: @@FullStackA + @@WebtestA / @@WebtestB joint task. @@WebtestA/B own the manual walkthrough patterns + could help shape the test fixtures; @@FullStackA implements the harness. New `web/tests-e2e/` (or similar) directory shape proposed; investigate at task-cut
  - dispatched for Round-3 Track 3 (cleanup + hardening + release readiness) against @@FullStackA + @@WebtestA/B; the critical bug fix above lands first via -a-N (v0.11.2 / Round-2 wave-2) and is one of the regression scenarios this test suite codifies

- **Feature**: markmap support in the editor (https://github.com/markmap/markmap)
  - flagged 2026-05-20 by @@Alex: "add support for markmap in our editor". markmap renders a markdown document's heading + list hierarchy as an interactive SVG mindmap. Existing implementations in Obsidian / VSCode / JetBrains plugins all share the same UX pattern: take the current doc, parse heading levels + list nesting, render as a radial tree, let the user pan / zoom / collapse branches
  - npm packages (canonical):
    * `markmap-lib` — markdown → JSON tree parser. Small (~30 KB minified).
    * `markmap-view` — SVG renderer. Pulls D3 (~70 KB). Bundle-size impact to measure at task-cut
    * `markmap-toolbar` (optional) — built-in controls (fit / save / reset). May be redundant if chan provides its own toolbar surface
  - **surface options** (implementer picks at task-cut; recommendation in 1):
    1. **Third mode in the existing StyleToolbar** (RECOMMENDED) — alongside the wysiwyg / source toggle landed in `-a-26`. Toggle reads as "wysiwyg ↔ source ↔ markmap". Markmap is read-only; editing happens in wysiwyg or source modes. Pane-pair pattern: open the same file in two Hybrid panes, one in wysiwyg / source, the other in markmap, with `fullstack-b-5` per-Hybrid theme override applying to both — gives live-preview reading without bidirectional-edit complexity
    2. **New tab type** alongside file / terminal / FB / graph / search. Tab spawned via a new chord (`Cmd+Shift+K` for "kmap"? — needs chord-namespace check against `-a-32`'s migration). Opens a doc as a markmap view in its own pane. Heavier UX surface, more flexible composability but more code
    3. **Pane-mode action** (Hybrid NAV `m` mnemonic) that spawns a markmap view from the current doc into the back side of a Hybrid. Composes with `-a-22`'s flip animation — flip front/back to swap source ↔ markmap. Cute, but ad-hoc compared to a first-class toolbar toggle
  - **read-only vs editable**: v1 read-only. Most markmap implementations don't attempt to write back to the source doc on user interaction. Editing happens in wysiwyg / source; markmap view re-renders on doc change with a debounce (typical: 250-500 ms after last keystroke). Bidirectional editing is a research project — out of scope for v1
  - **live-update vs static**: live-update with debounce. Background re-parse on doc change; SVG re-render on parse complete. Should compose cleanly with the existing CM6 + Wysiwyg edit pipeline (subscribe to document-change events)
  - **theming**: markmap-view exposes a CSS variable surface for colours / fonts. Wire chan's theme tokens (`--text-fg`, `--accent`, font-family) into the renderer config. Composes with `-b-5`'s per-Hybrid theme override (light / dark per pane). Box-drawing font fallback story from the now-deferred bundled-font work doesn't apply here (SVG text rendering uses the page's font stack directly)
  - **toolbar actions** (in the markmap view's chrome):
    * Fit-to-pane (auto-zoom + recenter)
    * Expand / collapse all
    * Save as SVG (uses markmap-view's built-in serializer)
    * Optional: save as PNG (needs canvas conversion; defer if scope-creeps)
    * Optional: save as standalone HTML (markmap's "export to file" mode; useful for sharing a doc's mindmap independent of chan)
  - **bundle-size budget**: measure markmap-lib + markmap-view + D3 transitive deps with `web/ npm run build` before committing. If it pushes the chan-server embedded bundle materially (say > 100 KB compressed delta), consider:
    * Lazy-loading: `markmap` deps live in a separate bundle chunk loaded only when the user first toggles to markmap mode
    * The same lazy-load pattern semantic search uses for the BGE model (per `systacean-6` / `systacean-7`)
  - **license check**: markmap is MIT (per the upstream repo). Compatible with chan's Apache 2.0. Include a row in the SettingsPanel About section attributions (mirroring the Source Code Pro OFL.txt row from `fullstack-b-12`) for proper third-party-dep credit
  - **composes with**:
    * `-a-26` StyleToolbar mode toggle (the third mode lands here)
    * `-b-5` per-Hybrid theme propagation (markmap picks up theme tokens correctly)
    * Future Infographics tab (round-2-plan item 4) — markmap could be one of the Infographics tab's content types if it ever wants per-drive "show me the structure of this doc" surfaces beyond the per-file view
  - lane: @@FullStackA (editor + StyleToolbar + new view component, all SPA-side). No chan-server / chan-drive work expected (parsing happens client-side on the loaded doc content)
  - sizing: medium task — new dep, new component, debounce wiring, toolbar actions, theme integration, bundle-size measurement. Could be split into a strict-v1 (read-only viewer + fit-to-pane only) + follow-up polish (export actions, theme refinement). Implementer picks the carve at task-cut
  - dispatched for Round-2 wave-2 against @@FullStackA. Pairs naturally with the rich-prompt session-evolution stack (both extend the SPA's content-surface vocabulary); fan out together when wave-2 dispatches

- FB spawn chord focuses existing FB instead of spawning a new tab
  - flagged 2026-05-20 by @@Alex: "im currently not capable of opening more than 1 file browser with cmd+o (which is moving to cmd+shift+e); when i hit this chord the focus goes to the 1 existing FB instead of creating a new FB tab"
  - current behaviour (post `-a-32` chord migration): pressing `Cmd+O` when an FB tab already exists anywhere in the layout shifts focus to that existing FB instead of spawning a fresh FB tab. User cannot open multiple FBs via the chord
  - root cause hypothesis: the FB-spawn helper in `store.svelte.ts` (or wherever `-a-32`'s context-aware spawn machinery lives) likely has a "find-existing-FB-tab → focus" fall-through that's the wrong shape for FB. The pattern probably matches Cmd+P's intentional toggle behavior ("if on terminal, toggle; if not on terminal, open one") but FB doesn't have that toggle semantic — FB should spawn new every time, matching Cmd+T's "new terminal every time" convention
  - chord-convention table (current vs intended):
    | Chord            | Current behaviour                                | Should be                            |
    |------------------|--------------------------------------------------|--------------------------------------|
    | `Cmd+T` (new term) | Spawns a new terminal every time               | Same (correct)                       |
    | `Cmd+O` (FB)     | Focuses existing FB if present, else spawns new | **Always spawn new** — fix this      |
    | `Cmd+P` (rich p) | Toggles if on terminal, opens if not             | Same (intentional toggle)            |
    | `Cmd+Shift+M` (graph) | Spawns new every time                       | Same (correct)                       |
  - want: FB-spawn always creates a new FB tab. Each FB tab has its own selection / expansion state per the per-tab pattern `-b-6` (watcher scope) + the FB-expansion-state bug filed earlier today (`fbe?` SerTab field). Multiple FBs in the same pane / across panes is the natural affordance for tree-comparison workflows
  - coupling with the Cmd+O rebind enhancement filed earlier today: the rebind moves FB to `Cmd+Shift+E` + makes `Cmd+O` an Open-file dialog. THIS bug applies to whichever chord ends up bound to FB-spawn — fix the helper, the chord move just changes which key triggers it. The two changes could land in the same -a-N task (one commit covering both) since they touch the same code path
  - acceptance: press the FB-spawn chord 3 times → 3 FB tabs in the layout, each with independent selection / expansion / scope. Each FB tab's title differentiable (likely numbered: `Files`, `Files 2`, `Files 3` — match the terminal-tab numbering pattern from `-b-2`)
  - small task; same lane as the Cmd+O rebind work (@@FullStackA, chord-handler + spawn helper). Both rides Round-2 wave-2 against @@FullStackA; cut as a single task at fan-out OR fold the bug-fix into the rebind task's scope (the simpler commit shape)
  - dispatched for Round-2 wave-2 against @@FullStackA, paired with the Cmd+O rebind enhancement

- ~~Cmd+F find-in-page UX is subpar~~ — **WITHDRAWN 2026-05-21 by @@Alex**: find-in-page is actually working. The earlier symptoms (subtle highlight + scroll-to-match desync) traced to a stuck chan-desktop UI state; closing + reopening the desktop-native cleared the staleness. Not a real bug. Original entry struck through to preserve the audit trail of "we looked at this + ruled out a Cmd+F-specific issue"
  - separately worth tracking as a stretch observation: chan-desktop UI state stuck-until-relaunch may be a real but distinct bug (a category of UX glitches that need a refresh-the-webview cure). NOT cutting a separate entry; if it surfaces again under any other symptom, add a new entry then

- Tab right-click "Reload" + "Open Inspector" entries no-op on chan-desktop (macOS)
  - flagged 2026-05-21 by @@Alex: right-clicking a tab in chan-desktop / Tauri webview on macOS surfaces a context menu with "Reload" + "Open Inspector" entries; clicking either does nothing. Both entries work (or have an analogue) in the web build via the browser's own Reload / Inspect Element behaviour
  - **dev-workflow severity**: Open Inspector is the gating affordance for debugging EVERY chan-desktop-specific bug. Without it, @@Alex can't DevTools-inspect the Cmd+F highlight CSS (just-filed bug), can't see why "File moved or deleted" surfaces (just-filed critical bug), can't profile the spinner-stuck-at-0:00 bubble, etc. This bug is a meta-blocker for the rest of the desktop-native UX bugs
  - root cause hypothesis: the SPA's tab context menu defines "Reload" + "Open Inspector" entries unconditionally (designed against the web build's browser-default surface), but on chan-desktop the entries don't have a Tauri IPC equivalent wired through. Two paths likely missing:
    1. **Reload**: should call Tauri 2's `WebviewWindow::reload()` (or `eval("location.reload()")` as a fallback). The SPA-side click handler probably calls `window.location.reload()` directly, which MIGHT work in the Tauri webview but is being silently dropped or — more likely — the menu entry's click handler is no-op'd for chan-desktop because the menu was built for web. Verify which by inspecting the entry's `on:click` in the SPA source
    2. **Open Inspector**: needs Tauri 2's `WebviewWindow::open_devtools()`. This requires the `devtools` feature in `tauri.conf.json` (or per-crate Cargo feature). chan-desktop may not have that feature enabled — `tauri.conf.json` `app.devTools` (or similar) must be `true`. Or the dev build has it, the release build doesn't. Confirm at task-cut whether chan-desktop's release config exposes devtools at all
  - want: both entries work on chan-desktop with the same UX as the web build:
    * **Reload**: re-fetches the current tab's content. For a file tab, re-reads the file from chan-drive (composes with the "File moved or deleted" detection fix). For a terminal tab, conceptually means "clear + restart" — but that may be a separate, surfacing question; v1 of this fix can just no-op the Reload entry for non-file tabs OR scope the menu to file tabs only
    * **Open Inspector**: opens Tauri's DevTools for the chan-desktop window. Standard webview inspector — element tree, console, network, etc.
  - first investigation steps:
    1. Audit `tauri.conf.json` for `app.devTools` (or whichever Tauri 2 key gates devtools). If false in release config, flip to true (gated on a build profile if shipping-with-devtools is undesirable for end users)
    2. Grep the SPA source for the tab context menu definition (likely in `Pane.svelte` or `TabStrip.svelte` adjacent). Find "Reload" + "Open Inspector" entries; check their click handlers
    3. For chan-desktop, add Tauri IPC commands in `desktop/src-tauri/src/main.rs` (or wherever IPC handlers live): `reload_window`, `open_devtools`. The SPA detects chan-desktop via the existing runtime check + invokes IPC; web build keeps using `window.location.reload()` + a no-op-or-instructional message for inspector
  - chord-binding consideration: Chrome uses `Cmd+R` for reload + `Cmd+Opt+I` for inspector. chan-desktop's `KEY_BRIDGE_JS` (from `-a-32`) should bind these accelerators to the same IPC commands so keyboard users get reload/inspector without the right-click. Check whether either chord works today as a separate axis
  - lane: @@FullStackB primary (Tauri config + IPC commands + KEY_BRIDGE_JS bindings — chan-desktop side is the load-bearing piece). @@FullStackA secondary (SPA tab context menu + runtime-aware dispatch). Coordinate at task-cut on which side cuts first; the SPA-side click handler should compose with the IPC commands @@FullStackB exposes
  - escalation: **v0.11.2 patch candidate** — even though this isn't user-facing UX itself, it's the **debugging affordance** that lets @@Alex (and webtest lanes with chan-desktop runtime permission) investigate the OTHER user-facing chan-desktop bugs. Bumps v0.11.2 patch scope from 4 → 5 items. Strong case for inclusion: meta-blocker for everything else
  - dispatched for v0.11.2 patch (if cut) OR Round-2 wave-2 hard-front against @@FullStackB primary + @@FullStackA secondary

- Source-code editor mode auto-intervenes with list typing (it shouldn't)
  - flagged 2026-05-21 by @@Alex: typing in source-mode (the raw CodeMirror view, not the wysiwyg renderer) still triggers list-continuation behaviour — e.g., typing `1.` + space + Enter auto-inserts `2.` on the next line; same for `-` / `*` bullets. Source mode should be 100% raw; no auto-continuation, no auto-renumber, no bullet smarts. The wysiwyg mode is where rendering intelligence lives; source mode is where the user reads / edits the raw markdown
  - root cause hypothesis: the editor's CM6 extension stack for source mode likely includes the same markdown-language extension that wysiwyg uses, OR shares a list-continuation keymap that fires regardless of mode. The mode toggle from `-a-26` swaps the RENDERER but the input keymap may not get stripped down to source-mode-appropriate behaviour
  - want: source mode is editor-plain (no list-handling, no auto-anything; just raw text with standard editor affordances — undo, multi-cursor, find-in-file). The list-aware behaviour stays in wysiwyg mode where it belongs
  - fix shape: at source-mode mount, load a stripped extension set — no `markdownLanguage` extension's list extensions, no chan-specific list keymaps. Or gate every list keymap on a "is-wysiwyg" flag at the keymap-handler level. Implementer picks
  - composes with the next bug (markdown wysiwyg sub-list numbering) — both touch the list-extension wiring; could land together as one editor-list-handling refactor task. Each is also independently shippable
  - lane: @@FullStackA (editor extensions + mode-toggle from `-a-26`)
  - escalation: paper-cut severity but daily for source-mode users. v0.11.2 candidate IF the patch wave gets cut (bumps scope to 6); otherwise Round-2 wave-2
  - dispatched for v0.11.2 (if cut) OR Round-2 wave-2 against @@FullStackA

- Markdown wysiwyg enumerated-list nested numbering: want outline-style dotted notation
  - flagged 2026-05-21 by @@Alex: nested numbered lists in wysiwyg currently use independent counters per depth (standard markdown spec):
    ```
    1. item
        1. sub-item   ← independent counter at depth 1
    2. another item
    ```
    @@Alex wants outline-style dotted numbering — depth carries forward as `1.N.`, `2.N.`, etc.:
    ```
    1. item
       1.1. sub-item
       1.2. another sub
    2. another item
       2.1. sub
    ```
    Multi-level nesting follows the same pattern: depth-3 would be `1.1.1.`, `1.1.2.`, etc.
  - context: outline-style dotted numbering is a real convention (used in technical specs, legal docs, RFC sections). It's NOT standard markdown — most renderers / GitHub / Obsidian use independent per-depth counters. @@Alex's preference is a custom rendering choice
  - two implementation shapes to pick between (flag for implementer + @@Alex confirmation):
    1. **Pure visual (CSS counters)**: underlying markdown source stays standard (`1. text\n   1. sub`). Render layer (wysiwyg) applies CSS `counter-reset` + `counter-increment` + `::marker content: counters(...)` to produce the dotted display. Pros: source stays portable across markdown tools (GitHub still renders cleanly with its own per-depth counters); chan's distinctive display lives only in chan's renderer. Cons: when user toggles to source mode, they see standard `1. / 1.` not dotted — could be confusing if they expect WYSIWYG-source parity
    2. **Source change**: when user types nested numbered list in wysiwyg, the editor literally inserts `1.1.` / `1.2.` / etc. as text content. Source view shows the dotted form too. Pros: WYSIWYG-source parity. Cons: breaks markdown standard — other tools (GitHub, Obsidian) won't render this correctly; treats `1.1.` as a literal heading rather than a list item marker
  - **architect recommendation**: option 1 (pure visual / CSS counters). Source portability is more valuable than WYSIWYG-source-view exact parity; @@Alex's source-view nuance can be a documentation point ("source view shows standard markdown; the dotted display is a chan render-time convention")
  - confirm @@Alex's preference before task-cut — the choice meaningfully affects how chan-authored docs render in other tools (notably: docs in `docs/journals/` are read by agents via filesystem, so the literal characters in source matter)
  - composes with the previous bug (source-mode list intervention) — both touch the list-extension wiring
  - lane: @@FullStackA (markdown renderer + wysiwyg extensions + CSS)
  - escalation: not patch-worthy (cosmetic / preference, not broken UX); Round-2 wave-2 against @@FullStackA. Cut as a single task with the source-mode list-intervention bug above OR as a paired task with clear coordination
  - dispatched for Round-2 wave-2 against @@FullStackA

- "Copied path" status-bar notification persists too long (doesn't auto-dismiss)
  - flagged 2026-05-21 by @@Alex (screenshot): triggering the "copy path" action surfaces a "Copied path" notification in the status bar that stays visible for an unusually long time, well past the expected toast-style auto-dismiss window. User has to wait it out or possibly click to dismiss
  - related context: `-a-2` (Round-1) reworked the status-bar click semantics (removed most click handlers; kept only notification expand/collapse) + flipped the notification flash colour blue → yellow. Per `-a-2`'s landed shape, status-bar notifications are surface-only with an expand-collapse affordance — auto-dismiss timing wasn't part of that fix's scope
  - root cause hypothesis:
    1. **Timeout duration too long**: the dismiss timer constant for "Copied path" (and probably every status-bar transient toast) is set to a value much higher than typical OS toast conventions (~3-5 s). Could be 30+ s or even no auto-dismiss at all (only manual). One-line constant fix
    2. **Timer not firing / getting reset**: the auto-dismiss `setTimeout` registers correctly but a re-render / state-update clears + re-registers the timer repeatedly, so the dismiss never actually fires. Common pattern when `$effect` reactivity treats the timer registration as a side-effect that re-runs on every poll
    3. **Status-bar notifications conflated with the persistent watcher-events panel**: status bar has BOTH transient toasts (copy path, save complete, etc.) AND persistent watcher-event notifications (unread events). If "Copied path" rides on the persistent-event channel, it stays until manually dismissed — wrong channel for a transient action
  - want, two pieces:
    1. **Auto-dismiss "Copied path" + similar transient toasts after a short window** (~3 s recommended; 4-5 s acceptable). Standard toast conventions: short enough to not crowd the UI, long enough to be read
    2. **Audit the status-bar notification taxonomy** at task-cut: which notifications are TRANSIENT (auto-dismiss after timeout) vs PERSISTENT (stay until user dismisses)? "Copied path" / "Saved" / "Build complete" are transient by convention. Watcher-event counts / error notifications are persistent. The distinction should be explicit in the data model + render path, not implicit in the timeout-or-not behavior
  - first investigation step: grep for "Copied path" string in the SPA source; find the emission call site + the status-bar notification-list datastructure; trace whether transient vs persistent channels exist or whether everything's on one path
  - lane: @@FullStackA (status-bar UI; same lane that owns `-a-2`)
  - severity: paper-cut UX (not blocking work; just visual clutter); v0.11.2 patch candidate if the patch wave gets cut (bumps scope from 5/6 → 6/7)
  - dispatched for v0.11.2 patch (if cut) OR Round-2 wave-2 against @@FullStackA

- Rich-prompt cursor renders OVER the default placeholder message
  - flagged 2026-05-21 by @@Alex (screenshot): when the rich prompt opens with an empty buffer + the default placeholder "Write a multi-line command and Cmd+Enter" visible (from `-a-24`), the CodeMirror I-beam cursor renders on TOP of the placeholder text (visible overlap on the leading "W" of "Write")
  - severity: small UX paper-cut; @@Alex's framing: "annoying bug we need to fix early next round" — explicitly NOT v0.11.2 scope, queued for Round-2 wave-2 early
  - root cause hypothesis: the placeholder overlay landed in `-a-24` uses `position: absolute` + `pointer-events: none` over the composer; the CodeMirror cursor element renders at column 0 + its own z-index puts it ABOVE the placeholder layer. Both painted at the same coordinate → visible overlap
  - fix-direction options:
    1. **`caret-color: transparent` while placeholder visible** (recommended) — empty-buffer state has nothing for the cursor to anchor on; making the caret invisible until the user starts typing is the standard browser placeholder pattern (`<input type="text" placeholder="...">` does this natively). One-line CSS rule conditioning on the placeholder-active state.
    2. **Lower placeholder z-index** so the cursor stays visible — opposite shape. Less natural; the cursor visible against placeholder text reads as "two things at once" which is confusing.
    3. **Hide the cursor element via display: none** on the empty-buffer + placeholder-shown state — more invasive than `caret-color: transparent` since CM6's selectionLayer expects the cursor element to be reachable; transparent caret is cleaner.
  - recommendation: option 1 (`caret-color: transparent`). Toggles off the moment the user types (placeholder dismissed → CSS rule no longer matches → caret-color reverts to theme token).
  - lane: @@FullStackA (rich prompt + placeholder CSS owner; same lane that landed `-a-24` + the v0.11.2 mini-wave)
  - dispatched for Round-2 wave-2 against @@FullStackA. Cut as a small task at fan-out; could fold with another small rich-prompt polish task if anything similar accumulates by then

- Hybrid hamburger menu missing Search entry
  - flagged 2026-05-21 by @@Alex (screenshot): the Hybrid pane hamburger shows Terminal / File Browser / Rich Prompt / Graph / Enter Hybrid NAV / Focus border colour. Search is missing — even though it's not a Hybrid SURFACE (per @@Alex's A.3 answer, search stays out-of-Hybrid as a global overlay), it should be reachable as a SPAWN entry from the hamburger alongside the four content-tab spawns
  - root cause: `-a-32`'s surface unification covered the four content-tab spawns (Terminal / FB / RichPrompt / Graph) across carousel slide 1 + pane hamburger + empty-pane right-click. Search overlay wasn't included in that set because it's a different surface category (overlay, not tab). The omission shows up in the hamburger
  - want: add a "Search" entry to the hamburger menu (between Graph and the Enter Hybrid NAV / palette separator). Click triggers the search overlay (same action as `Cmd+K F` from `-a-6` / `Cmd+. F` via Hybrid NAV). Chord hint on the row reads `Cmd+K F` (or whichever chord is canonical). Icon = magnifying-glass / `Search` lucide
  - consistency check at task-cut: same Search entry should ALSO appear on carousel slide 1 + empty-pane right-click menu per the `-a-32` unification — confirm whether @@Alex wants Search on those too OR just the hamburger. Recommend all three for consistency
  - lane: @@FullStackA (Pane.svelte hamburger + carousel + empty-pane right-click)
  - dispatched for Round-2 wave-2 against @@FullStackA. Small task; could fold with other hamburger / carousel polish at fan-out

- Add orange focus border colour after blue
  - flagged 2026-05-21 by @@Alex: focus-border palette today shows blue (selected) / green / pink. Want orange added AFTER blue (so palette order becomes blue / orange / green / pink)
  - small CSS + state addition. Orange colour value picks at task-cut (recommend matching the warm-accent token already used elsewhere in chan's theme — e.g., the yellow → warm-orange band from `-a-2`'s notification flash colour change; or a fresh CSS variable like `--focus-border-orange: #ff8c42` or similar)
  - persistence: the focus-border colour preference already round-trips via the existing pane-config / SerTab persistence; just add `"orange"` as a new accepted variant
  - lane: @@FullStackA (Pane.svelte palette + theme CSS)
  - dispatched for Round-2 wave-2 against @@FullStackA. Trivial; could fold with the Search-in-hamburger task above

- Unwanted black bar between terminal-host and rich prompt
  - flagged 2026-05-21 by @@Alex (screenshot): a solid black horizontal band paints between the bottom of the terminal output area and the top of the rich-prompt floating-pill in chan-desktop. Visible against the dark theme; not part of the intended `-a-24` floating-pill visual design
  - root cause hypothesis: leftover from `-a-4`'s dynamic `.terminal-host` margin-bottom (`heightPx + 12px`) painting at the wrong layer. The 12 px gap was supposed to be empty space the terminal area's background colour leaks through, not a separate painted band. Possible causes:
    1. The reserved-space element has `background: black` literally set somewhere instead of `transparent` / inheriting the pane bg.
    2. `-a-29`'s ResizeObserver-driven `measuredHeightPx` field is rendering a sibling spacer element that has the wrong bg colour.
    3. A separator border between `.terminal-host` and `.rich-prompt` was added intentionally somewhere but should have been `transparent` or omitted.
  - want: black band gone. If the spacer element is needed structurally (for the dynamic margin-recompute path), set its `background: transparent` so the pane bg paints through. If the spacer isn't needed (the margin alone reserves the space), delete it entirely
  - lane: @@FullStackA (rich-prompt + terminal-host layout in `TerminalRichPrompt.svelte` / `TerminalTab.svelte`)
  - dispatched for Round-2 wave-2 against @@FullStackA. Investigation + small CSS fix. v0.11.2 candidate IF root cause turns out to be a one-line CSS variable fix; otherwise defer to wave-2

- Terminal columns don't widen after pane / window resize (PTY stays at old cols)
  - flagged 2026-05-21 by @@Alex (screenshot): after resizing the chan window with multiple terminal panes laid out, agent output in most terminal panes renders very narrow — single-word-per-line wrapping that doesn't match the actual visible terminal width. The terminals appear to retain their pre-resize column count instead of expanding to the new pane width
  - **2026-05-21 follow-up from @@Alex**: nudging the window resize handle by a tiny amount AFTER the initial resize causes the terminals to immediately refresh to the correct full width. So the fit + PTY resize wiring is intact end-to-end; the bug is specifically that the FIRST resize transition's observer fire is missed/swallowed, and any subsequent micro-resize works. Narrows the root-cause hypothesis space: rules out hypothesis (c) (PTY resize call missing) and most of (b) (fit-addon call missing); points at (a) (ResizeObserver miss / debounce eating the event during the layout transition) or (d) (race with the broadcast/multi-tab layout path). Also unlocks a palliative-first option even before full root cause: trigger an explicit force-fit once when the resize-settled state is reached (e.g. on the trailing edge of a `requestAnimationFrame` / debounce window after the last resize event), so the missed first observer fire gets a guaranteed second chance. May be enough to ship if the deeper root-cause investigation runs long
  - root cause hypothesis: missing or unreliable SIGWINCH propagation on resize:
    a) **ResizeObserver miss**: the per-terminal ResizeObserver doesn't fire on the window-resize / pane-resize transition (or fires but is debounced past the visible-flicker window)
    b) **xterm.js fit-addon call missing**: even if the observer fires, `fitAddon.fit()` isn't called → xterm's internal cols/rows stay stale → PTY doesn't get SIGWINCH → agent's view of `$COLUMNS` is wrong
    c) **PTY resize call missing**: SPA may call `fit()` correctly but not propagate the new cols/rows to chan-server's `Session::resize` (which forwards SIGWINCH to the child process). The agent's PTY believes it's still at old cols
    d) **Race with the broadcast/multi-tab layout**: if the pane-resize happens during a layout transition (e.g., flipping Hybrid, splitting panes, the Round-1 work in `-b-2`'s lineHeight + scrollback changes), the resize event may be lost
  - want: every terminal pane sees correct cols/rows after any resize (window resize, pane drag, Hybrid flip, layout change). Agent output in the PTY reflows to the new width on the same frame as the visible terminal does
  - first investigation steps:
    1. Repro: open 4+ terminals in a tiled layout (similar to the screenshot's 6-pane setup), let an agent fill them with output, then resize the window. Observe whether subsequent agent output narrows / stays narrow.
    2. DevTools (post-v0.11.2 `-a-36` + `-b-17` unlock): inspect xterm.js's internal `term.cols` / `term.rows` before + after resize. If the values don't update, ResizeObserver / `fit()` is the bug.
    3. Server-side: log `Session::resize` calls from chan-server. If the SPA-side fit updates but the resize doesn't flow to the server, the PTY's view stays stale.
  - lane: @@FullStackB (terminal + PTY + chan-server `Session::resize` plumbing — same lane that owns `-b-2` terminal cluster work + `-b-11` scrollback/TERM settings + `-b-13` submit-mode)
  - severity: paper-cut + multi-occurrence; affects daily-driver flow when working with many agents in tiled layouts. NOT v0.11.2 candidate (patch wave is about to cut + this needs investigation time)
  - dispatched for Round-2 wave-2 against @@FullStackB

- chan-desktop leaves bundled `chan serve` sidecars orphaned after parent dies; new desktop launches can't bind the same drive
  - flagged 2026-05-21 by @@Alex (recovery walk after the `ci-8` dryrun.4 verification incident — see `webtest-b-1.md` "Unintended side effects" tail): when chan-desktop is killed (SIGTERM, crash, or any non-graceful exit), its bundled `chan serve` sidecar processes get re-parented to PID 1 and stay alive indefinitely. Each one keeps holding its drive lock + listening on its bound port. The next chan-desktop launch that tries to open the same registered drive can't bind because the orphan still holds it
  - repro: kill any running chan-desktop with `kill <pid>` (or it crashes naturally). `ps aux | grep 'chan serve'` shows the orphans still alive (PPID 1, original port). Launch chan-desktop again, click the drive in the launcher — drive doesn't open. No diagnostic surfaced; user has zero visibility into why
  - recovery today: manual `pkill chan` → if children remain, `kill -9 <pids>` individually. This dance is opaque + intimidating + the SIGTERM-vs-SIGKILL escalation isn't obvious. Regular users won't know to do this
  - root cause hypothesis:
    1. **chan-desktop doesn't reap its sidecars on shutdown**: the Tauri shell spawns `chan serve` children but doesn't propagate SIGTERM / SIGINT to them on its own death. Children inherit no process-group relationship that would auto-kill them when the parent dies. macOS launchd then re-parents them to PID 1 and they continue running until they exit on their own (which they never do)
    2. **Drive lock is process-scoped but not lifecycle-tracked**: chan-drive's atomic-write lock or the watcher's file-handle persists for the lifetime of the holding process, with no expiry / heartbeat / takeover mechanism for new chan-desktop sessions to claim it
    3. **Port binding conflict surfaces silently**: chan-desktop's launch-time bind probably fails with EADDRINUSE but the failure is swallowed (or the UI just doesn't enable the drive toggle without telling the user why)
  - want, TWO pieces:
    1. **Prevention — chan-desktop reaps its own sidecars on exit.** When chan-desktop receives SIGTERM / SIGINT / or its Tauri event loop exits cleanly, it should signal every spawned `chan serve` + `chan __mcp-proxy` child with SIGTERM (then SIGKILL after a brief deadline). Two implementation shapes worth considering: (a) put the children in a separate process group via `setpgid` + send signal to the group on exit; (b) keep a `Vec<Child>` of spawned children in `AppState` and walk it in a Drop / `on_window_event(CloseRequested)` handler. The Drop-on-AppState route is more portable; the process-group route is more robust to chan-desktop itself dying ungracefully. Recommend doing BOTH (defense in depth)
    2. **Recovery — drive lock takeover UX.** When the user clicks a registered drive in chan-desktop's launcher and the bind / lock acquisition fails because another process holds it: (a) identify the holder (via `lsof -i :<port>` shape, or via a known sidecar metadata file under TMPDIR with PID), (b) confirm the holder is itself an orphan `chan serve` for the same drive path (NOT some unrelated process — don't kill arbitrary PIDs), (c) auto-kill it (SIGTERM with deadline → SIGKILL) and proceed with the bind, (d) **warn the user we just did that** via a transient toast / status message ("Reclaimed drive from orphaned process from previous session"). If the holder turns out to be NOT a chan sidecar, refuse + surface the error with the offending PID + advice
  - lane: @@FullStackB (chan-desktop Tauri lifecycle owns prevention piece; takeover-UX dialog lives in chan-desktop too). Sidecar shape crosses into @@Systacean territory if chan-drive needs a lock-takeover protocol primitive — design the takeover signal at task-cut
  - severity: REGRESSION-class UX bug; surfaces every time chan-desktop is killed ungracefully (which Alex demonstrated happens; also any future crash). Blocks the user from reopening their drives without manual `pkill` knowledge
  - dispatched for Round-2 wave-2 against @@FullStackB

- Terminal watcher silently stops dispatching events mid-session (ingest wedge)
  - flagged 2026-05-21 by @@WebtestB (side observation during `-b-13` walkthrough on `/tmp/chan-survey-wb-r2`; see `event-webtest-b-architect.md` "Side observations" tail): after the watcher attached to a directory and dispatched the first two events successfully, subsequent file drops in the same directory stopped firing `dispatch_agent_event`. `dropped_events` counter stayed at 2; zero new log entries; multiple write strategies (Claude Write tool, atomic `mv`, /tmp vs /private/tmp canonical) all silent. Restarting the lane-B serve cleared the wedge
  - additional symptom on the restart: the SPA-side SerTab carried the watcher pill state across sessions, and after the restart the pill showed `watching /tmp/chan-survey-wb-r2 | Stop watching` despite the new server not having a watcher attached. First interaction surfaced `watch read failed: terminal watcher is not attached`
  - root cause hypothesis:
    1. **fsnotify ingest queue saturation or back-pressure**: the watcher's `tokio::sync::mpsc` (or whichever channel) between the OS notification source and the dispatch handler reached its bounded capacity, and subsequent producer pushes silently dropped without updating `dropped_events`. The `dropped_events` counter currently only tracks JSON-parse failures, not full-channel drops
    2. **Watcher task panic that didn't propagate**: the per-watcher tokio task panicked or returned an error, but the chan-server supervisor didn't restart it / didn't surface the failure to the SPA. The watcher pill stays "active" because state-of-record is the SerTab field, not the live task health
    3. **Macros / specific filesystem state**: /tmp and /private/tmp canonical-path resolution edge case in the watcher attach path that survives the first events then desyncs on later ones
  - want, two pieces:
    1. **Diagnose the wedge** — instrument the fsnotify ingest path with counter for ALL drop reasons (parse failure, channel-full, task-dead). Add a watcher health probe endpoint chan-server / SPA can poll. If the watcher task is dead but the SerTab pill says active, the UI should reflect that
    2. **SerTab state reconciliation on serve restart** — on chan-server boot, walk SerTab watcher pills + actively probe the underlying watcher state; if pill says active but no watcher exists, reset the pill OR auto-reattach. The current "stale pill + first-click reveals error" is bad UX
  - first investigation: re-run a watcher-attach session, fire ~10 events back-to-back, watch chan-server logs for any indication of the wedge point. If reproducible, narrow to ingest-channel-saturation vs task-death. Cross-reference `crates/chan-server/src/event_watcher.rs` + the watcher attach path in `crates/chan-server/src/routes/terminal.rs`
  - lane: @@Systacean (chan-server `event_watcher` + `terminal_sessions` plumbing — same lane that owns `systacean-9` + `systacean-10` filename-filter work)
  - severity: silent-failure UX bug; affects agent-pokes-reply walkthroughs which are the validation surface for the entire `-b-13` rich-prompt cluster. Not v0.11.2 candidate (needs investigation time + no clean repro yet)
  - dispatched for Round-2 wave-2 against @@Systacean

## Round 2 — needs deeper change

- Large markdown files block the editor with a spinner while loading
  - want: lazy / chunked loading as the user scrolls, instead of one big up-front parse
  - architectural change (virtual scroll + chunked parser), not a quick fix → carries to Round 2

- Watcher dialog rejects trailing `/` on directory paths it should detect as directories
  - flagged 2026-05-21 by @@Alex: pasting an absolute directory path with the trailing slash (e.g. `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-8/alex/`) into the watcher's "watch directory" dialog produces the inline error `path ends with /, type a name` and disables OK
  - in attach mode (`PathPromptMode === "attach"`), trailing `/` is semantically a directory hint, not "I haven't typed the basename yet" — the validator's existing rejection rule from `pathValidate.ts:45-54` doesn't distinguish create/move/rename (where the slash IS a missing-basename signal) from attach (where the slash IS the intent)
  - root cause: `web/src/components/PathPromptModal.svelte:105` calls `validatePath` without an `allowTrailingSlash` opt or mode-aware branch; `pathValidate.ts::validatePath` only knows the generic case
  - fix direction: extend `validatePath`'s `opts` with `allowTrailingSlash?: boolean` (or thread the `PathPromptMode` through) so the attach call site accepts the slash; resolver server-side already handles the trailing slash either way (the watcher resolver from `fullstack-b-3` / `systacean-5` treats the path as a directory regardless of the slash)
  - small SPA-side fix; @@FullStackA lane (validator + modal call site)
  - **NOT YET DISPATCHED** — Round-2 wave-2 candidate per @@Alex "for later"

- Hybrid pane drag-to-rearrange via top-bar dead zone (auto-enters NAV transaction mode)
  - feature ask 2026-05-21 by @@Alex: today's Hybrid NAV mode is keyboard-chord-driven; mouse interactions are limited to top-bar clicks. Mouse-native rearrange would feel like dragging windows in a tiling window manager
  - two entries to transaction mode, both targeting the same top-bar dead zone (space between last tab and hamburger menu):
    - **drag-start** from the dead zone enters NAV transaction mode WITH the originating pane as the first grab (drag-with-payload; fluent path).
    - **double-click** on the dead zone enters NAV transaction mode WITHOUT an originating grab (standby; next click+drag inside any Hybrid grabs that pane; discoverable affordance).
  - once in transaction mode: click anywhere inside any Hybrid grabs + drags that pane; Enter commits the rearrangement, Esc dismisses + reverts
  - keyboard NAV (`Cmd+.` + chord-rearrange) stays unchanged — transaction mode is a SUPERSET of mouse affordances on top
  - composes with `-a-32` chord migration + `-a-43` Hybrid back-side refactor (hard prereq — concurrent Pane.svelte edits would create merge pain)
  - dispatched as `fullstack-a-44` (queued; starts after `-a-43` commits + clears)

- `-a-37` suggest-reopen flow: indexer-timing-dependent gap on the FB suggest path
  - side observation 2026-05-21 by @@WebtestA during v0.11.2 lane-A walkthrough: pieces 1+2 (debounced recovery check + Re-open path) verified working. The suggest-from-FB-basename-match piece is intermittent — if the basename-matching path fires AFTER the indexer has picked up the moved file, suggestion appears; if the timing is reversed (indexer hasn't re-indexed yet), the suggestion never appears
  - root cause hypothesis: suggest-from-FB depends on indexer state; race against the move event vs the indexer's pickup latency
  - want: deterministic timing — either gate the suggest on indexer-up-to-date confirmation, or wait + retry on miss
  - NOT YET DISPATCHED — Round-2 wave-3 or v0.11.3 candidate; pieces 1+2 are the load-bearing fix from `-a-37` and held under all conditions
- `-a-39` title fallback `Files N` not exercised + chan-server-side `be` serialization gap
  - side observations 2026-05-21 by @@WebtestA during v0.11.2 lane-A walkthrough
  - **Title fallback**: every Cmd+Alt+O spawn in @@WebtestA's walk threaded the existing tab's `bs:"docs/journals"` selection, so new tabs fell back to the dir-name convention (`journals/`) rather than the `Files N` convention. Whether this is the intended user-facing behaviour OR a gap is open — flag for the implementer when this dispatches
  - **`be` serialization**: URL hash never carries `be` for any FB tab in repro, even when the active tab's tree has 4+ expanded dirs. The continuous tracker effect at `FileBrowserSurface.svelte:142-150` is wrapped in `untrack` (writes to `tab.expanded` don't propagate to the outer `persistLayoutToHash` effect); even if `tab.expanded` does update reactively, the hash-write trigger doesn't fire
  - NOT YET DISPATCHED — annotate against the existing `-a-39` lineage when cut
- `chan index enable-semantic` / `disable-semantic` against a live-served drive: misleading error wording
  - side observation 2026-05-21 by @@WebtestB during v0.11.2 lane-B walkthrough: error reads `Error: not a chan drive at <path>; run \`chan add <path>\` first` with the real cause `drive is locked by another process` demoted to a `Caused by:` line
  - misleading for scripted wrappers — a script that hits the failure may run `chan add` redundantly
  - pre-existing in v0.11.1; systacean-7's verdict tested toggles on an unserved drive so this didn't surface
  - want: top-line error names the real cause; the "not a chan drive" message reserved for actual not-a-drive paths
  - NOT YET DISPATCHED — Round-2 polish candidate; systacean-8 family
- Webtest tooling: terminal tab close buttons require full pointer sequence (headless-driving quirk)
  - side observation 2026-05-21 by @@WebtestB: terminal tab close buttons need a full pointerdown → mousedown → pointerup → mouseup → click sequence; bare `.click()` is sometimes dropped
  - NOT a real-user regression (mouse generates the full sequence); webtest-automation note only
  - want: future webtest automation lanes default to the full pointer sequence on close-button interactions, OR audit why the bare `.click()` is intermittent and document
  - NOT YET DISPATCHED — webtest-tooling tracking item, no end-user impact

- chan-reports settings toggle missing from Settings UI (regression)
  - flagged 2026-05-21 by @@Alex during the graph-overhaul scope conversation: "chan-reports disappeared and there's no setting to turn it on/off anymore... i want it back!"
  - reports toggle was specced in the Round-2 pre-flight feature toggles plan (`round-2-plan.md` §"Pre-flight feature toggles") alongside the semantic-search toggle from `-a-21`. Either the toggle never landed (only `-a-21` semantic-search landed) OR landed and got removed in a later refactor
  - want: chan-reports settings toggle restored + a discoverable home (current overlay or one of the Hybrid back-sides per the sequencing decision below)
  - sequenced with the broader graph overhaul per @@Alex 2026-05-21 "no v0.11.3 hotfix; v0.11.2 stays as-is; next cut bundles all of Round 2 (possibly Round 3)"
  - settings home is open question #2 in the architect-side graph overhaul plan (overlay vs FB-back vs Graph-back)
  - NOT YET DISPATCHED — folds into the graph-overhaul wave
- Graph: depth slider does not reveal more nodes as depth increases
  - flagged 2026-05-21 by @@Alex during the graph-overhaul scope conversation: "the depth slider seems not to be working at all.. that slider should reveal forward nodes as the depth increases"
  - "forward nodes" semantic: outgoing-edge targets from the current root (still needs confirmation per architect open question #5)
  - hypothesis space: (a) slider value not threading through to the layout/filter code; (b) slider value threads through but filter logic is wrong (no-op); (c) the slider does what its implementer intended but the semantic doesn't match user expectation
  - want: dragging the depth slider higher reveals additional forward-node hops from the current root; dragging lower hides them again
  - investigate first; if pure-SPA wire bug, @@FullStackA lane; if server-side depth gate, @@Systacean lane
  - NOT YET DISPATCHED — folds into the graph-overhaul wave

- Search overlay: remove the scope affordance entirely
  - feature ask 2026-05-21 by @@Alex: "remove the scope from the search overlay"
  - the current Cmd+K F search overlay has a scope control that @@Alex no longer wants; the search overlay simplifies to a single global query input
  - couples with the broader search overlay redesign (move Search Status panels into the carousel; add file-name search)
  - NOT YET DISPATCHED — Round-2 wave-3 candidate; couples with the carousel redesign (Item 1+4) sub-wave
- Search Status panels (Index + Code Report) move from search overlay into the carousel
  - feature ask 2026-05-21 by @@Alex: "the search status stuff... will move to the carousel" (with screenshot)
  - panels in scope:
    - **INDEX** panel: state, chunks, vectors, model (e.g. `BAAI/bge-small-en-v1.5`), Rebuild index button.
    - **CODE REPORT** panel: total files / SLOC / comments / complexity + per-language SLOC + file-count bars, plus a "Graph from here" button.
  - destination: the drive metadata carousel (currently the Infographics tab container per Round-2 Item 1+4)
  - the carousel becomes the canonical home for drive-level status surfaces; the search overlay reduces to the query surface
  - couples with: chan-reports settings restoration (G1 / Task F); carousel redesign + Infographics tab container (Item 1+4); chan-report cross-dir aggregation (`systacean-15`)
  - NOT YET DISPATCHED — Round-2 wave-3 candidate; couples with the carousel redesign sub-wave
- Search by file name (or parts of name) + per-file inspector like the prior image inspector
  - feature ask 2026-05-21 by @@Alex: "id like to be able to find all files by name or parts of the name via the search, and see their inspector like we used to have for images"
  - new search mode: file-name search distinct from the existing semantic + BM25 content search. Returns files matching a substring of the basename (or full path? scope question for fan-out)
  - inspector pattern: surface the metadata + actions inspector for any selected result, modeled on the image-file inspector pattern that existed previously (memory: `project_media_browser` planned non-editable files visible in tree; future media browser is first-class)
  - relates to the existing FB / Cmd+K F query plumbing — implementer audits whether the chan-server already exposes a file-name search endpoint (file-browser uses prefix search) or needs an addition
  - couples with: search overlay redesign (above); FB-side inspector pattern (used as visual reference for the search-result inspector)
  - NOT YET DISPATCHED — Round-2 wave-3 candidate; could ship alongside the search overlay redesign

- F3 SCOPE UPDATE 2026-05-21: unified entity search broadens beyond files
  - @@Alex follow-up: "also tags, contacts, anything"
  - F3 (filed above as "Search by file name...") is reframed: substring-match name search across EVERY indexed entity type — files, hashtags, contacts, mentions, languages, directories, anything in the chan-drive graph index
  - Result row tells the user what KIND of entity (file / hashtag / contact / mention / language / directory) + its name; clicking opens the inspector for that entity, per-type:
    - **File** → file inspector with Open + Graph-from-here (graph overhaul G4)
    - **Directory** → directory inspector with aggregated reports stats (graph overhaul G3)
    - **Hashtag** → tagged-files list
    - **Contact / Mention** → contact card
    - **Language** → first-depth dirs containing files of that language (graph overhaul G7/G8)
  - inspector pattern is shared with the graph overhaul's G3/G4 + the FB-side inspector — single component shape across all surfaces
  - implementer audits at fan-out: chan-server probably has separate endpoints per entity type; the unified search either fans out internally OR a new unified-search endpoint aggregates
  - the F3 entry above + this addendum together are the operative spec; round-2-plan §"Search overlay redesign" carries the consolidated version
- chan-desktop orphan-detection heuristic too loose (false-positive risk in noisy shell environments)
  - flagged 2026-05-21 by @@WebtestB during `-b-22` walkthrough (`webtest-b-3` verdict): the heuristic in chan-desktop's drive-lock-takeover path matches ANY process whose command line contains `chan` + ` serve ` + drive-key as three INDEPENDENT substrings, not a contiguous `chan serve <drive-key>` argv sequence
  - real-world likelihood narrow but non-zero: a `tail -f chan-serve.log` over the drive key, an IDE process inspecting the directory, a tmux pane with `chan serve <drive-key>` in visible scrollback that happens to be mid-process, etc. COULD enter the candidate set
  - destructive-action confirmation surface is opaque: the `promptDriveLockTakeover()` Tauri `ask()` dialog does NOT display candidate PIDs to the user (yes/no shape only); the user can't see what's about to be killed
  - want, TWO pieces:
    1. **Tighten the heuristic**: match `chan serve <drive-key>` as a contiguous argv sequence (regex or positional argv check) instead of three independent substrings
    2. **Render candidate PIDs in dialog**: replace Tauri's plain `ask()` with a custom modal so the user sees the offending PIDs + command lines before confirming the SIGTERM
  - lane: @@FullStackB (chan-desktop runtime)
  - NOT YET DISPATCHED — Round-2 wave-2/wave-3 polish for the `-b-22` follow-up; not regression-blocker (the existing `-b-22` shape works for the load-bearing case Alex demonstrated; this is hardening against edge cases)

- Linux binaries shipped on phase-8 next-release tags (chan CLI + chan-desktop)
  - feature ask 2026-05-21 by @@Alex: "next phase we should have binaries for linux too, chan and chan-desktop!"
  - state today:
    - **chan CLI Linux binaries**: blocked on the `release.yml` trigger glob mismatch caught by `ci-11`. With ci-11's fix landing (adds `chan-v*` to the trigger), the NEXT `chan-v*` tag fires `release.yml` and ships the matrix — IF Linux is already in the matrix shape. Audit needed at fan-out.
    - **chan-desktop Linux binaries**: per `ci-7` audit trail (2026-05-21), `release-desktop.yml` builds Linux .deb / .AppImage and uploads them as the workflow artifact `chan-desktop-linux-x86_64-unsigned` — but NOT as GitHub Release downloadables. The upload-to-release step apparently only handles the macOS DMG today
  - want: on the next `chan-v*` tag (v0.12.0 or whichever ships next), the GitHub Release page carries downloadable Linux binaries alongside the macOS DMG. Both chan CLI (.deb / .rpm / .tar.gz) AND chan-desktop (.deb / .AppImage) accessible from the Releases page
  - Linux binaries are unsigned (no equivalent of Apple Developer ID for the dogfood / public-flip window); Linux signing options exist but are NOT in scope for v0.12.0
  - couples with: `ci-11` (release.yml trigger fix, already cleared); a new `ci-N` task to wire the Linux artifact path into `release-desktop.yml`'s release-job; possible matrix audit in `release.yml` to confirm Linux targets are present
  - lane: @@CI primary; @@Systacean possibly if matrix shape needs cargo-target additions
  - architecture caveat (added 2026-05-21 by @@Alex): real release matrix needs BOTH aarch64 AND x86_64 Linux binaries. Today `release-desktop.yml` builds x86_64-only on Linux (per `ci-7` audit trail). aarch64 Linux is a forward-looking matrix expansion (Round-2 wave-3 or Round-3 polish depending on scope). Local dev validation via sdme + lima-vm is aarch64-only; CI on `ubuntu-latest` is x86_64. See memory `reference-local-linux-via-sdme.md` for the local-validation invocation shape.
  - NOT YET DISPATCHED — Round-2 wave-3 candidate; lands ahead of v0.12.0 cut
