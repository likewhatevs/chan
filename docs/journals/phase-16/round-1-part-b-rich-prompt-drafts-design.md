# Rich Prompt, Drafts-backed - @@LaneB design (answers Q1-5)

Spec: round-1-rich-prompt-drafts.md (@@Host reframe; SUPERSEDES the base64 /
temp-file design in round-1-part-b-rich-prompt-image.md, now dropped). Back the
Rich Prompt bubble with a chan-workspace DRAFT; image paste works like the
regular editor (images land in the draft folder); every MCP/disk agent reads
the media as files. Design-first: this answers the 5 questions, grounded in the
actual Drafts + editor-paste + endpoint mechanisms. NO code until sign-off.

## Grounding (what already exists, so this is mostly wiring)

- Drafts = `state_dir/drafts/<name>/draft.md` directories (chan-workspace
  drafts.rs), addressed in the unified namespace as `Drafts/<name>/...`, on disk
  under `~/.chan/.../drafts/`. Endpoints already exist:
  `api.createDraft()` -> POST /api/drafts/new -> `{path:"Drafts/<name>/draft.md",
  name}`; `api.discardDraft(path)` -> POST /api/drafts/discard (deletes the dir);
  draft text write via `PUT /api/files/<path>` (api.writeText).
- The editor's image paste is a reusable CM6 extension: `imageDropHandlers({
  getUploadDir, getCurrentPath })` (editor/bubbles/image_drop.ts). Its `paste`
  handler pulls `image/*` (+HEIC) blobs, calls `api.uploadAttachment(file, dir)`
  (POST /api/attachments, multipart, dir = the editing file's directory), and
  inserts `![](path)` at the caret. Point `getUploadDir` at the draft's dir and
  pasted images land in `Drafts/<name>/` with a markdown ref in draft.md - the
  SAME behavior as the regular editor, for free.
- Agents read the media as FILES: chan MCP `read_media`/`read_file`/`list_files`
  over `Drafts/<name>/...`, or directly on disk under `~/.chan`. No base64, no
  per-agent path injection, no codex gap.

## Q1 - DRAFT BINDING

One draft PER TERMINAL (matches @@Host's per-terminal lifecycle). Created
LAZILY on the first Rich Prompt open for that terminal via `api.createDraft()`;
store the returned path on the tab (`tab.richPromptDraftPath`). The draft's
DIRECTORY (`Drafts/<name>/`) is the stable place the agent is pointed at; its
`draft.md` is the prompt text and the folder holds the pasted media.
- Sub-decision (raise to @@Host): visibility stays the global toggle
  (Cmd+Shift+P shows the bubble bound to the ACTIVE terminal's draft) - the
  DRAFT is per-terminal, the open/closed flag can stay global for v1. (Could go
  fully per-terminal later; per-terminal draft + global visible is the minimal
  coherent step.)

## Q2 - REUSE

Keep the bubble's lightweight CM6 (markdown + history + the Cmd+Enter submit
keymap) and ADD exactly one editor extension: `imageDropHandlers` with
`getUploadDir = () => draftDir` and `getCurrentPath = () => draftPath`. That
buys real paste-to-Drafts + `![](path)` with zero new paste code. Bind the doc
to draft.md: load its content on open, debounced-write back on change (PUT
/api/files), reusing the editor's value-sync shape. NOT the full Wysiwyg - just
the markdown editor + the paste extension. (Drag-drop comes along for free since
imageDropHandlers also handles drop.)

## Q3 - SUBMIT

On Cmd+Enter, send the draft.md TEXT through the existing `prompt` frame
(unchanged). The text already contains the pasted images as `![](Drafts/<name>/
img.png)` refs, so the pointer to the media travels WITH the prompt; the agent
resolves each path via MCP/disk after the queue delivers the text. (Optional
nicety: prefix a one-line "media in Drafts/<name>/" hint, but the markdown refs
are sufficient.) Then RESET: clear draft.md text (truncate to empty via PUT) but
KEEP the folder + its media - the agent reads the media AFTER submit, so the
files must persist past the reset. Media accumulates in the folder across
submits and is cleaned on terminal close (Q4).

## Q4 - LIFECYCLE (@@Host rule)

Per-terminal draft folder, deleted on terminal CLOSE. Wire `api.discardDraft(
tab.richPromptDraftPath)` into the existing `registerTerminalCloseSink` path
(TerminalTab already registers a close sink) so closing ANY terminal (regular or
lead) removes its Rich Prompt draft.md + all pasted media - nothing leaks in
Drafts. Reset-on-submit = clear draft.md text, keep the folder (see Q3). A
window RELOAD re-attaches terminals (no close sink fires), so persist
`richPromptDraftPath` on the tab and rebind on reattach; if the draft was
GC'd/missing, recreate lazily (defensive). Sub-decision: clear-text-keep-folder
(my rec) vs new-draft-per-submit (more folders, more cleanup) - I recommend
clear-text-keep-folder.

## Q5 - SEAM (the simplification)

NO @@LaneA prompt-frame change. The frame stays `{type:"prompt", data, agent}`
carrying the draft.md TEXT. ALL file I/O (create draft, upload attachment, write
text, discard) goes through EXISTING chan-server routes (drafts/attachments/
files) - not the terminal WS. This is strictly simpler than the dropped
temp-file seam. (Confirm with @@LaneA only that they expect no image fields on
the frame - I believe none are needed.)

## Net shape / build order (after sign-off)

1. tabs.svelte.ts: add `richPromptDraftPath?` to TerminalTab + serialize it.
2. RichPrompt.svelte: on open, ensure the active terminal's draft
   (createDraft lazily, store path), load draft.md into the CM6 doc, add
   imageDropHandlers(uploadDir=draftDir), debounced write-back on change.
3. Submit: send draft.md text via the prompt frame (as today), then truncate
   draft.md.
4. TerminalTab.svelte: in the close sink, discardDraft(richPromptDraftPath).
5. Team Work lead bubble: it shares the prompt frame + could share the same
   draft-backing, but it ALREADY has its own buffer/editor; keep its current
   backing for v1 unless @@Host wants it Drafts-backed too (raise it).

## DECISIONS NEEDED before building

1. Per-terminal draft + GLOBAL visibility (my rec) vs fully per-terminal Rich
   Prompt state?
2. Reset-on-submit = clear draft.md text, keep folder + media (my rec) vs new
   draft per submit?
3. Does the Team Work lead bubble ALSO move to Drafts-backing now, or only the
   Rich Prompt for v1? (They share the frame; the lead bubble has its own
   editor today.)
4. Confirm Q5: no prompt-frame image fields (text-only) - @@LaneA ack.
