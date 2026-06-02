import { describe, expect, test } from "vitest";
import { parseErrorPos } from "./mermaid_render";

// mermaid numbers parse errors against the string it parses, which is
// `source.trim()`. parseErrorPos must add back the leading blank lines
// `.trim()` removed so the line lands in the ORIGINAL source.
describe("parseErrorPos", () => {
  const MSG =
    "Parsing failed: Lexer error on line 5, column 5: unexpected character: ->z<-";

  test("pulls line + column from a mermaid lexer error", () => {
    expect(parseErrorPos("pie\n a\n b\n c\n zzzz", MSG)).toEqual({
      line: 5,
      col: 5,
    });
  });

  test("adds back the leading blank lines .trim() dropped", () => {
    // Two leading blank lines: mermaid's "line 5" (in the trimmed text)
    // is line 7 in the original source.
    const source = "\n\npie\n a\n b\n c\n zzzz";
    expect(parseErrorPos(source, MSG)).toEqual({ line: 7, col: 5 });
  });

  test("line without a column still parses", () => {
    expect(parseErrorPos("x", "Error on line 3: boom")).toEqual({
      line: 3,
      col: undefined,
    });
  });

  test("no line in the message -> empty", () => {
    expect(parseErrorPos("x", "Something went wrong")).toEqual({});
  });
});
