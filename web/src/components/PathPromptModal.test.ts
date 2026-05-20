import { describe, expect, test } from "vitest";
import modal from "./PathPromptModal.svelte?raw";
import terminalRichPrompt from "./TerminalRichPrompt.svelte?raw";

// `fullstack-b-3`: the rich-prompt watcher dialog needed a path
// prompt that is neither "create" nor "move" / "rename". An
// existing directory shouldn't trigger an "overwrites" warning
// (attaching a watcher never overwrites), and a missing path
// shouldn't surface a "creates new directory" preamble for
// absolute paths (the SPA can't see the OS filesystem; the
// backend creates on demand). The new `attach` mode handles both.
//
// These checks pin the source so a future refactor that drops the
// branch — or accidentally routes the watcher dialog back through
// `mode: "move"` — trips the test.

describe("fullstack-b-3: PathPromptModal attach mode", () => {
  test("modal renders 'attach watcher to' label in attach mode", () => {
    expect(modal).toMatch(/status\.mode === "attach"[\s\S]{0,40}attach watcher to/);
  });

  test("existing-folder branch skips the overwrite warning in attach mode", () => {
    // The status derivation has an explicit `mode === "attach"`
    // branch that returns a `creates`-shaped status with empty
    // ancestors when the target is already in the tree.
    expect(modal).toMatch(/if \(pathPromptState\.mode === "attach"\) \{/);
    expect(modal).toMatch(/newAncestors: \[\]/);
  });

  test("absolute-path branch suppresses the ancestor preamble", () => {
    // Absolute paths bypass tree.entries — we don't fabricate a
    // mint-green ancestor chain that doesn't correspond to drive
    // state.
    expect(modal).toMatch(
      /pathPromptState\.mode === "attach" && path\.startsWith\("\/"\)/,
    );
  });

  test("pathSegments demotes the final segment when attaching to an existing dir", () => {
    // `tailIsExisting` flips the "new" colouring off so the visible
    // chunk reads as context, not as a fresh-create cue.
    expect(modal).toMatch(/const tailIsExisting =\s+s\.mode === "attach"/);
  });
});

describe("fullstack-b-10: TerminalRichPrompt watcher dialog uses attach mode", () => {
  test("watchDirectory passes mode: 'attach' to uiPathPrompt", () => {
    // `fullstack-b-3` introduced the `PathPromptMode = "attach"`
    // branches in PathPromptModal but the watcher-dialog call
    // site still passed `mode: "move"`, leaving the misleading
    // `⚠ overwrites existing directory <name>/` warning live for
    // existing in-drive dirs (@@WebtestB wave-1 verification on
    // 2026-05-20). The fix flips the call site so the new
    // branches are actually reached.
    expect(terminalRichPrompt).toMatch(
      /async function watchDirectory\(\): Promise<void> \{[\s\S]*?title: "watch directory",[\s\S]*?mode: "attach",[\s\S]*?\}\);/,
    );
    // Belt-and-suspenders: the old `mode: "move"` must not be
    // back in the watcher-dialog block.
    expect(terminalRichPrompt).not.toMatch(
      /async function watchDirectory\(\): Promise<void> \{[\s\S]*?mode: "move"/,
    );
  });
});
