# Phase 10 Track A Item 4 - Bootstrap Prompt

Dispatched by @@Architect on 2026-05-26. Paste the block below to start the
agent. Item 4 = the two Track B to Track A handoffs: Tauri app icon
regeneration plus the desktop docs/config audit.

---

You are a Phase 10 Track A agent on `chan`. @@Architect has dispatched you
item 4: two disjoint, desktop-facing subtasks. Suggested handle `@@IconDocs`
(rename if @@Architect assigns another).

Baseline: HEAD is `23fa3aa` (v0.15.4), working tree clean. Read `CLAUDE.md`,
`AGENTS.md`, and `desktop/CLAUDE.md` before touching anything.

## Subtask A: Tauri app icon regeneration

- Goal: Cmd+Tab and the Dock show a dark `#101112` background with the orange
  enso `#ef8f58`. Base the colors on the current dark site tokens.
- Source master is `desktop/src-tauri/icons/icon.png`. The `.icns`/`.ico`
  live at `desktop/src-tauri/icons/icon.icns` and `.../icons/icon.ico` (the
  source handoff doc says `src-tauri/` root; the real path is
  `src-tauri/icons/` - verified on disk). `tauri.conf.json` `bundle.icon`
  references `icons/32x32.png`, `icons/128x128.png`, `icons/128x128@2x.png`,
  and `icons/icon.png`.
- Prefer `cargo tauri icon <master.png>` so the whole size set regenerates
  from one source and stays consistent.
- VERIFY the generated macOS app icon in Cmd+Tab and the Dock, not only the
  source PNGs. Build the `.app` (`make build` in `desktop/`) and inspect the
  real artifact. Record exactly how you verified.

## Subtask B: Desktop docs/config audit (fresh-state only)

Correct stale release/install copy. No migration, old-contract, or
backward-compatibility framing.

- `desktop/README.md`: Linux/Windows wording.
- `desktop/design.md`: old `/dl/latest`, MSI, and updater language.
- `desktop/src-tauri/tauri.conf.json`: updater URL shape. It currently reads
  `https://chan.app/dl/desktop/{{target}}/{{current_version}}/latest.json`.
  CAUTION: this is the desktop *updater manifest* path, owned by
  chan-prod-setup per `desktop/CLAUDE.md`. It is NOT the removed CLI
  `/dl/latest` first-install contract. Confirm intent before changing it; the
  desktop manifest path may legitimately stay. If you are unsure whether it is
  stale, cut a question back to @@Architect rather than rewriting it.
- Watch for adjacent staleness while reading (e.g. `desktop/CLAUDE.md` still
  cites version `0.14.0`; `make run` was renamed to `make dev` in `cbfc1aa`).
  If you find stale copy OUTSIDE the three named files, flag it back to
  @@Architect; do not silently widen scope.

Read first: `desktop/README.md`, `desktop/design.md`, `desktop/CLAUDE.md`,
`desktop/src-tauri/tauri.conf.json`, `desktop/Makefile`,
`docs/journals/phase-10/track-a-handoff-from-track-b-logo-and-docs.md`, and
items 5 and 6 in `docs/journals/phase-10/track-a-round-3-handoff.md`.

## Coordination and task discipline

- `desktop/` is @@Desktect's active track. The tree is clean now, but if
  @@Desktect is mid-edit on a file you need, STOP and coordinate through
  @@Architect before editing.
- Append-only to your tasks. Once you have started a task, a new ask becomes a
  NEW task, not an amendment. If scope grows, or you hit a cross-cutting /
  intent decision (like the updater URL above), cut a task back to @@Architect
  instead of absorbing it.
- Shared-worktree commit hygiene: path-scoped `git add <paths>`; collapse
  `git add` + `git diff --staged --stat` audit + `git commit` into ONE chained
  bash invocation; verify with `git show --stat HEAD` afterward. Never touch
  unrelated dirty files. Icons are binary assets, so double-check the staged
  set is exactly your files.

## Writing rules

No em dashes. ASCII tables targeting 80 columns. Factual, no marketing.
Comments explain WHY, not WHAT.

## Verification gates (record exact commands + results in your journal)

- Icons: `make build` in `desktop/` (or `cargo build -p chan-desktop`)
  succeeds with the new assets; manual Cmd+Tab / Dock check on the built
  `.app`.
- Docs: if you touch any `docs/manual/` or `web-marketing/` copy, run
  `cd web-marketing && npm run check`. Pure `desktop/*.md` edits need no Rust
  gate; run `cargo fmt --check` only if you touched Rust.
- Journal: add a focused note (or append to an existing Track A note) under
  `docs/journals/phase-10/` recording what changed, how the icon was verified
  on the built app, and what the docs audit corrected versus left intentionally.

## The review loop (follow exactly to close out)

1. When the work is complete and self-verified, send a task to @@Architect for
   REVIEW: what changed, files touched, verification commands + results, and
   any items you cut back.
2. @@Architect reviews and sends a task back to APPROVE (or requests changes).
3. On approval: write your summary, post your FINAL update to the task and
   CLOSE it, then COMMIT (atomic, path-scoped, audited as above).
4. Send a task back to @@Architect signaling the loop is closed and you are
   ready to tear down.
5. Do not tear down (servers, temp dirs, worktree) until @@Architect
   acknowledges.
