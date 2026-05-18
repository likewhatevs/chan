# backsystacean-4: frontmatter kind ladder and tag/mention scope

Owner: @@Backsystacean
Status: REVIEW

## Goal

Confirm and document the `chan.kind: <name>` frontmatter ladder.
Keep `chan.kind: contact` exactly as today. Scaffold the renderer
registry so future kinds (chan.note, chan.task, chan.event, etc)
slot in without touching indexer or graph plumbing. Write down the
tag / mention markdown-only scope rule.

## Relevant links

* Request: [request.md](./request.md) architectural cleanups section,
  items 4.2 / 4.3.
* Design memo: [architect-2.md](./architect-2.md) (Frontmatter kind
  ladder, tag/mention scope sections).
* Existing contact ingest: `crates/chan-drive/src/drive.rs` around
  line 992, `crates/chan-drive/src/graph.rs` (contact node shape).
* Markdown parser: `crates/chan-drive/src/markdown/`.

## Scope

### Registry shape

* In chan-drive (or chan-server, as fits the existing structure),
  define a registry mapping `chan.kind` value to a small struct
  describing:
  * Indexer behavior (treat as markdown, set typed badge, include
    in special-kind tables if any).
  * Renderer hint for chan-server / web.
* The contact entry is the reference implementation and stays where
  it is; refactor only as needed to expose it through the registry.
* New kinds go through the registry; no inline branches in the
  indexer.

### Tag and mention scope

* Confirm in code that `#tag` and `@@mention` edges only come from
  markdown files (already the case in chan-drive). Add an assertion
  / unit test that pins the rule in place.
* Document the rule in `crates/chan-drive/design.md`.

### Inspector hookup

* The frontmatter kind badge surfaces via the inspector payload
  added in [backsystacean-3](./backsystacean-3.md). This task ships
  the registry entry the badge reads from; @@Frontend wires the
  visual in [frontend-4](./frontend-4.md).

## Out of scope

* Adding a new kind beyond `contact` (any new kind is Alex's call
  per [architect-2](./architect-2.md) open question 3).
* Frontend rendering of the badge (in
  [frontend-4](./frontend-4.md)).
* Renaming `contact` or changing the existing pill style.

## Acceptance criteria

* Registry shape lands with `contact` as the only entry.
* Tag / mention markdown-only rule pinned by a test in chan-drive.
* `crates/chan-drive/design.md` carries one paragraph stating the
  rule.
* No regression in contact-pill behavior on the live test service.

## Tests

* `cargo test -p chan-drive` (registry + tag/mention scope).
* `cargo test -p chan-server` (no shape regression on the contact
  route).
* Pre-push gate green.

## Review and hardening

* @@Backsystacean self-review for the registry indirection cost
  (should be free at indexing time; an inline table lookup is fine).
* @@Architect to verify the registry shape is consistent with
  [architect-2](./architect-2.md) before commit.

## Progress notes

* Added `markdown::CHAN_KIND_REGISTRY` / `chan_kind` with `contact`
  as the only entry. `parse_for_graph` now consumes the registry
  instead of branching directly on the frontmatter string.
* Tightened graph token filtering so non-markdown editable text drops
  both `#tag` and `@@mention` tokens.
* Extended `file_type_policy_end_to_end` to prove `.md` emits a
  mention edge and `.txt` does not.
* Documented the `chan.kind` registry and markdown-only tag/mention
  rule in `crates/chan-drive/design.md`.

## Completion notes

Ready for review. Contact behavior stays on the existing graph node
kind and route shape; unknown `chan.kind` values remain ordinary
markdown files.

Verified:

* `cargo test -p chan-drive chan_kind_registry`
* `cargo test -p chan-drive file_type_policy_end_to_end`
* `cargo test -p chan-server contacts`
* `cargo test -p chan-drive -- --test-threads=1`
* `cargo test -p chan-server`
* `cargo fmt --check`
* `cargo build --no-default-features`
* `cargo clippy --all-targets -- -D warnings`
* `scripts/pre-push`
