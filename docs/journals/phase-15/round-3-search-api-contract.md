# Round-3 search-API contract (A<->B seam, @@Architect-held)

Theme 4: "search understands mentions / paths". @@LaneA ran the PROBE the plan
called for; this pins what @@LaneB's SearchPanel can rely on. Author @@LaneA.

## PROBE result (live, 2026-06-01)

Served a seeded drive (mentions, real paths, .md refs, plus topical
distractors), embeddings built (10 docs / 10 vectors, BGE-small bundled), and
queried `/api/search/content`:

| query                                   | hits before | why                              |
|-----------------------------------------|-------------|----------------------------------|
| `@@LaneA`                               | 0           | tokenizer strips `@`             |
| `path/to/file`                          | 0           | (no doc had all 3 words)         |
| `.md`                                   | 0           | strips `.`                       |
| `search.rs`                             | 0           | strips `.`                       |
| `bootstrap.md`                          | 0           | strips `.`                       |
| `LaneA` / `routes` / `bootstrap` (bare) | hit         | bare words tokenize fine         |

Two findings:

1. SEMANTIC IS NOT ON THE QUERY PATH. Every search reported `mode:"bm25"`.
   `SearchOpts::default().mode == Bm25` (workspace.rs:103, facade Mode default
   is Bm25), and the route builds `SearchOpts { ..Default::default() }`
   (routes/search.rs ~180) - its "defaults to Hybrid" comment is STALE. The
   `chan search` CLI is BM25-only too. The dense vectors are built and stored
   but NO user-facing caller ever requests Hybrid/Semantic. So @@Host's "maybe
   we already cover mentions/paths with semantic" is empirically FALSE: semantic
   never runs at query time. (Flagged to @@Host as a separate product question -
   it is NOT this contract's scope to flip hybrid on.)

2. The gap is pure punctuation tokenization, NOT a missing analyzer. The bare
   words already match; only the literal `@`/`/`/`.` in the prefix regex made
   the punctuated forms return nothing.

## FIX (landed in @@LaneA's bm25.rs, server-internal, no reindex)

`try_build_prefix_query` now splits each whitespace token into the alphanumeric
subtokens tantivy's default tokenizer produced from the indexed text, and ANDs
them. So `@@LaneA` -> `lanea`; `notes/file.md` -> `notes` AND `file` AND `md`;
`src/routes/search.rs` -> `src` AND `routes` AND `search` AND `rs`. A bare word
yields one subtoken, so ordinary queries are byte-identical to before. Snippet
highlighting uses the same subtokens. Hyphenated queries still route to the
QueryParser (unchanged). Unit tests: mention / path / filename + the helper.

## CONTRACT for @@LaneB (SearchPanel)

THE RESPONSE SHAPE IS UNCHANGED. `GET /api/search/content?q=&limit=&scope=`
still returns:

    { ready: bool, mode: "bm25", hits: [ {path, chunk_id, heading,
      start_line, snippet, score} ] }

What changes is server-internal and transparent: a query the user types as
`@@handle`, `a/path/file.md`, or `name.md` now returns relevant hits instead of
an empty list.

=> @@LaneB's search FE is effectively DISPLAY-ONLY (the same conclusion the lane
doc reached for the "semantic already covers it" branch, but for a different
reason). DO NOT client-side parse / special-case mentions or paths and DO NOT
strip punctuation before sending - pass the user's raw query straight through
(SearchPanel already does). The server handles the shapes.

OPTIONAL (FE-only, your call, not required): a subtle affordance when the query
looks like a mention/path - e.g. a tiny "keyword match" hint - is fine, but it
is polish, not a contract dependency. Ship the round without it if time is
short.

NON-GOAL this round: node-level mention precision (distinguishing an `@@handle`
mention node from the bare word). That is a graph concern, out of Theme-4 BM25
scope. `@@LaneA` and `LaneA` are equivalent queries after this fix - correct for
a notes search box.
