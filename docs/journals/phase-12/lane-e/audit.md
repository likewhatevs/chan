# Lane E audit: shortcut policy gap table

@@LaneE, 2026-05-27. Baseline `f72b8a7`. SPEC = addendum-2 request.md Shortcuts
section; RATIFIED = round-n-review.md Q5-Q9 (win where they differ).

## Verdict

The shortcut system is mostly already in place. @@Alex's "we already have this"
instinct holds: the registry (`shortcuts.ts`), the web keymap (`App.svelte`),
the native key-bridge (`serve.rs` KEY_BRIDGE_JS), the native menu (`main.rs`),
and a custom find-in-document (`FindBar.svelte`) already implement the bulk of
the policy. The work is a SMALL set of gaps + a few verifications, not a rewrite.

Legend: OK = already matches policy (verify only). GAP = behavioral change
needed. DEC = needs a ruling before I implement.

## Platform model (ratified Q5) - how the two keymaps split

- WEB keymap = `App.svelte onWindowKey` (+ `onCtrlDCapture`). `meta` there means
  `metaKey || ctrlKey`.
- NATIVE keymap = `serve.rs KEY_BRIDGE_JS`, capture-phase, `metaKey || ctrlKey`,
  stopImmediatePropagation so the web handler never double-fires on desktop.
  This is why native + web are cleanly separable: changing the web handler does
  not touch native, and vice versa.

## Gap table - navigation

| Binding (policy)            | Web today        | Desktop today    | Status |
|-----------------------------|------------------|------------------|--------|
| web prev/next TAB           | Alt+Shift+[/]    | n/a              | OK     |
| web prev/next PANE          | Cmd/Ctrl+[/]     | n/a              | GAP    |
| desktop cmd+1..9 tabs       | n/a              | Cmd+1..9         | OK     |
| desktop cmd+shift+[/] tab   | n/a              | Cmd+Shift+[/]    | OK     |
| desktop cmd+[/] pane        | n/a              | Cmd+[/]          | OK     |

GAP - web pane nav. `App.svelte:723-731` binds pane prev/next to `meta+[/]`
(Cmd/Ctrl). Policy wants `Alt+[/]` on web. The whole point: on web `Cmd+[/]` is
browser back/forward, so nav must move to Alt. Fix = change that handler to
`altKey && !shiftKey && !meta` + BracketLeft/Right (matches by `e.code`,
preventDefault the typed glyph, exactly like the existing Alt+Shift+[/] tab
handler). Also update `shortcuts.ts` pane entries: web `Mod+[` -> `Alt+[`.
Native pane nav stays `Cmd+[/]` via KEY_BRIDGE (untouched).

## Gap table - close cascade (Q6)

| Step                              | Today                      | Status |
|-----------------------------------|----------------------------|--------|
| close current TAB                 | Ctrl+D + native Cmd+W      | OK     |
| no tabs -> close PANE             | empty pane Cmd+W if >1 pane| OK     |
| no panes -> close WINDOW + refocus| no-op on last empty pane   | GAP    |
| terminal keeps readline ctrl+d    | onCtrlDCapture skips term  | OK     |

GAP - cascade tail. `closeActiveEmptyPane` (`App.svelte:868-874`) returns false
when `leafPaneCount() <= 1`, so Cmd+W on the last empty pane does nothing. Policy
wants it to close the WINDOW and return focus to the native-desktop workspace
list. This is desktop-only (web has no window to close meaningfully). Fix needs
a `request_close_window` Tauri IPC (or reuse an existing one) + `main.rs` to
close the drive window and show the launcher ("Drives" / main window). I need to
confirm whether such an IPC exists or must be added (see DEC-1).

## Gap table - terminal collisions (Q7)

