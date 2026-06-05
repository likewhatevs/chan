// Central registry of every user-visible keyboard shortcut.
//
// One source of truth for:
//   1. App.svelte's `onWindowKey` (browser keymap).
//   2. chan-desktop's KEY_BRIDGE_JS (native keymap; rebroadcasts as
//      `chan:command` events that chan handles).
//   3. The empty-pane background table.
//   4. crates/chan/src/main.rs SERVE_LONG_ABOUT (the `chan serve
//      --help` text). Resync via `node web/scripts/shortcuts-table.mjs`.
//
// Chord grammar: a `+`-separated list of modifier tokens followed by
// a single key. Modifier tokens:
//   - `Mod`   → Cmd on macOS, Ctrl on Linux / Windows.
//   - `Cmd`   → literal Command / Meta (used for browser fallbacks
//               where Ctrl would collide with an existing chord).
//   - `Ctrl`  → literal Ctrl (used for the `Ctrl+Alt+...` web fallbacks
//               that mean the actual Control key on every platform).
//   - `Alt`   → Alt / Option.
//   - `Shift` → Shift.
// Keys are written as the user reads them (`P`, `[`, `Enter`, `1..9`).
//
// Per-platform variants: most shortcuts use the same chord across
// web and native, but some differ because the browser owns certain
// chords (Cmd+W, Cmd+N, Cmd+1..9, Cmd+F/G) and we have to fall back
// to Alt+Shift / Ctrl+Alt combos. Native (chan-desktop) intercepts
// the OS-reserved chords in a webview init script and replays the
// same `chan:command` event, so the in-app handler stays unified.

export type Chord = string;

/// The two surfaces chan ships. `web` is the in-browser fallback
/// chord set; `native` is the chord set chan-desktop's init script
/// binds (which layers VS Code-shaped chords on top of the web set).
export type Platform = "web" | "native";

/// OS only affects how `Mod` is rendered (Cmd vs Ctrl). The keymap
/// itself doesn't branch on OS - only the printable label does.
export type OS = "mac" | "linux" | "windows";

export type ShortcutGroup =
  | "App"
  | "File"
  | "Tabs"
  | "Panes"
  | "Find"
  | "Terminal"
  | "Editor";

export type Shortcut = {
  /// Stable command id. Matches `chan:command` event names so a
  /// caller can fire actions without going through the chord layer.
  id: string;
  /// Human-readable description for the table.
  label: string;
  /// Chord on the web fallback set. Omit when the action is not
  /// reachable via a chord in the browser (e.g. Cmd+S - browser
  /// shows the save-page dialog; chan can't preventDefault that on
  /// every browser reliably).
  web?: Chord;
  /// Chord chan-desktop's init script binds. Omit when the action
  /// has no native-specific chord (i.e. native and web share the
  /// same `web` chord).
  native?: Chord;
  group: ShortcutGroup;
  /// Optional trailing parenthetical for the table (e.g. "browser
  /// owns this chord - handled natively").
  note?: string;
  /// When true, `handleTerminalKeyEvent` in `TerminalTab.svelte`
  /// returns `false` for this chord so the event bubbles out of
  /// xterm to the App-level keymap. Default false (xterm consumes
  /// the keystroke as a shell input). Set true on App-group chords
  /// (Settings, TeamWork, Reload, FB toggle, Graph, NewDraft,
  /// Hybrid Nav, etc.) that must reach App.svelte regardless of
  /// terminal focus.
  escapeTerminal?: boolean;
};

