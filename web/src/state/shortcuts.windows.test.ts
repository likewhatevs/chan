import { describe, expect, test } from "vitest";
import {
  formatChord,
  osChord,
  type Platform,
  SHORTCUTS,
} from "./shortcuts";

// Phase-26 step 5: confirm the Windows shortcut set is Ctrl-based and
// Linux-like — no chord LABEL should render "Cmd" under Windows
// (Windows keyboards have no Command key; Win-key-as-Cmd is ruled out).

function windowsLabels(platform: Platform): { id: string; label: string }[] {
  const out: { id: string; label: string }[] = [];
  for (const s of SHORTCUTS) {
    const chord = osChord(s, platform, "windows");
    if (!chord) continue;
    out.push({ id: s.id, label: formatChord(chord, "windows") });
  }
  return out;
}

describe("Windows shortcut labels are Ctrl-based", () => {
  test("Mod renders Ctrl, not Cmd (web + native)", () => {
    for (const platform of ["web", "native"] as Platform[]) {
      const modBased = windowsLabels(platform).filter((x) =>
        SHORTCUTS.find((s) => s.id === x.id)?.[platform]?.includes("Mod"),
      );
      // Every Mod-token chord must surface as Ctrl on Windows.
      for (const { id, label } of modBased) {
        expect(label, `${id} (${platform})`).not.toContain("Cmd");
        expect(label, `${id} (${platform})`).toContain("Ctrl");
      }
    }
  });

  test("the shell-colliding + Win-key chords diverge on Windows", () => {
    const pick = (id: string, platform: Platform) =>
      formatChord(
        osChord(SHORTCUTS.find((s) => s.id === id)!, platform, "windows")!,
        "windows",
      );
    // Shell collisions → Ctrl+Shift (bare Ctrl+C/R stay SIGINT / search).
    expect(pick("app.window.reload", "native")).toBe("Ctrl+Shift+R");
    expect(pick("terminal.copy", "native")).toBe("Ctrl+Shift+C");
    expect(pick("terminal.paste", "native")).toBe("Ctrl+Shift+V");
    // Rich Prompt: Win key ruled out → Ctrl+Shift+P native, Alt+Shift+P web.
    expect(pick("terminal.richPrompt", "native")).toBe("Ctrl+Shift+P");
    expect(pick("terminal.richPrompt", "web")).toBe("Alt+Shift+P");
  });

  test("native (desktop) Windows has ZERO Cmd labels", () => {
    const offenders = windowsLabels("native").filter((x) =>
      x.label.includes("Cmd"),
    );
    expect(offenders).toEqual([]);
  });

  test("the only web Cmd labels are the documented Mac-web spawn fallbacks", () => {
    // These three web chords are Cmd+Alt+<key> Mac-only fallbacks (identical
    // on Linux); on Windows the real path is Hybrid Nav (Ctrl+. <key>), and
    // each carries a note saying so. The native column renders them as
    // Ctrl+<key>. Not a Windows-specific regression — left as-is this phase.
    const webCmd = windowsLabels("web")
      .filter((x) => x.label.includes("Cmd"))
      .map((x) => x.id)
      .sort();
    expect(webCmd).toEqual([
      "app.files.toggle",
      "app.terminal.teamWork",
      "app.terminal.toggle",
    ]);
  });
});
