# frontend-2: right-click menus and terminal bubble menu pass

Owner: @@Frontend
Status: REVIEW; CWD execution blocked on backend metadata

## Goal

Bring the right-click surfaces in line with the request: a shared
two-button menu (Reload + Inspector) on the empty PANE and on the
area outside an overlay, and a fully featured terminal right-click +
bubble menu.

## Relevant links

* Request: [request.md](./request.md)
* Journal: [journal.md](./journal.md)
* Prior frontend wave: [frontend-1](./frontend-1.md) (outside-overlay
  backdrop context menu already routed through the existing menu
  handlers; the menu content additions for PANE + outside-overlay
  land here).
* PANE menu today: `web/src/components/Pane.svelte` (search for
  "Empty-pane right-click menu" around line 109)
* Terminal top-bar today: see `web/src/terminal/` and the terminal
  tab component
* Related backend touch points: [backsystacean-6](./backsystacean-6.md)
  (tab-rename to env design memo)

## Scope

### PANE right-click

* Today: single button `Reload` on left-click in the PANE.
* Change: keep `Reload`, add `Toggle Inspector`. Same chord on
  left-click + on right-click (so users discover via either).
* The Inspector toggle uses the same store action the current
  Inspector toggle button uses; do not duplicate the state.

### Outside-overlay right-click

* Today: the area outside any open overlay falls through to the
  browser's default context menu.
* Change: capture right-click in that area and show the same two-
  button menu (`Reload`, `Toggle Inspector`) instead.
* Click outside the menu dismisses it. The browser default never
  fires for this region.

### Terminal top-bar -> bubble menu (done in frontend-1)

* [frontend-1](./frontend-1.md) REVIEW relocated the terminal
  header into the bubble menu (status, missed-bytes, find, copy
  scrollback, restart, resume). Find opens as a transient
  in-terminal search box. No further work here; verify visual /
  interaction during live smoke.

### Broadcast-mode bar, tab indicator, picker, and bubble menu reorder

* Today: terminal tabs carry a `broadcastEnabled` flag plus
  `broadcastTargetIds` (see
  `web/src/state/tabs.svelte.ts`); the toggle is reachable through
  the `app.terminal.broadcast.toggle` shortcut, but there is no
  visual indicator on the tab or near the terminal body.

* **Bubble menu order**: today the menu reads
  `Tab name input` -> `Broadcast Input Off Cmd+Shift+I` -> MCP
  items (`Set MCP env vars`, `Show MCP env in terminal`) ->
  target-tab checkbox list. Move the Broadcast toggle DOWN, next
  to the broadcast selectors. New order:
  1. Tab name input.
  2. `Set MCP env vars`.
  3. `Show MCP env in terminal`.
  4. Divider.
  5. `Broadcast Input On / Off` (Cmd+Shift+I).
  6. `Select All` / `Deselect All` button (see Picker change
     below).
  7. Target-tab checkbox list.

