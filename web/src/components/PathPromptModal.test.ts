import { describe, expect, test } from "vitest";
import modal from "./PathPromptModal.svelte?raw";
import teamWork from "./TeamWork.svelte?raw";

// `fullstack-b-3`: the rich-prompt watcher dialog needed a path
// prompt that is neither "create" nor "move" / "rename". An
// existing directory shouldn't trigger an "overwrites" warning
// (attaching a watcher never overwrites), and a missing path
// shouldn't surface a "creates new directory" preamble for
// absolute paths (the SPA can't see the OS filesystem; the
// backend creates on demand). The new `attach` mode handles both.
//
// These checks pin the source so a future refactor that drops the
// branch, or accidentally routes the watcher dialog back through
// `mode: "move"`, trips the test.

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
    // Absolute paths bypass tree.entries, so we don't fabricate a
    // mint-green ancestor chain that doesn't correspond to workspace
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

describe("new-file-and-draft-spec item 3: PathPromptModal notice line", () => {
  // The save-from-draft flow passes a `notice` to explain that the
  // whole draft directory is being saved as a directory (the Dir-only
  // `folder` mode). The modal renders it above the input as a
  // non-blocking info line (never gates submit). These checks pin the
  // render branch + the muted-info styling so a refactor that drops
  // the notice trips the test.
  test("modal renders the notice above the input when present", () => {
    expect(modal).toMatch(/\{#if pathPromptState\.notice\}/);
    expect(modal).toMatch(/<div class="notice">\{pathPromptState\.notice\}<\/div>/);
  });

  test("notice uses the muted info hue, not the error/warn colours", () => {
    // The status row owns err (red) / warn (amber); the notice is
    // context, so it reads as info-muted.
    expect(modal).toMatch(/\.notice \{[\s\S]{0,120}var\(--info-text/);
  });
});

describe("Team Work has no manual watcher dialog", () => {
  test("Team Work editor never opens PathPromptModal for watcher attach", () => {
    expect(teamWork).not.toMatch(/function watchDirectory/);
    expect(teamWork).not.toMatch(/uiPathPrompt/);
    expect(teamWork).not.toMatch(/mode: "attach"/);
  });
});
