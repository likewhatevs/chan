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

// Rich Prompt - the Drafts-backed bubble + its toggle + the sender. The
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
    expect(richPromptSrc).toMatch(
      /let \{[\s\S]{1,80}tab,[\s\S]{1,160}getTerminalCwdRel,[\s\S]{1,120}workspaceRoot = null,[\s\S]{1,400}tab: TerminalTab;[\s\S]{1,400}getTerminalCwdRel\?: \(\) => string \| null;[\s\S]{1,200}workspaceRoot\?: string \| null;[\s\S]{1,40}\} = \$props\(\)/,
    );
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

  test("submit routes to THIS terminal, then KEEPS the text as the greyed read-only card", () => {
    // Routes to the bubble's OWN tab with the chord THIS terminal reads
    // (submitAgent()) + a client message id, only beginning a pending when the
    // frame actually went out (the data-loss guard).
    expect(richPromptSrc).toMatch(/const id = crypto\.randomUUID\(\);/);
    // The wire payload is the draft text with image refs rewritten to plain
    // paths the receiving agent (at the terminal's live CWD) can read instead
    // of 404ing on ./image.png. The card/recall text (`lastQueued`) stays the
    // ORIGINAL text -- preview-correct.
    expect(richPromptSrc).toMatch(
      /import \{ rewriteImagePathsForDelivery \} from "\.\.\/editor\/deliver_images"/,
    );
    expect(richPromptSrc).toMatch(
      /const delivered = rewriteImagePathsForDelivery\(\s*text,\s*draftPath,\s*getTerminalCwdRel\?\.\(\) \?\? null,\s*workspaceRoot,?\s*\);/,
    );
    expect(richPromptSrc).toMatch(
      /if \(!sendPromptToTerminal\(tab\.id, delivered, submitAgent\(\), id\)\) return true;/,
    );
    expect(richPromptSrc).toMatch(/beginPendingPrompt\(tab, id\);/);
    const submitBody = richPromptSrc.match(/function submit\(\): boolean \{[\s\S]*?\n  \}/)?.[0];
    expect(submitBody).toBeTruthy();
    // Submit KEEPS the text (the greyed card), so it does NOT clear the
    // composer; it guards re-submit while the card is up (no double-deliver),
    // persists the text (reload-restores the card), and remembers it for recall.
    expect(submitBody).not.toContain('insert: ""');
    expect(submitBody).toContain("if (isPending) return true;");
    expect(submitBody).toContain("lastQueued = { id, text };");
    expect(submitBody).toContain("void flushWrite();");
    expect(richPromptSrc).not.toMatch(/type: "input"/);
    expect(richPromptSrc).not.toMatch(/discardDraft/);
  });

  test("delivered CLEARS the greyed card; rejected/failed un-grey + keep the text + warn", () => {
    const consumeBody = richPromptSrc.match(/function consumeTerminalPhase\([\s\S]*?\n  \}/)?.[0];
    expect(consumeBody).toBeTruthy();
    // Delivered: the agent consumed the message, so the card clears (text +
    // draft) back to an empty editable composer. The clear dispatch ALSO folds in
    // the readOnly->editable reconfigure (it must not lean on the out-of-band lock
    // $effect to unlock), and the focus is DEFERRED to a microtask so it lands
    // after that effect's trailing reconfigure — without both, WKWebView leaves
    // the cleared composer un-typeable until a hide/show remount.
    const deliveredBranch = consumeBody?.match(
      /if \(phase === "delivered"\)[\s\S]*?\n    \} else \{/,
    )?.[0];
    expect(deliveredBranch).toBeTruthy();
    expect(deliveredBranch).toContain('insert: ""');
    expect(deliveredBranch).toContain("lockCompartment.reconfigure(lockExtensions(false))");
    expect(deliveredBranch).toContain("queueMicrotask(() => view?.focus())");
    // Rejected/failed: clearing pending below un-greys; the text stays for a
    // retry; warn honestly.
    expect(richPromptSrc).toMatch(/queue full — try again/);
    expect(richPromptSrc).toMatch(/connection lost — message may still be queued/);
  });

  test("the greyed read-only card: readOnly lock + caret hidden, reconciled by type-to-move-on", () => {
    // The read-only/greyed/caret-hidden card is applied via a lock
    // compartment, but reconciles back-to-back by exiting on the first keystroke
    // (beforeinput move-on) rather than dropping the lock — so it never STICKS.
    expect(richPromptSrc).toMatch(/lockCompartment/);
    expect(richPromptSrc).toMatch(/EditorState\.readOnly\.of\(locked\)/);
    expect(richPromptSrc).toMatch(/caret-color: transparent/);
    // Type to move on: a user text input while pending clears the card + seeds a
    // fresh composer with what was typed.
    expect(richPromptSrc).toMatch(
      /beforeinput: \(event, v\) => \{[\s\S]{1,400}if \(!isPending\) return false;[\s\S]{1,400}enterLocalEdit\(\);/,
    );
    expect(richPromptSrc).toMatch(/insert: seed/);
  });

  test("↑ edits the queued message (from the card or an empty composer); Esc drops it", () => {
    expect(richPromptSrc).toMatch(/\{ key: "ArrowUp", run: recall \}/);
    // From the greyed card, recall un-greys (the text is already shown); from an
    // empty composer it restores the buffer. Both best-effort cancel.
    expect(richPromptSrc).toMatch(/if \(isPending\) \{[\s\S]{1,200}enterLocalEdit\(\);/);
    // The card-up recall MUST fold the readOnly->editable reconfigure into its
    // own dispatch and DEFER focus to a microtask — same WKWebView flip the
    // delivered path folds. Leaning on the out-of-band lock $effect + a
    // synchronous focus leaves the card un-typeable until a remount (the
    // ArrowUp-stuck-read-only regression).
    const recallPending = richPromptSrc.match(
      /if \(isPending\) \{[\s\S]*?return true;\n    \}/,
    )?.[0];
    expect(recallPending).toBeTruthy();
    expect(recallPending).toContain("lockCompartment.reconfigure(lockExtensions(false))");
    expect(recallPending).toContain("queueMicrotask(() => view?.focus())");
    expect(richPromptSrc).toMatch(/view\.state\.doc\.length > 0 \|\| !lastQueued\) return false/);
    expect(richPromptSrc).toMatch(/const \{ id, text \} = lastQueued;/);
    expect(richPromptSrc).toMatch(/sendCancelToTerminal\(tab\.id, id\)/);
    // Esc drops the queued message (card up, or empty composer with a queued
    // one): cancel + clear, keeping the bubble open; otherwise abandon the draft.
    expect(richPromptSrc).toMatch(
      /lastQueued && \(isPending \|\| view\.state\.doc\.length === 0\)\) \{[\s\S]{1,160}sendCancelToTerminal\(tab\.id, lastQueued\.id\)/,
    );
    expect(richPromptSrc).toMatch(/function abandonDraft\(\): void/);
    expect(richPromptSrc).toMatch(/hideRichPromptForTab\(tab\.id\)/);
    // The card's label IS its chrome: ↑ edit · esc cancel.
    expect(richPromptSrc).toMatch(/queued · ↑ edit · esc cancel/);
  });

  test("fast-path grace + ack timeout constants gate the chip and the dead-socket fail", () => {
    // 300ms: an idle agent drains within ~1 tick — no chip flash on routine
    // submits. 5s: no ack means the socket is effectively dead.
    expect(richPromptSrc).toMatch(/PENDING_CHIP_GRACE_MS = 300/);
    expect(richPromptSrc).toMatch(/PROMPT_ACK_TIMEOUT_MS = 5000/);
    expect(richPromptSrc).toMatch(/failPendingPrompt\(tab\);/);
  });

  test("label surfaces the queue depth (server + the local just-submitted) with the right affordance", () => {
    // queuedCount = max(server queueDepth, the local just-submitted message
    // after the grace window) — so a teammate `cs terminal write` and the
    // user's own queued messages both show.
    expect(richPromptSrc).toMatch(
      /Math\.max\(tab\.queueDepth \?\? 0, isPending && pendingChipVisible \? 1 : 0\)/,
    );
    // Card up (isPending): edit/cancel affordances ARE the chrome. Moved-on but
    // messages still queued: the recall hint + the submit hint.
    expect(richPromptSrc).toMatch(/isPending\) return `\$\{queuedCount\} queued · ↑ edit · esc cancel`/);
    expect(richPromptSrc).toMatch(/\$\{queuedCount\} queued · ↑ recall · \$\{submitLabel\}/);
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

describe("prompt-sink sender (tabs.svelte.ts)", () => {
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
