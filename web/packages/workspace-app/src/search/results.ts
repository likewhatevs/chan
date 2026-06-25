import type { ContentHit } from "../api/types";

function compareContentHits(a: ContentHit, b: ContentHit): number {
  const byScore = b.score - a.score;
  if (byScore !== 0) return byScore;
  return a.start_line - b.start_line;
}

export function collapseContentHitsByFile(hits: ContentHit[]): ContentHit[] {
  const bestByPath = new Map<string, ContentHit>();
  for (const hit of hits) {
    const prev = bestByPath.get(hit.path);
    if (!prev || compareContentHits(hit, prev) < 0) {
      bestByPath.set(hit.path, hit);
    }
  }
  return Array.from(bestByPath.values()).sort(compareContentHits);
}
