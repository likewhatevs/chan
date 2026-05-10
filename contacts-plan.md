# Contacts Plan

Branch: `contacts-api`. Companion to `candle-plan.md` and `design.md`.

This plan is the survivor of a longer plan that originally spanned an
OAuth-driven Google sync. We pivoted to a one-shot CSV-import model
because (a) Google offers no user-issued PAT for the People API, (b)
even with OAuth, browser export to CSV covers the actual use case
(import existing contacts as notes; chan is not a CRM), and (c) the
CSV path keeps chan single-binary, loopback-only, no shared client
secrets, no token refresh state.

## 1. Goals / non-goals

Goals:

- User imports a CSV exported from `contacts.google.com` and lands
  one `.md` per contact in a folder of their choice inside the drive.
- Imported contacts are first-class notes: indexed by the existing
  markdown indexer, surfaced in the existing search, addressable
  by wiki-link, distinguishable from plain notes via frontmatter.
- Contacts show up as a distinct node kind in the graph
  (`GraphNode::Contact`), with edges from any document referencing
  them.
- Editor `@<query>` opens a contacts picker; selection inserts a
  wiki-link to the corresponding contact note.
- Provider abstraction is a parser dispatch (Google CSV today;
  Outlook CSV / vCard later), not an API client.

Non-goals (v1):

- No OAuth, no API integration, no token storage, no `chan-gateway`
  involvement.
- No cache, no sync state, no SQLite. Contact notes ARE the source
  of truth; the existing markdown indexer + filesystem cover read.
- No two-way sync, no editing, no group/label management beyond
  what we surface from the imported CSV.
- No re-import diffing in v1. Re-running import either skips
  existing files or overwrites them based on a flag; no merge.

## 2. Architecture overview

```
                +------------------+
                |  chan (CLI)      |  chan contacts import csv ...
                +--------+---------+
                         |
                +--------v---------+
                |  chan-server     |  POST /api/contacts/import
                |  routes          |  GET  /api/contacts (picker)
                +--------+---------+
                         |
                +--------v---------+
                |  chan-core       |  contacts module
                |  contacts module |  - parse Google CSV
                |  (parser +       |  - render markdown
                |   emitter)       |  - filename slug rules
                +--------+---------+
                         |
                +--------v---------+
                |  drive           |  <drive>/<dir>/Jane Doe.md
                |  (filesystem)    |  with chan/contact frontmatter
                +--------+---------+
                         |
            +------------+------------+
            |                         |
   +--------v---------+      +--------v---------+
   |  markdown        |      |  graph builder   |
   |  indexer         |      |  emits Contact   |
   |  (BM25 + dense)  |      |  nodes from      |
   |  unchanged       |      |  frontmatter     |
   +------------------+      +------------------+
```

Principle: contact notes are regular markdown files. Frontmatter
`chan.kind: contact` is the only signal anything downstream needs.
No parallel index, no cache, no sidecar metadata.

## 3. Module layout

New module: `crates/chan-core/src/contacts/`.

| File         | Responsibility                                          |
| ------------ | ------------------------------------------------------- |
| `mod.rs`     | Re-exports + `Contact`, `ProviderKind`, error types     |
| `provider.rs`| `ProviderKind` enum; future trait if vCard/Outlook land |
| `google.rs`  | Google Contacts CSV parser                              |
| `emit.rs`    | Render `Contact` -> markdown (frontmatter + body)       |
| `slug.rs`    | Filename derivation, sanitization, collision suffixes   |

`crates/chan-core/src/lib.rs` re-exports `pub mod contacts;`.

New workspace dep: `csv = "1"` (see decision log §10).

## 4. Data model

```rust
// crates/chan-core/src/contacts/mod.rs

pub enum ProviderKind { Google }

pub struct Contact {
    pub provider: ProviderKind,
    pub remote_id: Option<String>,    // None for CSV (Google CSV
                                      // does not export resource
                                      // names); kept for future
                                      // providers that do.
    pub display_name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub emails: Vec<EmailAddress>,
    pub phones: Vec<PhoneNumber>,
    pub organizations: Vec<Organization>,
    pub notes: Option<String>,
    pub labels: Vec<String>,          // from "Labels" CSV column
}

pub struct EmailAddress { pub value: String, pub label: Option<String> }
pub struct PhoneNumber  { pub value: String, pub label: Option<String> }
pub struct Organization { pub name: String, pub title: Option<String> }
```

`ContactRef` (the editor-side reference) is dropped from the v1
plan: with all references serializing as wiki-links to local files,
the wiki-link itself IS the reference. The link extractor reads the
target file's frontmatter to classify.

## 5. Markdown emission

One file per contact.

### Filename

