# @@Frontend task 1

Status: Ready for review.

Goal: Fix the search overlay inspector layout so details stay inside the main search surface and the inspector hide control sits below the overlay close button.

Relevant links: [[phase-2/request.md]], [[phase-1/summary.md]]

Acceptance criteria:

- Search overlay header spans the whole overlay.
- Results and inspector share the body row below the header.
- Inspector close/details control is visually under the overlay close button.
- Existing search interactions still work: scope picker, search status button, hamburger menu, context menu, keyboard navigation, inspector navigation.

Test expectations:

- Run `cd web && npm run check`.

Progress notes:

- Confirmed `web/src/components/SearchPanel.svelte` mounted the header inside `.results`, leaving the inspector title beside the overlay chrome.
- Moved the Search overlay header above the results/inspector body row so the inspector title and close control render inside the overlay body, below the overlay close button.

Completion notes:

- Files changed: `web/src/components/SearchPanel.svelte`, `phase-2/frontend-1.md`.
- Tests run: `cd web && npm run check` (pass).
- Known risks: visual/browser smoke still needed to confirm exact placement in a running app.
- Commit readiness: ready after visual smoke or @@Webtest confirmation.