/// The complete chord registry. Order in this list is the order the
/// table renders rows within each group.
export const SHORTCUTS: readonly Shortcut[] = [
  // App-level navigation
  //
  // Cmd+, flips the focused pane (Terminal / Editor / Graph /
  // FB / Dashboard) to its back-of-card config surface via
  // `flipHybrid(layout.activePaneId)`. Press again to flip back.
  // The macOS preferences convention motivates this chord.
  {
    id: "app.settings.toggle",
    label: "Flip focused Hybrid",
    web: "Mod+,",
    native: "Mod+,",
    group: "App",
    escapeTerminal: true,
  },
  // Team Work chord: Mod+P native / Cmd+Alt+P web-Mac so the
  // spawn-chord family (Cmd+T/O/P, Cmd+Shift+M) reads uniformly.
  // Hybrid Nav `p` covers Win/Linux web where Cmd+P is the
  // browser's print dialog and Cmd+Alt+P isn't available.
  {
    id: "app.terminal.teamWork",
    label: "Team Work",
    web: "Cmd+Alt+P",
    native: "Mod+P",
    group: "App",
    note: "macOS web + native everywhere; all platforms via Mod+. p (Hybrid Nav)",
    escapeTerminal: true,
  },
  // Broadcast-input toggle for the active terminal (mirrors iTerm).
  // macOS-native ONLY: cmd+shift+i is the browser DevTools chord on
  // the web build, so there is no `web` binding. The native binding
  // is gated on metaKey (= Cmd) so Linux ctrl+shift+i stays DevTools.
  {
    id: "app.terminal.broadcastToggle",
    label: "Toggle broadcast to all terminals",
    native: "Cmd+Shift+I",
    group: "App",
    note: "macOS native only (cmd+shift+i is DevTools on web / Linux)",
    escapeTerminal: true,
  },
  // File-browser top-level chord. Native uses Cmd+O; web fallback
  // is Cmd+Alt+O because the browser owns Cmd+O for Open File on
  // Mac. Hybrid Nav `o` gives every platform a reachable chord
  // even when Cmd+Alt+O isn't bound on Win/Linux.
  {
    id: "app.files.toggle",
    label: "File browser",
    web: "Cmd+Alt+O",
    native: "Mod+O",
    group: "App",
    note: "macOS web + native everywhere; all platforms via Mod+. o (Hybrid Nav)",
    escapeTerminal: true,
  },
  // File-browser destructive delete. Bare Backspace (the Mac "delete"
  // key) or forward-Delete removes the selected entry; the dispatch
  // source is FileTree's `onTreeKeydown`, with the uiConfirm in
  // `fileOps.remove` as the safety gate. Recorded here so the FB
  // selection-menu hint reads the chord from the central store
  // (`chordFor`) and it ports across web/native. No modifier, so it
  // never escapes the terminal (`chordFromEvent` ignores modifierless
  // keys); `escapeTerminal` stays false to keep shell Backspace intact.
  {
    id: "app.files.delete",
    label: "Delete file or directory",
    web: "Backspace",
    native: "Backspace",
    group: "File",
  },
  // Graph top-level chord. Context-aware spawn: the focused doc /
  // terminal cwd seeds the graph's scope. Native and web share
  // the same chord because browsers don't reserve Cmd+Shift+M.
  // Hybrid Nav `v` is the fallback discoverability path.
  {
    id: "app.graph.toggle",
    label: "Graph",
    web: "Mod+Shift+M",
    native: "Mod+Shift+M",
    group: "App",
    note: "or Mod+. v (Hybrid Nav)",
    escapeTerminal: true,
  },
  // New terminal in the active pane as a direct chord. Browsers
  // reserve Cmd+T at the OS level, so the web variant uses
  // Cmd+Alt+T (Mac only). Ctrl+Alt+T on Win/Linux web is owned
  // by `app.tab.reopenClosed`, so Pane Mode is the fallback there.
  // Hybrid Nav `t` is the universal chord on every platform,
  // surfaced in the PaneModeHelp cheatsheet as an alias for `1`.
  {
    id: "app.terminal.toggle",
    label: "New terminal",
    web: "Cmd+Alt+T",
    native: "Mod+T",
    group: "App",
    note: "macOS web + native everywhere; all platforms via Mod+. t (Hybrid Nav)",
    escapeTerminal: true,
  },
  // Mod+. is not browser-reserved on macOS, so it survives both
  // web + native dispatch through the same chord descriptor.
  // Mod+, takes the macOS preferences convention (Settings flip),
  // leaving Mod+. for Hybrid Nav. The Flip chord (Mod+. Tab)
  // pairs with the same prefix for internal consistency.
  {
    id: "app.pane.mode",
    label: "Enter Hybrid Nav",
    web: "Mod+.",
    native: "Mod+.",
    group: "Panes",
    escapeTerminal: true,
  },
  // Window-level reload analogous to a browser Cmd+R. Routes
  // through `reloadWindow()` (chan-desktop IPC or
  // `window.location.reload()` on web). The Tauri-side binding
  // in chan-desktop's serve.rs is defense-in-depth.
  //
  // The stored chord is the macOS form (`Mod+R` -> Cmd+R). On
  // Linux/Windows plain Ctrl+R is the shell's reverse-search, so
  // reload diverges to Ctrl+Shift+R there. That divergence is real
  // (a different chord, not just a different label), so it lives in
  // `osChord` rather than the `Mod` token; the note documents it in
  // the macOS-rendered help table.
  {
    id: "app.window.reload",
    label: "Reload window",
    web: "Mod+R",
    native: "Mod+R",
    group: "App",
    note: "Ctrl+Shift+R on Linux / Windows",
    escapeTerminal: true,
  },
  // New Draft: creates a fresh draft dir under the configured
  // in-workspace Drafts folder (default `.Drafts`) and opens
  // `draft.md` in the Hybrid Editor. chan-desktop's "New Window"
  // accelerator is
  // Cmd+Shift+N, leaving plain Cmd+N for this SPA handler.
  {
    id: "app.draft.new",
    label: "New draft",
    web: "Mod+N",
    native: "Mod+N",
    group: "App",
    escapeTerminal: true,
  },
  // Manual screensaver lock. Routes through `screensaver.svelte::
  // lockNow()` which sets `locked=true`; the App-root
  // `ScreensaverOverlay` covers the SPA. Surfaced only via the
  // Hybrid Nav chain so plain Cmd+L stays free for the browser
  // location bar.
  {
    id: "app.screensaver.lock",
    label: "Lock screen",
    web: "Mod+. L",
    native: "Mod+. L",
    group: "App",
  },
  {
    id: "app.pane.flip",
    label: "Flip Hybrid",
    web: "Mod+. Tab",
    native: "Mod+. Tab",
    group: "Panes",
  },
  // Pane nav splits per platform. Desktop-native keeps Cmd+[/]
  // (no browser chrome to fight). The web build uses Alt+[/]
  // because Cmd+[/] is browser back/forward. Tab nav mirrors
  // this split (web Alt+Shift+[/], native Cmd+Shift+[/]). The
  // web handler matches by `e.code` and preventDefaults the
  // Option-mangled glyph, same as the tab handler.
  {
    id: "app.pane.prev",
    label: "Previous pane",
    web: "Alt+[",
    native: "Mod+[",
    group: "Panes",
    escapeTerminal: true,
  },
  {
    id: "app.pane.next",
    label: "Next pane",
    web: "Alt+]",
    native: "Mod+]",
    group: "Panes",
    escapeTerminal: true,
  },
  // Split-active chords are native-only; web reaches them via
  // Hybrid Nav `/` and `?`. Split-bottom is Cmd+Shift+/ rather
  // than Cmd+\ because 1Password registers Cmd+\ as a system-wide
  // macOS hotkey that the OS dispatches before the keystroke
  // reaches chan's webview. The mnemonic is `/` right, `?` bottom
  // (same physical key with/without Shift). Hybrid Nav mirrors it.
  {
    id: "app.pane.splitRight",
    label: "Split right",
    native: "Mod+/",
    group: "Panes",
  },
  {
    id: "app.pane.splitDown",
    label: "Split bottom",
    native: "Mod+Shift+/",
    group: "Panes",
  },
  {
    id: "app.pane.closeTabs",
    label: "Close all tabs in pane",
    web: "Mod+. x",
    native: "Mod+. x",
    group: "Panes",
  },
  {
    id: "app.pane.kill",
    label: "Kill pane",
    web: "Mod+. Backspace",
    native: "Mod+. Backspace",
    group: "Panes",
  },
  {
    id: "ui.overlay.dismiss",
    label: "Dismiss overlay",
    web: "Esc",
    native: "Esc",
    group: "App",
  },
  // Autosave is the canonical write path (debounced on idle +
  // tab-close + visibility hooks), so Cmd+S is free for
  // workspace-wide search. preventDefault on web suppresses the
  // browser save-page dialog. Distinct from Cmd+Shift+S
  // strikethrough (owned by the editor).
  {
    id: "app.search.toggle",
    label: "Search",
    web: "Mod+S",
    native: "Mod+S",
    group: "App",
    escapeTerminal: true,
  },
  // Dashboard direct chord, OUT of Hybrid Nav (it was the only surface still
  // mixed with it). Native (Tauri webview): Mod+Shift+D (Cmd+Shift+D mac /
  // Ctrl+Shift+D linux), free since there is no browser chrome to fight. Web:
  // Alt+Shift+D, because Cmd/Ctrl+Shift+D is the browser's "bookmark all tabs"
  // which page JS cannot reliably preventDefault (the same web-vs-native split
  // as tab/pane nav). escapeTerminal so the chord fires from a focused
  // terminal. Mod+. i (Hybrid Nav) + the hamburger remain as alternate paths.
  {
    id: "app.dashboard.open",
    label: "Dashboard",
    web: "Alt+Shift+D",
    native: "Mod+Shift+D",
    group: "App",
    note: "or Mod+. i (Hybrid Nav)",
    escapeTerminal: true,
  },
  // Tab navigation
  {
    id: "app.tab.close",
    label: "Close tab",
    web: "Ctrl+D",
    native: "Ctrl+D",
    group: "Tabs",
    note: "Cmd+W also closes the tab on native",
  },
  {
    id: "app.tab.reopenClosed",
    label: "Reopen closed tab",
    web: "Ctrl+Alt+T",
    native: "Mod+Shift+T",
    group: "Tabs",
  },
  {
    id: "app.tab.next",
    label: "Next tab",
    web: "Alt+Shift+]",
    native: "Mod+Shift+]",
    group: "Tabs",
  },
  {
    id: "app.tab.prev",
    label: "Previous tab",
    web: "Alt+Shift+[",
    native: "Mod+Shift+[",
    group: "Tabs",
  },
  {
    id: "app.tab.jump",
    label: "Jump to tab N",
    web: "Ctrl+Alt+1..9",
    native: "Mod+1..9",
    group: "Tabs",
  },
  // Find on page - browser owns Cmd+F/G/Shift+G in the web build.
  {
    id: "app.find.open",
    label: "Find on page",
    native: "Mod+F",
    group: "Find",
    note: "browser's own find dialog on web",
  },
  {
    id: "app.find.next",
    label: "Find next",
    native: "Mod+G",
    group: "Find",
  },
  {
    id: "app.find.prev",
    label: "Find previous",
    native: "Mod+Shift+G",
    group: "Find",
  },
  // Obsidian-style "Show Source Code" toggle. Cmd+E flips the
  // active editor tab between its rendered surface (wysiwyg /
  // pretty / table) and the raw source view. Web Mac has no
  // browser-reserved conflict on Cmd+E, so web and native share
  // Mod+E.
  {
    id: "app.editor.toggleMode",
    label: "Show Source Code (toggle rendered/source)",
    native: "Mod+E",
    web: "Mod+E",
    group: "Editor",
    // escapeTerminal stays FALSE: off macOS `Mod` is Ctrl, and Ctrl+E is
    // readline's move-to-end-of-line, which a focused terminal must keep.
    // Escaping bought nothing here anyway - the toggle targets the active
    // FILE tab, so from a focused terminal pane (no file tab) it is a no-op.
    // The editor reaches the chord directly (it is not a terminal), so the
    // Show Source toggle still fires when a file tab is focused.
    escapeTerminal: false,
  },
  // Bold + Italic are bound in the editor's CM6 keymap
  // (Wysiwyg.svelte -> fmt.toggleBold/Italic). These entries exist
  // for cheatsheet + StyleToolbar tooltip discoverability; the
  // editor keymap is the dispatch source, so no `escapeTerminal`
  // (CM6 keystrokes never route through the terminal escape path).
  {
    id: "app.editor.bold",
    label: "Bold",
    native: "Mod+B",
    web: "Mod+B",
    group: "Editor",
  },
  {
    id: "app.editor.italic",
    label: "Italic",
    native: "Mod+I",
    web: "Mod+I",
    group: "Editor",
  },
  // Terminal copy / paste. The ONE family whose chord can't use the `Mod`
  // token: `Mod+C` resolves to Ctrl+C on Linux / Windows, which is the
  // shell's SIGINT. So macOS uses literal Cmd+C / Cmd+V (Cmd never collides
  // with a control code) and every other platform uses the standard terminal
  // Ctrl+Shift+C / Ctrl+Shift+V, leaving bare Ctrl+C/V for the shell. Handled
  // terminal-locally in TerminalTab (handleTerminalClipboardChord), NOT via
  // the App-level keymap, so no `escapeTerminal` flag (same as Find). The
  // displayed hint is correct on macOS; the `note` documents the Linux /
  // Windows divergence the handler implements.
  {
    id: "terminal.copy",
    label: "Copy selection",
    web: "Cmd+C",
    native: "Cmd+C",
    group: "Terminal",
    note: "Ctrl+Shift+C on Linux / Windows",
  },
  {
    id: "terminal.paste",
    label: "Paste",
    web: "Cmd+V",
    native: "Cmd+V",
    group: "Terminal",
    note: "Ctrl+Shift+V on Linux / Windows",
  },
];

