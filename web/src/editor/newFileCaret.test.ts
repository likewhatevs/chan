import { describe, expect, test } from "vitest";
import source from "./Source.svelte?raw";
import wysiwyg from "./Wysiwyg.svelte?raw";

// A file opened without an explicit caret (File Browser double-click,
// `cs open <file>`) must still land with a usable, focused caret — not
// stay unfocused until the user clicks in. The Draft path (Cmd+N) works
// because it passes initialSelection; plain opens omit it. The fix lives
// in each editor's maybeRestoreCaret(): treat an absent caret as document
// start (0,0) and re-claim focus once content lands, instead of bailing.

const editors: Array<[string, string]> = [
  ["Source.svelte", source],
  ["Wysiwyg.svelte", wysiwyg],
];

describe("new-file caret + focus (no persisted caret)", () => {
  for (const [name, src] of editors) {
    test(`${name}: maybeRestoreCaret no longer bails when no caret is supplied`, () => {
      // The early-return guard must NOT include the !caretPending bail —
      // that is what skipped caret placement + the focus re-claim for
      // plain opens.
      expect(src).not.toMatch(
        /function maybeRestoreCaret\(\): void \{\s*if \([^)]*!caretPending[^)]*\) return;/,
      );
      expect(src).toMatch(
        /function maybeRestoreCaret\(\): void \{\s*if \(caretRestored \|\| !view\) return;/,
      );
    });

    test(`${name}: absent caret defaults to document start (0,0)`, () => {
      expect(src).toMatch(
        /const target = caretPending \?\? \{ from: 0, to: 0 \};/,
      );
    });

    test(`${name}: re-claims focus after placing the caret`, () => {
      // The dispatch + caretRestored + deferred focus must all sit inside
      // maybeRestoreCaret so the content-land path focuses regardless of
      // whether a caret was supplied.
      expect(src).toMatch(
        /function maybeRestoreCaret\(\): void \{[\s\S]*?caretRestored = true;[\s\S]*?requestAnimationFrame\(\(\) => \{[\s\S]*?view\.focus\(\);/,
      );
    });
  }
});
