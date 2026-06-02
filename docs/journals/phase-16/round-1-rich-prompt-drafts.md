# Rich Prompt, Drafts-backed (@@Host reframe) - SUPERSEDES the base64 image design

@@Host reframed both the image-paste problem AND the Rich Prompt bubble's
backing. This supersedes round-1-part-b-rich-prompt-image.md (base64/temp-file
+ per-agent path is DROPPED).

## The model (@@Host, verbatim intent)

The Rich Prompt bubble must be BACKED BY a chan-workspace DRAFT:
- The bubble edits a real `draft.md` in the Drafts folder - that file IS the
  prompt text. Reset (clear) the draft on Cmd+Enter.
- Image paste works exactly like the REGULAR EDITOR: pasted images land in the
  Drafts folder, alongside the draft (the draft's associated media).
- The Drafts folder is available BOTH on disk (`~/.chan/...`) AND via the chan
  MCP. So an agent can find the draft's directory + associated media to process
  the prompt: the prompt text comes through the queue, and the agent reads the
  media from Drafts via disk or MCP (`read_media`/`read_file`/`list_files`).

## Why this dissolves the image problem

The earlier finding was right that no agent reads inline base64. This model
sidesteps that entirely: images are FILES in Drafts, and EVERY agent wired to
the chan MCP (or with disk access to the workspace) can read them - claude,
gemini, AND codex. No base64, no per-agent path injection, no codex in-TUI gap.
"Seamless across all agents" is achieved via files + MCP, not encoding.

## What changes vs the merged Rich Prompt

The merged Rich Prompt (in-memory CM6 string) keeps its shell - the float,
inset, rounded, Cmd+Shift+P toggle, universal-on-every-terminal + lead-terminal
model, Cmd+Enter submit through the queue. ONLY the BACKING changes: the editor
is bound to a Drafts `draft.md` (reuse the editor's draft + image-paste
machinery) instead of an in-memory string; paste saves images to Drafts.

## DESIGN-FIRST questions for @@LaneB (post a finding before building)

Ground these in the actual Drafts mechanism (chan-workspace Drafts: "uncommitted
workspaces" per CLAUDE.md), the regular editor's image-paste -> workspace save
path, and the MCP read surface:
1. DRAFT BINDING: one draft per terminal? per lead? a single Rich-Prompt draft?
   created lazily on first open? Where exactly under Drafts does it live
   (a stable path the agent can be pointed at)?
2. REUSE: does the bubble reuse the editor's draft editing + image-paste
   wholesale (so paste-to-Drafts comes free), or a lightweight subset? Keep it
   as light as the bubble needs while getting real image paste.
3. SUBMIT: on Cmd+Enter, what goes through the queue - the draft.md TEXT plus a
   pointer to the draft's directory so the agent knows where the media is? How
   is the agent told where to look (the prompt references the Drafts dir / the
   image paths)? Then RESET the draft.
4. LIFECYCLE (@@Host rule): the Rich Prompt draft is PER-TERMINAL - each
   terminal's Rich Prompt has its OWN Drafts folder (draft.md + pasted media),
   keyed so it can be found + cleaned up. **When the terminal is CLOSED, DELETE
   that terminal's Rich Prompt Drafts folder** (the draft + all its media).
   @@Host named the LEAD terminal specifically; since Rich Prompt is universal,
   apply the same per-terminal close->delete cleanup to every terminal's Rich
   Prompt draft folder. Also decide: reset-on-submit = clear draft.md (keep the
   folder for the next prompt) vs new draft per submit; media handling on reset.
   Net: draft folder lifecycle is tied to the terminal lifecycle (gone on
   close), so nothing leaks in Drafts.
5. SEAM: does this still need anything from @@LaneA's prompt frame, or does the
   Drafts/editor machinery handle the file write so the frame just carries the
   text? (Likely SIMPLER than the temp-file seam - confirm.)

## Sequencing

This re-architects the merged Rich Prompt backing + folds in image paste. It is
@@LaneB's, design-first: post the revised design (answering 1-5, grounded) to
event-lane-b.md for @@Lead/@@Host sign-off BEFORE building. The Team Work lead
bubble shares the prompt frame, so it inherits this too.