// Editor strikethrough / inline-code chords are not in this registry:
// strike (Cmd+Shift+S) is owned by the editor keymap directly and
// inline code remains a click-only target in StyleToolbar.svelte.
// Bold (Cmd+B) + Italic (Cmd+I) are in the registry above because
// the editor keymap binds them and tooltips need to discover them.

const MOD_LABEL: Record<OS, string> = {
  mac: "Cmd",
  linux: "Ctrl",
  windows: "Ctrl",
};

/// Replace `Mod` with the OS-appropriate label. Leaves explicit
/// `Ctrl` alone so a "Ctrl+Alt+N" chord (the web fallback) stays
/// literal even on macOS.
export function formatChord(chord: Chord, os: OS): string {
  return chord.replaceAll(/\bMod\b/g, MOD_LABEL[os]);
}

/// Shortcuts whose chord diverges by OS, not just by label (macOS keeps the
/// stored chord; Linux / Windows get a DIFFERENT chord because the macOS one
/// collides with a control code there).
const RELOAD_SHORTCUT_ID = "app.window.reload";
const TERMINAL_COPY_ID = "terminal.copy";
const TERMINAL_PASTE_ID = "terminal.paste";

/// Resolve a shortcut's chord for a platform with chan's OS-level chord
/// overrides applied. Most chords differ only by LABEL (`Mod` -> Cmd/Ctrl);
/// a few diverge into a different chord entirely on Linux / Windows:
///   - Reload: Cmd+R (mac) vs Ctrl+Shift+R (plain Ctrl+R is the shell's
///     reverse-search).
///   - Terminal copy / paste: Cmd+C/V (mac) vs Ctrl+Shift+C/V (bare Ctrl+C/V
///     is the shell's SIGINT / EOF). TerminalTab's clipboard handler splits on
///     the same rule at the event layer; this keeps the displayed hint + the
///     help table correct per-OS.
/// The registry stores the macOS form; this function is the ONE place the
/// divergence lives, so the escape matcher, the on-screen labels, and the
/// help table all agree. App.svelte's keymap and chan-desktop's KEY_BRIDGE_JS
/// branch on the same rule (Cmd vs Ctrl+Shift) at the raw-event layer.
export function osChord(
  s: Shortcut,
  platform: Platform,
  os: OS,
): Chord | undefined {
  const chord = s[platform];
  if (!chord) return undefined;
  if (s.id === RELOAD_SHORTCUT_ID && os !== "mac") return "Mod+Shift+R";
  if (s.id === TERMINAL_COPY_ID && os !== "mac") return "Mod+Shift+C";
  if (s.id === TERMINAL_PASTE_ID && os !== "mac") return "Mod+Shift+V";
  return chord;
}

