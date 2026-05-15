# Kind taxonomy + editor widening plan

Phased rollout of a unified `FileKind` taxonomy across chan + chan-core
plus widening the editor to open any non-binary file. Phases 0-4 are
landed in code; browser verification has not happened. Phase 5 (alias
resolution) is the remaining piece and has open scope questions.

Touches across: `crates/chan-server/src/routes/files.rs`,
`crates/chan-drive/src/{fs_ops.rs,drive.rs,index/facade.rs,tests/file_types.rs}`,
`web/src/{state/{kinds.ts,fileTypes.ts,tabs.svelte.ts,pdfViewer.ts},
api/types.ts,design.md,components/{KindChip,FileEditorTab,FileInfoBody,
FileTree,SearchPanel,GraphPanel,TagInfoBody}.svelte,
editor/{Source.svelte,JsonPretty.svelte,JsonNode.svelte,CsvTable.svelte,
csv.ts,csv.test.ts,bubbles/{heic.ts,image.ts,image_drop.ts}}}`.

---

## Status

```
phase  what                                                     state
-----  -------------------------------------------------------  --------
0      web taxonomy refactor (kinds.ts + KindChip + chip /       code  
       icon unification across tree, search, graph, inspector)        
1      chan-drive: editable-text gate widens to FileClass::Text  code  
       (.py, .json, Makefile, ...) + narrow indexer gate via           
       is_indexable_text                                              
2      chan-server: /api/files projects full kind per entry      code  
       (document | contact | text | media | binary); PDF folds        
       into media                                                     
3      editor: source-only tab for text-class files; lazy        code  
       CodeMirror lang packs; per-tab "syntax highlight" toggle       
4      issue #30 HEIC -> WebP client conversion                  code  
       issue #27 PDF viewer overlay + media promotion            code  
       issue #28 JSON renderer (pretty/source toggle)            code  
       issue #29 CSV/TSV table renderer (click-to-edit cells)    code  
5      alias resolution (contact frontmatter aliases: + @@       open  
       typeahead)                                                     
```

`code` = svelte-check + cargo build + cargo test + npm test all
pass; no live UI verification. `open` = scope decisions pending,
nothing implemented.

Phase 4 GH issues: chan-writer/chan#27 (PDF), #28 (JSON), #29
(CSV), #30 (HEIC). All implemented but stay open until the
browser verification confirms acceptance bullets.

---

## Where the taxonomy lives in code

```
surface                code-side anchor
---------------------  ----------------------------------------------
canonical types        web/src/state/kinds.ts (FileKind | EntityKind
                       | ContainerKind, classifyEntry, classifyFile,
                       labelFor, colorVarFor, iconFor)
path-only classifier   web/src/state/fileTypes.ts (classifyPath,
                       isEditableText, isImage, isPdf, isJson, isCsv,
                       csvDelimiter)
chip component         web/src/components/KindChip.svelte
server projection      crates/chan-server/src/routes/files.rs
                       (project_kind)
chan-drive classifier  crates/chan-core/crates/chan-drive/src/fs_ops.rs
                       (FileClass + classify + is_editable_text +
                       is_indexable_text)
design ref             web/src/design.md "Kind taxonomy" section
                       (color tokens + per-kind glyph table)
```

The two gates in chan-drive are load-bearing:

- `is_editable_text(rel)` returns true for `EditableText | Text`.
  Drives `Drive::read_text` / `write_text` (the editor's read/write
  boundary).
- `is_indexable_text(rel)` returns true only for `EditableText`.
  Drives the indexer, graph rebuild, and link-rewrite on rename.
  `.py` files are editable but not indexable: a `#include` would
  otherwise leak into the graph as a `#tag`.

---

## Browser verification (parking lot)

The whole rollout shipped on top of green typecheck + build + tests
but no one has opened a running `chan serve` against a real drive.
41 checks are parked in
`~/.claude/projects/-Users-fiorix-dev-github-com-chan-writer-chan/memory/project_phase3_browser_smoke_test.md`
(steps 1-41 organized by phase). Plan: run the lot in one session
once phase 5 lands so the verification cost amortizes.

Quick recap of what to look at, in order:

1. **Phase 3** (steps 1-10): kind icons in file tree; `.py` opens
   source-only with Python syntax color; syntax highlight toggle
   persists across reload; markdown still flips wysiwyg <-> source.
2. **Phase 4 #30** (11-16): drag a `.heic` into the editor; expect
   `Converting <name>...` -> file lands as `.webp` and renders.
   Network tab: heic2any chunk only loads on first HEIC.
