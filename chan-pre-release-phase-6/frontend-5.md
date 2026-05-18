# frontend-5: terminology codemod (web side)

Owner: @@Frontend
Status: PARTIAL ships this phase (user-visible "directory" copy
+ wire-vocabulary updates done); broad compat-sensitive
identifier codemod PARKED to follow-up per
[architect-4](./architect-4.md). Remaining matches
(`kind: "folder"` in graph filters, persisted scope keys) need
a deliberate wire-format compatibility pass and don't block
the architectural close-out.

## Goal

Replace "folder" with "directory" (or "dir" in tight UI) across the
web codebase. Coordinated with [backsystacean-4](./backsystacean-4.md)
on the crates side.

## Relevant links

* Request: [request.md](./request.md)
* Design memo: [architect-2.md](./architect-2.md) (Terminology
  section)
* Journal: [journal.md](./journal.md)

## Scope

* User-visible copy: "Folder" -> "Directory", "folder" -> "directory"
  across components, tooltips, menu labels, settings copy, docs.
* Doc comments: same replacement; keep code style consistent with
  the file (the design notes use prose).
* Identifiers: rename web-side identifiers that use `folder` /
  `Folder` to `directory` / `Directory` where the rename is
  contained to the web crate (no cross-crate API change required;
  the API shape change, if any, is owned by
  [backsystacean-4](./backsystacean-4.md)).
* `dir` is acceptable as a short form in tight UI spots
  (icon labels, single-column dropdowns).

## Out of scope

* Persisted state keys: leave alone this phase. Recorded as a
  follow-up in [journal.md](./journal.md).
* Crates-side renames: lives in
  [backsystacean-4](./backsystacean-4.md).

## Acceptance criteria

* `rg -n '[Ff]older' web/src` returns no matches in user-visible
  copy or identifiers (single-line allowances ok if scoped to a
  comment that references a third-party library API; record any
  exceptions in the task's progress notes).
* The web bundle still builds and tests pass.

## Tests

* `npm --prefix web run check` clean.
* `npm --prefix web test -- --run` green (existing tests may need
  text-match updates).
* `npm --prefix web run build` clean.

## Review and hardening

* @@Frontend self-review for missed copies on dropdowns / context
  menus.

## Progress notes

* User-visible "directory" copy landed in the touched browser, tree,
  inspector, prompt, dashboard, and import surfaces.
* Fixed the remaining visible import-wizard copy:
  `Replace existing files in this directory`.
* Fixed the remaining safe presentation strings from webtest:
  `KindChip` now renders `folder` as `directory`, the filesystem
  graph filter chip reads `directory`, direct graph/search scope
  labels say `directory` / `parent directory`, and path prompts /
  overwrite confirmations use directory terminology.
* Updated frontend fs-graph wire vocabulary to match the crates-side
  codemod: requests now use `scope=directory`, fs-graph nodes accept
  `kind: "directory"`, and language graph edge targets normalize
  `directory:<path>` (with legacy `folder:<path>` still tolerated).
* Broad `folder` identifier cleanup across `web/src` is still
  pending. The remaining matches are mostly API/domain terms
  (`kind: "folder"`, graph filters, persisted scope keys) and need a
  deliberate compatibility pass rather than a blind rename.
* Cleaned stale prose/comments that referred to folders where the
  runtime concept is now a directory. Remaining `folder` matches are
  intentionally compatibility/internal: icon imports, CSS class names,
  `PathPromptKind = "folder"`, graph filter state, persisted hash
  slots, and internal canvas kind aliases.

## Completion notes

Verification:
* `npm run check` in `web` passed.
* `npm test -- --run` in `web` passed: 18 files, 173 tests.
* `npm run build` in `web` passed with existing chunk-size warnings.
