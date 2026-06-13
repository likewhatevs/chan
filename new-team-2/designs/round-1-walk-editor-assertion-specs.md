# Round-close WKWebView walk — @@Editor assertion specs (items 1/4 + item-2 SPA)

Author: @@Editor (task-Conductor-Editor-32). Consumer: joint walk
session with @@Desktop (task-Conductor-Desktop-33). Build: b82a0a27,
binary sha 58b6d195 — provenance check before any assertion
(@@Desktop owns).

## Driver-capability split — read first

The honest-split rule turns on ONE capability question for @@Desktop:
does the harness have NATIVE input (real OS clicks/keys, e.g.
cliclick/CGEvent), or only in-page synthetic `dispatchEvent`?

Synthetic events FIRE LISTENERS but SKIP BROWSER DEFAULT ACTIONS.
Item 4's bug IS a default action (mousedown's focus-the-tabindex-div
beats the pulse microtask). With synthetic clicks the steal never
happens, so the assertion would pass WITHOUT the fix — a vacuous
pass, worse than none. Hence:

- Native input available → A4 runs instrumented.
- Synthetic only → A4 is [hand-smoke] with that one-line reason;
  everything in A1/I2 below is still honestly assertable (none of it
  depends on a default action — chord handlers, CM6 keymaps, and
  Svelte handlers all fire on synthetic events).

Keyboard chords (Cmd+. / Cmd+, / Cmd+Z / Cmd+Enter) go through app
window-keydown listeners and CM6 keymaps → synthetic
`new KeyboardEvent("keydown", {key, metaKey, bubbles:true})` on the
right target is sufficient. NOTE from my Chrome pass: Cmd+N is
browser-reserved in Chrome but is a Tauri menu accelerator on the
desktop — @@Desktop should fire New Draft via the menu/emit, not a
synthetic key, if A1.6 runs.

## Fixture

Throwaway workspace (isolated $HOME side, @@Desktop owns), seeded
with two long docs (~1200 lines, decoration-bearing). Generator (run
once into the workspace dir):

    python3 - <<'EOF'
    lines = ["# Walk doc A", ""]
    for i in range(1, 181):
        lines += [f"## Section {i}", "",
          f"Para {i} with **bold**, `code`, [link](https://example.com/{i}). #tag{i % 7}",
          "", "- item one", "- item two", ""]
    open("walk-doc-a.md", "w").write("\n".join(lines))
    open("walk-doc-b.md", "w").write("\n".join(lines).replace("Section", "Chapter").replace("doc A", "doc B"))
    EOF

Pane setup for A1/A4: one pane, tabs = [Terminal-1, walk-doc-a.md,
walk-doc-b.md]. Item-2 setup adds the busy-loop terminal (below).

## A4 — item 4: tab-click focus chain  [NATIVE INPUT ONLY]

- A4.1 native click on the TERMINAL tab header →
  poll ≤200ms: `document.activeElement.className` contains
  `xterm-helper-textarea`. Then native-type `echo ok\n` → readback
  the terminal scrollback contains `ok` (proves keystrokes flow).
- A4.2 native click on the walk-doc-a tab header → poll ≤200ms:
  activeElement has class `cm-content` AND
  `document.querySelector('.editor-tab.active').contains(document.activeElement)`.
- A4.3 (rich-prompt guard) with the Rich Prompt bubble OPEN over the
  active terminal: native click the terminal's own tab header →
  activeElement stays INSIDE the bubble (the TerminalTab guard).
- Synthetic-only fallback: all three → [hand-smoke: synthetic
  dispatchEvent skips the mousedown default action that causes the
  bug; a synthetic pass is vacuous].

## A1 — item 1: keep-alive (all synthetic-safe)

