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

  test("submit routes to THIS terminal with its OWN agent and a tagged id, keeping the text", () => {
    // Routes to the bubble's OWN tab (not the focused pane's active terminal),
    // submits with the chord THIS terminal reads (submitAgent()) plus a
    // client-generated message id, and only begins a pending when the frame
    // actually went out (the data-loss guard: a failed send keeps the text).
    expect(richPromptSrc).toMatch(/const id = crypto\.randomUUID\(\);/);
    expect(richPromptSrc).toMatch(
      /if \(!sendPromptToTerminal\(tab\.id, text, submitAgent\(\), id\)\) return true;/,
    );
    expect(richPromptSrc).toMatch(/beginPendingPrompt\(tab, id\);/);
    // Queue visibility: submit does NOT clear the composer — the text stays
    // (read-only) until the server's prompt-delivered. No raw input frame,
    // no folder/media delete on submit (that happens on terminal close).
    const submitBody = richPromptSrc.match(/function submit\(\): boolean \{[\s\S]*?\n  \}/)?.[0];
    expect(submitBody).toBeTruthy();
    expect(submitBody).not.toContain('insert: ""');
    // Second Cmd+Enter while a message is in flight is a no-op (one message
    // at a time; replace needs cancel-by-id, deferred to v2 together).
    expect(submitBody).toContain("if (tab.pendingPrompt) return true;");
    // The draft persists what was submitted (reload mid-pending restores it).
    expect(submitBody).toContain("void flushWrite();");
    expect(richPromptSrc).not.toMatch(/type: "input"/);
    expect(richPromptSrc).not.toMatch(/discardDraft/);
  });

  test("delivered clears the composer + draft; rejected/failed unlock and keep the text", () => {
    // The ONLY doc-clear lives in the terminal-phase consumer, on
    // "delivered" (the agent consumed the message; the draft clears HERE).
    expect(richPromptSrc).toMatch(
      /function consumeTerminalPhase\([\s\S]{1,700}changes: \{ from: 0, to: view\.state\.doc\.length, insert: "" \}/,
    );
    expect(richPromptSrc).toMatch(
      /if \(phase === "delivered"\) \{[\s\S]{1,400}void flushWrite\(\);[\s\S]{1,80}view\.focus\(\);/,
    );
    // Honest labels on the non-delivery exits; both keep the text.
    expect(richPromptSrc).toMatch(/queue full — try again/);
    expect(richPromptSrc).toMatch(/connection lost — message may still be queued/);
  });

  test("pending locks the editor read-only + reads as a read-only card (caret hidden)", () => {
    // readOnly blocks doc edits; editable stays TRUE so the caret/keymap stay
    // live for ↑/Esc — an editable:false editor would not receive the key.
    expect(richPromptSrc).toMatch(
      /\[EditorState\.readOnly\.of\(locked\), EditorView\.editable\.of\(true\)\]/,
    );
    // Locked from creation when the bubble mounts mid-pending (hide/show) or a
    // reload restored a queued message; reconfigured live on phase changes.
    expect(richPromptSrc).toMatch(/lockCompartment\.of\(lockExtensions\(isPending\)\)/);
    expect(richPromptSrc).toMatch(
      /lockCompartment\.reconfigure\(lockExtensions\(isPending\)\)/,
    );
    // The read-only LOOK: hide the caret while pending so it isn't an editable field.
    expect(richPromptSrc).toMatch(
      /\.rich-prompt\.pending :global\(\.cm-content\) \{\s*caret-color: transparent;/,
    );
  });

  test("queued/sent: ↑ recalls to edit (optimistic local unlock), Esc abandons, affordances shown", () => {
    // ArrowUp recalls to EDIT from anywhere (no doc-start guard — the card reads
    // read-only, so there's no caret nav to protect).
    expect(richPromptSrc).toMatch(/\{ key: "ArrowUp", run: recall \}/);
    expect(richPromptSrc).not.toMatch(/!sel\.empty \|\| sel\.from !== 0/);
    // Recall accepts queued (acked) AND sent (pre-ack), unlocks LOCALLY (clears
    // the pending), and best-effort cancels in the background — never waiting on
    // the ack, so the editor can't get stuck read-only.
    expect(richPromptSrc).toMatch(/phase !== "queued" && phase !== "sent"\) return false/);
    expect(richPromptSrc).toMatch(/function enterLocalEdit\(\): void/);
    expect(richPromptSrc).toMatch(/tab\.pendingPrompt = undefined;/);
    expect(richPromptSrc).toMatch(/sendCancelToTerminal\(tab\.id, id\)/);
    // Esc enqueued -> dequeue + DROP the draft now; else a real abandon (clear
    // the draft before hiding so onDestroy can't re-persist stale text).
    expect(richPromptSrc).toMatch(/phase === "queued" \|\| phase === "sent"/);
    expect(richPromptSrc).toMatch(/function abandonDraft\(\): void/);
    expect(richPromptSrc).toMatch(/hideRichPromptForTab\(tab\.id\)/);
    // The affordance hint in the queued label.
    expect(richPromptSrc).toMatch(/queued\$\{position\} · ↑ edit · esc cancel/);
  });

  test("fast-path grace + ack timeout constants gate the chip and the dead-socket fail", () => {
    // 300ms: an idle agent drains within ~1 tick — no chip flash on routine
    // submits. 5s: no ack means the socket is effectively dead.
    expect(richPromptSrc).toMatch(/PENDING_CHIP_GRACE_MS = 300/);
    expect(richPromptSrc).toMatch(/PROMPT_ACK_TIMEOUT_MS = 5000/);
    expect(richPromptSrc).toMatch(/failPendingPrompt\(tab\);/);
  });

  test("idle label surfaces teammate queue depth from the prompt itself", () => {
    expect(richPromptSrc).toMatch(/\$\{tab\.queueDepth\} queued · \$\{submitLabel\}/);
    expect(richPromptSrc).toMatch(/queued — waiting for agent/);
  });

  test("submitAgent picks the chord from the terminal's negotiated protocol", () => {
    // modifyOtherKeys -> claude; kitty -> codex; neither (shell / gemini) -> a
    // bare CR = the gemini chord, which a plain shell also understands (the
    // old no-agent path defaulted to claude's CSI, unreadable by a shell).
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

describe("App.svelte Rich Prompt toggle", () => {
  test("imports + binds the per-terminal toggle on a KeyP chord", () => {
    expect(app).toMatch(
      /import \{ toggleRichPromptForTab \} from "\.\/state\/richPrompt\.svelte"/,
    );
    // The chord diverges by surface/OS like the Dashboard chord: mac uses
    // Cmd+Shift+P; off mac the Win/Super key is ruled out, so native uses
    // Ctrl+Shift+P and web uses Alt+Shift+P. Resolved into a `richPromptChord`
    // boolean gated on isTauriDesktop() + currentOS().
    expect(app).toMatch(/const richPromptChord = isTauriDesktop\(\)/);
    // mac path keeps metaKey; off-mac native is ctrlKey, web is altKey.
    expect(app).toMatch(/e\.metaKey && !e\.ctrlKey && !e\.altKey && e\.shiftKey/);
    expect(app).toMatch(/e\.ctrlKey && !e\.metaKey && !e\.altKey && e\.shiftKey/);
    expect(app).toMatch(/e\.altKey && e\.shiftKey && !e\.metaKey && !e\.ctrlKey/);
    // Toggles ONLY the focused terminal; no-op when it isn't a terminal.
    expect(app).toMatch(
      /if \(richPromptChord\)[\s\S]{1,200}activeTerminalTab\(\)[\s\S]{1,80}toggleRichPromptForTab\(term\.id\)/,
    );
  });
});

describe("prompt-sink send seam (tabs.svelte.ts)", () => {
  test("registry + per-terminal sender exist, distinct from the input sink", () => {
    expect(tabs).toMatch(/export function registerTerminalPromptSink\(/);
    // The trailing id is optional: the team orchestrator's lead-identity
    // call sites pass none and stay legacy fire-and-forget.
    expect(tabs).toMatch(
      /export function sendPromptToTerminal\(\s*tabId: string,\s*data: string,\s*agent\?: string,\s*id\?: string,\s*\): boolean/,
    );
    expect(tabs).toMatch(/const terminalPromptSinks = new Map/);
  });

  test("per-terminal draft path is a tab field + persisted (rpd) for leak-free cleanup", () => {
    expect(tabs).toMatch(/richPromptDraftPath\?: string;/);
    expect(tabs).toMatch(/rpd\?: string;/);
    expect(tabs).toMatch(/rpd: t\.richPromptDraftPath/);
  });
});
