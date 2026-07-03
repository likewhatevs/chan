import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import sourceEditor from "../editor/Source.svelte?raw";
import wysiwygEditor from "../editor/Wysiwyg.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";
import pane from "./Pane.svelte?raw";
import fileEditor from "./FileEditorTab.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";
import browser from "./FileBrowserSurface.svelte?raw";
import graph from "./GraphPanel.svelte?raw";
import dashboard from "./DashboardTab.svelte?raw";

describe("Track C: Hybrid surface body themes", () => {
  test("Pane no longer themes the whole Hybrid chrome", () => {
    expect(pane).not.toContain("data-theme={pane.theme}");
    expect(pane).toContain("<HybridTerminalConfig onDone=");
    expect(pane).toContain("<HybridEditorConfig onDone=");
  });

  test("CSS token blocks can apply to any themed surface subtree", () => {
    expect(app).toContain(":global([data-theme=\"dark\"])");
    expect(app).toContain(":global([data-theme=\"light\"])");
    expect(app).not.toContain(":global(.pane[data-theme=\"dark\"])");
    expect(app).not.toContain(":global(.pane[data-theme=\"light\"])");
  });

  test("front-side Hybrid bodies opt into their surface override only", () => {
    expect(fileEditor).toContain('data-theme={surfaceThemeOverride("editor")}');
    expect(terminal).toContain('data-theme={surfaceThemeOverride("terminal")}');
    expect(browser).toContain(
      'data-theme={isTab ? surfaceThemeOverride("browser") : undefined}',
    );
    expect(graph).toContain(
      'data-theme={tab ? surfaceThemeOverride("graph") : undefined}',
    );
    expect(dashboard).toContain(
      'data-theme={surfaceThemeOverride("dashboard")}',
    );
  });

  test("terminal and CodeMirror palettes follow surface theme resolution", () => {
    expect(terminal).toContain('effectiveHybridSurfaceTheme("terminal")');
    expect(sourceEditor).toContain('effectiveHybridSurfaceTheme("editor")');
    // Wysiwyg themes on a `surface` prop (default "editor"; the Rich Prompt
    // composer passes "terminal") so it can match the surface it floats over.
    expect(wysiwygEditor).toContain('effectiveHybridSurfaceTheme(surface)');
  });

  test("shared back-side shell owns the per-surface switch and footer OK", () => {
    expect(shell).toContain("setHybridSurfaceTheme(surface, choice)");
    expect(shell).toContain("effectiveHybridSurfaceTheme(surface)");
    expect(shell).toContain('class="config-footer"');
    expect(shell).toContain('class="config-ok"');
  });
});
