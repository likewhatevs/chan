# Draft Workspaces

Date: 2026-05-24
Owner: @@Architect
Status: design note for Phase 9 implementation

## Decision

Drafts remain metadata-backed workspaces, not committed drive content.

The File Browser must stop showing `Drafts` entirely. Drafts should not appear
as a synthetic root row, and users should not be able to create, rename, move,
delete, or drag files through the File Browser using `Drafts/...` paths.

The MCP server should continue exposing draft paths. A user may open a terminal
next to a draft and launch an agent to help write it. MCP tool descriptions must
make the contract explicit: `Drafts/...` is a draft workspace namespace backed
by chan metadata, not a directory under the drive root, and it is not committed
drive content until the user saves the draft to the drive.

Cmd+N continues to create one draft directory under metadata and a `draft.md`
inside it. Image paste and drag/drop in the editor continue to write files next
to `draft.md` in that draft directory.

Closing a draft tab is a lifecycle decision, not a normal tab close:

- Discard Draft: move the whole draft directory to metadata draft trash.
- Save to Drive: first flush the editor buffer, then promote the draft into the
  drive through chan-server and chan-drive.

## Save Contract

The save API should be explicit and no-clobber by default. It must not reuse
ambient `write_text` or `rename` behavior where overwrite semantics are a side
effect.

For a draft directory containing only `draft.md`, Save to Drive prompts for a
final markdown file path. The backend promotes `draft.md` to that file. The
target must be sandboxed under the drive root and must not already exist.

For a draft directory containing anything beyond `draft.md`, Save to Drive
prompts for a target directory path.

If the directory exists, chan-drive moves the draft directory contents into
that directory. If the directory does not exist, chan-drive promotes the draft
directory as that target directory. In both cases the operation is recursive
and preserves normal files and subdirectories created from the editor or a
terminal.

Any target collision should abort before touching the drive. This includes a
file or directory that would be overwritten anywhere under the destination.
Directory replacement is not implicit.

## Preflight

Before promoting a draft, chan-drive should build a plan and validate it:

- the source draft directory exists under this drive's metadata root
- `draft.md` exists and is a regular file
- every entry is inspectable
- every entry is a regular file or directory
- every destination path is sandboxed under the drive root
- no destination path already exists

Symlinks, FIFOs, sockets, devices, unreadable entries, and traversal attempts
make the draft broken. The UI should show a clear error and keep the draft
open when possible.

The promotion itself should be a logical move. Same-filesystem rename is fine
when available. Cross-filesystem promotion should either use a durable
copy-then-delete path with rollback discipline, or fail cleanly and mark the
draft broken. It must not leave a silent partial save.

## Broken Drafts

Drive boot should include a draft preflight pass over the metadata draft root.
This is the right place to detect problems left by failed saves, failed
discards, manual terminal edits, or permission changes.

A broken draft should be recorded or surfaced with a reason. Examples:

- missing `draft.md`
- unreadable draft directory
- unsafe entry type
- save or discard operation left an incomplete marker
- destination cleanup failed after a successful copy

Every drive boot should surface broken drafts to the user until they are
resolved. The warning should identify the draft and the reason. It should not
block opening the drive unless the metadata root itself cannot be inspected.

## Normal File Close

The current editor does have a dirty-buffer close path: `closeTab` prompts when
`content != saved`. Autosave is the normal path, but close does not force an
immediate save before closing.

Phase 9 should tighten this for normal file tabs. Closing a dirty committed
file should attempt an immediate save and close only after success. If the save
conflicts or fails, keep the tab open and show the existing conflict or error
surface.

Draft tabs use the draft lifecycle modal instead. Save to Drive should flush
the current buffer before promotion. Discard should not silently drop a dirty
buffer without the user choosing Discard Draft.

## MCP Contract

MCP may list, read, write, and resolve `Drafts/...` paths so agents can work on
draft content. Tool descriptions and prompts must say that these paths are
draft workspaces and may resolve to metadata outside the drive root.

Agents should not treat `Drafts/...` as committed drive content. If an agent
needs a host filesystem path for a draft terminal workflow, it must call
`resolve_path`.

The web File Browser and MCP do not need identical visibility rules. File
Browser hides Drafts to prevent accidental file-management operations against
metadata. MCP exposes Drafts because agent-assisted drafting is a supported
workflow.

## Collision Audit Notes

Herschel's 2026-05-24 audit found that existing collision behavior is uneven:

- `Drive::promote_draft` must not be exposed as-is. It joins `target_rel`
  directly under the drive root without the normal sandbox validation.
- `POST /api/files` blocks existing regular files but create-dir is
  idempotent for existing directories because `Drive::create_dir` uses
  `create_dir_all`.
- File Browser move UI can imply directory overwrite, but backend rename only
  overwrites regular files. Existing target directories are rejected.
- Attachment upload suffixes regular-file collisions, but directory or special
  collisions are rejected. There is still a race between suffix selection and
  write.
- Delete moves into unique trash entries. Restore collision is guarded in
  chan-drive, but there is no active web restore route in this flow.

Draft save/promote should not inherit these inconsistencies. It should use a
dedicated no-clobber plan with explicit 409 conflict errors for occupied
destinations, 400 errors for invalid paths or unsafe draft contents, and 500
only for unexpected internal failures.

## Ownership

chan-drive owns draft validation, discard, metadata trash, broken-draft scan,
and promote operations.

chan-server owns the HTTP routes and maps draft-save conflicts to user-facing
status codes.

The web UI owns the draft close modal, save target prompts, and hiding Drafts
from the File Browser. It should not infer filesystem state beyond what the
server returns.

MCP descriptions and prompts live with chan-llm and should be updated in the
same wave that changes Drafts visibility.

## Implementation Log

2026-05-24: First implementation slice landed the visibility split. The
File Browser `/api/files` route no longer injects `Drafts` into root listings
and rejects direct `dir=Drafts` expansion, while `/api/files/Drafts/...`
read/write behavior remains available for editor tabs, graph, terminals, and
MCP. The Svelte tree refresh path now ignores Drafts watcher events because
Drafts are no longer visible in File Browser. chan-llm prompts and tool
descriptions now describe `Drafts/...` as uncommitted metadata-backed
workspaces rather than committed drive content.

2026-05-24: The chan-drive draft lifecycle core now has explicit inspect,
discard, and no-clobber promote primitives. Promotion validates the draft
workspace, rejects path escapes and occupied destinations, saves single-file
drafts as a final markdown/text file, and saves workspace drafts by creating
or merging a target directory without overwriting existing files.
