// Unit tests for the CSV parser + serializer. The CSV table
// renderer (#29) round-trips user edits through these helpers, so
// regressions here would corrupt files on save.

import { describe, expect, test } from "vitest";
import { maxRowWidth, parseCsv, serializeCsv } from "./csv";

describe("parseCsv", () => {
  test("empty input returns no rows", () => {
    expect(parseCsv("", ",")).toEqual([]);
  });

  test("simple rectangular CSV", () => {
    const input = "a,b,c\nd,e,f\n";
    expect(parseCsv(input, ",")).toEqual([
      ["a", "b", "c"],
      ["d", "e", "f"],
    ]);
  });

  test("CRLF line endings normalize to one row per record", () => {
    const input = "a,b\r\nc,d\r\n";
    expect(parseCsv(input, ",")).toEqual([
      ["a", "b"],
      ["c", "d"],
    ]);
  });

  test("quoted fields preserve embedded delimiter", () => {
    const input = 'a,"b,c",d\n';
    expect(parseCsv(input, ",")).toEqual([["a", "b,c", "d"]]);
  });

  test("quoted fields preserve embedded newline", () => {
    const input = 'a,"line1\nline2",b\n';
    expect(parseCsv(input, ",")).toEqual([["a", "line1\nline2", "b"]]);
  });

  test("escaped double-quote inside quoted field", () => {
    const input = 'a,"with ""quote""",b\n';
    expect(parseCsv(input, ",")).toEqual([["a", 'with "quote"', "b"]]);
  });

  test("tab delimiter for TSV", () => {
    const input = "a\tb\tc\nd\te\tf\n";
    expect(parseCsv(input, "\t")).toEqual([
      ["a", "b", "c"],
      ["d", "e", "f"],
    ]);
  });

  test("ragged rows preserved verbatim", () => {
    const input = "a,b,c\nd,e\nf\n";
    expect(parseCsv(input, ",")).toEqual([
      ["a", "b", "c"],
      ["d", "e"],
      ["f"],
    ]);
  });

  test("missing trailing newline still emits the last row", () => {
    const input = "a,b\nc,d";
    expect(parseCsv(input, ",")).toEqual([
      ["a", "b"],
      ["c", "d"],
    ]);
  });

  test("empty fields preserved", () => {
    const input = ",,\na,,c\n";
    expect(parseCsv(input, ",")).toEqual([
      ["", "", ""],
      ["a", "", "c"],
    ]);
  });
});

describe("serializeCsv", () => {
  test("empty rows array returns empty string", () => {
    expect(serializeCsv([], ",")).toBe("");
  });

  test("simple rectangular round-trip", () => {
    const rows = [
      ["a", "b", "c"],
      ["d", "e", "f"],
    ];
    expect(serializeCsv(rows, ",")).toBe("a,b,c\nd,e,f\n");
  });

  test("fields with delimiter get quoted", () => {
    const rows = [["a", "b,c", "d"]];
    expect(serializeCsv(rows, ",")).toBe('a,"b,c",d\n');
  });

  test("fields with embedded newline get quoted", () => {
    const rows = [["a", "line1\nline2", "b"]];
    expect(serializeCsv(rows, ",")).toBe('a,"line1\nline2",b\n');
  });

  test("fields with embedded double-quote get quoted + escaped", () => {
    const rows = [["a", 'with "quote"', "b"]];
    expect(serializeCsv(rows, ",")).toBe('a,"with ""quote""",b\n');
  });

  test("plain fields stay unquoted", () => {
    const rows = [["hello", "world"]];
    expect(serializeCsv(rows, ",")).toBe("hello,world\n");
  });

  test("tab delimiter uses tab separator", () => {
    const rows = [
      ["a", "b"],
      ["c", "d"],
    ];
    expect(serializeCsv(rows, "\t")).toBe("a\tb\nc\td\n");
  });
});

describe("parseCsv -> serializeCsv round-trip", () => {
  const samples = [
    "a,b,c\nd,e,f\n",
    'a,"b,c",d\n',
    'a,"with ""quote""",b\n',
    'a,"line1\nline2",b\n',
    ",,\na,,c\n",
  ];
  for (const sample of samples) {
    test(JSON.stringify(sample), () => {
      expect(serializeCsv(parseCsv(sample, ","), ",")).toBe(sample);
    });
  }
});

describe("maxRowWidth", () => {
  test("returns the widest row's length", () => {
    expect(
      maxRowWidth([
        ["a"],
        ["a", "b", "c"],
        ["a", "b"],
      ]),
    ).toBe(3);
  });

  test("empty array returns 0", () => {
    expect(maxRowWidth([])).toBe(0);
  });
});
