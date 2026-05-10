// Synthetic GraphView for the standalone graph-force playground.
//
// Mirrors what `chan-drive` produces in production: file nodes
// (documents + images), tag nodes, mention nodes, date nodes; edges
// are link / tag / mention / date.
//
// Deterministic via a tiny seeded PRNG so reload yields the same
// shape unless the seed input changes — handy when comparing two
// force tunings against an identical graph.

import type { GraphView, GraphViewEdge, GraphViewNode } from "../api/types";

export type GraphSpec = {
  seed: number;
  documents: number;
  images: number;
  tags: number;
  mentions: number;
  dates: number;
  // Probability a given document references another document.
  // Outgoing wiki-link edges per doc are drawn from Binomial(documents-1, p).
  linkDensity: number;
  // Probability a given document references each image.
  imageRefDensity: number;
  // Average tags per document.
  tagsPerDoc: number;
  // Average mentions per document.
  mentionsPerDoc: number;
  // Average date stamps per document.
  datesPerDoc: number;
};

export const defaultSpec: GraphSpec = {
  seed: 1,
  documents: 28,
  images: 7,
  tags: 10,
  mentions: 5,
  dates: 4,
  linkDensity: 0.06,
  imageRefDensity: 0.06,
  tagsPerDoc: 1.6,
  mentionsPerDoc: 0.4,
  datesPerDoc: 0.3,
};

function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const DOC_NAMES = [
  "intro", "design", "tour", "roadmap", "ideas", "weekly",
  "tour-mac", "tour-ios", "release-notes", "todo", "shopping",
  "errands", "deep-work", "reading", "watchlist", "travel-jp",
  "travel-it", "kitchen", "garden", "music", "photo-trip",
  "snippets", "regex-cheatsheet", "ssh-tips", "rust-notes",
  "go-notes", "graph-spec", "search-spec",
];
const TAG_NAMES = [
  "todo", "idea", "wip", "review", "ship", "later",
  "personal", "work", "research", "snippet",
];
const MENTION_NAMES = [
  "alice", "bob", "carol", "dave", "erin",
];
const DATE_NAMES = [
  "2026-01-12", "2026-02-04", "2026-03-19", "2026-05-02",
];

export function makeFakeGraph(spec: GraphSpec = defaultSpec): GraphView {
  const rng = mulberry32(spec.seed);
  const nodes: GraphViewNode[] = [];
  const edges: GraphViewEdge[] = [];

  const docIds: string[] = [];
  for (let i = 0; i < spec.documents; i++) {
    const name = DOC_NAMES[i % DOC_NAMES.length] +
      (i >= DOC_NAMES.length ? `-${Math.floor(i / DOC_NAMES.length)}` : "");
    const path = `${name}.md`;
    const id = `file:${path}`;
    docIds.push(id);
    nodes.push({ kind: "file", id, label: name, path });
  }

  const imageIds: string[] = [];
  for (let i = 0; i < spec.images; i++) {
    const name = `img-${(i + 1).toString().padStart(2, "0")}`;
    const path = `assets/${name}.png`;
    const id = `file:${path}`;
    imageIds.push(id);
    nodes.push({ kind: "file", id, label: `${name}.png`, path });
  }

  const tagIds: string[] = [];
  for (let i = 0; i < spec.tags; i++) {
    const name = TAG_NAMES[i % TAG_NAMES.length] +
      (i >= TAG_NAMES.length ? `-${Math.floor(i / TAG_NAMES.length)}` : "");
    const id = `tag:${name}`;
    tagIds.push(id);
    nodes.push({ kind: "tag", id, label: `#${name}` });
  }

  const mentionIds: string[] = [];
  for (let i = 0; i < spec.mentions; i++) {
    const name = MENTION_NAMES[i % MENTION_NAMES.length];
    const id = `mention:${name}`;
    mentionIds.push(id);
    nodes.push({ kind: "mention", id, label: `@${name}` });
  }

  const dateIds: string[] = [];
  for (let i = 0; i < spec.dates; i++) {
    const name = DATE_NAMES[i % DATE_NAMES.length];
    const id = `date:${name}`;
    dateIds.push(id);
    nodes.push({ kind: "date", id, label: name });
  }

  const seenLink = new Set<string>();
  const pushLink = (src: string, tgt: string, broken = false) => {
    if (src === tgt) return;
    const k = `${src}|${tgt}|link`;
    if (seenLink.has(k)) return;
    seenLink.add(k);
    edges.push(
      broken
        ? { source: src, target: tgt, kind: "link", broken }
        : { source: src, target: tgt, kind: "link" },
    );
  };

  // Doc -> doc wiki links.
  for (const src of docIds) {
    for (const tgt of docIds) {
      if (rng() < spec.linkDensity) pushLink(src, tgt);
    }
  }

  // Doc -> image embeds.
  for (const src of docIds) {
    for (const tgt of imageIds) {
      if (rng() < spec.imageRefDensity) pushLink(src, tgt);
    }
  }

  // A couple of broken wiki links to ghost files, to exercise the
  // dashed-edge style.
  for (let i = 0; i < 2; i++) {
    const ghostPath = `missing-${i}.md`;
    const ghostId = `file:${ghostPath}`;
    nodes.push({
      kind: "file", id: ghostId, label: ghostPath,
      path: ghostPath, missing: true,
    });
    const src = docIds[Math.floor(rng() * docIds.length)];
    if (src) pushLink(src, ghostId, true);
  }

  // Doc -> tag/mention/date attachments.
  const attachAvg = (src: string, pool: string[], avg: number, kind: GraphViewEdge["kind"]) => {
    if (pool.length === 0) return;
    // Poisson-ish: draw `avg + jitter` rounded, capped to pool size.
    const n = Math.max(0, Math.min(pool.length, Math.round(avg + (rng() - 0.5) * 2)));
    const picks = new Set<number>();
    while (picks.size < n) picks.add(Math.floor(rng() * pool.length));
    for (const i of picks) {
      const tgt = pool[i];
      if (tgt) edges.push({ source: src, target: tgt, kind });
    }
  };
  for (const src of docIds) {
    attachAvg(src, tagIds, spec.tagsPerDoc, "tag");
    attachAvg(src, mentionIds, spec.mentionsPerDoc, "mention");
    attachAvg(src, dateIds, spec.datesPerDoc, "date");
  }

  return { nodes, edges };
}
