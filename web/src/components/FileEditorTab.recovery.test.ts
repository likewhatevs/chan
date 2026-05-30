import { describe, expect, test } from "vitest";
import source from "./FileEditorTab.svelte?raw";

// Structural guards on the hang-recovery wiring. The behavior is
// verified in the browser (Svelte reactivity is not exercised by a
// static read), but these lock the invariants that, if reverted,
// reintroduce the false "unsaved changes from a previous session"
// banner or break cross-reload recovery. See editorBuffer.test.ts for
// the decision-logic coverage.
describe("FileEditorTab hang-recovery wiring", () => {
  const src = source.replace(/\s+/g, " ");

  test("recovery decision consumes disk content + saved mtime, not live edits", () => {
    // The divergence check must read `tab.saved` (bound as `saved`) and
    // `tab.savedMtimeNs`. Feeding it `tab.content` would make the effect
    // re-run on every keystroke and surface the user's own in-progress
    // edits as a recovered prior session.
    expect(src).toMatch(
      /divergentBufferOrNull\(\s*tab\.path,\s*tab\.path,\s*saved,\s*tab\.savedMtimeNs/,
    );
  });

  test("recovery + persistence effects wait for the disk load", () => {
    // Until `tab.saved` is defined (and loading finished) the editor
    // holds placeholder content; evaluating recovery or queuing a write
    // then would race the file fetch.
    expect(src).toMatch(/if \(saved === undefined \|\| tab\.loading\) return;/);
  });

  test("clean-state branch preserves a pending recovery buffer", () => {
    // While a recovery banner is offered, the persistence effect must
    // not clear the stored buffer (only when `recoveredBuffer === null`),
    // or a tab switch before the user acts would lose the unsaved work.
    expect(src).toContain("if (recoveredBuffer === null) clearEditorBuffer(tab.path)");
  });

  test("the buffer is keyed on tab.path across every call site", () => {
    // tab ids regenerate on every page load, so a path key is what
    // survives the reload the recovery exists for.
    expect(src).toContain("queueBufferWrite(tab.path, content, tab.path)");
    expect(src).toContain("cancelPendingBufferWrite(tab.path)");
  });

  test("discardBuffer clears by tab.path", () => {
    expect(src).toMatch(
      /function discardBuffer\(\): void \{ clearEditorBuffer\(tab\.path\); recoveredBuffer = null;/,
    );
  });
});