export function currentOS(): OS {
  if (typeof navigator === "undefined") return "linux";
  const ua = navigator.userAgent;
  if (/Mac OS X|Macintosh/.test(ua)) return "mac";
  if (/Windows/.test(ua)) return "windows";
  return "linux";
}

/// Return the formatted chord for a shortcut id on the current
/// platform + OS, or `null` if the shortcut isn't wired (the chord
/// for the resolved platform is undefined). Tooltips and button
/// labels use this to stay in sync with the keymap layer without
/// duplicating chord strings inline.
export function chordFor(id: string): string | null {
  const s = SHORTCUTS.find((x) => x.id === id);
  if (!s) return null;
  const os = currentOS();
  const chord = osChord(s, currentPlatform(), os);
  if (!chord) return null;
  return formatChord(chord, os);
}

/// Derive the platform-resolved chord from a raw `KeyboardEvent`.
/// Used by `handleTerminalKeyEvent` to detect whether the incoming
/// keystroke matches an `escapeTerminal` shortcut and should bubble
/// out of xterm.
///
/// Returns a chord string of the same shape the registry uses
/// (e.g. `"Mod+P"`, `"Cmd+Alt+P"`, `"Ctrl+Alt+1"`). `Mod` is
/// emitted for `metaKey` on macOS + `ctrlKey` on Linux/Windows;
/// `Cmd` is emitted for `metaKey` regardless of platform when
/// `ctrlKey` is also present-or-absent. Keys are normalised to
/// the registry's casing (`P`, `Enter`, `[`, etc.).
///
/// `null` when the event carries no modifier OR the key isn't a
/// recognisable shortcut surface (printable characters typed
/// into the editor don't match anything in the registry).
export function chordFromEvent(e: KeyboardEvent): string | null {
  const parts: string[] = [];
  const os = currentOS();
  // `Mod` semantics: Cmd on macOS, Ctrl elsewhere. Emit `Mod`
  // when the platform-canonical modifier fires; emit `Cmd` /
  // `Ctrl` separately when the *non-platform* form fires (the
  // Cmd+Alt+P web-Mac fallback always uses `Cmd+...`).
  const modIsMeta = os === "mac";
  const hasPlatformMod = modIsMeta ? e.metaKey : e.ctrlKey;
  const hasNonPlatformMeta = modIsMeta ? false : e.metaKey;
  const hasNonPlatformCtrl = modIsMeta ? e.ctrlKey : false;
  if (hasPlatformMod) parts.push("Mod");
  if (hasNonPlatformMeta) parts.push("Cmd");
  if (hasNonPlatformCtrl) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  const key = canonicalKey(e);
  if (!key) return null;
  parts.push(key);
  if (parts.length <= 1) return null;
  return parts.join("+");
}

