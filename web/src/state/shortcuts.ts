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
//   - `Ctrl`  → literal Ctrl (used for the `Ctrl+Alt+…` web fallbacks
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
/// itself doesn't branch on OS — only the printable label does.
export type OS = "mac" | "linux" | "windows";

export type ShortcutGroup = "App" | "File" | "Tabs" | "Panes" | "Find";

export type Shortcut = {
  /// Stable command id. Matches `chan:command` event names so a
  /// caller can fire actions without going through the chord layer.
  id: string;
  /// Human-readable description for the table.
  label: string;
  /// Chord on the web fallback set. Omit when the action is not
  /// reachable via a chord in the browser (e.g. Cmd+S — browser
  /// shows the save-page dialog; chan can't preventDefault that on
  /// every browser reliably).
  web?: Chord;
  /// Chord chan-desktop's init script binds. Omit when the action
  /// has no native-specific chord (i.e. native and web share the
  /// same `web` chord).
  native?: Chord;
  group: ShortcutGroup;
  /// Optional trailing parenthetical for the table (e.g. "browser
  /// owns this chord — handled natively").
  note?: string;
};

/// The complete chord registry. Order in this list is the order the
/// table renders rows within each group.
export const SHORTCUTS: readonly Shortcut[] = [
  // App-level navigation
  {
    id: "app.settings.toggle",
    label: "Settings",
    web: "Mod+,",
    native: "Mod+,",
    group: "App",
  },
  {
    id: "app.files.toggle",
    label: "Files",
    web: "Mod+P",
    native: "Mod+P",
    group: "App",
  },
  {
    id: "app.search.toggle",
    label: "Search across files",
    web: "Mod+Shift+F",
    native: "Mod+Shift+F",
    group: "App",
  },
  {
    id: "app.graph.toggle",
    label: "Graph",
    web: "Mod+Shift+M",
    native: "Mod+Shift+M",
    group: "App",
  },
  {
    id: "app.terminal.toggle",
    label: "Terminal",
    web: "Mod+`",
    native: "Mod+`",
    group: "App",
  },
  {
    id: "app.terminal.broadcast.toggle",
    label: "Terminal broadcast",
    web: "Mod+Shift+I",
    native: "Mod+Shift+I",
    group: "App",
  },
  {
    id: "app.terminal.richPrompt",
    label: "Terminal rich prompt",
    web: "Alt+Space",
    native: "Alt+Space",
    group: "App",
  },
  {
    id: "ui.overlay.dismiss",
    label: "Dismiss overlay",
    web: "Esc",
    native: "Esc",
    group: "App",
  },
  // File / save / create
  {
    id: "app.save",
    label: "Save",
    web: "Mod+S",
    native: "Mod+S",
    group: "File",
  },
  {
    id: "app.file.new",
    label: "New file",
    web: "Ctrl+Alt+N",
    native: "Mod+N",
    group: "File",
  },
  // Tab navigation
  {
    id: "app.tab.close",
    label: "Close tab",
    native: "Mod+W",
    group: "Tabs",
    note: "browser closes its own tab on Mod+W",
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
  // Pane navigation
  {
    id: "app.pane.prev",
    label: "Previous pane",
    web: "Mod+Alt+[",
    native: "Mod+[",
    group: "Panes",
  },
  {
    id: "app.pane.next",
    label: "Next pane",
    web: "Mod+Alt+]",
    native: "Mod+]",
    group: "Panes",
  },
  // Find on page — browser owns Cmd+F/G/Shift+G in the web build.
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
];

// Editor formatting chords (bold / italic / strike / inline code)
// are NOT wired through any keymap today — they only exist as click
// targets in StyleToolbar.svelte. Add them back to this registry
// once the editor's keymap layer binds them.

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
  const chord = s[currentPlatform()];
  if (!chord) return null;
  return formatChord(chord, currentOS());
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
/// No box-drawing, no Unicode — matches the project's writing rules.
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
      const chord = formatChord(s[platform]!, os);
      const suffix = s.note ? `   (${s.note})` : "";
      lines.push(`${label}${gap}${chord}${suffix}`);
    }
    lines.push("");
  }
  // Drop the trailing blank line.
  if (lines[lines.length - 1] === "") lines.pop();
  return lines.join("\n");
}
