// `fullstack-a-35`: inline file-rename band. Raw-source pins
// guard the wiring shape inside FileEditorTab — the trigger
// (`doRename` flips state instead of opening the modal), the
// commit/cancel keymap (Enter / Escape), the band markup (lives
// above the editor body, outside the page-width cap), and the
// `fileOps.renameInPlace` entry point in store.svelte.ts that
// bypasses the modal for inline commits.
//
// CM6 + Wysiwyg internals make a behavioural test through a
// real EditorView heavy; the raw-source approach matches the
// pattern used by `revealBrowserActions.test.ts`,
// `paneModeKeymap.test.ts`, etc.

import { describe, expect, test } from "vitest";
import fileEditor from "./FileEditorTab.svelte?raw";
import store from "../state/store.svelte.ts?raw";

describe("fullstack-a-35: inline file-rename band", () => {
  test("doRename flips state instead of opening the modal", () => {
    // Pre-fix: `doRename` called `fileOps.rename(tab.path, false)`
    // which pops `uiPathPrompt`. Post-fix: it primes the draft
    // + focuses the band's input.
    expect(fileEditor).toContain("function doRename(): void");
    expect(fileEditor).toContain("renameDraft = tab.path;");
    expect(fileEditor).toContain("renameActive = true;");
    // Must NOT call the modal-driven path from doRename.
    expect(fileEditor).not.toMatch(
      /function doRename\(\): void \{[\s\S]{0,200}?fileOps\.rename\(/,
    );
  });

  test("commit calls fileOps.renameInPlace, cancel clears state", () => {
    expect(fileEditor).toContain("async function commitRename()");
    expect(fileEditor).toContain("fileOps.renameInPlace(tab.path, next, false)");
    expect(fileEditor).toContain("function cancelRename()");
    expect(fileEditor).toMatch(/cancelRename[\s\S]{0,80}renameActive = false/);
  });

  test("keydown handler binds Enter to commit, Escape to cancel", () => {
    // Both branches preventDefault so the editor below doesn't
    // also receive the keystroke (Enter would otherwise insert
    // a newline into the focused editor when the rename was
    // open).
    expect(fileEditor).toMatch(
      /function onRenameKeydown[\s\S]*?key === "Enter"[\s\S]*?preventDefault\(\)[\s\S]*?commitRename\(\)/,
    );
    expect(fileEditor).toMatch(
      /function onRenameKeydown[\s\S]*?key === "Escape"[\s\S]*?preventDefault\(\)[\s\S]*?cancelRename\(\)/,
    );
  });

  test("rename band renders above the editor toolbar gated on renameActive", () => {
    // The band must come BEFORE the editor-toolbar / editor body
    // in the template so it sits in a header position. The
    // {#if renameActive} block wraps a `.rename-band` div with
    // the input wired to `bind:value={renameDraft}`.
    expect(fileEditor).toMatch(
      /\{#if renameActive\}[\s\S]*?class="rename-band"[\s\S]*?bind:value=\{renameDraft\}/,
    );
    // The band sits above tab.fileMissing / tab.error toolbar
    // blocks; the regex confirms ordering by matching the band
    // before the editor-toolbar block.
    expect(fileEditor).toMatch(
      /\{#if renameActive\}[\s\S]*?\{\/if\}[\s\S]*?\{#if tab\.fileMissing\}[\s\S]*?editor-toolbar/,
    );
  });

  test("rename input has no page-width cap (full pane width)", () => {
    // CSS for .rename-band must declare `width: 100%` and
    // sit OUTSIDE the editor's --chan-page-max-width cap.
    // The band lives directly inside .editor-tab (above the
    // editor body) so it never inherits the cap from
    // .editor-wrap.
    expect(fileEditor).toMatch(/\.rename-band\s*\{[\s\S]*?width:\s*100%/);
    // The input itself should be `flex: 1` so it grows to
    // fill the band's full row.
    expect(fileEditor).toMatch(/\.rename-input\s*\{[\s\S]*?flex:\s*1/);
  });

  test("fileOps.renameInPlace exists in store.svelte and uses performMove", () => {
    // Inline-rename entry point that bypasses the modal.
    // Same overwrite-confirm + link-rewrite + tab-rekey
    // bookkeeping as the modal-driven `rename` (both go
    // through `performMove`).
    expect(store).toContain(
      "async renameInPlace(path: string, next: string, isDir = false)",
    );
    expect(store).toMatch(
      /renameInPlace[\s\S]*?await performMove\(path, target\)/,
    );
    // preserveExtension is the rule that lets the user type
    // `foo` to rename `foo.md` → `foo.md`. Match for both
    // directories (verbatim) and files (extension-preserved).
    expect(store).toMatch(
      /renameInPlace[\s\S]*?preserveExtension\(path, trimmed\)/,
    );
  });
});