3. **Phase 4 #27** (17-22): `.pdf` in tree shows media chip + icon;
   "View PDF" opens fullscreen `<embed>`; Escape closes; backlinks
   surface "linked from" only.
4. **Phase 4 #28** (23-31): `.json` opens in pretty tree;
   collapse triangles work; right-click copies JSONPath; flipping
   to source shows lang-json color; bad JSON refuses to save.
5. **Phase 4 #29** (32-41): `.csv` opens in table; click cell ->
   edit -> Enter commits; toggling source <-> table preserves the
   buffer; `.tsv` uses tab as delimiter.

Anything that fails in there is a real bug in the phase that
introduced it.

---

## Phase 5: alias resolution (the open piece)

Today `@@alice` resolves to `Contacts/alice.md` only by **filename
stem match** (`crates/chan-server/src/routes/graph.rs:330` builds
`mention_to_contact: HashMap<stem, path>`). No way to declare
that `@@ali` should also resolve to Alice.

Decisions still pending (user input needed):

### Decision 1: scope split

```
option                                            tradeoff
------------------------------------------------  --------------------
A. all of 5 in one go (backend + frontend picker) one long session;
                                                  picker UX may be
                                                  rough edges
B. (rec) 5a backend + server only; defer picker   alias resolution
                                                  works end-to-end in
                                                  the source; picker
                                                  is a follow-up
C. 5a backend; review before frontend             slowest, lowest risk
```

### Decision 2: storage

```
option                                            tradeoff
------------------------------------------------  --------------------
A. (rec) aliases TEXT column on `nodes` table,    matches the existing
   space-joined like the `emails` column          `emails` precedent;
                                                  one schema bump v5
                                                  -> v6
B. separate node_aliases(node_rel_path, alias)    cleaner; adds a
   table                                          join on every
                                                  contacts query
C. derive at query time from frontmatter          zero schema change;
                                                  proportional to
                                                  contact count
                                                  (<1000 typical)
```

### What 5a looks like (backend + server)

1. **chan-drive frontmatter -> aliases**:
   `crates/chan-core/crates/chan-drive/src/contacts/mod.rs` Contact
   struct gains `aliases: Vec<String>`. The generic frontmatter
   parser (`markdown/frontmatter.rs`) already accepts the `aliases:
   [...]` key without modification.

2. **chan-drive emit**: `contacts/emit.rs` writes the chan block
   with `aliases: [ali, al]` when present. (Read-write symmetric.)

3. **graph schema bump** (if option 2A): v5 -> v6 adds
   `aliases TEXT NOT NULL DEFAULT ''` to `nodes`. Migration is a
   single ALTER TABLE. Indexer populates from frontmatter at
   parse time alongside `emails`.

4. **server resolver extension**:
   `crates/chan-server/src/routes/graph.rs:321-455` build a
   richer `mention_to_contact` map that also walks each contact's
   aliases list, mapping each alias -> contact path.

5. **`/api/contacts` shape**: add `aliases: string[]` so the
   frontend picker can show alternate names.

### What 5b looks like (frontend picker)

1. Contact picker (`web/src/editor/bubbles/contact.ts`) shows
   aliases as a dim secondary line.

2. New trigger: `@@` in raw markdown source opens the same picker.
   Commit writes `@@<alias-or-stem>` instead of the wikilink form.

3. Trigger detector (`web/src/editor/bubbles/triggers.ts`) adds
   the `@@` shape alongside the existing `@`.

4. Resolved-mention pill: dimmed when unresolved (today they
   render the same as resolved). `web/src/editor/widgets/mention.ts`.

---

## In-flight elsewhere (not in this plan)

The chan repo's working tree carries assistant-overlay tweaks
unrelated to this rollout (lib.rs route export, App.svelte
`effective_enabled`, store.svelte.ts inspector-bit hash encoding,
new `AssistantInspectorBody.svelte`, SettingsPanel reshuffle,
InlineAssist rework, `api_llm_keys_status`, chan-llm config).
Those are the user's parallel track and stay outside this plan.

---

## Resuming after this snapshot

1. Run the browser parking-lot (~41 checks). File one GH issue per
   real regression found. Close #27-#30 if their acceptance bullets
   pass.
2. Settle the two phase 5 decisions above (scope split + storage).
3. Implement 5a, then 5b (if going option B), or both together
   (option A).
4. Phase 5 wraps the kind-taxonomy work. After that the deferred
   followups are: PDF per-page renderer for printing/OCR (still in
   #27's tail), JSON5 parser for `.json5` files (#28 tail), CSV
   sort/filter/multi-cell edit (#29 tail).