Derived from `display_name`, then fall back in this order:
1. First non-empty email local part.
2. First non-empty phone (digits only, prefixed `phone-`).
3. `unnamed-<n>` where `n` increments within the import batch.

Then sanitize: replace `/`, `\`, `:`, control chars, leading/trailing
whitespace with `_`. Trim to 120 chars (UTF-8-safe). Append `.md`.

Collision: if `Jane Doe.md` exists in the destination folder,
suffix `Jane Doe (2).md`, `Jane Doe (3).md`, etc. Skip with `--overwrite`
to force replace (single-shot, no merge).

### File content

```markdown
---
chan:
  kind: contact
  provider: google
  imported_at: 2026-05-10T12:34:56Z
  frontmatter_version: 1
contact:
  display_name: Jane Q. Doe
  given_name: Jane
  family_name: Doe
  emails:
    - { value: jane@example.com, label: work }
  phones:
    - { value: "+1-555-0100", label: mobile }
  organizations:
    - { name: Acme Corp, title: Engineer }
  labels: [Friends, Work]
---

# Jane Q. Doe

Notes from the CSV "Notes" column go here verbatim.
```

Round-trip rule: chan never rewrites a contact file after import.
The file is fully user-owned the moment it lands. Re-import with
`--overwrite` replaces the whole file; otherwise we skip.

`frontmatter_version: 1` is for forward compat if the shape changes.
Any document containing `[[Contacts/Jane Q. Doe]]` (or whatever path)
resolves through the regular wiki-link machinery; the link extractor
classifies the target as a Contact node by reading its frontmatter.

## 6. CLI

New subcommand group on `chan`:

```
chan contacts import csv FILE
                          --into DIR              # required
                          [--provider google]     # default google
                          [--dry-run]             # parse + report; no writes
                          [--overwrite]           # replace existing files
                          [--drive PATH]          # default: resolved drive
```

Resolution of `DIR`: drive-relative path. Created if absent.

Output (table): per-row `WROTE | SKIPPED | OVERWROTE | FAILED` with
filename and a short reason. Final summary: `N wrote, M skipped, K
overwrote, F failed`. `--dry-run` reports the same table without
touching disk.

Manpage-style help on each subcommand per repo convention.

## 7. HTTP API

One new route in `chan-server`, gated by the existing per-launch
token middleware:

```
POST /api/contacts/import        multipart/form-data
  parts:
    file:     bytes (CSV)
    dest_dir: string (drive-relative folder)
    provider: string (default "google")
    overwrite: bool (default false)
  response 200:
    {
      "wrote":      ["Contacts/Jane Doe.md", ...],
      "skipped":    [{"path": "...", "reason": "exists"}, ...],
      "overwrote":  [...],
      "failed":     [{"name": "...", "reason": "..."}, ...]
    }
  response 400 on parse failure with first-error context
```

Plus one read route used by the editor `@` picker:

```
GET /api/contacts?q=<prefix>&limit=<N>
  walks the drive, filters files whose frontmatter has
  chan.kind == contact, returns:
    [{"path": "...", "display_name": "...", "emails": [...]}, ...]
  prefix-matches display_name OR any email local part.
  v1: brute-force walk (low N expected); promote to indexed
  read if it ever bites.
