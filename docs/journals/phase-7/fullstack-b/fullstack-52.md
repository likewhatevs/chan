# fullstack-52: drop "New Terminal" from terminal menu + sharpen Restart prompt

Owner: @@FullStackB
Cut by: @@Architect
Date: 2026-05-19

## Goal

Two coupled fixes in the terminal tab menu:

1. Drop "New Terminal" entirely from the terminal
   right-click / kebab menu. Cmd+K 1 is now the
   canonical way to spawn a terminal (per
   `fullstack-42` menu cleanup + `fullstack-43`
   context-aware Pane Mode). The menu entry is
   redundant *and* sitting one row below "Restart",
   which is where the mis-click hazard lives.
2. Sharpen the "Restart" confirmation prompt so it
   tells the user the **shell** is going to be
   killed and restarted, not just that "the session
   will be closed and replaced". Current wording is
   too neutral; users mis-click and confirm without
   registering that a running command dies.

The proximity hazard goes away on its own once
"New Terminal" is gone (Restart loses its
mis-clickable neighbour). The prompt sharpening is
defense-in-depth for any remaining accidental
clicks.

## Relevant code

* `web/src/components/TerminalTab.svelte:995` —
  the "New Terminal" `mbtn` button, sitting
  immediately below the "Restart" button at
  line 988. Drop the whole `<button>` block.
* `web/src/components/TerminalTab.svelte:493` —
  `restart()` function. `uiConfirm` prompt at
  496-502 already exists but the `message:` field
  reads "The current terminal session will be
  closed and replaced." — too soft.
* `chordFor("app.terminal.toggle")` reference at
  line 1000 — verify that removing this button
  doesn't leave a dangling shortcut binding (it
  shouldn't; `chordFor` is just a label lookup).
* Audit any other menus / surfaces that still
  carry "New Terminal" as a label (the
  `fullstack-42` cleanup may have missed copies);
  drop them.

## Acceptance criteria

### Menu cleanup

* The "New Terminal" `mbtn` button is removed from
  the terminal-tab menu surface. Cmd+K 1 stays
  the canonical entry; no replacement label.
* Audit pass: grep the SPA for `"New Terminal"`
  (and any per-locale British spelling variants
  introduced by `fullstack-46`); confirm no other
  user-facing copy remains. Cite what you find +
  remove.
* `openNewTerminal` handler: if the only caller
  was this button, drop the handler too. If it's
  still used elsewhere, leave it.

### Prompt sharpening

* `restart()` `uiConfirm` updates:
  * `title`: keep `"Restart terminal?"` or
    refine to something equally direct.
  * `message`: explicitly tell the user the
    **shell** will be killed and restarted, and
    that any running command will be terminated.
    Suggested:
    `"The shell in this terminal will be killed
    and a fresh one started in its place. Any
    running command will be terminated."`
    Wording is your call; the requirement is
    that "shell will restart" + "running command
    will die" both land in the message body.
  * `confirmLabel`: keep `"Restart"`.
  * `destructive: true` stays.
* The "Restart now" button at line 929 (rename-
  pending banner) also routes through `restart()`,
  so the prompt covers it for free. Verify.

### Tests

* Unit test for the menu structure: assert the
  rendered terminal menu no longer contains a
  "New Terminal" entry. Component test or
  snippet-grep over the compiled component
  source, your call.
* Optionally extend an existing terminal-tab test
  to assert the `uiConfirm` call site exists with
  the new message string. Light-touch is fine —
  the value here is the audit grep, not exhaustive
  coverage.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* No backend changes. `api.restartTerminal` and
  the rename-pending plumbing already exist.
* Visual eyeball not strictly required (label
  change + button removal), but if you want to
  ad-hoc-chan-serve to verify the menu reads
  cleanly without the dropped row, lane-boundary
  rule allows it. Teardown after.
* Standing topic-level commit clearance.

## 2026-05-19 14:50 BST — implementation

**Audit.** Grep'd `web/src` for `"New Terminal"` and
`openNewTerminal`: a single call site at
`TerminalTab.svelte:995` plus the `openNewTerminal`
handler at `:619`. The `fullstack-42` cleanup left
only this one entry behind. No British-spelling
variants in play. Dropped:

* `web/src/components/TerminalTab.svelte:995-1001` —
  the "New Terminal" `mbtn` block.
* `web/src/components/TerminalTab.svelte:619-622` —
  the `openNewTerminal` handler.
* `web/src/components/TerminalTab.svelte:17` — the
  `Terminal as TerminalIcon` import (only used by
  the removed button).
* `web/src/components/TerminalTab.svelte:38` — the
  `openTerminalInPane` import (only used by the
  removed handler; `openTerminalInActivePane` still
  drives the seed-input new-terminal path at :838).

**Restart prompt.** `restart()` `uiConfirm`
`message:` field bumped from
`"The current terminal session will be closed and
replaced."` to
`"The shell in this terminal will be killed and a
fresh one started in its place. Any running
command will be terminated."`. Both load-bearing
phrases ("shell will be killed and restarted",
"running command will be terminated") land in the
message body per the acceptance criteria.
`confirmLabel: "Restart"` and
`destructive: true` unchanged.

**Coverage check.** Three `restart()` call sites
(menu Restart at `:988`, rename-pending banner
Restart-now at `:923`, sessionClosed Start-New-
Session at `:929`). All three route through the
same function and pick up the sharpened prompt. The
sessionClosed path skips the confirm (no live
session to kill — `tab.terminalSessionId` is falsy
by then), which is the correct gating.

**Test.** New block in
`web/src/components/TerminalTab.test.ts`:
`TerminalTab menu > "kebab menu no longer renders a
New Terminal entry"`. Mounts the component with an
active `terminalSessionId`, opens the tab-menu via
`openTabMenu(tab.id, anchor)`, scrapes all
`.mbtn-label` text nodes, asserts:

* `labels` non-empty (menu rendered).
* `labels` does NOT include `"New Terminal"`.
* `labels` DOES include `"Restart"` (so a future
  refactor that drops both rows is loud).

**Gate.** `npm run check` 0/0 warnings; `npm run
test` 35 files / 343 tests passed (was 319; +24
from concurrent lane work + my +1, the +1 is
`TerminalTab menu > kebab menu no longer renders a
New Terminal entry`); `npm run build` clean;
`scripts/pre-push` green.

**Visual eyeball.** Skipped per the task note ("not
strictly required"). The change is a label tweak
+ row removal; no pixel surface in play.

**Commit readiness.** Files staged:

* `web/src/components/TerminalTab.svelte` (the
  source change).
* `web/src/components/TerminalTab.test.ts` (the
  menu-grep test).
* This task file.
* `docs/journals/phase-7/fullstack-b/journal.md`
  (journal append).
* `docs/journals/phase-7/alex/event-fullstack-b-architect.md`
  (event-log append).

Proposed commit message:

```
Drop "New Terminal" menu entry and sharpen Restart prompt (fullstack-52)
```

Standing topic-level commit clearance applies.
No HOLD pokes since the 14:40 BST cut.
