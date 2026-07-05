import { describe, expect, test } from "vitest";
import { renderTable } from "./shortcuts";

describe("shortcut table", () => {
  test("advertises Command launcher as Ctrl+Alt+K on web and Cmd+K on native mac", () => {
    expect(renderTable("web", "mac")).toMatch(/^Command launcher\s+Ctrl\+Alt\+K/m);
    expect(renderTable("native", "mac")).toMatch(/^Command launcher\s+Cmd\+K/m);
  });

  // "New terminal" is a direct chord so power users can spawn a
  // terminal without entering Pane Mode. Cmd+T on native;
  // Cmd+Alt+T on web-Mac (Ctrl+Alt+T on Win/Linux web is owned
  // by tab.reopenClosed).
  test("advertises New terminal under Cmd+Alt+T (web mac)", () => {
    const table = renderTable("web", "mac");
    expect(table).toMatch(/^New terminal +Cmd\+Alt\+T\b/m);
  });

  test("advertises New terminal under Cmd+T (native mac)", () => {
    const table = renderTable("native", "mac");
    expect(table).toMatch(/^New terminal +Cmd\+T\b/m);
  });

  // The sibling label `Team Work` must not match the bare-Terminal
  // regex; these guards anchor to that exact word to catch an
  // accidental rename or duplicate entry.
  test("does not advertise a bare 'Terminal' row (web)", () => {
    const table = renderTable("web", "mac");
    expect(table).not.toMatch(/^Terminal +Cmd\+Alt\+T\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+`\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+T\b/m);
  });

  test("does not advertise a bare 'Terminal' row (native)", () => {
    const table = renderTable("native", "mac");
    expect(table).not.toMatch(/^Terminal +Cmd\+T\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+Alt\+T\b/m);
  });

  test("advertises Hybrid Nav (Cmd+.) as the canonical spawn surface", () => {
    // Mod+. avoids the browser-reserved Mod+, (Settings) and is
    // not reserved by any browser, surviving both web + native.
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Enter Hybrid Nav\s+Cmd\+\.$/m);
    expect(native).toMatch(/^Enter Hybrid Nav\s+Cmd\+\.$/m);
  });

  test("close-tab chord is Ctrl+D on both platforms", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Close tab\s+Ctrl\+D/m);
    expect(native).toMatch(/^Close tab\s+Ctrl\+D/m);
  });

  // Pane nav splits per platform: web uses Alt+[/] because Cmd+[/]
  // is browser back/forward; desktop-native keeps Cmd+[/].
  test("advertises pane nav: web Alt+[/], native Cmd+[/]", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Previous pane\s+Alt\+\[/m);
    expect(web).toMatch(/^Next pane\s+Alt\+\]/m);
    expect(native).toMatch(/^Previous pane\s+Cmd\+\[/m);
    expect(native).toMatch(/^Next pane\s+Cmd\+\]/m);
  });

  // Cmd+S = workspace-wide search; autosave is the canonical write
  // path so this chord is free.
  test("advertises Cmd+S search on both platforms", () => {
    expect(renderTable("web", "mac")).toMatch(/^Search\s+Cmd\+S/m);
    expect(renderTable("native", "mac")).toMatch(/^Search\s+Cmd\+S/m);
  });

  // Dashboard now has a direct chord, out of Hybrid Nav: native Cmd+Shift+D
  // (free in the Tauri webview), web Alt+Shift+D (Cmd+Shift+D is the
  // browser's bookmark-all on web). Mod+. i stays as an alternate path.
  test("advertises Dashboard direct chord (native Cmd+Shift+D, web Alt+Shift+D)", () => {
    expect(renderTable("web", "mac")).toMatch(/^Dashboard\s+Alt\+Shift\+D/m);
    expect(renderTable("native", "mac")).toMatch(/^Dashboard\s+Cmd\+Shift\+D/m);
  });

  test("advertises editor Bold (Cmd+B) and Italic (Cmd+I)", () => {
    expect(renderTable("web", "mac")).toMatch(/^Bold\s+Cmd\+B/m);
    expect(renderTable("web", "mac")).toMatch(/^Italic\s+Cmd\+I/m);
    expect(renderTable("native", "mac")).toMatch(/^Bold\s+Cmd\+B/m);
    expect(renderTable("native", "mac")).toMatch(/^Italic\s+Cmd\+I/m);
  });

  // Web uses Ctrl+Alt+/ because Ctrl+/ is claimed by terminal/editor surfaces.
  // Native split-bottom remains Cmd+Shift+/ because 1Password registers Cmd+\
  // as a system-wide macOS hotkey before chan's webview receives it.
  test("advertises split chords on web and native", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Split right\s+Ctrl\+Alt\+\//m);
    expect(web).toMatch(/^Split bottom\s+Ctrl\+Alt\+\?/m);
    expect(native).toMatch(/^Split right\s+Cmd\+\//m);
    expect(native).toMatch(/^Split bottom\s+Cmd\+Shift\+\//m);
  });

  test("advertises Hybrid Nav close-all and kill-pane chords", () => {
    const table = renderTable("web", "mac");
    expect(table).toMatch(/^Close all tabs in pane\s+Cmd\+\. x/m);
    expect(table).toMatch(/^Kill pane\s+Cmd\+\. Backspace/m);
  });

  test("advertises screen lock only through Hybrid Nav", () => {
    const table = renderTable("web", "mac");
    expect(table).toMatch(/^Lock screen\s+Cmd\+\. L/m);
    expect(table).not.toMatch(/^Lock screen\s+Cmd\+L/m);
  });
});