A1.1 THE repro (raw flash + scroll reset) — the round's headline:
  1. Activate walk-doc-a (synthetic mousedown on its tab header is
     fine — the handler sets activeTabId; focus is not asserted
     here). Set `s = document.querySelector('.editor-tab.active
     .cm-scroller'); s.scrollTop = 3000;` record `pre = s.scrollTop`
     (CM6 may clamp; use the readback, not 3000).
  2. Activate walk-doc-b (assert its host visible, doc-a host
     `visibility: hidden`, BOTH hosts in DOM:
     `document.querySelectorAll('.editor-tab').length === 2`).
  3. Activate walk-doc-a again. IMMEDIATELY (same tick, then one
     rAF later — both readbacks):
     - `s.scrollTop === pre` (±1px) at BOTH readbacks (scroll reset
       was the WKWebView symptom).
     - RAW-FLASH probe at both readbacks: over the VISIBLE
       `.cm-line`s (getBoundingClientRect within viewport), text
       must NOT contain `**bold**` or `[link](` (decorations
       replace them when the walker has run); decoration-node count
       in the visible region > 50
       (`s.querySelectorAll('[class*="deco"], .cm-widgetBuffer,
       [class*="pill"]').length`). On pre-fix builds the raw text
       sat visible until a click — one-frame readback catches it.
  4. Repeat step 3's readbacks after a 500ms settle (catches a
     late decoration drop).

A1.2 Hybrid Nav + flip cycles (synthetic window keydown):
  - Cmd+. → assert `.pane-mode-preview` present and every
    `.editor-tab` computed `visibility: hidden`; Escape → active
    host visible again, scrollTop still `pre`.
  - Cmd+, (flip) → `.back-side` present, hosts hidden; Cmd+, again
    → host visible, scrollTop still `pre`, raw-flash probe clean.

A1.3 undo/edit survival ACROSS a switch (CM6 keymap, synthetic):
  - In doc-a type marker text (synthetic insertion via keydown
    sequence or execCommand-equivalent: simplest honest form is
    synthetic keydowns for 3 chars into the focused cm-content; if
    focus can't be established synthetically, drive insertion via
    a native type when available, else mark this sub-item
    hand-smoke). Switch b → a, then Cmd+Z → marker gone, doc head
    intact; further Cmd+Z ×5 → doc NEVER empties (bb877a87's
    boundary holds on WKWebView too:
    `s.textContent.length > 1000` after spam).

A1.4 session-restore caret-lands-once (THE check Chrome could not do
  — the desktop window HAS OS focus):
  - State: 3+ tabs (terminal + 2 docs), doc-a active with caret
    mid-doc. Reload via the app's own reload path (Cmd+R window
    reload). After restore settles (poll: both editor hosts present
    + active host visible + content non-empty), assert:
    - `document.hasFocus() === true` (else the whole assertion is
      void — record and fall back to hand-smoke),
    - exactly ONE `.cm-editor.cm-focused` in the DOM,
    - it is inside `.editor-tab.active`,
    - `document.activeElement` class contains `cm-content`.
  - 2-pane variant if the harness can split panes (Cmd+/): caret
    lands in the ACTIVE pane's active tab only.

A1.5 ~20-tab memory sanity: harness-side process RSS readback if
  cheap (@@Desktop call), else [hand-smoke: Activity Monitor].

A1.6 new-draft caret (positive form of the Chrome-untestable check):
  trigger New Draft via the app menu (not synthetic Cmd+N) → poll
  ≤1s: activeElement is cm-content inside the new draft's host
  (the autoFocus={focused} + restore-focus path, real-window form).

A1.7 tab DnD reorder + cross-pane drag: native drag only —
  synthetic mouse sequences cannot start HTML5 dnd. Native available
  → drag doc-a header past doc-b: assert tab order swapped in DOM
  and no focus loss; else [hand-smoke: HTML5 dragstart needs real
  input].

A1.8 OS-file drop allowlist: real OS drag required; if @@Desktop's
  phase-23 file-drop instrumentation can synthesize the Tauri-side
  drop events honestly, their call to run it; else [hand-smoke].

## I2 — item-2 SPA states (task-PromptQueue-Conductor-28 list)

Server-side actions (busy loop, cs writes, kill serve, second
window) are @@Desktop-side shell steps; SPA readbacks below are
mine. Busy agent = `while true; do date; sleep 0.3; done` in
Terminal-1 (sub-800ms gaps hold the quiet gate).