```

Both routes wired in `crates/chan-server/src/routes/contacts.rs`.

`/api/drive` payload: no change. There is no mode or connection
state to surface.

## 8. Frontend

File Browser popover (`web/src/components/FileBrowserOverlay.svelte`)
gains an "Import Contacts" `<li>` next to "New File" / "New Folder".

Clicking it opens a 4-step modal wizard:

1. **Provider**. List with one entry: "Google Contacts". Selectable
   for future-proofing the UI; only one option in v1.
2. **Get the CSV**. Shows instructions: "Visit
   contacts.google.com -> Export -> Google CSV -> Download". File
   input accepts the resulting `.csv`.
3. **Destination folder**. Folder-only tree (subset of the existing
   `tree.entries`); user clicks to confirm. Default selection: the
   folder currently focused in the file browser.
4. **Confirm**. Shows count of contacts parsed (client-side preflight)
   and the destination. Submit triggers the multipart POST. Display
   result summary with a "View imported" button that navigates the
   file tree to the destination folder.

Modal scaffolding: reuse `OverlayShell.svelte` pattern.

## 9. Graph integration

`GraphNode` gains a `Contact` variant in
`crates/chan-core/src/link_index.rs`:

```rust
pub enum GraphNode {
    File { ... },
    Tag { ... },
    Mention { ... },
    Date { ... },
    Contact { id: NodeId, label: String, file_path: String },
}
```

Graph extraction rule: when the link extractor resolves a wiki-link
target to a file whose frontmatter has `chan.kind: contact`, emit a
`Contact` node aliased to that file (single graph node, not double).
Edges: `GraphEdge { source: file_id, target: contact_id, kind:
GraphEdgeKind::Contact }` from the referencing document.

Frontend graph rendering changes: out of scope for v1. The API just
starts returning `Contact` nodes; clients can render or ignore.

## 10. Editor `@` picker

Phase 5 work. Two coupled changes:

### 10a. Migrate `@today` / `@date` to `!/today` / `!/date`

Free `@` for live contact search. In
`web/src/editor/Wysiwyg.svelte` the trigger arms change from
`endsWith("@today")` / `endsWith("@date")` to the `!/`-prefixed
forms. On-disk format unchanged: dates serialize as the formatted
string (e.g., `02 Jan 2029`); `decorateSmartNodes` re-pills via
regex against that text on load.

The two-char `!/` prefix is the chan convention for command-style
inline insertions (vs single chars `/`, `;`, `!`, `:` which all
flicker the picker on common prose patterns -- paths, sentence-end
`!`, emoji `:smile:`). Collision-free with prose, so no
word-boundary guard is required. See the chan_command_trigger
project memory for the rationale and the full options-table that
led here.

Doc sweep: `README.md`, `chan --help`, any onboarding text mentioning
`@today` / `@date` (incl. `web/src/components/SettingsPanel.svelte`
date-format hint and `web/src/api/types.ts` doc comment).

### 10b. `@` picker

`@` opens a popover that re-queries `GET /api/contacts?q=...&limit=10`
on each keystroke until whitespace, Enter, or Escape. Selection
inserts a wiki-link `[[<file_path>]]` (relative form per the existing
wiki-link convention). On serialization to markdown, this is a plain
wiki-link; on load, `decorateSmartNodes` already pills wiki-links --
the contact pill is a styling variant triggered by the target's
`chan.kind: contact` frontmatter.

Picker rendering: rows show `display_name` primary, first email
secondary. No avatars in v1.

## 11. Filesystem rule

Contacts feature writes inside the user's drive only when the user
runs the import action (CLI or wizard) and only into the destination
folder they pick. Nothing else. No `.chan/` sidecar in the drive,
no thumbnails, no cache. All chan-internal state -- if any is ever
added -- lives in `~/.chan/` per the existing convention.

This is the second case (after attachments) where chan creates
user-visible files in the drive, and like attachments it is an
explicit user opt-in, not a chan-internal convenience.

## 12. Phased delivery

| Phase | Deliverable                                                      |
| ----- | ---------------------------------------------------------------- |
| 0     | `contacts` module: types (§4), Google CSV parser, md emitter (§5), slug rules. Workspace `csv = "1"` dep. Unit tests on parser fixtures. |
| 1     | CLI `chan contacts import csv` (§6). Integration test against tempdir. |
| 2     | HTTP `POST /api/contacts/import` (§7). Integration test via existing server harness. |
| 3     | Frontend wizard (§8). Manual browser verification per CLAUDE.md UI rule. |
| 4     | Graph: `GraphNode::Contact` (§9), link extractor classifies by frontmatter. |
| 5     | Editor `@` picker + `!/today` / `!/date` migration (§10). `GET /api/contacts` read route. |

Phases 0-3 deliver "import contacts as markdown" (the user's
original ask). Phases 4-5 deliver "make them first-class" (graph
+ `@mention`).

Tests are woven into each phase, not a separate phase.

## 13. Decisions log

Resolved (from the original plan + this session):

- **CSV-only, no OAuth.** Google offers no user-PAT for People
  API; OAuth has no shipping advantage over CSV import for the
  read-once use case.
- **Contact reference syntax: regular wiki-link.** No
  `contact://` URL scheme. Frontmatter on the target classifies
  the link as a contact.
- **`!/today` / `!/date` for date pills.** Two-char prefix
  collision-free with prose / paths / emoji shortcodes; frees `@`
  for the contact picker. See chan_command_trigger memory.
- **Provider abstraction is parser dispatch.** Not an API client.
  `enum ProviderKind { Google }` today; add Outlook / vCard variants
  later by adding parser modules.
- **No persistent contacts config.** No mode, no sync_dir global,
  no ContactsConfig. The wizard / CLI takes the destination folder
  as an argument each time.

Implicit but worth pinning:

- **`csv` crate** for parsing (workspace dep). Decision rationale:
  `serde`-integrated, well-maintained, no async needed for one-shot
  import.
- **CLAUDE.md anchor drift:** the original plan referenced
  `docs/architecture.md` as the surface-table source. This repo's
  CLAUDE.md no longer mandates that file (no `docs/` dir exists).
  Top-level convention here is `candle-plan.md`, `design.md`,
  `contacts-plan.md`. New routes/CLI subcommands update CLAUDE.md
  layout block instead.
