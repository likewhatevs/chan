import { describe, expect, test } from "vitest";
import modal from "./PathPromptModal.svelte?raw";
import store from "../state/store.svelte.ts?raw";
import tree from "./FileTree.svelte?raw";
import surface from "./FileBrowserSurface.svelte?raw";

// Unified "New File or Directory" dialog and FileTree Settings flip.
// The dialog detects file-vs-dir from the path's trailing slash.
// `onFlip` is piped from FBSurface so the in-tree Settings entry
// calls `flipHybrid(pane.id)`.

describe("PathPromptKind union extension", () => {
  test("PathPromptKind union accepts \"either\"", () => {
    expect(store).toMatch(
      /export type PathPromptKind = "file" \| "folder" \| "either";/,
    );
  });
});

describe("PathPromptModal handles \"either\"", () => {
  test("isEitherDir helper detects trailing slash", () => {
    expect(modal).toMatch(
      /function isEitherDir\(trimmed: string\): boolean \{[\s\S]{1,200}return trimmed\.endsWith\("\/"\);/,
    );
  });

  test("detectedEitherKind derives from the live value when kind is \"either\"", () => {
    expect(modal).toMatch(
      /const detectedEitherKind = \$derived\.by<"file" \| "folder" \| null>\(\(\) => \{[\s\S]{1,400}if \(pathPromptState\.kind !== "either"\) return null;[\s\S]{1,400}return isEitherDir\(value\.trim\(\)\) \? "folder" : "file";/,
    );
  });

  test("effectiveKind resolves either → detected, otherwise pass-through", () => {
    expect(modal).toMatch(
      /const effectiveKind = \$derived\.by<"file" \| "folder">\(\(\) => \{[\s\S]{1,600}if \(pathPromptState\.kind === "either"\)[\s\S]{1,400}return detectedEitherKind \?\? "file";/,
    );
  });

  test("`.md` auto-append gates on effectiveKind, not raw kind", () => {
    expect(modal).toMatch(
      /if \(effectiveKind !== "file"\) \{[\s\S]{1,200}return \{ path: trimmed, autoSuffix: "" \};/,
    );
  });

  test("placeholder branches on the \"either\" kind", () => {
    expect(modal).toMatch(
      /pathPromptState\.kind === "either"[\s\S]{1,400}\? "file\/path or directory\/path\/"/,
    );
  });

  test("wantDir collision-check uses effectiveKind", () => {
    expect(modal).toMatch(/const wantDir = effectiveKind === "folder";/);
  });
});

describe("fileOps.createFileOrDir helper", () => {
  test("createFileOrDir opens the prompt with kind: \"either\"", () => {
    expect(store).toMatch(
      /async createFileOrDir\(parentPath: string\): Promise<void> \{[\s\S]{1,800}kind: "either",/,
    );
  });

  test("trailing-slash result dispatches to api.create(..., true) + revealAndSelect", () => {
    expect(store).toMatch(
      /async createFileOrDir[\s\S]{1,2000}const isDir = next\.endsWith\("\/"\);[\s\S]{1,400}if \(isDir\) \{[\s\S]{1,400}await api\.create\(next, true\);[\s\S]{1,400}revealAndSelect\(next\);/,
    );
  });

  test("non-slash result dispatches to api.create(path, false, \"\") + openInActivePane after appendDefaultMd", () => {
    expect(store).toMatch(
      /async createFileOrDir[\s\S]{1,3000}const path = appendDefaultMd\(next\);[\s\S]{1,400}await api\.create\(path, false, ""\);[\s\S]{1,400}await openInActivePane\(path, \{ landAtTop: true \}\);/,
    );
  });
});

describe("FileTree wiring", () => {
  test("FileTree exposes onFlip prop alongside onClickRow", () => {
    expect(tree).toMatch(
      /onFlip\?: \(\) => void;/,
    );
  });

  test("newFileOrDir helper routes through fileOps.createFileOrDir", () => {
    expect(tree).toMatch(
      /async function newFileOrDir\(parentPath: string\): Promise<void> \{[\s\S]{1,400}menu = null;[\s\S]{1,200}await fileOps\.createFileOrDir\(parentPath\);/,
    );
  });

  test("flipFromMenu calls the surface-supplied onFlip", () => {
    expect(tree).toMatch(
      /function flipFromMenu\(\): void \{[\s\S]{1,200}menu = null;[\s\S]{1,200}onFlip\?\.\(\);/,
    );
  });

  test("menu unifies New File + New Directory into a single entry gated on isDir", () => {
    expect(tree).toMatch(
      /\{#if menu\.isDir\}[\s\S]{1,800}onclick=\{\(\) => newFileOrDir\(menu!\.path\)\}[\s\S]{1,400}<span>New File or Directory<\/span>/,
    );
  });

  test("Settings flip entry sits at the foot, gated on onFlip", () => {
    expect(tree).toMatch(
      /\{#if onFlip\}[\s\S]{1,400}<div class="ctx-sep" role="separator"><\/div>[\s\S]{1,400}onclick=\{flipFromMenu\}[\s\S]{1,400}<span class="menu-row-label">Settings<\/span>/,
    );
  });
});

describe("FileBrowserSurface pipes onFlip down to FileTree (tab variant only)", () => {
  test("FileTree onFlip={isTab ? onFlip : undefined}", () => {
    expect(surface).toMatch(
      /<FileTree[\s\S]{1,400}onFlip=\{isTab \? onFlip : undefined\}/,
    );
  });
});
