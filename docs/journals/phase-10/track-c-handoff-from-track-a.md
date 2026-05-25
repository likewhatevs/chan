# Phase 10 Track C Handoff From Track A

Date: 2026-05-25.

Track A is cutting the remaining browser/editor validation items to Track C.
This note is intentionally separate from `roadmap-track-c.md` so Track C can
merge or sequence it without us editing their main roadmap.

## Rich Prompt Browser Validation

Ownership reason:

- The remaining work is browser/editor behavior, not server or desktop
  architecture.
- CodeMirror input, prompt UI state, archive UX, and clipboard preflight are
  Track C surfaces.

Tasks:

- Validate non-empty CodeMirror prompt submit in a browser environment that can
  type into CodeMirror.
- Verify Rich Prompt archive contents.
- Verify clear-on-submit behavior.
- Verify the edited-during-submit race.
- Validate clipboard-dependent Spawn agents preflight.

Track A status:

- Server routes and workspace APIs already exist.
- Track A has no remaining server ownership unless Track C finds a backend
  defect while validating.

## Rapid-Edit Browser/Editor Validation

Ownership reason:

- Track A already pinned the server-side watch/index behavior.
- The remaining repro is editor buffer behavior under rapid saves and reloads.

Tasks:

- Reproduce rapid-edit stale editor/index races in a browser/editor workflow.
- Distinguish editor stale-buffer behavior from server stale-index behavior.
- If queue churn appears rather than editor state drift, send the finding back
  to Track A with a minimal repro.

Track A status:

- Commit `fbacdd9` added
  `rapid_modify_burst_indexes_latest_file_body`.
- The test proves a rapid rewrite burst indexes the final file body and drops
  stale search tokens on the server-side watch apply path.
- Low-file-descriptor smoke and throttle decision are complete; no new fd
  throttle is planned unless Track C finds queue-level churn.
