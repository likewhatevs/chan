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

// Rich Prompt - the Drafts-backed bubble + its toggle + the send seam. The
// terminal wiring (mount / menu / sink registration / close-discard) is covered
// in richPromptTerminalWiring.test.ts. Component markup is asserted as source
// shape (it is a Svelte component, not pure); the real interaction (paste ->
// Drafts, submit carries the ref, close deletes the folder) is browser-smoked.

describe("richPrompt state module", () => {
  beforeEach(() => {
    richPrompt.visible = false;
  });

  test("toggle / show / hide drive `visible` (visibility-only; text lives in the draft)", () => {
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
});

describe("RichPrompt.svelte component", () => {
  test("lightweight CM6: markdown (no addKeymap) + history + default keymap", () => {
    expect(richPromptSrc).toMatch(/markdown\(\{ addKeymap: false \}\)/);
    expect(richPromptSrc).toMatch(/history\(\)/);
    expect(richPromptSrc).toMatch(/keymap\.of\(\[\.\.\.defaultKeymap, \.\.\.historyKeymap\]\)/);
    // Lightweight v1: no Wysiwyg widget/decoration imports; the ONE editor
    // reuse is the image-paste extension (bubbles/image_drop), which is exactly
    // the draft image-paste win.
    expect(richPromptSrc).not.toMatch(/from "[^"]*(wysiwyg|widgets)/i);
  });

  test("markdown-aware editing: Enter continues lists, Backspace dedents, Cmd+Enter submits", () => {
    expect(richPromptSrc).toMatch(
      /import \{[\s\S]*?deleteMarkupBackward[\s\S]*?insertNewlineContinueMarkup[\s\S]*?\} from "@codemirror\/lang-markdown"/,
    );
    expect(richPromptSrc).toMatch(
      /Prec\.high\(\s*keymap\.of\(\[[\s\S]*?\{ key: "Mod-Enter", run: submit \}[\s\S]*?\{ key: "Enter", run: insertNewlineContinueMarkup \}[\s\S]*?\{ key: "Backspace", run: deleteMarkupBackward \}/,
    );
    expect(richPromptSrc).not.toMatch(/\{ key: "Enter", run: submit \}/);
  });

  test("Drafts-backed: per-terminal draft.md + editor image paste into the draft folder", () => {
    // Bound to the terminal's draft (a prop), created lazily, content loaded
    // from + written back to draft.md; pasted images use the editor's
    // imageDropHandlers pointed at the draft folder.
    expect(richPromptSrc).toMatch(/let \{ tab \}: \{ tab: TerminalTab \} = \$props\(\)/);
    expect(richPromptSrc).toMatch(
      /import \{ imageDropHandlers \} from "\.\.\/editor\/bubbles\/image_drop"/,
    );
    expect(richPromptSrc).toMatch(
      /imageDropHandlers\(\{[\s\S]{1,160}getUploadDir: \(\) => draftDir\(\)[\s\S]{1,120}getCurrentPath: \(\) => draftPath/,
    );
    expect(richPromptSrc).toMatch(/await api\.createDraft\(\)/);
    expect(richPromptSrc).toMatch(/tab\.richPromptDraftPath = path/);
    expect(richPromptSrc).toMatch(/await api\.read\(path\)/);
    expect(richPromptSrc).toMatch(/await api\.write\(draftPath, text\)/);
  });

  test("submit routes through the queue, then clears draft.md text (keeps the folder)", () => {
    expect(richPromptSrc).toMatch(/sendPromptToActiveTerminal\(text\)/);
    // Reset = clear the doc + persist the empty draft.md; NO raw input frame,
    // and NO folder/media delete on submit (that happens on terminal close).
    expect(richPromptSrc).toMatch(/insert: "" \}/);
    expect(richPromptSrc).toMatch(/void flushWrite\(\)/);
    expect(richPromptSrc).not.toMatch(/type: "input"/);
    expect(richPromptSrc).not.toMatch(/discardDraft/);
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

  test("per-terminal draft path is a tab field + persisted (rpd) for leak-free cleanup", () => {
    expect(tabs).toMatch(/richPromptDraftPath\?: string;/);
    expect(tabs).toMatch(/rpd\?: string;/);
    expect(tabs).toMatch(/rpd: t\.richPromptDraftPath/);
  });
});