| Binding            | macOS today        | Linux today           | Status |
|--------------------|--------------------|-----------------------|--------|
| ctrl+a editor      | line-start (CM6)   | select-all (CM6)      | OK*    |
| cmd+a editor       | select-all (CM6)   | n/a                   | OK*    |
| ctrl+a terminal    | readline (xterm)   | readline (xterm)      | OK     |
| ctrl+d close/EOF   | term=EOF else close| term=EOF else close   | OK     |
| ctrl+w terminal    | readline (xterm)   | KEY_BRIDGE closes tab | GAP    |

OK* (verify) - ctrl+a / cmd+a. The editor uses CodeMirror's `defaultKeymap`
unmodified. CM6's defaultKeymap binds, on macOS, Ctrl-a -> line-start (emacs
style) and Cmd-a -> selectAll; on Linux/Windows, Ctrl-a -> selectAll. That
already matches the policy on both platforms. The terminal never escapes ctrl+a
(not in the escapeTerminal set), so xterm forwards it to readline on both. This
looks correct already; I will confirm with the real CM6 version + a browser walk
rather than assume.

GAP - Linux desktop ctrl+w. KEY_BRIDGE_JS (`serve.rs:615`) fires `app.tab.close`
on `metaKey||ctrlKey + KeyW`. On Linux the platform mod is Ctrl, so Ctrl+W
always closes the tab in capture phase BEFORE xterm sees it, breaking readline
delete-word inside a focused terminal. Q6 wants the terminal to keep readline
ctrl+w. macOS is fine (Cmd+W is not a readline key; Ctrl+W still reaches the
shell). Fix = make the close chord context-aware like Ctrl+D already is: when a
terminal is focused, do not intercept the platform-mod+W (let the shell have it);
close otherwise. See DEC-2 for where that check lives (key-bridge vs SPA).

Note: Ctrl+D already gives Linux a working close that is terminal-aware, so the
Linux "ctrl+w OR ctrl+d" requirement is partly satisfied; the bug is that the
current ctrl+w path is NOT terminal-aware.

## Gap table - find triad (Q9)

| Binding                | Web today      | Desktop today    | Status |
|------------------------|----------------|------------------|--------|
| cmd+f find in document | browser find   | Cmd+F -> FindBar | OK     |
| cmd+g next             | browser        | Cmd+G            | OK     |
| cmd+shift+g prev       | browser        | Cmd+Shift+G      | OK     |
| ESC closes find bar    | -              | FindBar onKeydown| OK     |
| no auto-scroll except  | -              | scroll on edit + | OK*    |
|   on keypress          |                |   index change   |        |

OK* (verify) - find is fully wired on desktop (FindBar.svelte + editor/find.ts).
ESC closes; Enter/Shift+Enter and Cmd+G/Cmd+Shift+G advance and scroll the match
into view. `scrollIntoView` fires only on (a) a debounced query edit and (b) a
prev/next index change - i.e. on user keystrokes, not idle. Matches Q9. The one
edge worth a look: an external doc change while find is open re-scans (docText is
a scan dependency) and re-anchors, which can scroll; that is a legit reload, not
an idle scroll. I will validate the triad on web (browser-owned) AND desktop in a
walkthrough rather than change code unless the walk shows a deviation.

## Gap table - other desktop chords + commands

| Binding (policy)        | Today                       | Status |
|-------------------------|-----------------------------|--------|
| cmd+, settings          | menu accel + web Mod+,       | OK     |
| cmd+. Hybrid Nav        | Mod+. (web+native via SPA)   | OK     |
| cmd+s search            | NO chord; command exists     | GAP    |
| cmd+/ split right       | only Hybrid Nav Mod+. /      | GAP    |
| cmd+\ split bottom      | only Hybrid Nav Mod+. \      | GAP    |
| cmd +/-/0 zoom          | KEY_BRIDGE -> zoom IPC       | OK     |
| cmd+i / cmd+. i infogfx | NO chord; command exists     | GAP    |