/// Normalise a `KeyboardEvent.key` to the registry's casing.
/// Letters are uppercased; printable specials map to their
/// registry-form (`,`, `[`, `]`, etc.). Returns `null` for
/// modifier-only events (Shift / Alt / Control / Meta on their
/// own).
function canonicalKey(e: KeyboardEvent): string | null {
  const k = e.key;
  if (!k || k === "Shift" || k === "Alt" || k === "Control" || k === "Meta") {
    return null;
  }
  if (k.length === 1) return k.toUpperCase();
  // Multi-char keys: registry uses the browser's `KeyboardEvent.key`
  // names verbatim (`Enter`, `Tab`, `Escape`, `ArrowLeft`, ...).
  return k;
}

/// Chord-escape lookup. Returns true when the incoming
/// `KeyboardEvent` matches any registry entry flagged
/// `escapeTerminal: true`. `handleTerminalKeyEvent` calls this;
/// on true, returns `false` to xterm so the event bubbles to
/// the App-level keymap.
///
/// Matches BOTH the platform-resolved chord AND the
/// cross-platform `Cmd+` literal alias (the registry's `Mod`
/// expands to Cmd on Mac + Ctrl elsewhere; `Cmd+` is the
/// literal Cmd key used by the web-fallback chords). The
/// matcher normalises both sides to a canonical token set so
/// `Mod+Alt+P` (event) === `Cmd+Alt+P` (registry web Mac
/// fallback) on Mac.
export function shouldEscapeTerminal(e: KeyboardEvent): boolean {
  const chord = chordFromEvent(e);
  if (!chord) return false;
  const eventTokens = canonicalChordTokens(chord);
  const platform = currentPlatform();
  for (const s of SHORTCUTS) {
    if (!s.escapeTerminal) continue;
    const registryChord = osChord(s, platform, currentOS());
    if (!registryChord) continue;
    if (sameChord(eventTokens, canonicalChordTokens(registryChord))) {
      return true;
    }
  }
  return false;
}