- I2.1 busy submit: open Rich Prompt, insert text, submit
  (Cmd+Enter synthetic into the bubble editor — CM6 keymap, fires).
  Assert: bubble text STILL VISIBLE + editor read-only
  (`.cm-content[contenteditable="false"]` or the bubble's readonly
  class), "queued" chip appears within 300ms ±200, tab-strip
  `.queue-pill` shows "1".
- I2.2 `cs terminal write` ×3 (desktop side) → poll pill text
  2 → 3 → 4 after each.
- I2.3 drain: Ctrl-C the loop (native key or `cs terminal write`
  $'\x03' raw — @@Desktop's call). Poll: composer clears EXACTLY
  when its message appears in scrollback (two readbacks bracketing
  the print; generous 5s window), pill counts down to 0/absent.
- I2.4 reload mid-pending: re-queue (I2.1), Cmd+R, after restore:
  draft text restored, editor WRITABLE (contenteditable true), pill
  re-synced to server depth.
- I2.5 idle fast path: with the loop stopped, submit → assert the
  queued chip NEVER appears across 1s of rAF-polling (no flash) and
  the composer clears.
- I2.6 rejected-at-cap: needs ~100 queued writes; if the harness
  scripts the loop cheaply, assert keep-text + "queue full" note;
  else [hand-smoke: cap fill is operationally noisy].
- I2.7 hide/reshow (flags 2/6): hide bubble mid-pending (Cmd+Shift+P
  synthetic), resolve while hidden (drain), reshow → composer +
  draft cleared. Variant: deliver-while-hidden then KILL SERVE
  (desktop side) then reshow → failure note shown, no zombie
  pending state.
- I2.8 second window (desktop side opens it): pill ticks in window
  2 while composer in window 2 stays unlocked.
- I2.9 flipped pill non-mirror (the claim I reviewed): flip the
  pane (Cmd+,) with a non-zero pill → assert
  `getComputedStyle(document.querySelector('.tabs.flipped .tab
  .queue-pill')).transform` is `matrix(-1, 0, 0, 1, 0, 0)` (the
  counter-mirror applied; with the face's own mirror this is what
  renders the digit upright).

## Cross-cutting console watch (whole walk)

At session start, driver installs:
  `window.__errs = []; window.addEventListener('error', e =>
  __errs.push(String(e.message))); const ow = console.warn, oe =
  console.error; console.warn = (...a) => { __errs.push(a.join(' '));
  ow(...a) }; console.error = (...a) => { __errs.push(a.join(' '));
  oe(...a) };`
At end: dump `__errs`; FAIL the sweep on any `state_unsafe_mutation`
or uncaught error; `ownership_invalid_mutation` advisories are
pre-existing dev-mode noise — record count, don't fail (and they
should be absent entirely in a release build).

## Session protocol (peer-to-peer)

One driver at a time per task-33. Proposed order: @@Desktop runs
provenance + fixture + harness bring-up, then A1 block, A4 block (or
its hand-smoke marks), I2 block (needs their shell steps
interleaved), console dump, teardown. I'm on the bus for live spec
amendments; ambiguities resolve in favor of [hand-smoke] over a
forced assertion. Results: @@Desktop writes the completion table,
I co-sign with my own report through @@Conductor.

---

## ADDENDUM (post task-Desktop-Editor-34) — resolved against the harness contract

Appended by @@Editor after @@Desktop's capability contract. This
section RESOLVES the open switches above; where it conflicts with the
body, the addendum wins. Net: every line is now either
driver-translatable 1:1 or explicitly [hand-smoke].

### Capability resolution

- Harness is SYNTHETIC-ONLY → **A4.1/A4.2/A4.3 = [hand-smoke]**,
  reason as specced (synthetic dispatchEvent skips the mousedown
  default action that IS the item-4 bug; a synthetic pass is
  vacuous). 30-second human script for @@Alex's list: click terminal
  tab → type immediately, click doc tab → type, click terminal tab
  with Rich Prompt open → caret stays in bubble.
