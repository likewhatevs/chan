# drive -> workspace rename: scope (phase 12, @@LaneB)

Scoping-architect deliverable. Phase 1 = scope, NOT codemod. This doc
inventories every "drive" surface, characterizes the risks, records @@Alex's
ratified decisions, and proposes the codemod sequencing vs @@LaneA / @@LaneC.

main baseline at scope time: `fe6e126`. Nothing here is committed yet; the
codemod (phase 2) is gated on @@Architect picking the windows. All five scope
decisions are RATIFIED (section 1).

## 0. Ratified decisions (settled with @@Alex 2026-05-27)

| # | Decision                              | RULING                            |
|---|---------------------------------------|-----------------------------------|
| 1 | Top-level term + "workspace" collision| Option C: Workspace = the drive,  |
|   |                                       | rename the incumbents out first   |
| 2 | Rename tunnel domain `drive.chan.app` | NO. Domain string stays as-is.    |
| 3 | On-disk + routes: break or pin        | FULL CLEAN BREAK. Refactor as if  |
|   |                                       | "drive" never existed. No migrate.|
| 4 | uniffi native bindings impact         | None today (no live bindings).    |
| 5 | Rename the `Library` handle too       | NO. `Library` is the right name.  |

## 1. Decision detail + outcomes

### 1.1 Top-level term: Option C (ratified)

Background: "workspace" is ALREADY the most overloaded term in the codebase,
used for FOUR distinct concepts, all of which (except Cargo) denote a
SUB-directory of working material INSIDE a drive (under the per-drive Drafts
metadata subtree):

| Existing meaning        | Where                                          |
|-------------------------|------------------------------------------------|
| Cargo `[workspace]`     | root Cargo.toml (the Rust workspace)           |
| team workspace          | `chan-drive/src/teams.rs`, `routes/teams.rs`,  |
|                         | `team-{name}/` dirs, `TeamRef`/`TeamConfig`,   |
|                         | `web/src/state/teamOrchestrator.svelte.ts`     |
| draft workspace         | `chan-drive/src/drafts.rs` (`promote_workspace`|
|                         | etc.; a draft that is a directory)             |
| Rich Prompt workspace   | `chan-drive/src/rich_prompts.rs`,              |
|                         | `RichPromptWorkspace`, `create_rich_prompt_*`, |
|                         | `web/src/components/TerminalRichPrompt.svelte` |

`Drive` -> `Workspace` would otherwise add a fifth meaning AND invert the
hierarchy (a "Workspace" drive would contain "team workspaces"). @@Alex chose
Option C: keep `Workspace` as the top-level term (the renamed drive) but FIRST
free the word by renaming the incumbents. The Cargo `[workspace]` is unrelated
(Rust tooling) and stays. The incumbent renames are scoped as codemod chunk 0
(section 5.0).

### 1.2 Tunnel domain: keep `drive.chan.app` (ratified)

The domain `drive.chan.app` / `{user}.drive.chan.app` stays exactly as-is. It
is DNS + TLS cert + nginx + gateway + marketing, orthogonal to the code rename,
and not worth the infra/redirect churn now. The codemod LEAVES every
`drive.chan.app` string untouched (~40 hits across chan-tunnel-*, chan-server,
chan CLI, README/AGENTS/CLAUDE, marketing, manual). Any future domain rename is
a separate release-lane decision. NOTE: this is the one place where "drive" the
STRING survives the "as if drive never existed" rule, by explicit @@Alex
ruling; the codemod must not sweep it away.

### 1.3 On-disk + routes: full clean break (ratified)

@@Alex: "we will refactor like the name drive never existed; we are in
pre-release phase, we can make breaking changes now or never." So the rename
reaches the wire surfaces too, with NO back-compat shim and NO migration:

  - On-disk: `default_drive_root` -> `default_workspace_root`; `[[drives]]` ->
    `[[workspaces]]`; the `~/.chan/drives/<key>/` metadata dir ->
    `~/.chan/workspaces/<key>/`. (`~/.chan/` itself is the brand, stays.)
  - HTTP routes: `/api/drive` -> `/api/workspace`, `/api/drive/bootstrap` ->
    `/api/workspace/bootstrap`, `/api/cloud-drives` -> `/api/cloud-workspaces`.

Consequence (recorded, accepted): existing `~/.chan/config.toml` registries do
not load after the rename; `[[drives]]` rows are lost and re-added on next
register; old `~/.chan/drives/<key>/` trees are orphaned (re-index from
scratch). Any bookmarked `/api/drive` URL or saved client breaks. Acceptable
pre-release. Sequencing note: the WIRE-string flip (routes + on-disk serde)
does not have to land in the same commit as the Rust type rename; it lands with
the frontend consumer (section 5) so the end state is clean WITHOUT forcing one
mega-commit.

