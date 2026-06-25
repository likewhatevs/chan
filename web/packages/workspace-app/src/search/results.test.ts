import { describe, expect, test } from "vitest";
import type { ContentHit } from "../api/types";
import { collapseContentHitsByFile } from "./results";

function hit(
  path: string,
  chunk_id: string,
  heading: string,
  score: number,
  start_line = 0,
): ContentHit {
  return {
    path,
    chunk_id,
    heading,
    score,
    start_line,
    snippet: `${path}:${heading}`,
  };
}

describe("collapseContentHitsByFile", () => {
  test("keeps only the strongest section hit per file", () => {
    const hits = [
      hit("notes/a.md", "a-1", "Intro", 3),
      hit("notes/a.md", "a-2", "Details", 7),
      hit("notes/b.md", "b-1", "Plan", 5),
    ];

    expect(collapseContentHitsByFile(hits)).toEqual([
      hit("notes/a.md", "a-2", "Details", 7),
      hit("notes/b.md", "b-1", "Plan", 5),
    ]);
  });

  test("uses the earlier section when scores tie within a file", () => {
    const hits = [
      hit("notes/a.md", "a-2", "Later", 4, 20),
      hit("notes/a.md", "a-1", "Earlier", 4, 10),
    ];

    expect(collapseContentHitsByFile(hits)).toEqual([
      hit("notes/a.md", "a-1", "Earlier", 4, 10),
    ]);
  });

  test("returns collapsed file hits in ranked order", () => {
    const hits = [
      hit("notes/a.md", "a-1", "A", 2),
      hit("notes/b.md", "b-1", "B", 5),
      hit("notes/c.md", "c-1", "C", 3),
      hit("notes/b.md", "b-2", "B2", 1),
    ];

    expect(collapseContentHitsByFile(hits).map((h) => h.path)).toEqual([
      "notes/b.md",
      "notes/c.md",
      "notes/a.md",
    ]);
  });
});
