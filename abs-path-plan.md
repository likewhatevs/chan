# Absolute + parent-relative link resolution

Branch (chan):       `abs-path-resolver`
Branch (chan-core):  `abs-path-resolver`

Companion to `contacts-plan.md`. Two separate releases (chan-core
first, chan optional follow-up); see §5 for the split rationale.

## 1. Problem

Markdown link hrefs land in the graph DB as the literal string the
author wrote. So:

- `[link](/images/foo.png)` is stored as `/images/foo.png`
- `[link](../images/foo.png)` from `notes/x.md` is stored as
  `../images/foo.png`

Neither matches a `nodes` row in the graph DB, which is keyed on
clean drive-relative POSIX paths (no leading `/`, no `..`). The
visible symptom: the graph view's incoming-link counter on an image
node shows 0 even when N markdown files reference it. Also,
clicking these links in the editor doesn't navigate anywhere
useful, and chan can't be a daily driver for editing GitHub repo
markdown (where `/` and `..` are the dominant link styles).

## 2. Goal / non-goals

Goals:

- `[link](/path)` resolves to `<drive_root>/path`. Drive-rooted.
  Matches GitHub repo-rendering of leading-slash links in READMEs.
- `[link](../path)` resolves against the source file's directory,
  collapsing `..` lexically. Standard relative resolution.
- Same shape for image embeds: `![alt](/images/x.png)` and
  `![alt](../images/x.png)` participate in the graph and the
  click / preview path identically.
- Editor click handler navigates correctly for both forms.

Non-goals (this scope):

- No URL-scheme rewriting. `http://`, `mailto:`, `tel:` etc. stay
  external and untouched.
- No fragment-only changes. `#section` keeps its current behavior
  (in-document anchor).
- No symlink chasing. Path resolution is purely lexical.
- No case folding policy change. The drive's existing
  case-sensitivity convention (POSIX-as-stored, scope filter is
  ASCII-insensitive) stays as-is.
- No wiki-link picker insertion-form change. The picker keeps
  inserting `[[Contacts/Jane Doe]]` (drive-rooted, no leading
  slash). Both forms now resolve so adding the slash is cosmetic
  churn.

## 3. Architecture

```
                +------------------------+
                |  markdown::extract     |
                |  _links (chan-drive)   |
                |  pulldown-cmark        |
                +-----------+------------+
                            |
                            v          source_dir =
                +------------------------+  dir of source file
                |  markdown::            |
                |  normalize_href        |  -> Option<String>
                |  (NEW, chan-drive)     |
                +-----------+------------+
                            |
            +---------------+----------------+
            |                                |
   +--------v---------+              +-------v----------+
   |  graph builder   |              |  web/editor      |
   |  (chan-drive)    |              |  links.ts        |
   |  writes clean    |              |  (mirror of      |
   |  edges to        |              |  normalize_href) |
   |  nodes table     |              |                  |
   +------------------+              +------------------+
                                              |
                                              v
                                     editor click handler
                                     navigates to file
```

The normalizer is one pure function; both the indexer (chan-drive)
and the editor (chan) call it. The TS port is a hand-written
mirror, not a wasm bridge: tiny logic, no need for a build-time
bundle, and the click handler runs hot.

### `normalize_href` semantics

Input: `(href: &str, source_dir: &str)`
Output: `Option<String>` (drive-relative POSIX path; `None` for
external / fragment-only / escapes-the-drive)

Rules, in order:

1. Strip a leading `#` fragment-only ref -> `None` (unchanged
   intra-doc behavior, no graph edge).
2. Detect URL scheme. If the href contains `:` before any `/`, `#`,
   or `?`, treat as external -> `None`.
3. Drop a trailing `#anchor` and `?query` portion if present;
   anchor is preserved separately by the caller (graph already
   stores anchor as its own edge column).
4. If href starts with `/`, strip the leading slash; the remainder
   is treated as drive-rooted.
5. Otherwise, treat as relative: prepend `source_dir` (already
   drive-relative POSIX, no leading slash), then collapse the
   path:
   - `./x` -> `x`
   - `a/./b` -> `a/b`
   - `a/b/../c` -> `a/c`
   - leading `..` from drive root -> `None` (escapes drive)
6. Reject results containing `\0` or other invalid path
   components.
7. Return the cleaned POSIX string with no leading or trailing
   slash.

### What changes in the graph DB

Edges already stored before this lands keep their old (literal,
unnormalized) target strings until the next reindex pass rewrites
them. Nothing breaks; old edges just continue not to resolve. A
full `reindex` after the chan-core release rebuilds the edges
table cleanly.

No graph schema change needed. Same `edges(src, dst, kind, anchor)`
shape; only the values written to `dst` get cleaner.

## 4. Repo split

The chan-core piece (Phases A, B, C) and the chan piece (Phase D)
are independent and can ship in separate releases.

| Piece                    | Repo      | Depends on                |
| ------------------------ | --------- | ------------------------- |
| `normalize_href` + tests | chan-core | nothing                   |
| Graph builder wires it   | chan-core | normalize_href            |
| Image-counter verify     | chan-core | graph wiring              |
| Web mirror + click route | chan      | shape of normalize_href   |

The web mirror has zero runtime dep on chan-core (it's a TS port
of a pure function). It can ship before, after, or alongside the
chan-core release; behavior compounds rather than blocks. The
sensible ordering:

1. **chan-core release v0.8.1**: Phases A + B + C. Image-link
   counter starts working immediately for graph queries; backlinks
   and link-targets land on the right files.