### 1.4 uniffi: no impact (ratified, FYI)

No `uniffi` dependency in any Cargo.toml, no `.udl`, no `#[uniffi::export]`.
Every "uniffi" reference is a forward-looking doc comment. Zero live bindings
to break. The only effect is forward-looking: a future native shell links
`chan-workspace`, and `ChanError` carries renamed variants. The
"keep-it-uniffi-clean" design comments get their text updated; nothing else.

### 1.5 `Library` handle: unchanged (ratified)

`Library` is the per-machine handle that owns the registry and hands out
per-drive handles. @@Alex: "Library is the right name." It has no user-facing
footprint (never in CLI/config/URLs). It stays `Library`; only its methods that
contain "drive" rename (`open_drive` -> `open_workspace`, `register_drive` ->
`register_workspace`, `default_drive_root` -> `default_workspace_root`,
`list_drives` -> `list_workspaces`, `move_drive`/`reset_drive`/
`unregister_drive`/`drive_paths_for` similarly).

## 2. Full surface inventory

Scale measured at baseline `fe6e126`. Counts are occurrences (rg), not unique
symbols, and include comments + tests + docs unless noted.

### 2.1 Crate + Rust identifiers

| Surface                          | Scale                              |
|----------------------------------|------------------------------------|
| Files referencing `chan[-_]drive`| 147 (incl. docs, .git logs)        |
| `chan_drive::` path-qualified    | chan 34, chan-server 367, llm 29,  |
|                                  | report 2 (~432 total)              |
| `Drive` identifier (whole word)  | chan-server 194, chan-drive 178,   |
|                                  | chan 24, llm 10, report 1,         |
|                                  | tunnel-* ~10 (~420 total)          |
| Crate dir + Cargo.toml package   | `crates/chan-drive/`, member list  |
|                                  | in root Cargo.toml + 4 dependents  |

Rename candidates: `Drive` -> `Workspace`, `KnownDrive` -> `KnownWorkspace`,
`DrivePaths` -> `WorkspacePaths`, `DriveCell` (chan-server state.rs) ->
`WorkspaceCell`, `DriveLock` (internal) -> `WorkspaceLock`, the `Library::
*_drive*` method family (1.5), `ChanError::{DriveNotRegistered, DriveLocked,
DriveAlreadyOpen}` -> `Workspace*`. `Library` stays (1.5). chan-tunnel-* crate
NAMES are "tunnel" not "drive" and do NOT rename; their internal "drive" prose
+ the domain string are governed by 1.2.

### 2.2 HTTP / wire surface (clean break per 1.3)

| Route (now)              | Route (after)              | File              |
|--------------------------|----------------------------|-------------------|
| `GET/PATCH /api/drive`   | `/api/workspace`           | lib.rs, drive.rs  |
| `GET /api/drive/bootstrap`| `/api/workspace/bootstrap`| routes/drive.rs   |
| `GET /api/cloud-drives`  | `/api/cloud-workspaces`    | routes/drive.rs   |

Consumed by `web/src/api/client.ts` + `types.ts`. The route handler FILE
`routes/drive.rs` -> `routes/workspace.rs`. CAUTION: `host.rs` prefix tests use
`/driveway/api/drive` and `/drive/api/drive` deliberately; update by HAND, not
find/replace.

### 2.3 On-disk (clean break per 1.3)

`default_drive_root` -> `default_workspace_root`; `[[drives]]` ->
`[[workspaces]]`; `~/.chan/drives/<key>/` -> `~/.chan/workspaces/<key>/`. Serde
field/table renames in `crates/chan-drive/src/registry.rs` + the path
constructors in `paths.rs` (`drive_paths_for_metadata_key`, the `"drives"` dir
literal). No migration code.

### 2.4 CLI + user-facing copy (`crates/chan/src/main.rs`)

~331 lines mention "drive": help/about strings, the `--drive PATH` flag on
`contacts import` (-> `--workspace`), the `chan serve` tunnel help block (keep
the `drive.chan.app` URL string per 1.2, reword surrounding prose), error /
refusal messages. NO subcommand is named "drive", so the CLI grammar is stable;
only copy + the one flag change.

### 2.5 Frontend (`web/src`, user-facing) -- ~1145 lines, ~50 files