/// Normalise a chord string into a Set-shape comparable across
/// the `Mod` / `Cmd` aliasing on Mac. Both `Mod+Alt+P` and
/// `Cmd+Alt+P` collapse to the same key set on Mac; on
/// Linux/Windows they stay distinct (Mod → Ctrl, Cmd → literal
/// Cmd which most keyboards don't have anyway).
function canonicalChordTokens(chord: string): Set<string> {
  const tokens = new Set(chord.split("+"));
  if (currentOS() === "mac" && tokens.has("Cmd")) {
    tokens.delete("Cmd");
    tokens.add("Mod");
  }
  return tokens;
}

function sameChord(a: Set<string>, b: Set<string>): boolean {
  if (a.size !== b.size) return false;
  for (const t of a) if (!b.has(t)) return false;
  return true;
}

/// Tauri injects `window.__TAURI_INTERNALS__` (newer versions) or
/// `window.__TAURI__` (older). Either marker means we're inside the
/// native shell and chan-desktop's init script owns the OS-reserved
/// chords; web fallbacks are inert.
export function currentPlatform(): Platform {
  if (typeof window === "undefined") return "web";
  const w = window as unknown as Record<string, unknown>;
  if (w.__TAURI_INTERNALS__ || w.__TAURI__) return "native";
  return "web";
}

