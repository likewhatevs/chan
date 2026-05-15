# martin-10: Banner reflects selected backend; switch preserves session

Owner: Martin. Depends on: nothing structural; independent of bob-5 /
martin-7 / 8 / 9, can ship in parallel.

## Why

User feedback after manual testing:

1. Selecting Gemini still prints the Claude banner. The banner text
   / branding is hard-coded to Claude instead of reflecting the
   selected backend.
2. Switching backends clears the current chat session. Users expect
   the session to survive a backend swap so they can continue a
   conversation with a different model behind it.

## Files to touch

- `web/src/components/agentBanner.ts` (banner factory + names; see
  `displayAgentName` and `banner` exports imported by InlineAssist
  at line 74).
- `web/src/components/InlineAssist.svelte` where the banner is
  rendered (look for `.agent-banner.claude` styling at line 2913
  and the import site at line 74).
- Wherever backend switching kicks off today: trace from the
  Settings panel's `default_backend` write or martin-7's dropdown
  back into anything that resets `assistantConversations` or
  `assistantStream`.

## Required changes

### Banner

`displayAgentName` and `banner` must dispatch on the active
backend:
- `claude_cli` → "Claude" branding.
- `gemini_cli` → "Gemini" branding.
- `codex_cli` → "Codex" branding (or whatever Codex CLI naming
  matches chan-llm's display strings; check
  `chan_llm::backends::BackendKind::display_name` if it exists).

CSS class `agent-banner.claude` at line 2913 should become a
backend-keyed class (`agent-banner.claude_cli`, etc.) or a single
class with a theme variable. Keep this minimal — three coloured
variants are fine.

### Session preservation on backend switch

Walk the call chain from "user picked a different backend in
Settings" to the conversation state. The current behaviour clears
the session; the fix is to NOT clear it. Conversations live in
`assistantConversations` (file/group/drive scopes); look for any
`endAssistantStream` or `assistantConversations = {...}`
assignment in the backend-switch path and remove the reset.

The reasonable invariant: switching backend changes which CLI is
used for the NEXT turn. In-flight turns either complete on the
old backend or get cancelled (existing cancel path handles that);
already-committed conversation history stays. Don't wipe.

If the cancel-and-restart pattern was intentional for a reason
that isn't immediately obvious (e.g. some piece of state is
backend-specific in a way that breaks on switch), document and
ask. Otherwise just remove the reset.

## Acceptance criteria

1. `npm run check` and `npm test -- --run` are green.
2. Manual: with active backend set to Gemini, open the assistant
   overlay; banner reads "Gemini", not "Claude". Same for Codex.
3. Have a few-turn conversation open. Switch the active backend
   in Settings. The conversation is still there; the next message
   gets answered by the new backend.
4. The banner colour / icon / wording is consistent across the
   overlay, inspector, and any other surface that displays it.

## Out of scope

- Backend-specific feature gating (e.g. greying out tools the
  selected backend can't run). Leave for a separate task if it
  comes up.

## Done means

Post an update to `tasks/journal.md` (status DONE for martin-10,
plus a one-line log entry).