2. **chan release v0.6.17**: Phase D + bump chan-core path-dep to
   0.8.1. Editor click handler navigates `/path` and `../path`
   correctly. Optional in the sense that the chan-core release
   alone is a real improvement; this just rounds out the editor
   side.

User action between releases: `chan index <drive>` to rebuild the
graph against the cleaner normalizer. Stale edges from before are
harmless until they're rewritten.

## 5. Phasing

### Phase A (chan-core): `normalize_href` + tests

Module home: `crates/chan-drive/src/markdown/links.rs` (sibling
of `extract_links`). Pure function, no I/O. Comprehensive tests
on the cases below.

Test matrix:
- `("/x.md", "notes")` -> `Some("x.md")`
- `("/images/x.png", "deep/nested")` -> `Some("images/x.png")`
- `("../x.md", "notes")` -> `Some("x.md")`
- `("../../x.md", "a/b")` -> `Some("x.md")`
- `("../../../x.md", "a/b")` -> `None` (escapes)
- `("./x.md", "notes")` -> `Some("notes/x.md")`
- `("x.md", "notes")` -> `Some("notes/x.md")`
- `("https://x.com/", "notes")` -> `None` (scheme)
- `("mailto:a@b", "notes")` -> `None` (scheme)
- `("#section", "notes")` -> `None` (fragment only)
- `("a.md#sec", "notes")` -> `Some("notes/a.md")` (anchor stripped)
- `("/a.md#sec", "notes")` -> `Some("a.md")` (anchor stripped)
- `("a.md?q=1", "notes")` -> `Some("notes/a.md")` (query stripped)
- `("", "notes")` -> `None` (empty)
- `("/", "notes")` -> `None` (root-only, no file)
- `("/contacts/Jane Doe.md", "notes")` -> `Some("contacts/Jane Doe.md")` (spaces preserved)

### Phase B (chan-core): graph builder runs hrefs through normalize_href

`build_edges` (in `crates/chan-drive/src/drive.rs`) calls
`normalize_href(link.target, source_dir)` and skips the edge if it
returns `None`. `source_dir` is derived from the source file's
rel_path via `Path::parent`.

Wiki-links (`[[target]]`) get the same treatment: a wiki-link
`[[/Contacts/Jane Doe]]` resolves to drive-root, `[[../foo]]`
walks up. This makes the wiki-link picker's existing
drive-rooted-without-slash convention work alongside the new
absolute form without ambiguity.

Image embeds (`![alt](src)`) follow the same path; the
resolver is type-agnostic. Image-counter symptom resolves once
the next reindex runs.

### Phase C (chan-core): image-counter verify

End-to-end test in `crates/chan-drive/tests/`:
1. Build a tempdir drive with `notes/post.md` referencing
   `/images/foo.png` and `../images/foo.png`, plus a
   `images/foo.png` placeholder file.
2. `Drive::reindex`.
3. Assert `GraphView::backlinks("images/foo.png")` returns 2
   (one per source link), not 0.

Also touch the existing graph tests for any assertion that
encoded the unnormalized string as the dst; rewrite them to
the normalized form.

### Phase D (chan): web mirror + click handler

TS port lives in `web/src/editor/links.ts` (or new sibling).
Same name + same logic as the Rust normalizer. Tested with a
parallel Vitest suite if the repo has one, otherwise inline
unit-test docstrings.

Editor click handler (in Wysiwyg.svelte's link-click path)
routes hrefs through the normalizer before opening the target
in a new tab / pane. Existing wiki-link click handler
(`handleWikiClick` in extensions/wikiLink.ts) gets the same
treatment.

Insertion form: unchanged. Wiki-link picker keeps inserting
`[[Contacts/Jane Doe]]` (drive-rooted, no slash). Both forms
resolve identically post-normalizer; the picker stays as-is.

### Tests woven through each phase, not a separate phase.

## 6. design.md updates

- chan-drive: `crates/chan-drive/design.md`. Add a "Link
  resolution" subsection under §3 Components describing
  `normalize_href` semantics + the graph-builder integration.
  Same commit as Phase B.
- chan: `design.md`. Note that the editor's click handler routes
  through chan-drive's normalizer mirror. Same commit as Phase D.

## 7. Decisions log

Resolved:

- **Drive-rooted-without-slash stays the wiki-link insertion
  form.** The picker keeps inserting `[[Contacts/Jane Doe]]`
  rather than `[[/Contacts/Jane Doe]]`. Both forms now resolve;
  the leading slash would be cosmetic churn. Slightly weird that
  `[[file.md]]` could mean either local-dir or drive-root, but
  the resolver tries source_dir first and falls back to root,
  matching today's heuristic, so no ambiguity hits the user.
- **Case sensitivity unchanged.** Drive sticks with POSIX-as-
  stored. Comparison logic (search scope, etc.) handles ASCII-
  insensitive folding where needed.
- **Two-release split.** chan-core ships the on-disk fix
  (Phases A-C) as v0.8.1; chan picks up the editor click handler
  (Phase D) as v0.6.17. Either lands independently.

Implicit but worth pinning:

- **No graph schema change.** Edges keep their existing column
  shape. Old edges remain (with stale dst strings) until the
  next reindex pass rewrites them.
- **No symlink chasing.** Lexical resolution only; consistent
  with chan-drive's path-sandbox philosophy.
- **The TS mirror is a hand-port.** Not generated, not bundled
  via wasm. Small enough that the duplication cost is lower than
  the build/runtime cost of bridging.
