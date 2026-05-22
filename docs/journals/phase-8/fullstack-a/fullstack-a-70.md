# fullstack-a-70 — Editor mention/matching gap (@@<Name> not matched even when @@ exists elsewhere in repo)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Fix the mention-matching gap surfaced by @@Alex
in [`../alex/addendun-a.md`](../alex/addendum-a.md):

> Mentions and matching: we've got tons of tags to my
> name in this repo and yet when I'm on the text
> editor I get this: ![](./image.png#w=250)

Screenshot `addendun-a/image.png` shows the editor's
mention completion / matching feature failing on a
name that has many existing references in the repo.

## Reference

* `addendun-a.md` "## Bugs" — mentions item.
* Screenshot at `docs/journals/phase-8/alex/image.png`.

## Audit hooks

1. Inspect the editor's mention-completion path —
   most likely lives in the WYSIWYG / Source editor
   extensions (`web/src/editor/*`).
2. Trace the data source: does the matcher query
   chan-server (mention-index) or only local
   in-file state?
3. Compare expected behavior (auto-complete on `@@`
   from the corpus of known handles) vs observed
   (the screenshot shows no match).

Likely root cause: matcher only looks at a stale
local cache OR doesn't include the mention-index that
chan-server emits via the graph route.

## Acceptance

1. **Typing `@@<partial>` in the editor** brings up
   matches from the existing mention corpus (the same
   handles that contributed to the deduped contact
   nodes via `systacean-22`).
2. **Match list includes all 49+ unique handles** in
   the chan-source seed.
3. **Selection completes** the mention to the chosen
   handle.

### Tests

Vitest pin on the mention-completion query + match
list shape.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA SPA primary.
* If audit reveals chan-server emit gap (e.g. mention
  corpus not exposed), fire scope poke + I route the
  chan-server piece.

## Authorization

Yes for editor + mention-completion SPA files + tests
+ task tail + outbound. Scope-poke if chan-server
changes needed.

## Numbering

This is `-a-70`.

## 2026-05-22 — audit findings + scope-poke (chan-server gap; no SPA change yet)

Audited the editor mention-completion path
end-to-end. Findings + scope-poke routed to
architect for @@Systacean lane.

### Audit summary

**Symptom**: typing `@@<partial>` in the
editor brings up zero matches even when the
repo has many `@@<token>` mentions in body
text.

**Trigger surface**: `web/src/editor/bubbles/contact.ts::openContactBubble`,
called from the WYSIWYG / Source extensions
when the user types `@@`. The bubble lists
candidates fetched from `api.contacts(query,
limit)` at `web/src/api/client.ts:218`.

**Data source the picker hits**:
`GET /api/contacts?q=<...>&limit=<...>` →
chan-server returns `Array<{path, label,
emails?, aliases?}>`. Each item is a
**contact file** (a markdown note with
`chan.kind: contact` frontmatter).

**The gap**: mentions in the body text of
the repo (`@@Alex` written in a journal,
say) are NOT backed by a contact file
unless the user has explicitly created one.
The picker queries contact files only;
free-form mentions are invisible to it.

**Where the mention corpus DOES live**:
chan-server's `/api/graph` route walks
edges of `EdgeKind::Mention` and builds a
`mention_set: BTreeSet<String>` of distinct
mention dst tokens
(`crates/chan-server/src/routes/graph.rs:1142-1163`).
These become `GraphNodeView::Mention { id,
label }` nodes in the graph payload. But
they're only reachable via the heavy graph
query; no dedicated picker-friendly
endpoint exposes them.

### Routing decision

The SPA can't close the gap alone — the
mention corpus lives in chan-drive's graph
db; the SPA needs an endpoint to query it.
Three shapes possible:

1. **New `/api/mentions?q=<prefix>&limit=...`
   endpoint** — chan-server side. Walks the
   same `EdgeKind::Mention` aggregation as
   `/api/graph` but returns a flat
   `Array<{label: string}>` (or
   `Array<{label, count}>` if ranking by
   reference count is cheap). The SPA's
   contact bubble unions the contact-file
   hits with the mention-token hits.
2. **Extend `/api/contacts` to include
   mention tokens as a "ghost contact" tier**
   — wider blast radius (changes the
   contact-file shape consumers depend on).
3. **SPA queries `/api/graph` once per
   editor session + filters in-memory** —
   heavy; the graph payload can be MB-scale
   on the chan-source seed; not viable for
   a keystroke-driven picker.

**Routing #1.** Cleanest seam + matches the
existing `/api/contacts` shape so the SPA
union is straightforward.

### Scope-poke to @@Systacean (via architect)

`crates/chan-server/src/routes/graph.rs`
(or a new `routes/mentions.rs`):

* New handler `api_mentions(state, query)
  -> Response`.
* Query params: `?q=<prefix>&limit=<int>`.
  `q` filters case-insensitively by prefix;
  `limit` caps the response (mirror
  `/api/contacts`'s default 10).
* Implementation:
  1. Open `drive.graph()`.
  2. List files via `graph.files()`.
  3. For each file, walk
     `graph.neighbors(f)` filtering to
     `EdgeKind::Mention`. Collect distinct
     `e.dst` strings into a `BTreeSet`.
  4. Filter the set by case-insensitive
     prefix match on `q` (empty `q` returns
     all).
  5. Sort by label asc; cap at `limit`.
  6. Return `Array<{label: string}>` JSON.
* Wire the route at `/api/mentions` in
  `lib.rs`.
* Re-export from `routes/mod.rs`.
* Rust pin asserting the route returns the
  observed mention tokens after a synthetic
  fixture insert.

Performance note: the full file-walk +
neighbor enumeration is O(F × E) where F is
file count + E is edges-per-file. For the
chan-source seed (~1973 files) the per-call
cost may be noticeable; if too slow,
@@Systacean's call to lift the
mention-extraction into the indexer's
boot-time pass (cache the set on the
graph handle).

### Follow-up SPA side (after the endpoint lands)

Once `/api/mentions` lands, the SPA work is
small:
* `api.mentions(q, limit)` client method.
* `openContactBubble` queries BOTH
  `api.contacts(q, limit)` AND
  `api.mentions(q, limit)` in parallel;
  merges the results (contact-file hits
  first, mention-token hits after,
  deduped). Threshold: top 8 across both
  per the existing `PAGE_LIMIT = 8`.
* Visual: maybe a dim style for
  mention-only hits to signal "no contact
  file" vs "first-class contact". Optional
  polish.

### No commit this round

Audit-only. The deliverable is:
* This impl note documenting the gap +
  routing decision.
* Outbound poke to architect for
  @@Systacean routing of the
  `/api/mentions` endpoint.

No SPA code change yet — the consumer wiring
shape is gated on the endpoint shape. Cheap
to wire once @@Systacean's PR lands.

### Acceptance (pending chan-server piece)

1. Typing `@@<partial>` brings up matches
   from the mention corpus ✓ (once
   endpoint lands + SPA merges).
2. Match list includes all 49+ unique
   handles ✓ (mention_set already exposes
   them via /api/graph; the new endpoint
   surfaces the same set).
3. Selection completes the mention ✓
   (existing bubble behaviour; the merged
   result list just needs the token string).

### Suggested commit subject (when shipping)

```
docs(fullstack-a-70): audit + scope-poke for chan-server mention corpus endpoint
```

### Files for `git add` (per-path discipline)

* `docs/journals/phase-8/fullstack-a/fullstack-a-70.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for the chan-server
endpoint landing + the SPA-side follow-up.
