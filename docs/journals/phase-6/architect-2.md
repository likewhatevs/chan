# @@Architect task 2: design memo for the layered graph

Owner: @@Architect
Status: IN_PROGRESS (memo first, refine as implementation feedback
lands)

## Goal

Land a shared picture for the phase-6 architectural cleanups so the
@@Backsystacean and @@Frontend tracks share the same contract.

## Relevant links

* Request architectural section: [request.md](./request.md)
* Journal checklist: [journal.md](./journal.md)
* chan-drive design (filesystem boundary):
  [../crates/chan-drive/design.md](../crates/chan-drive/design.md)
* Workspace design: [../design.md](../design.md)

## Decisions

### 1. Graph rooting: filesystem first, drive as the default scope

The graph's primary layer is the filesystem rooted at the drive.

* Default scope across every entry point ("Graph this" buttons, the
  empty-pane menu graph opener, hash-restored graph state) resolves
  to the drive scope. Drive scope option already exists; today it is
  selected only when the user explicitly walks to drive root in the
  file tree.
* Per-file / per-dir "Graph this" actions become "Graph from here":
  they pivot the scope to that file or directory but the empty
  default lands on drive. Wording stays "Graph this" inside the file
  tree row context menu and elsewhere; what changes is the scope
  picked when no preference exists.
* Inspector nodes:
  1. Drive (existing inspector widget; gains language breakdown and
     filesystem-level counters).
  2. Directories (new full inspector; today there is partial info).
  3. Files. Subcases: markdown (regular + frontmatter), text, binary.
* The graph treats locked / read-only directories as dead-ends: their
  node renders, but the subtree is not expanded for edges. Same for
  symlinks pointing outside the drive root: surface the node, do not
  traverse.

### 2. File classifier in chan-drive

chan-drive owns the boundary; classifier lives next to the existing
`is_indexable_text` gate.

The classifier categorizes every path the indexer touches:

| Category     | Detection                                          | Graph behavior         |
|--------------|----------------------------------------------------|------------------------|
| Directory    | std::fs metadata is_dir                            | Edge parent for files  |
| Symlink      | symlink_metadata file_type is_symlink              | Surface, do not follow |
| Hardlink     | nlink > 1 on regular files                         | Surface as note        |
| FIFO         | file_type is_fifo                                  | Skip from graph        |
| Socket       | file_type is_socket                                | Skip from graph        |
| BlockDevice  | file_type is_block_device                          | Skip from graph        |
| CharDevice   | file_type is_char_device                           | Skip from graph        |
| RegularFile  | file_type is_file                                  | Normal handling        |

Permissions:

* `read_only`: `metadata.permissions().readonly()` (POSIX bit AND, not
  full Unix-mode parsing). Read-only directories render as dead-ends.
* `writable`: the inverse. Drive root writability is already required
  by `Drive::new`.

The classifier never opens files. It runs from metadata.

Existing code paths to keep aligned:

* `chan-drive` watcher classification (already special-cases
  `.git/HEAD`, `.git/index`, `.hg/dirstate` from phase 5; the new
  classifier does not change that).
* `chan-drive` write path (`Drive::write_text`, `Drive::write_bytes`)
  already refuses special files. The classifier is the read-side
  twin so the inspector and the graph share the verdict.

### 3. Markdown vs text vs binary

| Layer        | Inspector content                                                                    |
|--------------|--------------------------------------------------------------------------------------|
| Markdown     | title, h1, frontmatter kind (if any), links, tags, mentions, word count, mtime.     |
| Frontmatter  | kind ladder + contact pill (existing); future `chan.{other}` slots scaffolded.       |
| Text         | language detection, chan-report data, byte size, line count, encoding, mtime.       |
| Binary       | byte size, file kind from extension + libmagic-free sniff, mtime. No content read.   |

Tag and mention edges only originate from markdown files. Plain-text
and binary files do not contribute `#tag` or `@@mention` nodes; this
matches today's chan-drive indexing and is now load-bearing for the
inspector contract.

### 4. Frontmatter kind ladder

