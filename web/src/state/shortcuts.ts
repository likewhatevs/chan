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
///
/// `fullstack-42` (2026-05-19) pruned every chord whose action is
/// now covered by Pane Mode (`Cmd+K` …) so the keymap stops
/// shipping two shortcuts for the same action. Removed at the
/// time:
///
///   app.files.toggle    (was Mod+P)              → Cmd+K 2
///   app.search.toggle   (was Mod+Shift+F)        → Cmd+K f
///                                                  (was `Cmd+K s` post-fullstack-42;
///                                                   moved to `f` per fullstack-74 so
///                                                   WASD can fully own swap-tile)
///   app.graph.toggle    (was Mod+Shift+M)        → Cmd+K 3
///   app.terminal.toggle (was Cmd+Alt+T / Mod+T)  → Cmd+K 1
///   app.file.new        (was Ctrl+Alt+N / Mod+N) → Cmd+K 4
///   app.pane.prev/next  (was Mod+Alt+[ / ])      → Cmd+K ← / →
///
/// `fullstack-b-9` (2026-05-19) brought `app.terminal.toggle`
/// back as a direct chord (Cmd+T native / Cmd+Alt+T web Mac /
/// universal `Mod+. t` via Hybrid NAV).
///
/// `fullstack-a-32` (2026-05-20) re-adds `app.files.toggle`,
/// `app.graph.toggle`, and updates `app.terminal.richPrompt` to
/// the consistent spawn-chord shape:
///
///   app.files.toggle           Cmd+O native / Cmd+Alt+O web Mac / Mod+. o universal
///   app.graph.toggle           Cmd+Shift+M native + web        / Mod+. v universal
///   app.terminal.richPrompt    Cmd+P native / Cmd+Alt+P web Mac / Mod+. p universal
///
/// Inside Hybrid NAV, the numeric `1/2/3/4` cases drop (they
/// duplicated the new mnemonic chords); `t/T`, `o/O`, `p/P`,
/// `v/V` cover the same actions with first-letter mnemonics.
/// `f/F` (Search) and `h/H` (Help) stay. The Cmd+K entry chord
/// itself was already swapped to Cmd+. by `fullstack-a-7`.
///
/// `app.tab.close` was rewired to `Ctrl+D` on both web and native
/// (a different action than Pane Mode's `x` / `k`, per
/// `fullstack-41`); the native `Mod+W` fallback still fires through
/// `KEY_BRIDGE_JS` in `desktop/src-tauri/src/serve.rs`.
export const SHORTCUTS: readonly Shortcut[] = [
  // App-level navigation
  {
    id: "app.settings.toggle",
    label: "Settings",
    web: "Mod+,",
    native: "Mod+,",
    group: "App",
  },
  // `fullstack-a-32`: Rich prompt chord migrates to Mod+P (native)
  // / Cmd+Alt+P (web Mac) so the spawn-chord family (Cmd+T/O/P,
  // Cmd+Shift+M) reads uniformly. The Alt+Space chord stays bound
  // in App.svelte as a secondary alias for muscle memory but is
  // not advertised in the registry to avoid duplicate rows.
  // Universal Hybrid NAV `p` (was added in `fullstack-50`) covers
  // every platform including Win/Linux web where Cmd+P is owned
  // by the browser's print dialog and Cmd+Alt+P isn't a thing.
  {
    id: "app.terminal.richPrompt",
    label: "Terminal rich prompt",
    web: "Cmd+Alt+P",
    native: "Mod+P",
    group: "App",
    note: "macOS web + native everywhere; all platforms via Mod+. p (Hybrid NAV); legacy Alt+Space alias still bound",
  },
  // `fullstack-a-32`: file-browser top-level chord. Same shape as
  // `app.terminal.toggle` — native uses Cmd+O; web fallback is
  // Cmd+Alt+O (browser owns Cmd+O for Open File on Mac). Universal
  // Hybrid NAV `o` is added in this task so every platform has
  // a reachable chord even when Cmd+Alt+O isn't bound on
  // Win/Linux.
  {
    id: "app.files.toggle",
    label: "File browser",
    web: "Cmd+Alt+O",
    native: "Mod+O",
    group: "App",
    note: "macOS web + native everywhere; all platforms via Mod+. o (Hybrid NAV)",
  },
  // `fullstack-a-32`: graph top-level chord. `Cmd+Shift+M` was the
  // pre-`fullstack-42` binding and lands again here, this time
  // wired with context-aware spawn semantics (the focused doc /
  // terminal cwd seeds the graph's scope). Native AND web both
  // use the same chord since browsers don't reserve it. Universal
  // Hybrid NAV `v` covers fallback discoverability.
  {
    id: "app.graph.toggle",
    label: "Graph",
    web: "Mod+Shift+M",
    native: "Mod+Shift+M",
    group: "App",
    note: "or Mod+. v (Hybrid NAV)",
  },
  // `fullstack-b-2`: Cmd+T comes back for "new terminal in active
  // pane" (the action behind Pane Mode's `Cmd+K 1`) as a direct
  // chord. Browsers reserve `Cmd+T` at the OS level so the web
  // variant uses `Cmd+Alt+T` — Mac-only; `Ctrl+Alt+T` on
  // Win/Linux web is already owned by `app.tab.reopenClosed` and
  // we'd rather leave Pane Mode as the fallback than collide.
  //
  // `fullstack-b-9`: `Mod+. t` (Hybrid NAV `t` mnemonic) is the
  // universal chord — works on every web platform including
  // Win/Linux where `Cmd+Alt+T` isn't a thing. Surfaces in the
  // PaneModeHelp cheatsheet as an alias for `1` so the discovery
  // path stays inside the Hybrid NAV overlay.
  {
    id: "app.terminal.toggle",
    label: "New terminal",
    web: "Cmd+Alt+T",
    native: "Mod+T",
    group: "App",
    note: "macOS web + native everywhere; all platforms via Mod+. t (Hybrid NAV)",
  },
  // `fullstack-a-7`: Hybrid NAV chord swapped from Mod+K to
  // Mod+. so Mod+, can own Settings (macOS preferences
  // convention; `app.settings.toggle` above). Mod+. is not
  // browser-reserved on macOS and survives both web + native
  // dispatch through the same chord descriptor. The Flip
  // chord (Mod+. Tab) follows the same swap so the chain
  // stays internally consistent.
  {
    id: "app.pane.mode",
    label: "Enter Hybrid NAV",
    web: "Mod+.",
    native: "Mod+.",
    group: "Panes",
  },
  // `fullstack-a-73`: window-level reload, like a browser Cmd+R.
  // SPA chord routes through `reloadWindow()` (chan-desktop IPC
  // OR `window.location.reload()` on web). chan-desktop's
  // serve.rs:1140 Tauri-side binding stays as defense-in-depth.
  {
    id: "app.window.reload",
    label: "Reload window",
    web: "Mod+R",
    native: "Mod+R",
    group: "App",
  },
  // `fullstack-a-66`: New Draft action — creates a fresh draft
  // dir under chan-drive's metadata-side Drafts folder + opens
  // `draft.md` in the Hybrid Editor. chan-desktop's
  // `-b-27` moved its "New Window" accelerator to Cmd+Shift+N
  // so plain Cmd+N is reserved for this SPA handler.
  {
    id: "app.draft.new",
    label: "New draft",
    web: "Mod+N",
    native: "Mod+N",
    group: "App",
  },
  {
    id: "app.pane.flip",
    label: "Flip Hybrid",
    web: "Mod+. Tab",
    native: "Mod+. Tab",
    group: "Panes",
  },
  {
    id: "ui.overlay.dismiss",
    label: "Dismiss overlay",
    web: "Esc",
    native: "Esc",
    group: "App",
  },
  // `fullstack-56`: dropped `app.save` (Cmd+S) — autosave is the
  // canonical write path (debounced on idle + tab-close + visibility
  // hooks). No File→Save menu item existed, so dropping the keystroke
  // collapses the surface entirely. Cmd+Shift+S strikethrough is
  // owned by the editor and unaffected.
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
