import { beforeEach, describe, expect, test } from "vitest";
import richPromptSrc from "./RichPrompt.svelte?raw";
import app from "../App.svelte?raw";
import tabs from "../state/tabs.svelte.ts?raw";
import {
  hideRichPromptForTab,
  isRichPromptVisible,
  richPrompt,
  showRichPromptForTab,
  toggleRichPromptForTab,
} from "../state/richPrompt.svelte";

// Rich Prompt - the Drafts-backed bubble + its toggle + the send seam. The
// terminal wiring (mount / menu / sink registration / close-discard) is covered
// in richPromptTerminalWiring.test.ts. Component markup is asserted as source
// shape (it is a Svelte component, not pure); the real interaction (paste ->
// Drafts, submit carries the ref, close deletes the folder) is browser-smoked.

describe("richPrompt state module", () => {
  beforeEach(() => {
    richPrompt.byTab = {};
  });

  test("per-terminal toggle / show / hide keyed by tab id (text lives in the draft)", () => {
    expect(isRichPromptVisible("t1")).toBe(false);
    toggleRichPromptForTab("t1");
    expect(isRichPromptVisible("t1")).toBe(true);
    // Independent per terminal: opening t1 does not affect t2.
    expect(isRichPromptVisible("t2")).toBe(false);
    toggleRichPromptForTab("t1");
    expect(isRichPromptVisible("t1")).toBe(false);
    showRichPromptForTab("t2");
    expect(isRichPromptVisible("t2")).toBe(true);
    hideRichPromptForTab("t2");
    expect(isRichPromptVisible("t2")).toBe(false);
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

  test("submit routes to THIS terminal with its OWN submit agent, then clears draft.md text", () => {
    // Routes to the bubble's OWN tab (not the focused pane's active terminal),
    // submits with the chord THIS terminal reads (submitAgent()), and only
    // reaps the composer when the frame actually went out (the data-loss guard:
    // a failed send keeps the text instead of clearing it).
    expect(richPromptSrc).toMatch(
      /if \(!sendPromptToTerminal\(tab\.id, text, submitAgent\(\)\)\) return true;/,
    );
    // Reset = clear the doc + persist the empty draft.md; NO raw input frame,
    // and NO folder/media delete on submit (that happens on terminal close).
    expect(richPromptSrc).toMatch(/insert: "" \}/);
    expect(richPromptSrc).toMatch(/void flushWrite\(\)/);
    expect(richPromptSrc).not.toMatch(/type: "input"/);
    expect(richPromptSrc).not.toMatch(/discardDraft/);
  });

  test("submitAgent picks the chord from the terminal's negotiated protocol", () => {
    // modifyOtherKeys -> claude; kitty -> codex; neither (shell / gemini) -> a
    // bare CR = the gemini chord. Fixes @@Alex's shell case (the old no-agent
    // path defaulted to claude's CSI, unreadable by a shell).
    expect(richPromptSrc).toMatch(/function submitAgent\(\): string/);
    expect(richPromptSrc).toMatch(
      /kp\.xtermModifyOtherKeys > 0\) return "claude"/,
    );
    expect(richPromptSrc).toMatch(/return "codex"/);
    expect(richPromptSrc).toMatch(/return "gemini";/);
  });

  test("Tab indents the list item (Shift+Tab outdents), never escaping to the browser", () => {
    expect(richPromptSrc).toMatch(
      /import \{[\s\S]{1,80}indentListItem,[\s\S]{1,40}outdentListItem,[\s\S]{1,40}\} from "\.\.\/editor\/commands\/list"/,
    );
    expect(richPromptSrc).toMatch(
      /key: "Tab",[\s\S]{1,80}run: \(v\) => indentListItem\(v\) \|\| indentMore\(v\),[\s\S]{1,90}shift: \(v\) => outdentListItem\(v\) \|\| indentLess\(v\),/,
    );
  });

  test("floating bubble with the submit-with-cmd+enter label", () => {
    expect(richPromptSrc).toMatch(/class="rich-prompt"/);
    expect(richPromptSrc).toMatch(/submit with cmd\+enter/);
    expect(richPromptSrc).toMatch(/position: absolute/);
  });
});

describe("App.svelte Cmd+Shift+P toggle", () => {
  test("imports + binds Cmd+Shift+P to the per-terminal toggle (shift, not alt)", () => {
    expect(app).toMatch(
      /import \{ toggleRichPromptForTab \} from "\.\/state\/richPrompt\.svelte"/,
    );
    // Resolves the focused terminal, then toggles ONLY that terminal; no-op
    // when the focused tab is not a terminal.
    expect(app).toMatch(
      /e\.metaKey && !e\.altKey && e\.shiftKey && !e\.ctrlKey && e\.code === "KeyP"[\s\S]{1,200}activeTerminalTab\(\)[\s\S]{1,80}toggleRichPromptForTab\(term\.id\)/,
    );
  });
});

describe("prompt-sink send seam (tabs.svelte.ts)", () => {
  test("registry + per-terminal sender exist, distinct from the input sink", () => {
    expect(tabs).toMatch(/export function registerTerminalPromptSink\(/);
    expect(tabs).toMatch(
      /export function sendPromptToTerminal\(tabId: string, data: string, agent\?: string\): boolean/,
    );
    expect(tabs).toMatch(/const terminalPromptSinks = new Map/);
  });

  test("per-terminal draft path is a tab field + persisted (rpd) for leak-free cleanup", () => {
    expect(tabs).toMatch(/richPromptDraftPath\?: string;/);
    expect(tabs).toMatch(/rpd\?: string;/);
    expect(tabs).toMatch(/rpd: t\.richPromptDraftPath/);
  });
});
