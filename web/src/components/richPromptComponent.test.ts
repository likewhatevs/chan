import { beforeEach, describe, expect, test } from "vitest";
import richPromptSrc from "./RichPrompt.svelte?raw";
import app from "../App.svelte?raw";
import tabs from "../state/tabs.svelte.ts?raw";
import {
  hideRichPrompt,
  richPrompt,
  showRichPrompt,
  toggleRichPrompt,
} from "../state/richPrompt.svelte";

// Rich Prompt - the bubble component + its toggle + the send seam (slice 1).
// The terminal wiring (mount / menu / sink registration) is covered in
// richPromptTerminalWiring.test.ts. Component markup is asserted as source
// shape (it is a Svelte component, not pure); the real interaction is
// browser-smoked.

describe("richPrompt state module", () => {
  beforeEach(() => {
    richPrompt.visible = false;
    richPrompt.draft = "";
  });

  test("toggle / show / hide drive `visible`", () => {
    expect(richPrompt.visible).toBe(false);
    toggleRichPrompt();
    expect(richPrompt.visible).toBe(true);
    toggleRichPrompt();
    expect(richPrompt.visible).toBe(false);
    showRichPrompt();
    expect(richPrompt.visible).toBe(true);
    hideRichPrompt();
    expect(richPrompt.visible).toBe(false);
  });

  test("draft is a mutable shared field", () => {
    richPrompt.draft = "hello";
    expect(richPrompt.draft).toBe("hello");
  });
});

describe("RichPrompt.svelte component", () => {
  test("lightweight CM6: markdown (no addKeymap) + history + default keymap", () => {
    expect(richPromptSrc).toMatch(/markdown\(\{ addKeymap: false \}\)/);
    expect(richPromptSrc).toMatch(/history\(\)/);
    expect(richPromptSrc).toMatch(/keymap\.of\(\[\.\.\.defaultKeymap, \.\.\.historyKeymap\]\)/);
    // Lightweight v1: no imports from the Wysiwyg widget / decoration / bubble
    // modules (markdown syntax + history + keymap only).
    expect(richPromptSrc).not.toMatch(/from "[^"]*(wysiwyg|widgets|bubbles)/i);
  });

  test("markdown-aware editing: Enter continues lists, Backspace dedents, Cmd+Enter submits", () => {
    expect(richPromptSrc).toMatch(
      /import \{[\s\S]*?deleteMarkupBackward[\s\S]*?insertNewlineContinueMarkup[\s\S]*?\} from "@codemirror\/lang-markdown"/,
    );
    // One high-prec keymap: Mod-Enter submit + markdown Enter/Backspace, above
    // defaultKeymap so list continuation wins on Enter.
    expect(richPromptSrc).toMatch(
      /Prec\.high\(\s*keymap\.of\(\[[\s\S]*?\{ key: "Mod-Enter", run: submit \}[\s\S]*?\{ key: "Enter", run: insertNewlineContinueMarkup \}[\s\S]*?\{ key: "Backspace", run: deleteMarkupBackward \}/,
    );
    // Enter is NOT bound to submit (that would block newlines / list continue).
    expect(richPromptSrc).not.toMatch(/\{ key: "Enter", run: submit \}/);
  });

  test("submit routes through the queue seam + clears, never a raw input frame", () => {
    expect(richPromptSrc).toMatch(/sendPromptToActiveTerminal\(text\)/);
    expect(richPromptSrc).toMatch(/richPrompt\.draft = ""/);
    expect(richPromptSrc).not.toMatch(/type: "input"/);
  });

  test("floating bubble with the submit-with-cmd+enter label", () => {
    expect(richPromptSrc).toMatch(/class="rich-prompt"/);
    expect(richPromptSrc).toMatch(/submit with cmd\+enter/);
    expect(richPromptSrc).toMatch(/position: absolute/);
  });
});

describe("App.svelte Cmd+Shift+P toggle", () => {
  test("imports + binds Cmd+Shift+P to toggleRichPrompt (shift, not alt)", () => {
    expect(app).toMatch(
      /import \{ toggleRichPrompt \} from "\.\/state\/richPrompt\.svelte"/,
    );
    expect(app).toMatch(
      /e\.metaKey && !e\.altKey && e\.shiftKey && !e\.ctrlKey && e\.code === "KeyP"[\s\S]{1,120}toggleRichPrompt\(\)/,
    );
  });
});

describe("prompt-sink send seam (tabs.svelte.ts)", () => {
  test("registry + active-terminal sender exist, distinct from the input sink", () => {
    expect(tabs).toMatch(/export function registerTerminalPromptSink\(/);
    expect(tabs).toMatch(
      /export function sendPromptToActiveTerminal\(data: string, agent\?: string\): boolean/,
    );
    expect(tabs).toMatch(/const terminalPromptSinks = new Map/);
  });
});
