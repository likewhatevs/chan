import { describe, expect, test } from "vitest";
import promptModal from "../components/PathPromptModal.svelte?raw";
import store from "./store.svelte.ts?raw";

describe("File Browser no-clobber move policy", () => {
  test("performMove rejects Drafts sources and targets before api.move", () => {
    expect(store).toMatch(
      /const draftsReason =[\s\S]{1,200}fileBrowserDraftsPathReason\(path\) \?\? fileBrowserDraftsPathReason\(target\);[\s\S]{1,200}ui\.status = `move failed: \$\{draftsReason\}`;[\s\S]{1,80}return;/,
    );
  });

  test("performMove refuses existing directory targets without overwrite confirm", () => {
    expect(store).toMatch(
      /if \(existing\.is_dir\) \{[\s\S]{1,160}ui\.status = `rename failed: '\$\{target\}' is an existing directory`;[\s\S]{1,80}return;[\s\S]{1,120}title: "Overwrite existing file\?"/,
    );
  });
});

describe("File Browser Drafts create guard", () => {
  test("create prompts reject Drafts paths in their validators", () => {
    expect(store).toMatch(/function fileBrowserDraftsPathReason\(path: string\): string \| null/);
    expect(store).toMatch(
      /async createFile\(parentPath: string\): Promise<void> \{[\s\S]{1,1600}fileBrowserDraftsPathReason\(path\) \?\?/,
    );
    expect(store).toMatch(
      /async createDir\(parentPath: string\): Promise<void> \{[\s\S]{1,600}validate: fileBrowserDraftsPathReason,/,
    );
    expect(store).toMatch(
      /async createFileOrDir\(parentPath: string\): Promise<void> \{[\s\S]{1,600}validate: fileBrowserDraftsPathReason,/,
    );
  });
});

describe("PathPrompt move collision copy", () => {
  test("existing directory targets are invalid for move", () => {
    expect(promptModal).toMatch(
      /if \(pathPromptState\.mode === "move"\) \{[\s\S]{1,240}if \(targetEntry\.is_dir\) \{[\s\S]{1,240}existing directory; choose a new path/,
    );
  });
});