* Format: nested map. The frontmatter carries a `chan:` block
  whose `kind:` value names the ladder entry. Example:

  ```yaml
  chan:
    kind: contact
  ```

  (Corrected 2026-05-18 after @@WebtestA's read of
  `crates/chan-drive/src/markdown/frontmatter.rs`. Earlier drafts
  showed the flat shorthand `chan.kind: contact`; that was a doc
  shorthand and is not the YAML the registry actually parses.)
* Today's only ladder entry: `contact`.
* Renderer registry lives in chan-server (route) + web (component).
* New kinds add a renderer entry + a token; the indexer treats them
  as markdown files with a typed badge in the inspector.
* @@Backsystacean writes the registry shape in
  [backsystacean-3](./backsystacean-3.md); first follow-up kind goes
  to a later phase unless Alex names one in scope.

### 5. Language binding

Language is a per-directory roll-up rendered as a chip set:

* Drive inspector: full breakdown (sorted by byte count or file count;
  pick byte count for stable ordering across small + large files).
* Directory inspector: same shape, scoped to the subtree.
* File inspector: single language chip if known, none otherwise.

Detection lives in chan-report (already accumulates report data per
file). The aggregation up the tree is a chan-report responsibility;
chan-server exposes it on `/api/files` payloads or a dedicated
endpoint as @@Backsystacean decides in
[backsystacean-2](./backsystacean-2.md).

### 6. Color tokens

The current palette collides on green:

* `--chan-color-tag` (green): used today by tag chips.
* Language chips also picked up green in places, which is what Alex
  is calling out.

Phase 6 introduces `--chan-color-language` ("royal pink"). Concrete
hex: `#C71585` (Medium Violet Red, the canonical "royal pink" sRGB
value). Sits between the existing tag green and the contact pill
pink, distinct enough that side-by-side chips read as three lanes.

Dark mode value: `#FF4DB8` (eyeballed for contrast on the existing
dark surface). @@Frontend confirms in [frontend-4](./frontend-4.md)
against the live palette and may propose alternates before commit.

Token rules:

* `--chan-color-language` is the single source for the language chip
  fill and the graph language ring.
* The tag green stays.
* The contact pill keeps its own variable.
* `--chan-color-code` is an alias for the language token for clarity
  in component templates; the underlying value is shared so the
  palette stays small.

### 7. Terminology: directory, not folder

* Replace `folder` with `directory` in copy and identifiers.
* `dir` is allowed as a short form (matches existing chan-drive API
  surface that already mixes `dir` and `path`).
* Persisted state keys keep their current names where they exist to
  avoid forcing a migration this phase. Recorded as a follow-up in
  the journal.

### 8. Out of scope for phase 6

* New frontmatter kinds beyond contact.
* Migrating persisted state keys to the new vocabulary.
* libmagic-style binary sniff: extension + small header check is
  enough; no new dependency.
* Multi-user / multi-drive graph rooting (single-user, single-machine
  remains the contract).

## Decisions confirmed by Alex (2026-05-18)

1. **Royal-pink LGTM.** `#C71585` light / `#FF4DB8` dark are the
   final hex values. Token slot `--chan-color-language` (alias
   `--chan-color-code`).
2. **"Graph from here" across all surfaces.** Drop "Graph this"
   everywhere. Default scope is drive; the scope-pivot action
   reads "Graph from here" in file tree row context menus, file
   editor menu, graph overlay openers, and the empty-pane menu.
3. **Frontmatter kinds beyond `contact` are next-phase work.**
   `backsystacean-4` ships the registry scaffold with `contact`
   only.
4. **Ghost-node indexer-progress UX gap is in scope this phase.**
   Tracked in [backsystacean-7](./backsystacean-7.md) (server-side
   indexer state endpoint) and [frontend-6](./frontend-6.md)
   (graph panel live status + ghost-hint upgrade).

## Progress

* 2026-05-18 Memo drafted, contracts handed to the wave-1 tracks.

## Completion notes

(populated as @@Backsystacean and @@Frontend tracks land and the
contracts shake out under live use)