* **Broadcast status bar (new, replaces the old "connected -
  WxH" strip that [frontend-1](./frontend-1.md) removed)**: when
  the tab is in the broadcast group, render a one-line strip in
  the same DOM slot the old status bar occupied. Shape:

  ```
  [broadcast-icon]  [member1 x]  [member2 x]  [member3 x]  ...  [off]
  ```

  * The `broadcast-icon` is the radio-wave glyph; left-aligned.
    **It acts as a mute toggle**: click to mute (this tab stays
    in the group but stops both sending its keystrokes to other
    members and receiving keystrokes from them). Click again to
    unmute and resume normal in/out flow. The icon reflects
    state (active vs muted, e.g., a slash through the glyph
    when muted, or a dimmer fill). Mute is distinct from
    `[off]`: muting does not change group membership and the
    bar stays visible.
  * Each `member` chip carries the tab's name and a small `[x]`
    button on the chip. Clicking the chip body focuses that tab;
    clicking `[x]` removes that member from the broadcast group.
  * The remove action is **peer**: from any terminal in the
    group you can `[x]` any other member. The bar shows the
    same set of "other" members from every participating tab's
    perspective (i.e., the bar viewed on tab A lists B, C; on
    B it lists A, C; on C it lists A, B). The bar's presence
    implies self is in.
  * `[off]` button is right-aligned and removes **this** tab from
    the group (equivalent to another member clicking `[x]` on
    self). The bar then hides on this tab because self is out.
  * When the group shrinks to ≤ 1 member, the group dissolves
    and the bar disappears on the lone remaining tab.
  * State-model note: today the model is asymmetric (source has
    `broadcastEnabled` + `broadcastTargetIds`; targets are
    unaware). To make the bar + peer-remove work, lift the group
    into a symmetric set (e.g., a `broadcastGroupId` shared by
    every participant, or store the same member list on every
    member's tab record). @@Frontend picks the cleanest shape;
    the user-visible contract is "any participant can manage
    the group from their bar".
  * The bar is hidden entirely when self is not in any
    broadcast group; no leftover empty strip.
  * Color: use the contact pill pink (or another distinct slot;
    the royal-pink language token is reserved for code).

* **Tab-strip indicator**: the small per-tab marker next to the
  title (separate from the in-body bar). Same trigger
  (`broadcastEnabled`); render a compact `BCAST` chip or
  radio-wave glyph next to the tab name in the tab strip so the
  user can spot broadcast-mode tabs without focusing them.
  Tooltip on hover: `Broadcasting to N tab(s)`. Optional on
  target tabs this round (file a follow-up if it adds clarity).

* **Picker change**: the broadcast target picker grows a
  `Select All` / `Deselect All` button. `Select All` includes the
  current terminal tab (the source) too. Single toggle that flips
  label based on state.

### Terminal right-click

* Today: right-click inside the terminal falls through to the
  browser context menu.
* Change: chan menu with:
  * Copy (when there is a selection)
  * Paste (always; uses navigator.clipboard.readText)
  * Copy path to CWD (resolves the current working directory; falls
    back to a "PTY did not report CWD" toast if unavailable)
  * Show Dir (opens the CWD in the file browser)
  * Graph dir (graph-this on the CWD path, drive default still
    applies if CWD is the drive root per
    [architect-2](./architect-2.md))
  * New Terminal (opens a fresh Terminal-N tab in the same pane)
  * **New File** (opens the new-file dialog seeded with the
    terminal's CWD as the parent directory; reuses the dialog
    from [frontend-1](./frontend-1.md))
  * Split-pane buttons (horizontal + vertical, matching the existing
    pane split affordances elsewhere)
  * Search buttons (open the existing in-terminal search overlay)
  * Settings (opens the existing terminal settings surface)
* CWD discovery: PTY CWD source. If chan-server already exposes it
  through the terminal session metadata, use it. If not, file a
  follow-up for @@Backsystacean to add a route (POSIX:
  `readlink /proc/<pid>/cwd` on Linux,
  `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, ...)` on macOS).

### `^D` close-hint UI (verify)

* [backsystacean-1](./backsystacean-1.md) added Ctrl+D close
  handling after the shell exits. Verify the hint UI behavior:
  one-line message visible after exit, Ctrl+D closes the tab,
  Reload or fresh PTY clears the hint.
* If a hint string is missing or unclear, add it in this lane.

### Tab-rename stale-env prompt

* Contract is spawn-time-only per
  [backsystacean-6](./backsystacean-6.md) and Alex's decision
  2026-05-18: `$CHAN_TAB_NAME` inside the running shell stays at
  the inherited value until the user restarts the terminal.
* Behavior on rename commit:
  1. If the tab has an active PTY session
     (`tab.terminalSessionId` is non-null) AND the new name
     differs from the value passed to PTY spawn, surface an
     inline prompt next to the title (or as a small modal-style
     line in the bubble menu): "Tab name changed.
     `$CHAN_TAB_NAME` will stay at the old value until restart."
     with two actions: `Restart now` and `Later (keep stale env)`.
  2. `Restart now` calls the existing
     `restart()` from `web/src/components/TerminalTab.svelte:383`
     (closes the session, tears down xterm, starts fresh; new
     PTY picks up the new `CHAN_TAB_NAME`).
  3. `Later` dismisses the prompt but leaves a small stale-env
     badge near the title. The badge clears when the user
     restarts.
* Track the "name at spawn" in tab state so the comparison is
  cheap. Re-evaluate on every rename commit and on every fresh
  spawn (badge clears on spawn because the new value is captured).

### File Browser opens collapsed

* Today: opening the file browser auto-expands every directory.
* Change: first open lands collapsed; only the drive root is
  visible. The user expands directories on demand.
* Persisted-state interaction: if a saved session already has
  expansion state, restore it as before; only the first-open / no-
  state case lands collapsed.
* See `web/src/components/FileTree.svelte` and the file browser
  overlay open path.

## Out of scope

* New terminal naming (in [frontend-1](./frontend-1.md)).
* Theme refresh on terminals (in [frontend-1](./frontend-1.md)).
* Tab disambiguation (in [frontend-3](./frontend-3.md)).
* Backend PTY signal wiring (in [backsystacean-5](./backsystacean-5.md)).

## Acceptance criteria

* PANE left-click and right-click both show Reload + Inspector.
* Outside-overlay right-click shows the same two-button menu.
  Browser default does not appear.
* Terminal top bar collapses to just the title; all controls live
  in the bubble menu including the `rows x cols` line.
* Terminal right-click menu present with every action listed
  including New File. Copy / paste interact with the live PTY.
* `^D` hint shows when shell exits; Ctrl+D closes the tab.
* File browser opens collapsed on first open when no expansion
  state is persisted.
* Broadcast-mode tab-strip indicator visible next to the
  terminal tab title when `broadcastEnabled` is true; tooltip
  names the target count.
* Broadcast-mode in-body status bar appears in the slot the old
  "connected - WxH" strip occupied, shows `[broadcast-icon]
  [members...] [off]`, and disappears when the user clicks Off
  or toggles broadcast off from the bubble menu.

## Tests

* Vitest coverage for: PANE menu items, outside-overlay capture
  handler, terminal right-click menu actions (mocked clipboard +
  navigator), bubble menu reorganization, close-hint state machine.
* `npm --prefix web run check` clean.
* `npm --prefix web test -- --run` green.
* `npm --prefix web run build` clean.

## Review and hardening

* @@Frontend self-review for stop-propagation correctness on the
  outside-overlay handler (no double-firing inside open overlays).
* @@WebtestA live click-through pass on the new menus.

## Progress notes

* Added the empty-pane click and right-click menu entries for
  `Reload` and `Toggle Inspector`.
* Fresh file-browser opens now seed only the drive root as expanded;
  restored expansion state still wins.
* Terminal bubble menu now includes `New File`, moves MCP env rows
  above broadcast controls, adds `Select All` / `Deselect All`, and
  includes the current terminal in the target picker.
* Terminal tabs show a compact `BCAST` marker when broadcast input is
  enabled, with a target-count tooltip.
* Broadcast membership is now synchronized across participating
  terminals using the existing persisted `broadcastTargetIds` field as
  the member set. The in-terminal broadcast strip renders
  `[broadcast-icon] [member x] ... [off]`; any participant can remove
  any other, and groups dissolve when fewer than two members remain.
  The broadcast icon now toggles mute for the current tab: membership
  stays intact, but muted tabs neither send broadcast input nor receive
  input from other members.
* Terminal CWD menu rows are now present (`Copy path to CWD`,
  `Show Dir`, `Graph dir`) and route to the specified fallback status
  message, `PTY did not report CWD`, until backend/session metadata
  exposes the live PTY directory.
* Terminal tabs now track the tab name captured at PTY session start.
  Renaming a live terminal surfaces a stale `$CHAN_TAB_NAME` prompt
  with `Restart now` and `Later`; no shell command is injected.
* Ctrl+D after an exited terminal is now handled in xterm's custom
  key-event path, with the tab-shell keydown handler left as a
  fallback. This should make the "press Ctrl+D to close this tab"
  hint actionable even when xterm consumes the keyboard event before
  it bubbles to the Svelte wrapper.
* Terminal CWD execution (`Copy path to CWD`, `Show Dir`, `Graph dir`,
  CWD-seeded `New File`) remains backend/session-metadata dependent.
  The menu rows are present; CWD-dependent rows show the fallback
  status, and `New File` is present but currently falls back to drive
  root.

## Completion notes

Verification:
* `npm run check` in `web` passed.
* `npm test -- --run` in `web` passed: 18 files, 173 tests.
* `npm run build` in `web` passed with existing chunk-size warnings.
