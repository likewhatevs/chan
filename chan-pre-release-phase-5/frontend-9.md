# @@Frontend task 9: Alt-key word navigation in the embedded terminal

Owner: @@Frontend
Status: REVIEW
Severity: MEDIUM — usability gap on every terminal tab; readline-
style word motions are muscle memory for shell users.
Source: Alex's request 2026-05-17, with iTerm2 reference settings
attached.

## What's broken today

In the embedded terminal (`web/src/components/TerminalTab.svelte`,
xterm.js running in the browser):

* `Alt+←` and `Alt+→` print raw escape sequences instead of moving
  the cursor by word.
* `Alt+Backspace` does not delete the previous word.
* `Alt+Delete` / `Alt+d` does not delete the next word.

The shell (bash / zsh / fish) is fine — it expects readline's
M-prefix sequences (`\x1b b`, `\x1b f`, `\x1b\x7f`, `\x1b d`). The
gap is in xterm.js's default keyboard translation: it doesn't
emit those byte sequences when the user presses `Alt+letter` or
`Alt+arrow` on macOS, so the shell receives bytes it doesn't
recognise and prints them.

## iTerm2 reference (from Alex's screenshots)

* Keys → General:
  * "Treat ⌥ as Alt for special keys like arrows" ✓.
  * Left Option (⌥) key: `Esc+`. Right Option (⌥) key: `Esc+`.
  * "Apps can change how keys are reported" ✓.
* Profile → Keys → Key Bindings (Default profile):
  * `⌥←` sends `^[ b` (Esc + b).
  * `⌥→` sends `^[ f` (Esc + f).

These two settings together make Alt act as Meta for normal
letters (`Esc+letter`) and explicitly remap Alt+Arrow to the
readline word-motion sequences. The chan terminal needs the same
contract.

## Fix shape

### 1. xterm.js Terminal options

In the TerminalTab constructor opts, add:

```ts
new Terminal({
  // ...existing opts...
  macOptionIsMeta: true,
  // optional: macOptionClickForcesSelection: false, // unrelated, leave default
});
```

`macOptionIsMeta: true` makes `Alt+letter` emit `Esc letter`,
which is exactly the readline M-prefix. This alone fixes
`Alt+b`, `Alt+f`, `Alt+d`, `Alt+.`, and every other M-key
binding readline / zsh-line-editor understands.

### 2. Custom key handler for Alt+Arrow and Alt+Backspace

xterm.js doesn't translate `Alt+ArrowLeft / ArrowRight / Backspace`
to the readline sequences via `macOptionIsMeta`, so attach a
custom handler:

```ts
term.attachCustomKeyEventHandler((ev: KeyboardEvent) => {
  if (ev.type !== "keydown") return true;
  if (!ev.altKey || ev.ctrlKey || ev.metaKey) return true;
  let bytes: string | null = null;
  switch (ev.key) {
    case "ArrowLeft":  bytes = "\x1bb";   break;
    case "ArrowRight": bytes = "\x1bf";   break;
    case "Backspace":  bytes = "\x1b\x7f"; break;
    case "Delete":     bytes = "\x1bd";   break;
  }
  if (bytes !== null) {
    pty.send(bytes);   // whatever send-to-WS helper TerminalTab uses
    ev.preventDefault();
    return false;      // tell xterm not to also process the key
  }
  return true;
});
```

Return `false` to swallow the event from xterm's default
handler. Return `true` for any key we don't care about so the
normal xterm path continues to work.

`pty.send` here is shorthand for whichever helper the file
already uses to push bytes onto the PTY WebSocket; reuse the
existing one, don't introduce a parallel channel.

### 3. Order of operations inside the file

Set `macOptionIsMeta` in the Terminal constructor opts (one-line
change). Attach the custom key handler **after** the existing
xterm initialisation but **before** the WebSocket connects (so
the first user keypress already gets the right handler).

## Configurability (out of scope for this task)

Alex asked what should be configurable. Recommendation: **leave
this hard-coded as the default for Phase 5**. Almost every shell
user on macOS wants Option-as-Meta + readline word motions; iTerm2
ships these as a one-click preset because that's the obvious
contract. If a user with an unusual setup ever needs the raw
behaviour, expose two config knobs in a later phase:

* `terminal.option_as_meta: bool` (default `true`) — flips
  `macOptionIsMeta`.
* `terminal.alt_word_motions: bool` (default `true`) — toggles
  the custom key handler.

Don't add config plumbing in this task. Note the future shape in
a comment near the handler so the next reader knows where it
goes.

## Acceptance criteria

In a fresh terminal tab against bash or zsh on macOS:

* Type `the quick brown fox`. Press `Alt+←` four times: cursor
  walks back word by word to before "the".
* From the same state press `Alt+→` four times: cursor walks
  forward to after "fox".