GAP - cmd+s search. `app.search.toggle` exists in `runCommand` (`App.svelte:941`)
but nothing fires it: no registry entry, no web handler, no key-bridge case.
(History: `shortcuts.ts:295` shows Cmd+S was dropped when save went autosave-only;
the policy now reclaims it for drive-wide search.) Fix = web handler in
onWindowKey that preventDefaults Cmd+S and opens search (Q5 explicitly authorizes
preventDefault here) + KEY_BRIDGE `KeyS` -> `app.search.toggle` + a registry
entry (web `Mod+S`, native `Mod+S`) so the help table lists it.

GAP - cmd+/ cmd+\ split. Only reachable inside Hybrid Nav today. `splitActive
(direction)` exists at `tabs.svelte.ts:2964` (top-level, non-pane-mode), so the
wire-up is clean: add `app.pane.splitRight`/`app.pane.splitDown` commands in
runCommand calling splitActive("row"/"column"), KEY_BRIDGE cases for Cmd+/ and
Cmd+\, + registry entries (native-only per policy - split is listed under
desktop-native).

GAP - infographics. `app.infographics.open` exists (`App.svelte:972`) and is
wired to menus, but `handlePaneModeKey` has no `i` case and there is no direct
chord. Fix = add `i`/`I` to handlePaneModeKey (commit then open, same shape as
search/lock) so `Mod+. i` works everywhere. Direct `cmd+i` is a DEC (see DEC-3).

## Gap table - Hybrid "start from here" chords (verify + keep)

| Action      | Native | Web fallback | Hybrid Nav | Status |
|-------------|--------|--------------|------------|--------|
| terminal    | Cmd+T  | Cmd+Alt+T    | Mod+. t    | OK     |
| file browser| Cmd+O  | Cmd+Alt+O    | Mod+. o    | OK     |
| new draft   | Cmd+N  | Cmd+N        | Mod+. n    | OK     |
| graph       | Cmd+Sh+M| Cmd+Shift+M | Mod+. v    | OK     |
| rich prompt | Cmd+P  | Cmd+Alt+P    | Mod+. p    | OK     |

All five present and context-aware ("start from here" seeds scope from the
focused doc/terminal). Pure verify - confirm in a walkthrough, no code change
expected.

## Proposed slices (post-review)

i.   nav + close cascade + split + zoom: web pane nav Cmd->Alt; cmd+s search
     (web+desktop+registry); desktop cmd+/ cmd+\ split; close-cascade tail
     (DEC-1); verify zoom. Mostly App.svelte + serve.rs + shortcuts.ts.
ii.  find triad polish: verify web + desktop walkthrough; code only if the walk
     deviates from Q9.
iii. terminal collisions: Linux ctrl+w terminal-awareness (DEC-2); verify ctrl+a
     both OSes. Touches the terminal-focus path -> coordinate with @@LaneC.
iv.  infographics Mod+. i + Hybrid chord verification; resync the help table via
     `node web/scripts/shortcuts-table.mjs`.

## Open decisions for @@Lead / @@Alex

- DEC-1 (close-window IPC): does a Tauri IPC to close the current drive window +
  show the launcher already exist, or do I add one? This is the cascade tail.
  Routine plumbing - I will add `request_close_window` if none exists unless you
  want it shaped differently.
- DEC-2 (ctrl+w context): the Linux terminal-readline fix needs the close chord
  to defer to a focused terminal. Cleanest is a small SPA-side check the
  key-bridge consults (e.g. a window flag set when a terminal is focused) since
  the key-bridge fires before xterm and cannot otherwise know focus. This is the
  shared seam with @@LaneC's terminal work - I will coordinate on c-e and propose
  the concrete mechanism there.
- DEC-3 (direct cmd+i): policy lists `cmd+i` for infographics but @@Alex flagged
  "maybe Hybrid Hamburger only". `cmd+i` is currently FREE (editor italic is not
  keymap-bound). I lean Mod+. i only (matches @@Alex's recollection of "cmd+. i")
  and treat direct cmd+i as optional. Confirm or I ship Mod+. i alone.
