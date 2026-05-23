import { describe, expect, test } from "vitest";
import tree from "./FileTree.svelte?raw";

// `fullstack-a-67e`: FileTree in-tree selection menu reshape
// per addendum-a's FB selection-menu spec. Slice 1 covers the
// "From selection" header, the New Graph entry, the relabels
// (Search this → Search; Terminal from here → New Terminal),
// and the visual separator between workflow + per-row ops.
// The unified "New File or Directory" dialog (one input that
// detects file vs dir) is deferred to slice 2 — needs a
// `kind: "either"` extension to PathPromptModal.

describe("fullstack-a-67e: FileTree selection menu header + new entries", () => {
  test("From-selection label rendered at the top of the ctx menu", () => {
    expect(tree).toMatch(
      /\{#if menu\}[\s\S]{1,2000}<div class="from-selection-label">From selection<\/div>/,
    );
  });

  test("Search label relabelled (was \"Search this\")", () => {
    expect(tree).toMatch(/<span>Search<\/span>/);
    expect(tree).not.toMatch(/<span>Search this<\/span>/);
  });

  test("New Terminal label relabelled (was \"Terminal from here\")", () => {
    expect(tree).toMatch(/<span>New Terminal<\/span>/);
    expect(tree).not.toMatch(/<span>Terminal from here<\/span>/);
  });

  test("New File / New Directory entries kept (gated on isDir)", () => {
    expect(tree).toMatch(/<span>New File<\/span>/);
    expect(tree).toMatch(/<span>New Directory<\/span>/);
  });

  test("New Graph entry added, routes to graphThis", () => {
    expect(tree).toMatch(
      /onclick=\{\(\) => graphThis\(menu!\.path, menu!\.isDir\)\}[\s\S]{1,400}<span>New Graph<\/span>/,
    );
  });

  test("ctx-sep separator between workflow + per-row ops", () => {
    expect(tree).toMatch(
      /<span>New Graph<\/span>[\s\S]{1,400}<div class="ctx-sep" role="separator"><\/div>[\s\S]{1,400}<span>Copy Path<\/span>/,
    );
  });
});

describe("fullstack-a-67e: per-row ops kept (Copy Path / Rename / Delete)", () => {
  test("Copy Path / Rename / Move / Delete labels preserved", () => {
    expect(tree).toMatch(/<span>Copy Path<\/span>/);
    expect(tree).toMatch(/<span>Rename \/ Move<\/span>/);
    expect(tree).toMatch(/<span>Delete<\/span>/);
  });
});