Hottest: `state/store.svelte.ts` 176, `components/GraphPanel` 100,
`state/tabs.svelte.ts` 45, `components/FileBrowserSurface` 42,
`components/DriveWarningsModal` 40, `state/scope.svelte.ts` 34, `api/types.ts`
31, `api/client.ts` 29, `GraphCanvas` 23, `SettingsPanel` 22, `DriveInfoBody`
16, `FileTree` 14, `App.svelte` 13. Renamed files: `DriveWarningsModal.svelte`
-> `WorkspaceWarningsModal.svelte`, `DriveInfoBody.svelte` ->
`WorkspaceInfoBody.svelte`, `state/driveWarnings.test.ts` ->
`workspaceWarnings.test.ts`.

CRITICAL overlap: `store.svelte.ts`, `GraphPanel`, `FileBrowserSurface`,
`scope.svelte.ts`, `tabs.svelte.ts`, `FileTree`, `App.svelte` are EXACTLY the
files @@LaneA is editing for the overlay/scope-wipe + FB work. See section 5.

### 2.6 Desktop (Tauri, `desktop/`)

~700 lines across `src-tauri/src/{main,serve,default_drive,config,registry,
tunnel/*,embedded,watcher}.rs` + `src/main.js`. Includes `default_drive.rs`
(-> `default_workspace.rs`) and the load-bearing default-Chan-drive lifecycle.
No other lane touches desktop: zero cross-lane risk, rides with the backend
chunk.

### 2.7 Docs + meta

`design.md`, `crates/chan-drive/design.md` (file moves with the crate dir to
`crates/chan-workspace/design.md`), `CLAUDE.md`, `AGENTS.md`, `README.md`,
`CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`, `docs/config-reference.md`,
`docs/manual/tunnel.md`, `web/EDITOR.md`, the `crates/*/README.md`. Leave
`drive.chan.app` strings (1.2).

## 3. Naming scheme (final, Option C)

```
-- chunk 0: free the word (incumbents, see 5.0) --
team workspace (prose only)      -> "team"          (idents already Team*)
draft workspace fns (drafts.rs)  -> promote_draft, preflight_draft_merge,
                                    copy_draft_to_new_dir, copy_draft_into_*
RichPromptWorkspace (pub type)   -> RichPromptSession  [PROPOSED, see 5.0]
*_rich_prompt_workspace methods  -> *_rich_prompt_session
TerminalRichPrompt workspace*    -> session* (state fields + .workspace-row CSS)

-- chunk 1+2: the drive -> workspace rename --
chan-drive (crate)               -> chan-workspace
chan_drive:: (paths)             -> chan_workspace::
Drive (type)                     -> Workspace
KnownDrive                       -> KnownWorkspace
DrivePaths                       -> WorkspacePaths
DriveCell (chan-server)          -> WorkspaceCell
DriveLock (internal)             -> WorkspaceLock
Library                          -> Library  (UNCHANGED, 1.5)
Library::open_drive et al.       -> open_workspace, register_workspace,
                                    unregister_workspace, move_workspace,
                                    reset_workspace, list_workspaces,
                                    default_workspace_root, workspace_paths_for
ChanError::DriveNotRegistered    -> WorkspaceNotRegistered
ChanError::DriveLocked           -> WorkspaceLocked
ChanError::DriveAlreadyOpen      -> WorkspaceAlreadyOpen
default_drive_root (config key)  -> default_workspace_root
[[drives]] (config table)        -> [[workspaces]]
~/.chan/drives/<key>/            -> ~/.chan/workspaces/<key>/
/api/drive, /api/drive/bootstrap -> /api/workspace, /api/workspace/bootstrap
/api/cloud-drives                -> /api/cloud-workspaces
routes/drive.rs                  -> routes/workspace.rs
DriveWarningsModal.svelte        -> WorkspaceWarningsModal.svelte
DriveInfoBody.svelte             -> WorkspaceInfoBody.svelte
desktop default_drive.rs         -> default_workspace.rs
drive.chan.app                   -> UNCHANGED (1.2)
Cargo [workspace]                -> UNCHANGED (Rust tooling)
```

## 4. Back-compat statement (final)

Full clean break, pre-release, no migration (1.3). After the codemod: existing
`~/.chan/config.toml` registries fail to load and are rebuilt by re-registering
workspaces; orphaned `~/.chan/drives/<key>/` trees re-index from scratch; any
bookmarked `/api/drive`-class URL or persisted client breaks. The one preserved
"drive" string is the tunnel domain `drive.chan.app` (1.2).

## 5. Codemod sequencing proposal