* From "the quick brown fo|x" (cursor between o and x), press
  `Alt+Backspace`: word "fo" disappears, line is "the quick
  brown |x".
* From "the |quick brown fox", press `Alt+Delete` (or `Alt+d`):
  word "quick " disappears.
* `Alt+letter` chords that readline binds (`Alt+.` for last
  argument, `Alt+r` for revert-line, etc.) work too.
* No regression to the existing `Mod+Shift+I` broadcast-input
  toggle, `Mod+[` / `Mod+]` pane chords, or the
  `CHAN_TAB_NAME` env. Those go through the desktop key bridge
  / app-level shortcuts, not xterm.js's key path.

Cross-platform note: on Linux/Windows, `macOptionIsMeta` is a
no-op (xterm.js gates it to macOS in `Terminal._handleMacAlt` or
similar). The custom-handler branch fires regardless of platform
on `altKey + arrow/backspace/delete`, which is the right
behaviour for Linux too (Alt = Meta there as well).

## Verification

* `npm --prefix web run check`.
* `npm --prefix web test -- --run`.
* `npm --prefix web run build`.
* Live test in a real shell: the four acceptance criteria above.

## Test expectations

* A small unit test in `web/src/components/TerminalTab.test.ts`
  (or wherever TerminalTab-adjacent tests live; create the file
  if absent) covering the custom-key-handler matrix: assert that
  pressing Alt+ArrowLeft yields the byte sequence `"\x1bb"` etc.
  Mock `pty.send` for the assertion.

## Hardening

* @@Systacean review the byte sequences against `readline(3)`
  defaults and confirm there's nothing platform-specific we're
  missing (e.g. zsh's `bindkey` defaults vs bash's readline;
  the M-prefix path is identical on both).
* Confirm `attachCustomKeyEventHandler` returning `false`
  actually prevents xterm's default emission. Some earlier
  xterm.js versions had a subtle bug where the handler ran but
  the default path still fired for arrow keys; verify against
  the version pinned in `web/package.json` and bump if needed.

## Coordination

* Owner is @@Frontend; @@Architect captured the architectural
  read-out above in the role of @@Systacean-advise.
* @@Systacean: please post-review the diff for any byte-sequence
  surprise (zsh emacs-mode vs vi-mode, fish equivalents).
* @@Webtest A picks up live verification on the same
  `chan-test-phase5` drive after the next bundle rebuild.

## Out of scope

* Native macOS Cmd-shortcut overrides (Cmd+Left = beginning-of-
  line, Cmd+Right = end-of-line). iTerm2 has these too; they're
  a separate set and easy to add later if Alex misses them.
* Configurability surfaces (see "Configurability" section above).
* Anything below the WebSocket — chan-server's PTY plumbing is
  unaffected.

## Progress

* 2026-05-17 @@Frontend started after the coordination poke.
* Enabled xterm.js `macOptionIsMeta` in `TerminalTab.svelte` so
  ordinary `Alt+letter` chords emit readline Meta-prefix input.
* Added a focused terminal keymap helper for `Alt+ArrowLeft`,
  `Alt+ArrowRight`, `Alt+Backspace`, and `Alt+Delete`, wired through
  `attachCustomKeyEventHandler` before the terminal websocket connects.
  The handler uses the normal user-input path, so broadcast-input mode
  sees the translated Meta sequences too.
* Added unit coverage for the readline byte matrix and the xterm
  swallow/pass-through return contract.
* 2026-05-17 @@Systacean post-review: byte sequences match the
  readline/zle/fish Meta-prefix contract:
  `Alt+ArrowLeft -> ESC b`, `Alt+ArrowRight -> ESC f`,
  `Alt+Backspace -> ESC DEL`, `Alt+Delete -> ESC d`. The handler
  correctly returns `false` for mapped keys to suppress xterm's
  default emission and passes non-target Alt-letter chords through
  to `macOptionIsMeta`.

## Completion notes

Diff locations:

* `web/src/components/TerminalTab.svelte`
* `web/src/terminal/keymap.ts`
* `web/src/terminal/keymap.test.ts`

Test matrix:

* `Alt+ArrowLeft` -> `\x1bb`
* `Alt+ArrowRight` -> `\x1bf`
* `Alt+Backspace` -> `\x1b\x7f`
* `Alt+Delete` -> `\x1bd`
* Non-target `Alt+letter`, Ctrl/Meta-modified chords, bare arrows,
  and keyup events pass through to xterm.

Verification:

* `npm --prefix web run check`
* `npm --prefix web test -- --run` (17 files / 155 tests)
* `npm --prefix web run build` (existing Vite chunk/dynamic-import
  warnings only)

Live shell verification remains for @@Webtest A on the next rebuilt
bundle.