- **A1.7 (DnD) + A1.8 (OS drop) = [hand-smoke]** per your stated
  limits (no synthetic dragstart in WKWebView; OS-level drops).
- **A1.5 (memory)**: optional shell-side automation — if the
  text-input contract below works, open the 20 pre-seeded docs via
  Cmd+S + insertText + Enter per doc, then read the app process RSS
  shell-side (`ps -o rss= -p <pid>`) before/after; PASS = no runaway
  (< ~150MB delta is fine, judgement call). If text input is flaky:
  [hand-smoke: Activity Monitor].

### Text-input contract (fixes a real gap in the body spec)

Synthetic keydown fires KEYMAP commands (chords, Enter, Backspace)
but does NOT insert characters (real text goes through the browser
editing pipeline CM6 observes). So:

1. PRIMARY: focus the target (`el.focus()` method call on
   `.cm-content` — method calls are fine in-page), then
   `document.execCommand("insertText", false, "<text>")` — WebKit
   routes it through beforeinput/input, which CM6 handles. Probe it
   ONCE at harness bring-up (insert + read back + Cmd+Z) and report
   which branch the walk used.
2. FALLBACK for A1.3 only: synthetic Enter on the focused doc
   (keymap insertNewline = a real undoable edit). Marker = doc
   line-count delta instead of text: record `lineCount`, Enter ×2,
   assert +2, switch b→a, Cmd+Z ×2 → back to `lineCount`, Cmd+Z ×5
   more → doc NEVER empties (`textContent.length > 1000`).
3. The Rich Prompt composer CANNOT use the newline fallback:
   `RichPrompt.svelte:257` trim-guards the submit, whitespace never
   sends. If execCommand fails → I2.1/I2.4's composer-content
   sub-assertions (text-stays, read-only flip, draft-restored) =
   [hand-smoke: no honest synthetic text path into the composer],
   while their pill/depth sub-assertions STAY automated via your
   `cs terminal write` feeds (no composer needed).

### Other resolutions

- **A1.6 (new draft)**: no Tauri menu needed — drive the PANE
  HAMBURGER in the DOM: synthetic click on the pane `.actions`
  hamburger trigger, then the "New Draft" `role="menuitem"`. Poll
  ≤1s: activeElement has class `cm-content` inside the NEW draft's
  `.editor-tab.active` (and exactly one `.cm-editor.cm-focused`).
- **A1.4 (restore)**: reload via synthetic Cmd+R (the app chord —
  pinned by cmdRWindowReload.test.ts). Your injected driver rides
  web/dist so it survives the reload; correlate via your report
  timestamps. Keep the `document.hasFocus()` pre-assertion — if the
  walk window somehow lacks OS focus, record and fall back to
  hand-smoke rather than failing.
- **A1.4 two-pane variant**: optional, synthetic Cmd+/ (split
  right) then re-run the restore assertion (caret in ACTIVE pane's
  active tab only).
- **I2.3 (Ctrl-C the busy loop)**: prefer your shell side —
  `cs terminal write` raw `$'\x03'` through the control socket (real
  PTY path) over synthetic keydown into xterm.
- **Console watch**: your onerror + unhandledrejection +
  console.error hooks DO catch the state_unsafe_mutation class
  (Svelte 5 throws it). Please add a console.warn hook too (1 line)
  — belt-and-braces; the prod bundle should emit ~zero warns, so
  anything captured is signal.
- Everything else in the A1/I2 body (raw-flash probe readbacks,
  scrollTop, visibility flips, host counts, pill counts, flipped
  transform matrix, settle timings) maps 1:1 onto your driver
  capability list #1 — no changes.

### Hand-smoke ledger (for the report table)

A4.1 A4.2 A4.3 (default-action), A1.7 (dragstart), A1.8 (OS drop),
A1.5 (unless RSS path taken), I2.6 (cap fill, unless you script ~100
writes cheaply), I2.1/I2.4 composer-content sub-assertions (only if
execCommand probe fails).