The frontend chunk overlaps @@LaneA almost completely on the hottest files
(`store.svelte.ts`, `GraphPanel`, `FileBrowserSurface`, `scope`, `tabs`,
`FileTree`, `App.svelte`); a mid-flight frontend codemod forces @@LaneA to
rebase a ~1100-line diff. @@LaneC is ad-hoc, lower-volume, terminal/shortcut
focused. Refinement on the clean-break ruling: "clean break" describes the END
STATE, not a one-commit constraint. The Rust crate/type rename can land EARLY
while the route-path + on-disk SERDE STRINGS are left as literals; those strings
flip together with the frontend consumer in the last chunk. End state is 100%
clean; sequencing stays splittable.

### 5.0 Chunk 0 -- free the word (Option C precondition)

Frees "workspace" for the drive rename. Three incumbents:
  - team: prose/comment edits only in `teams.rs` + `routes/teams.rs` +
    `teamOrchestrator.svelte.ts` (identifiers are already `Team*`). Trivial.
  - draft: rename PRIVATE fns in `drafts.rs` (`promote_workspace` ->
    `promote_draft`, `preflight_workspace_merge`, `copy_workspace_*`) + prose.
    Internal to the crate.
  - rich-prompt: `RichPromptWorkspace` (PUBLIC type, re-exported) -> proposed
    `RichPromptSession`; Drive methods `*_rich_prompt_workspace` ->
    `*_rich_prompt_session`; `chan-server/src/routes/rich_prompts.rs`; frontend
    `TerminalRichPrompt.svelte` state fields (`workspaceError`/`workspacePath`/
    `workspaceBusy`/`workspaceAbs`/`copyWorkspacePath`) + `.workspace-row` CSS.
    NOTE: the rich-prompt API JSON shape changes with the struct, so its
    frontend consumer moves in the SAME chunk.
  SUB-DECISION (for @@Alex/@@Architect): `RichPromptWorkspace` ->
  `RichPromptSession` (active compose-and-submit lifecycle) is my proposal;
  `RichPromptDraft` is the alternative (it lives under Drafts metadata).
  Defaulting to `RichPromptSession` unless overridden.
  Overlap: terminal/rich-prompt frontend, NOT @@LaneA graph/FB. Possible
  @@LaneC terminal overlap -> declare on the cross-lane channel. Independently
  gated; can land early once the rich-prompt target name is set.

### 5.1 Chunk 1 -- backend internals + crate rename (Rust-only, wire deferred)

`chan-drive` -> `chan-workspace` (dir, package, `chan_workspace::` paths in
server/llm/report), the Rust types (2.1), `Library` methods (1.5), `ChanError`
variants, CLI copy + `--workspace` flag, desktop. KEEP the route-path strings
(`/api/drive`) and on-disk serde names (`[[drives]]`, `default_drive_root`,
`drives/`) as literals for now (emitted by renamed code; app still works). Zero
frontend, zero @@LaneA files. Lands EARLY in a short @@Architect backend freeze
(no lane blocked: both are frontend). Independently gated + merge-ready.

### 5.2 Chunk 2 -- wire flip + frontend (atomic, the clean-break change)

Flip route paths -> `/api/workspace*` + `/api/cloud-workspaces`; flip on-disk
serde -> `[[workspaces]]` / `default_workspace_root` / `workspaces/` dir; rename
the entire `web/src` (api/client + types + components + state + the renamed
`.svelte` files). Producer (route/on-disk strings) + consumer (frontend) move
together so there is no broken intermediate. Lands LAST, after @@LaneA reports
graph/FB merged + paused, in a brief `web/src` + routes freeze. Mind the
`host.rs` prefix tests (2.2).

### 5.3 Chunk 3 -- docs sweep

`README`/`AGENTS`/`CLAUDE`/`CONTRIBUTING`/`SECURITY`/`CHANGELOG`/`design.md`/
`config-reference`/`manual` + crate READMEs + the `crates/chan-workspace/
design.md` move. Leave `drive.chan.app` strings (1.2). Mechanical; rides with
chunk 1 or trails as cleanup.

### 5.4 Cross-lane

@@LaneB announces chunk-0 (rich-prompt/terminal touch -> @@LaneC), chunk-1
(backend freeze), and chunk-2 (web/src + routes freeze) to @@Architect.
@@LaneA's scope-wipe renames the "scope" CONCEPT while this renames "drive":
orthogonal terms, same shared files (`store`, `GraphPanel`, `scope`, `tabs`),
so chunk 2 comes AFTER @@LaneA, never interleaved. @@Architect picks the windows
and owns the freeze announcements.

## 6. Status

All five scope decisions ratified by @@Alex (section 0). Phase 2 (codemod) is
ready to begin on @@Architect's window calls. Outstanding minor sub-decision:
the `RichPromptWorkspace` target name (5.0), defaulting to `RichPromptSession`.
Worktree `../chan-lane-b` to be created when @@Architect greenlights chunk 0/1.
