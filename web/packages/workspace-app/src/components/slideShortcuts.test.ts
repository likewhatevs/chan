import { describe, expect, test } from "vitest";
import source from "./FileEditorTab.svelte?raw";

describe("slide editor shortcuts", () => {
  test("preview and present are gated by slide frontmatter", () => {
    expect(source).toContain('import { parseSlidesSpec } from "../editor/slides";');
    expect(source).toContain("const slidesSpec = $derived(parseSlidesSpec(tab.content));");
    expect(source).toMatch(
      /function onSlideShortcutKeydown\(e: KeyboardEvent\): void \{[\s\S]*?!slidesSpec[\s\S]*?e\.key !== "Enter"[\s\S]*?if \(e\.shiftKey\) playSlides\(\);[\s\S]*?else previewSlides\(\);/,
    );
  });

  test("uses platform Mod and captures the shortcut before CodeMirror", () => {
    expect(source).toMatch(
      /return slideShortcutOS === "mac" \? e\.metaKey : e\.ctrlKey;/,
    );
    expect(source).toMatch(
      /<div[\s\S]{0,240}class="editor-host"[\s\S]{0,240}onkeydowncapture=\{onSlideShortcutKeydown\}[\s\S]{0,800}<Wysiwyg/,
    );
    expect(source).toMatch(
      /<div[\s\S]{0,240}class="editor-host"[\s\S]{0,240}onkeydowncapture=\{onSlideShortcutKeydown\}[\s\S]{0,800}<Source/,
    );
  });

  test("refocuses the editor after closing slide preview", () => {
    expect(source).toContain('import { onDestroy, tick } from "svelte";');
    expect(source).toMatch(
      /function refocusAfterSlidePreviewClose\(\): void \{[\s\S]*?void tick\(\)\.then\(\(\) => \{[\s\S]*?if \(!active \|\| !focused\) return;[\s\S]*?focusActiveEditor\(\);/,
    );
    expect(source).toMatch(
      /onClose: \(\) => \{[\s\S]*?setTabSlidePreviewOpen\(tab, false\);[\s\S]*?slidePreviewHandle = null;[\s\S]*?refocusAfterSlidePreviewClose\(\);[\s\S]*?\},/,
    );
  });
});