/// Render a plain-ASCII table of shortcuts visible on `platform`,
/// with `Mod` formatted for `os`. Layout: a centered title, then
/// each group as an underlined subheader followed by `label  chord`
/// rows. Column gap auto-derived from the longest label.
///
/// No box-drawing, no Unicode - matches the project's writing rules.
/// Output is intended for the empty-pane background AND the
/// `chan serve --help` text; resync the latter via the
/// `web/scripts/shortcuts-table.mjs` helper.
export function renderTable(platform: Platform, os: OS): string {
  const groups = new Map<ShortcutGroup, Shortcut[]>();
  for (const s of SHORTCUTS) {
    const chord = s[platform];
    if (!chord) continue;
    const list = groups.get(s.group) ?? [];
    list.push(s);
    groups.set(s.group, list);
  }
  // Width = longest label across all visible rows so every group's
  // chord column lines up at the same column.
  let labelW = 0;
  for (const arr of groups.values()) {
    for (const s of arr) {
      if (s.label.length > labelW) labelW = s.label.length;
    }
  }
  const gap = "    ";
  const lines: string[] = [];
  for (const [name, arr] of groups) {
    lines.push(name);
    lines.push("-".repeat(name.length));
    for (const s of arr) {
      const label = s.label.padEnd(labelW);
      const chord = formatChord(osChord(s, platform, os)!, os);
      const suffix = s.note ? `   (${s.note})` : "";
      lines.push(`${label}${gap}${chord}${suffix}`);
    }
    lines.push("");
  }
  // Drop the trailing blank line.
  if (lines[lines.length - 1] === "") lines.pop();
  return lines.join("\n");
}
