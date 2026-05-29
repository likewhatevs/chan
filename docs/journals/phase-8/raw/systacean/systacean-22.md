# systacean-22 — Contact-node dedup audit + fix (1973 vs 49 = ~40x over-emission); plus optional GraphNodeView::File bucket emit

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two chan-server-side graph-emit hygiene items, one
load-bearing + one nice-to-have:

1. **Contact-node dedup** (PRIMARY): the graph view
   surfaces **1973 contact nodes** on the chan repo
   seed, vs only **49 unique `@@<Name>` handles** in
   the entire `docs/` tree. ~40x over-emission ratio.
   Strong evidence the contact node dedup is broken.
2. **Optional cleanup**: add `bucket: Option<FileBucket>`
   to `GraphNodeView::File` so SPA can stop running
   client-side regex classification. From `-a-57`'s
   audit-finding: SPA currently classifies via
   `classifyFile` because the graph emit doesn't
   carry the bucket; cleanup would put truth on the
   server side.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "Contact-node
count seems anomalously high (1973 contacts on the chan
repo seed); audit for over-emission / dedup gap" —
full bug body with hypotheses + investigation hooks +
architect-side spot-check (49 unique handles in `docs/`).

## Audit-first shape (PRIMARY: contact dedup)

### Step 1 — Inspect the wire

1. Spin up a test server against the chan repo seed.
2. `curl http://127.0.0.1:<port>/api/graph?scope=drive`
   + filter the response for contact / mention nodes.
3. Count unique `id` values vs total node count.
4. Sample 5-10 contact nodes; check their `id` shape
   — is `@@Architect` keyed once, or N times (where N
   is the number of files / occurrences)?
5. Verdict at task tail: per-handle / per-file /
   per-occurrence emission shape.

### Step 2 — Fix per audit outcome

* **If per-occurrence**: dedup key should be the
  handle string itself; collapse to 1 node per unique
  handle + N edges (one per mention site).
* **If per-file**: dedup needs to extend across files
  (handle string is the global key; file path becomes
  an edge attribute or mention-edge source).
* **If something else**: audit reveals the right
  semantic.

### Step 3 — Cross-check

After fix, the chan repo seed should display ~49
contact nodes (matching the unique-handle count).

## Optional secondary scope: GraphNodeView::File bucket emit

From `-a-57`'s audit-finding (file
`fullstack-a-57.md:104-127`): chan-server's
`GraphNodeView::File` emits id + label + path +
path_class + node_kind + missing — but no `bucket`
field. The chan-report `FileStats.bucket` exists from
`-16` but doesn't propagate to the graph route.

Add `bucket: Option<FileBucket>` to
`GraphNodeView::File`. Populate from the underlying
`chan_drive::Drive::report()` data already consulted
in the graph route. Backward-compat (optional field;
serde-skip-when-None).

This unblocks @@FullStackA from removing the
client-side `classifyFile` regex helper in a future
SPA polish task. Not load-bearing for users; reduces
the SPA's duplicated logic.

**Implementer's choice**: ship contact-dedup alone OR
bundle the bucket emit. If audit reveals the contact-
dedup fix touches the same graph-route code as the
bucket-emit addition, bundle. Otherwise ship the
contact-dedup standalone + flag the bucket-emit for a
follow-up.

## Acceptance criteria

### Contact dedup (load-bearing)

1. **Wire audit verdict** appended to task tail
   identifying the over-emission shape.
2. **Fix lands**: `/api/graph?scope=drive` against the
   chan repo seed displays ~49 contact nodes (matching
   the unique-handle count from
   `grep -rEoh '@@[A-Z][a-zA-Z0-9]+' docs/ | sort -u | wc -l`).
