# Phase 10 Track B Round 3 Handoff

Date: 2026-05-25.

Snapshot code baseline: `454bfa2` (`fix(desktop): install web deps before
build`).

This handoff lists Track B work still pending at the snapshot code baseline
above.
Track A has a separate Round 3 handoff. Track C is complete.

Track B owns public site, manual pages, install surface, generated manual
bundle, release link validation, and public copy freshness.

## Current State

Implemented according to `roadmap-track-b.md`:

- `web-marketing` static generator and GitHub Pages workflow.
- `docs/manual/` as manual source.
- Public install split between desktop artifacts and CLI installer.
- Windows install surface removal.
- GitHub Releases-based `chan upgrade` and first-install links.
- Release workflow shape for `chan-v*` tags.
- Manual bundle generation through `npm run bundle:manual`.
- `npm run check` as the local and CI site gate.
- `npm run smoke:dist` for generated dist HTTP smoke.

## Cross-Track Context Since Round 2

These ad-hoc fixes are already in the snapshot code baseline and should be
treated as current behavior when Track B updates public docs or manual copy.

- Terminal behavior is xterm.js again, not ghostty-web. Commit `3ce1db0`
  fixed terminal session isolation across pane splits, tab moves, reconnects,
  broadcast input, and Rich Prompt target injection.
- Draft management now skips team workspace metadata under `Drafts/` during
  draft preflight/list handling. Commit `05c5cee` prevents those directories
  from being reported as broken drafts missing `draft.md`.
- Desktop build docs can assume the current Makefile path installs `web`
  dependencies before building the embedded web bundle. Commit `454bfa2`
  landed that new-clone build fix.

## Pending Queue

### 1. Full Release Verification After Next Public Tag

Status: blocked on next suitable release tag and public repository access.

Run after:

- the next `chan-v*` tag exists;
- that tag includes the manual bundle;
- the repository is public enough for unauthenticated latest-download HEAD
  checks.

Command:

```bash
cd web-marketing
npm run verify:release
```

Do not use:

- `--allow-missing-manual`
- `--skip-latest-download-heads`

Expected verification:

- desktop downloads exist for DMG, AppImage, and deb;
- standalone CLI tarballs exist for Linux x86_64, Linux aarch64, and macOS
  aarch64;
- `VERSION` exists;
- `SHA256SUMS` exists;
- `chan-manual-<version>.tar.gz` exists;
- latest-download URLs resolve.

### 2. Manual And Public Site Updates For Final Behavior

Status: pending docs update.

Update `docs/manual/` and generated public copy for the current behavior of:

- partial read-only content while large files are still loading;
- editing enabled only after full file and CAS metadata arrive;
- Reload starting a fresh read when a file is still loading;
- inspector report, backlinks, and references filling progressively;
- graph nodes and edges drawing in batches before graph stream `done`;
- slow-read bug report details:
  - path;
  - size;
  - drive type;
  - whether inspector, report, or graph panes were also loading;
- shared inspector Upload and Download actions.

Keep the same source tree serving both:

- public manual pages;
- desktop first-launch seeded manual content.

### 3. Site Gate After Manual Edits

Status: pending after item 2.

Run:

```bash
cd web-marketing
npm run check
```

Expected coverage:

- script syntax checks;
- static site build;
- generated link validation;
- stale copy validation;
- release-contract validation;
- shell syntax for generated `dist/install.sh`.

### 4. Optional Dist Smoke After Manual Edits

Status: recommended when manual routing or generated pages change.

Run:

```bash
cd web-marketing
npm run smoke:dist
```

Expected routes:

- `/`
- `/install/`
- `/manual/`
- at least one nested manual page;
- `/install.sh`;
- removed `/install.ps1` remains absent.

## Coordination Notes

- Track B should not edit desktop implementation files for icon or desktop
  docs/config tasks. Those are handed to Track A in
  `track-a-handoff-from-track-b-logo-and-docs.md`.
- Track B should keep public copy fresh-state only. Do not add migration or
  old-contract framing unless explicitly requested.
- Track B should not re-open Track C. Track C's streaming UI, inspector
  transfer, Rich Prompt browser validation, and rapid-edit validation are
  recorded complete.

## Recommended Round 3 Order

1. Update manual and public site copy for current streaming and transfer
   behavior.
2. Run `npm run check`.
3. Run `npm run smoke:dist` if generated routes changed.
4. Defer full release verification until the next public `chan-v*` tag exists
   with the manual bundle.
