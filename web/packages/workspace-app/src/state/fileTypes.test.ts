import { describe, expect, test } from "vitest";

import { classifyPath, isEditableText, isExcalidraw, isMarkdown } from "./fileTypes";

// The path-only classifier is the fallback the editor uses when it holds a
// bare path without a server-projected `kind` (graph ghost rows, broken
// link targets). It must mirror the server's `project_kind`: only Markdown
// (.md) is the `document` wire kind; .txt is `text` alongside source files,
// even though the editor still renders .txt as markdown.
describe("classifyPath: only .md is a document", () => {
  test("md maps to document, txt + source map to text", () => {
    expect(classifyPath("notes/a.md")).toBe("document");
    expect(classifyPath("notes/plain.txt")).toBe("text");
    expect(classifyPath("src/main.rs")).toBe("text");
    expect(classifyPath("README")).toBe("text");
    expect(classifyPath("logo.png")).toBe("media");
    expect(classifyPath("doc.pdf")).toBe("media");
    expect(classifyPath("archive.zip")).toBe("binary");
  });

  test("the document/text split does not change the editor's view of .txt", () => {
    // .txt stays editable and still renders through the markdown editor;
    // only its graph/wire identity changed.
    expect(isEditableText("notes/plain.txt")).toBe(true);
    expect(isMarkdown("notes/plain.txt")).toBe(true);
    expect(isMarkdown("notes/a.md")).toBe(true);
    expect(isMarkdown("src/main.rs")).toBe(false);
  });
});

describe("excalidraw scenes are editable text", () => {
  test("an .excalidraw path classifies as editable text, not binary", () => {
    // .excalidraw is JSON on disk, so it joins TEXT_EXTENSIONS in lockstep
    // with the server: the write gate then accepts CREATING a new board.
    // The tab opens it in canvas mode; the wire kind stays plain `text`.
    expect(classifyPath("draw/board.excalidraw")).toBe("text");
    expect(isEditableText("draw/board.excalidraw")).toBe(true);
    // Not markdown-class, so it never renders through the markdown editor.
    expect(isMarkdown("draw/board.excalidraw")).toBe(false);
  });

  test("isExcalidraw keys strictly on the .excalidraw extension", () => {
    expect(isExcalidraw("draw/board.excalidraw")).toBe(true);
    expect(isExcalidraw("BOARD.EXCALIDRAW")).toBe(true);
    expect(isExcalidraw("draw/board.excalidraw.bak")).toBe(false);
    expect(isExcalidraw("notes/a.md")).toBe(false);
    expect(isExcalidraw("data.json")).toBe(false);
  });
});