3. **Mention edges preserved**: each occurrence of a
   `@@Handle` mention still produces a mention edge
   to the deduped contact node (so the graph's "who
   mentions whom" view stays meaningful).

### Optional bucket emit (if bundled)

4. `GraphNodeView::File` carries
   `bucket: Option<FileBucket>` populated from chan-
   report stats.
5. Backward-compat: missing field deserializes as
   None.

### Tests

* New test: contact-dedup keying — fixture drive with
  multiple files mentioning `@@A` + `@@B`; assert 2
  contact nodes + N mention edges.
* (Optional) test: GraphNodeView::File `bucket` field
  populated for markdown + source fixtures.

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-server`,
  `RUSTFLAGS="-D warnings" cargo build
  --no-default-features` all green.
* CI smoke via `gh workflow run ci.yml --ref
  systacean-22-smoke` on a fresh smoke branch.

## Coordination

* @@Systacean lane (chan-server graph route owner).
* Atomic-audit-commit discipline.
* If audit reveals the fix needs chan-drive-side
  mention extraction changes (not just graph route),
  fire scope poke + I route the cross-lane piece.

## Authorization

**Yes** for `crates/chan-server/src/routes/graph.rs`
(contact dedup + optional bucket emit) + related
tests + task tail + outbound. If chan-drive-side
mention extraction needs touching: scope-poke first.

## Numbering

Highest committed `systacean-N` is `-21` (cache-bust
enrich-poke). This is `-22`.

## Out of scope

* SPA-side rendering of contact nodes (lane: @@FullStackA
  if surface-only).
* Mention-edge semantics (link kinds, rendering) — that's
  rendering polish, not data shape.
* Parent-edge invariant fix (separate task `-a-58` on
  @@FullStackA's lane).
* Ghost-node parent invariant (filed; rides with `-a-58`).

## 2026-05-22 — audit verdict + Option A fix + bucket emit bundled

Architect accepted Option A (filter unreferenced contacts) + bundled the optional bucket emit. See [`../alex/event-architect-systacean.md`](../alex/event-architect-systacean.md) "ACCEPT Option A" routing.

### Audit verdict (recap)

The task body's hypothesis was per-occurrence mention dedup gap. My empirical throwaway-drive test ruled that out:
* 8912 raw `@@Handle` occurrences across docs/.
* ~50 unique handle strings.
* 47 mention nodes emitted (mention_set HashSet dedup IS working).
* Adding 1 contact-frontmatter file → 1 contact File node. Per-file emission, not per-occurrence.

So @@Alex's 1973 contact nodes ≈ 1973 imported contact files in their `contacts/` directory. The over-emission is **scope** (unreferenced contacts get nodes), not **dedup**.

### Fix shape (Option A)

In `crates/chan-server/src/routes/graph.rs::api_graph`:

1. **Collect `referenced_contact_paths`** during the existing mention-edge rewrite loop. When `mention_to_contact.get(&stripped)` resolves a mention to a contact file path, insert that path into a new `HashSet<String>`.
2. **Filter at per-file emit**: before constructing a `GraphNodeView::File` for any path, call `should_emit_contact_file(path, &contact_paths, &referenced_contact_paths)`. Plain non-contact files always pass; contact files pass only when in `referenced_contact_paths`.
3. **Helper function extracted**: `fn should_emit_contact_file(path, contact_paths, referenced) -> bool` lives at module scope (alongside `is_image_path` / `drive_disk_files` / etc.) so it's unit-testable directly.

### Bucket emit bundle (optional scope from task body)

Added `bucket: Option<ReportFileBucket>` to `GraphNodeView::File`:

* Re-export `FileBucket as ReportFileBucket` from chan-drive (already present from `-16`).
* Build a `report_buckets: HashMap<String, ReportFileBucket>` lookup once at the top of api_graph from `drive.report()`.
* Populate the field at the per-file emit + at the referenced-disk-files emit. Ghosts + fs-graph merge sites get `bucket: None` (no real file data to consult).
* Backward-compat: optional + serde-skip-when-None; missing field deserializes as None.

### Empirical verification (throwaway drive)

Set up `/tmp/chan-22-audit/` with full `docs/` copy + 2 contact files (`contacts/alice.md` + `contacts/bob.md` synthesized via `chan.kind: contact` frontmatter). Tested:

**Pre-fix (`chan` binary from main HEAD)**:
```
total: 791 nodes
  file: 661 (1 contact File node — alice; bob's index hadn't propagated yet)
  mention: 47
  ... etc
```

**Post-fix (rebuilt `chan` binary)**:
```
total: 794
  file: 664
  mention: 47
  file node_kind buckets: {None: 664}
  contact nodes: 0  ← bob + alice + (later) charlie all dropped (unreferenced)
  bucket emit kinds: {'markdown': 554, 'source_code': 3}  ← 557 of 664 files have buckets
```

Then added `test-mention-alice.md` with `@@alice` body, reindexed:
```
contact nodes after @@alice mention added: 1; paths: ['contacts/alice.md']  ← referenced contact emits
```

✅ Filter works as designed. Bucket emit populates for tracked files; missing for non-tokei files (binary, images, etc.).

### Teardown audit

* `chan serve` PIDs spawned + killed by captured PID (no `pkill`).
* `/tmp/chan-22-audit/` removed.
* Port 8866 freed; verified via `lsof`.
* `chan remove /tmp/chan-22-audit` already not-registered (drive wasn't in the registry after teardown).
* @@Alex's chan.app + registered drives **untouched** (verified via `pgrep -fl "chan serve"` showing only `/Users/fiorix/Documents/ChanRoadmap` on port 63701, which is theirs).

### Tests added

* `should_emit_contact_file_drops_unreferenced_keeps_referenced_and_non_contacts` — pure unit test on the filter helper. Pins: alice (referenced) emits; bob + charlie (unreferenced contacts) drop; `notes/intro.md` + `src/lib.rs` (non-contacts) always emit.
* `contact_dedup_end_to_end_drops_unreferenced_imported_contacts` — end-to-end: fixture drive with 3 contact files + 1 markdown mentioning `@@alice`. Replays the api_graph mention-edge resolution → builds `referenced_contact_paths` → asserts the filter behaves per the helper. Pins the full chain so regressions on either the mention-edge resolution OR the filter helper get caught.

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server`: **211 passed; 0 failed** (was 209 pre-`-22`; +2 new tests).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                       | +    | -   |
|--------------------------------------------|------|-----|
| `crates/chan-server/src/routes/graph.rs`   | +234 | -6  |

Plus task tail + outbound poke. 3 paths total. Foreign files in dirty tree stay un-staged.

### Suggested commit subject

```
chan-server: filter unreferenced contact File nodes + emit FileBucket on graph nodes (systacean-22)
```

### Smoke plan

Atomic-audit-commit pattern + push to fresh `systacean-22-smoke` branch + `gh workflow run ci.yml --ref systacean-22-smoke`. Expected: all 5 jobs green (rustfmt + web + Ubuntu + macOS + no-default-features build). Backward-compat schema change; no risk of unexpected reds.

Holding for @@Architect commit clearance + smoke-branch authorization.
