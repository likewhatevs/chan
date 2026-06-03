# task-LaneB-LaneA-8: R2-3 DONE - per-terminal survey (SPA side, verified e2e)

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-6 + followup-LaneA-LaneB-2

## Result: surveys are PER-TERMINAL. Verified END-TO-END against @@LaneD's real
   `tabName` transport. @@Alex's ask + image-3 met.

## Implemented (the B1 rich-prompt pattern, for surveys)

- survey.svelte.ts: state keyed by SLOT - `byTab: Record<tabId,{spec,busy}>` +
  a `windowWide` slot. show/pick/followup/surveyFor/surveyBusy take a
  `SurveySlot` (tab id, or null = window-wide fallback). Two terminals reply
  independently.
- store.svelte.ts (your authorized open_survey HANDLER region): added the
  `tabName?: string|null` frame field (camelCase, per the amendment) +
  `terminalSlotForName(name)` (resolves frame.tabName -> tab.id, matching
  terminalEnvTabName then title, via allTerminalTabs) + the handler
  `showSurvey(frame.survey, slot)`. Falls back to window-wide when tabName is
  absent/unmatched (forward-compat).
- BubbleOverlay.svelte: parameterized by `tabId`. null = window-wide centered
  modal (the App-root fallback mount); a tab id = PER-TERMINAL, anchored over
  the terminal (`.per-terminal`, absolute, z 24000). Keydown moved from the
  window to the card so each survey owns its 1..N/F keys.
- TerminalTab.svelte: mounts `<BubbleOverlay tabId={tab.id} />` over the visible
  terminal (like RichPrompt). App.svelte keeps the App-root `<BubbleOverlay/>`
  (default tabId null) as the window-wide fallback.
- Tests: survey store (per-terminal INDEPENDENCE), BubbleOverlay (window-wide vs
  .per-terminal anchored), TerminalTab (inverted the old "survey overlay no
  longer mounted per terminal" pin -> now asserts the per-terminal mount).

Casing aligned: @@LaneD pinned `#[serde(rename = "tabName")]` (camelCase) on
OpenSurvey, exactly my `frame.tabName` read. No wire drift.

## Files changed (blob fingerprints)

  web/src/state/survey.svelte.ts          blob 014e83003f7856d0117a08643074cb9ce8ccddba
  web/src/state/store.svelte.ts           blob 0a2d96def360c3d56cc8c6d0425b8f14b0ea5dfa
  web/src/components/BubbleOverlay.svelte  blob b223b18894d16e572d9957fb0ea726bffd76ad06
  web/src/components/TerminalTab.svelte    blob 14d89d7088a394c9ebc7274ac672a8033884e551
  web/src/App.svelte                       blob f9221a22ddc61d110a66c71b6036135c0b53bf26 (comment only)
  web/src/state/survey.svelte.test.ts      blob 4e773ba9e683cd5a65ccd742a3fe7fa6ae0ea2e4
  web/src/components/BubbleOverlay.test.ts blob 0225112efa9c20bb0f1ff51286c7bd61421e66b0
  web/src/components/TerminalTab.test.ts   blob 6814564d3a7633c21d5287ce689dae41cba0f6bc

## store.svelte.ts attribution (for your merged-file commit)

  - R2-3 (mine):  lines 60 (allTerminalTabs import), 703-706 (open_survey frame
                  tabName field), 952-963 (terminalSlotForName), 1031-1040 (the
                  handler). 
  - B9 (@@LaneC) + B4 (mine) are already COMMITTED in round-1 (03bb91f8), so
    they are NOT in this round-2 diff. The store.svelte.ts round-2 diff is
    purely my R2-3 (no @@LaneC R2-2 churn - R2-2 is editor blocks.ts/Wysiwyg).

## Own-gate (scoped) - GREEN

  npm test (full vitest)        PASS (1663)
  npm run check (svelte-check)  0 ERRORS in my files
  npm run build                 OK

svelte-check NOTE: the full-tree run reports 8 errors, NONE in my files - they
are other lanes' in-flight WIP (TeamDialogConfig.mcpEnv missing in the team
orchestrator test fixtures = your B5 DIALOG half; a SearchPanel.svelte union
type). Reporting scoped-green per the isolated-gate model.

## Empirical END-TO-END (Chrome, fresh binary WITH @@LaneD's real transport)

Provenance verified (tabName in the binary). 2 panes / 2 terminals:
- `cs terminal survey --tab-name=Terminal-1` + `--tab-name=Terminal-2` raised
  in parallel -> Survey A anchored over Terminal-1 (LEFT), Survey B over
  Terminal-2 (RIGHT), BOTH coexisting, NOT a window-wide modal.
- Answered Survey A -> A dismissed, Survey B INTACT (independence).
- Answered Survey B -> replies round-tripped (A-one / B-two) + both blocked CLIs
  unblocked.
This is @@Alex's exact ask ("each terminal could have their own survey ... not
impact each other") + image-3. Torn down (server by PID, chan remove, rm temp;
no broad pkill).

## Status

R2-3 done - my LAST round-2 item. Everything on my side (B8, B1, B12, cs
mcp_env, B4, R2-3) landed + gate-green + empirically verified. Nothing pushed.
Ready for round close or the next item.
