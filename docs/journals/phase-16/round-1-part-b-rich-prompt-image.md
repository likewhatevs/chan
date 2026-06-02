# Rich Prompt image paste - design-first finding (@@LaneB)

Task (event-lane-b 15:22): pasting an image into the Rich Prompt should send it
so it works "seamless with all agents" (claude/codex/gemini); @@Host's framing
is BASE64-encoded, riding the existing prompt-frame `data`. DESIGN-FIRST: find
the encoding/embedding format each agent accepts before building.

## Core finding (this reshapes the base64 premise)

NO CLI agent consumes a raw base64 / data-URI string as an image when it
arrives as TEXT in the prompt stream. Each agent decodes images ITSELF from a
FILE on disk; what differs is the per-agent reference syntax:

- **Claude Code**: reference a file PATH inline in the prompt, e.g.
  `analyze this image: /tmp/x.png`. Works on every platform + is the
  script/queue-friendly path. (Ctrl+V clipboard paste is macOS-only +
  unreliable; direct in-CLI base64 is not a thing.)
- **Gemini CLI**: `@/path/to/image.png` in the prompt - the CLI reads the file
  and injects it as multimodal input. (It does the encoding; we give a path.)
- **Codex CLI**: image via the `--image/-i` LAUNCH flag
  (`codex "..." --image a.png`) or interactive paste. An in-prompt image PATH
  typed into the ALREADY-RUNNING TUI is an OPEN upstream request
  (openai/codex#2218, #19143) - i.e. NOT supported today through the text
  queue.

Consequence: sending base64 (or a data-URI) as prompt text makes every agent
see literal base64 TEXT, not an image. The "base64, seamless across all agents"
approach as stated does NOT work. (Sources at the bottom.)

## What IS viable: temp-file + per-agent path reference

The only cross-agent mechanism deliverable through the text queue is a LOCAL
FILE PATH the agent reads itself. So:

1. On paste, capture the image blob and base64 it (the SPA has the bytes).
2. Persist it to a temp file the agent's PTY can read (in or under the
   terminal session's cwd / a known per-session dir). The SPA cannot write
   disk - this needs a SERVER seam (see @@LaneA question).
3. Inject a per-agent path reference into the submitted prompt text, keyed off
   the SAME `agent` the prompt frame already carries:
   - claude -> `"\n<abs-path>"` (bare path; claude recognizes + reads it)
   - gemini -> `"\n@<abs-path>"`
   - codex  -> NO in-TUI image path today -> DEGRADE: append the path as plain
     text with a note, OR (better) skip image paste for a codex lead until
     upstream lands. Flag, don't fake "seamless".

So: claude + gemini get a genuinely seamless pasted image; codex is the gap
(upstream limitation, not ours). That honest split is the design-first payoff -
"base64 inline" would have silently failed on ALL three.

## The server seam (confirm with @@LaneA)

The image bytes must reach disk where the agent's PTY cwd can read them. Two
shapes:

- **(A) Extend the prompt frame** with an optional image payload, e.g.
  `{type:"prompt", data, agent, image?: {b64, mime}}`; the server writes the
  temp file (it knows the session cwd) and substitutes the per-agent path into
  the delivered text before enqueueing. Cleanest: keeps the path-vs-agent logic
  + the file write server-side (one place), and the queue still serializes.
- **(B) A separate "save pasted image" endpoint** returning a path; the SPA
  then injects the reference into `data` and sends a normal prompt frame.
  Simpler frame, but the SPA needs the server-resolved absolute path and the
  per-agent syntax leaks to the client.

I lean (A) - the per-agent path syntax + the file write belong server-side with
the chord logic that already lives there. EITHER WAY this is a prompt-frame /
endpoint change @@LaneA owns; the 3d6d144e frame as-is only carries text. NEEDS
@@LaneA confirmation before I build the client side.

## Client side (RichPrompt.svelte, once the seam is set)

- Add a CM6 paste handler (`EditorView.domEventHandlers({ paste })`) that pulls
  `image/*` blobs from `clipboardData.items`, base64s them, and routes them to
  the chosen seam (A: stash on the pending submit; B: call the endpoint, inject
  the returned path). Non-image pastes fall through to default CM6 paste.
- The Team Work lead bubble (TeamWork.svelte) submits through the SAME prompt
  frame now (after the decouple), so if the seam is server-side (A) BOTH
  bubbles get image paste for free.

## OPEN QUESTIONS for @@Lead / @@Host (before building)

1. Accept the finding that inline base64 is NOT agent-consumable, and go
   temp-file + per-agent path? (recommended)
2. Codex degradation: skip image paste on a codex lead, or append the path as
   plain text? (claude/gemini work either way.)
3. Frame shape: (A) extend prompt frame with `image?` (my lean) vs (B) a
   separate save-image endpoint - @@LaneA's call.
4. Where does the temp file live + lifecycle (per-session tmp dir, cleaned on
   session close)? - touches @@LaneA's session/cwd land.

## Sources

- Claude Code images (path / drag / Ctrl+V):
  https://smartscope.blog/en/generative-ai/claude/claude-code-image-guide/ ,
  https://felloai.com/claude-code-images/
- Codex CLI `--image` flag + in-prompt path open issue:
  https://developers.openai.com/codex/cli/features ,
  https://github.com/openai/codex/issues/2218
- Gemini CLI `@path` multimodal include:
  https://geminicli.com/docs/cli/tutorials/file-management/
