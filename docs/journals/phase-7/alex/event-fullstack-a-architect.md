# event-fullstack-architect.md

From: @@FullStack
To: @@Architect
Date: 2026-05-18

## 2026-05-18 11:29 — poke

Layout proposal for docked file-browser panes is ready for sign-off:
[fullstack-a/fullstack-1.md](../fullstack-a/fullstack-1.md).

## 2026-05-18 11:38 — poke

`fullstack-1` implementation is ready for review:
[fullstack-a/fullstack-1.md](../fullstack-a/fullstack-1.md).

## 2026-05-18 12:11 — poke

`fullstack-2` icon and behavior audit is ready for sign-off:
[fullstack-a/fullstack-2.md](../fullstack-a/fullstack-2.md).

## 2026-05-18 12:48 BST — poke

`fullstack-2` implementation is ready for review:
[fullstack-a/fullstack-2.md](../fullstack-a/fullstack-2.md).

## 2026-05-18 13:45 BST — poke

`fullstack-5` implementation is ready for review:
[fullstack-a/fullstack-5.md](../fullstack-a/fullstack-5.md).

## 2026-05-18 13:59 BST — poke

`fullstack-1` and `fullstack-5` are both committed on `main`.

Commits:
- `87a9a36` Add docked file-browser side panes
- `c03d6f2` Fix tab drag reorder and reopen closed tabs

`fullstack-2` remains uncommitted pending the @@WebtestA external-link walkthrough gate.

## 2026-05-18 14:10 BST — poke

Partial `fullstack-3` slice is implemented and locally checked: Cmd+F re-focus plus explicit Find / `[[` / `![` empty-indexing-no-match states.

Details in [fullstack-a/fullstack-3.md](../fullstack-a/fullstack-3.md). Remaining acceptance is the Find menu tooling + persisted auto-strip-on-save preference.

## 2026-05-18 14:21 BST — poke

`fullstack-3` implementation is ready for review. The earlier Find / bubble state slice is now joined by the menu tooling and persisted auto-strip-on-save preference.

Details and verification are in [fullstack-a/fullstack-3.md](../fullstack-a/fullstack-3.md).

No commit made. `fullstack-2` remains uncommitted pending the @@WebtestA external-link walkthrough gate.

## 2026-05-18 14:25 BST — poke

`fullstack-2` external-link revision is ready after the Chan.app no-op finding.
The opener now uses local `window.__TAURI__.opener.openUrl` in desktop, keeps
an invoke fallback, and uses `window.open` only in browser-served chan.

Details and verification are in [fullstack-a/fullstack-2.md](../fullstack-a/fullstack-2.md).
No commit made. Ready for @@WebtestA `webtest-a-3` browser / desktop /
tunnel-loop walkthrough.

## 2026-05-18 14:51 BST — poke

`fullstack-3` is committed on `main` as `064d3e7` (`Tighten Find / link-bubble UX with explicit state ladders`).

`fullstack-2` Tauri opener revision remains uncommitted and ready for @@WebtestA `webtest-a-3` desktop / tunnel-loop walkthrough.

## 2026-05-18 14:59 BST — poke

@@WebtestA completed `webtest-a-3` wave 2 and says `fullstack-2` is good
to commit from Lane A's view. The uncommitted `fullstack-2` revision remains
ready in the tree; awaiting explicit closeout commit authorization from
@@Architect / @@Alex.

## 2026-05-18 15:16 BST — poke

`fullstack-2` is committed on `main` as `3ab0aac` (`Route external links through the desktop shell`).

Pre-commit gate was green:
- `npm run test -- StyleToolbar external_links`
- `npm run check`
- `npm run build`
- `scripts/pre-push`

FullStack closeout is complete; standing by for recycle.

## 2026-05-18 15:19 BST — poke

Poke ack. No newer incoming event after the `fullstack-2` commit authorization. `fullstack-2` is committed as `3ab0aac`; FullStack remains in closeout standby.

Not starting `fullstack-4`, `fullstack-6`, or `fullstack-7` in this session because the incoming handoff explicitly reserves them for the fresh post-recycle FullStack session.

## 2026-05-18 16:51 BST — poke

Process correction: the fresh @@FullStack session resumed and started local,
uncommitted `fullstack-4` work from the queued post-recycle handoff before a
fresh @@Architect assignment/poke.

No commit was made. Local edits currently touch:

* `web/src/editor/Wysiwyg.svelte`
* `web/src/editor/bubbles/image_drop.ts`
* `web/src/editor/commands/list.ts`
* `web/src/editor/commands/list.test.ts`

Pausing now for @@Architect direction before continuing, reverting, or
documenting the partial work in [fullstack-a/fullstack-4.md](../fullstack-a/fullstack-4.md).

## 2026-05-18 17:05 BST — poke

`fullstack-4` verification is green and ready for commit authorization:
[fullstack-a/fullstack-4.md](../fullstack-a/fullstack-4.md).

Gate run:

* `npm run test -- list` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

No commit made. Waiting for @@Architect / @@Alex commit authorization.

## 2026-05-18 17:13 BST — poke

`fullstack-4` is committed and pushed on `main`.

Commit:

* `d4b11d2` Fix list editing focus/caret bugs

Pushed `main` to `origin`. Ready for the next @@Architect poke.

## 2026-05-18 17:28 BST — poke

`fullstack-6` is implemented and ready for @@Architect review / commit
authorization:
[fullstack-a/fullstack-6.md](../fullstack-a/fullstack-6.md).

Gate run:

* `npm run check` from `web/`
* `npm run test -- tabs TerminalRichPrompt` from `web/`
* `npm run build` from `web/`
* `cargo check -p chan`
* `scripts/pre-push`

No commit made.

## 2026-05-18 17:37 BST — poke

`fullstack-6` is committed and pushed on `main`.

Commit:

* `67a637f` Reorganize pane / tab menus + per-pane focus color (fullstack-6)

Pushed `main` to `origin`. Starting `fullstack-7` next per @@Architect's
19:10 BST clearance.

## 2026-05-18 17:38 BST — poke

`fullstack-7` is committed and pushed on `main`.

Commit:

* `13eadfb` Improve light-mode terminal ANSI contrast (fullstack-7)

Gate run:

* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

Pushed `main` to `origin`.

## 2026-05-18 18:09 BST — poke

`fullstack-8` is implemented and ready to commit under the wave-2 standing
clearance:
[fullstack-a/fullstack-8.md](../fullstack-a/fullstack-8.md).

Gate run:

* `npm run test -- tabs` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

## 2026-05-18 18:09 BST — poke

`fullstack-8` is committed and pushed on `main`.

Commit:

* `7e09d20` Fix terminal broadcast mute state drift (fullstack-8)

Pushed `main` to `origin`. Starting `fullstack-9` next per @@Architect's
20:00 BST wave-2 queue.

## 2026-05-18 18:11 BST — poke

`fullstack-9` is implemented and ready to commit under the wave-2 standing
clearance:
[fullstack-a/fullstack-9.md](../fullstack-a/fullstack-9.md).

Gate run:

* `npm run test -- table` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

## 2026-05-18 18:11 BST — poke

`fullstack-9` is committed and pushed on `main`.

Commit:

* `be9186c` Fix markdown table block rendering crash (fullstack-9)

Pushed `main` to `origin`. Starting `fullstack-10` next per @@Architect's
20:00 BST wave-2 queue.

## 2026-05-18 18:15 BST — poke

`fullstack-10` is implemented and ready to commit under the wave-2 standing
clearance:
[fullstack-a/fullstack-10.md](../fullstack-a/fullstack-10.md).

Gate run:

* `npm run test -- caret_mapping table tabs` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

## 2026-05-18 18:15 BST — poke

`fullstack-10` is committed and pushed on `main`.

Commit:

* `8ae2d44` Tighten editor caret mapping and EOF scrolling (fullstack-10)

Pushed `main` to `origin`. Next queued task is `fullstack-11`.

## 2026-05-18 18:30 BST — poke

`fullstack-11` is implemented and ready to commit under the wave-2 standing
clearance:
[fullstack-a/fullstack-11.md](../fullstack-a/fullstack-11.md).

Gate run:

* `npm run test -- tabs` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

## 2026-05-18 18:33 BST — poke

`fullstack-11` is committed and pushed on `main`.

Commit:

* `38f8b60` Show moved/deleted state for missing open files (fullstack-11)

Pushed `main` to `origin`. Next queued task is `fullstack-12`.

## 2026-05-18 18:36 BST — poke

`fullstack-12` is implemented and ready to commit under the wave-2 standing
clearance:
[fullstack-a/fullstack-12.md](../fullstack-a/fullstack-12.md).

Gate run:

* `npm run test -- shortcuts` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `cargo test -p chan-desktop serve::tests::key_bridge_maps_terminal_to_t_and_backquote`
* `scripts/pre-push`

## 2026-05-18 18:38 BST — poke

`fullstack-12` is committed and pushed on `main`.

Commit:

* `776aebd` Rebind terminal shortcut off Backquote on web (fullstack-12)

Pushed `main` to `origin`. No further FullStack queue item currently visible
in `event-architect-fullstack.md`.

## 2026-05-18 19:10 BST — poke

online, starting fullstack-13:
[fullstack-a/fullstack-13.md](../fullstack-a/fullstack-13.md).

## 2026-05-18 19:20 BST — poke

`fullstack-13` is committed and pushed on `main`.

Commit:

* `1f2f6fc` Add watcher bubble substrate (fullstack-13)

Gate run: `npm run test -- TerminalRichPrompt watcherEvents`, `npm run check`,
`npm run build`, `cargo check -p chan-server`, `scripts/pre-push`.

## 2026-05-18 19:59 BST — poke

`fullstack-18` is committed and pushed on `main`.

Commit:

* `2d1c719` Simplify bubble survey UI (fullstack-18)

Gate run: `npm run test -- BubbleOverlay watcherEvents TerminalRichPrompt`,
`npm run check`, `npm run build`, `scripts/pre-push`.

## 2026-05-18 20:14 BST — poke

`fullstack-19` SPA switch is implemented locally but blocked on the backend
lane: `systacean-11` endpoint is not on `main` yet, and `scripts/pre-push`
currently fails in dirty `crates/chan-server/src/terminal_sessions.rs`
with `E0597: session does not live long enough`.

Details: [fullstack-a/fullstack-19.md](../fullstack-a/fullstack-19.md).

## 2026-05-18 20:33 BST — poke

`fullstack-19` is committed and pushed on `main`.

Commit:

* `7bc2897` Route survey replies through terminal endpoint (fullstack-19)

Gate run: `npm run test -- BubbleOverlay watcherEvents`, `npm run check`,
`npm run build`, `scripts/pre-push`.

## 2026-05-18 20:37 BST — poke

online, starting `fullstack-14` after `fullstack-19` landed.

## 2026-05-18 20:45 BST — poke

`fullstack-14` is committed and pushed on `main`.

Commit:

* `a2fb205` Migrate graph and file browser into tabs (fullstack-14)

Gate run: `npm run test -- store tabs`, `npm run check`,
`npm run build`, `scripts/pre-push`.

## 2026-05-18 20:54 BST — poke

online, starting `fullstack-15` after `fullstack-14` landed.

## 2026-05-18 20:59 BST — poke

`fullstack-15` is committed and pushed on `main`.

Commit:

* `e4f9d28` Add pane body tab detach substrate (fullstack-15)

Gate run: `npm run test -- tabs`, `npm run check`,
`npm run build`, `scripts/pre-push`.

## 2026-05-19 05:18 BST — poke

`fullstack-21` is committed and pushed on `main`.

Commit:

* `07a79d5` Swap pane menus back (fullstack-21)

Gate run: `npm run check`, `npm run build`,
`bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Note: plain `scripts/pre-push` first hit the local macOS fd limit
(`Too many open files` in `chan-drive` tests); same gate passed with
fd limit 4096 outside the sandbox.

## 2026-05-19 04:31 BST — poke

online, starting `fullstack-17` after `fullstack-16` landed.

## 2026-05-19 04:37 BST — poke

`fullstack-17` is committed and pushed on `main`.

Commit:

* `0c2faa7` Polish watcher and terminal UX (fullstack-17)

Gate run: `npm run test -- BubbleOverlay TerminalRichPrompt watcherEvents pathValidate`,
`npm run check`, `npm run build`, `scripts/pre-push`.

## 2026-05-19 04:49 BST — poke

online, starting `fullstack-20` after `fullstack-17` landed.

## 2026-05-19 04:56 BST — poke

`fullstack-20` frontend is implemented locally and frontend verification
is green, but landing is gated on `systacean-12` reaching `main` so the
visible Spawn agent affordance does not call a missing `/api/terminals`
endpoint. Latest note:
[../fullstack-a/fullstack-20.md](../fullstack-a/fullstack-20.md#2026-05-19-0456-bst--backend-gate).

## 2026-05-19 05:07 BST — poke

`fullstack-20` is committed and pushed on `main`.

Commit:

* `f2094c3` Add spawn-from-rich-prompt UI (fullstack-20)

Gate run: `npm run test -- BubbleOverlay TerminalRichPrompt watcherEvents tabs`,
`npm run check`, `npm run build`, `scripts/pre-push`.

## 2026-05-19 05:07 BST — poke

online, starting `fullstack-21` after `fullstack-20` landed.

## 2026-05-18 21:01 BST — poke

online, starting `fullstack-16` after `fullstack-15` landed.

## 2026-05-18 21:06 BST — poke

`fullstack-16` is committed and pushed on `main`.

Commit:

* `44d9749` Add transactional pane mode (fullstack-16)

Gate run: `npm run test -- tabs`, `npm run check`,
`npm run build`, `scripts/pre-push`.

## 2026-05-19 05:18 BST — poke

online, starting `fullstack-22` after `fullstack-21` landed.

## 2026-05-19 05:23 BST — poke

`fullstack-22` is committed and pushed on `main`.

Commit:

* `f4ab310` Make BCAST window-wide (fullstack-22)

Gate run: `npm run test -- tabs`, `npm run check`,
`npm run build`, `bash -lc 'ulimit -n 4096; scripts/pre-push'`.

## 2026-05-19 05:32 BST — poke

online, starting `fullstack-23` after `fullstack-22` landed.

## 2026-05-19 05:39 BST — poke

`fullstack-23` is committed and pushed on `main`.

Commit:

* `e60287c` Add survey follow-up state (fullstack-23)

Gate run: `npm run test -- BubbleOverlay watcherEvents`,
`npm run check`, `npm run build`, and
`bash -lc 'ulimit -n 4096; scripts/pre-push'` in a clean temporary
worktree with only the `fullstack-23` patch applied.

## 2026-05-19 06:11 BST — poke

online, starting `fullstack-25` after architect go-ahead.

## 2026-05-19 06:13 BST — poke

online, starting `fullstack-24` after `fullstack-25` landed on main.

## 2026-05-19 06:15 BST — poke

`fullstack-24` is committed and pushed on `main`.

Commit:

* `a8b52a0` Promote survey follow-up to button (fullstack-24)

Gate run: `npm run test -- BubbleOverlay watcherEvents`,
`npm run check`, `npm run build`, and
`bash -lc 'ulimit -n 4096; scripts/pre-push'`.

## 2026-05-19 06:26 BST — poke

online, starting `fullstack-26` after `fullstack-24` handoff.

## 2026-05-19 06:28 BST — poke

`fullstack-26` is committed and pushed on `main`.

Commit:

* `5806343` Drop terminal broadcast mute (fullstack-26)

Gate run: `npm run test -- tabs`, `npm run check`,
`npm run build`, and `bash -lc 'ulimit -n 4096; scripts/pre-push'`.

BCAST is now binary in/out only; pink tab-strip indicator is the only
visible state. Ready for @@WebtestB formal walkthrough.

## 2026-05-19 06:37 BST — poke

online, starting `fullstack-27` after `fullstack-26` handoff.

## 2026-05-19 06:39 BST — poke

`fullstack-27` is committed and pushed on `main`.

Commit:

* `ebb347b` Read pre-flight watcher files (fullstack-27)

Gate run: `npm run test -- watcherEvents BubbleOverlay`,
`npm run check`, `npm run build`, and
`bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Ready for @@WebtestA item-4 re-test: the SPA watcher reader now loads
`pre-flight-*.md/json` event files emitted by chan-server.

## 2026-05-19 07:40 BST — poke

online, starting `fullstack-28` after architect poke.

## 2026-05-19 07:44 BST — poke

`fullstack-28` is committed and pushed on `main`.

Commit:

* `06739a9` Restore empty pane context menu (fullstack-28)

Gate run: `npm run test -- Pane`, `npm run check`,
`npm run build`, and `bash -lc 'ulimit -n 4096; scripts/pre-push'`.

## 2026-05-19 08:09 BST — poke

online, starting `fullstack-29` audit after reframed architect cut.

## 2026-05-19 08:19 BST — poke

`fullstack-29` is committed and pushed on `main`.

Commit:

* `e995575` Route file reveals to browser tabs (fullstack-29)

Gate run: `npm run test -- store revealBrowserActions`,
`npm run check`, `npm run build`, `npm run test`, and
`bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Summary: terminal/doc/search/graph reveal actions now route through
first-class File Browser tabs, existing browser tabs are focused instead
of duplicated, legacy File Browser OverlayShell component was removed,
and the required audit summary is appended to the task file.

## 2026-05-19 08:22 BST — poke

online, starting `fullstack-30` focus color + pane hamburger reorder.

## 2026-05-19 08:27 BST — poke

`fullstack-30` is committed and pushed on `main`.

Commit:

* `95aaef5` Make pane focus color window-wide (fullstack-30)

Gate run: `npm run test -- tabs Pane`, `npm run check`,
`npm run build`, `npm run test`, and
`bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Summary: focus border color is now stored once on the window layout,
serialized as `wc`, legacy per-pane `pc` colors are ignored on restore,
and the pane hamburger order is now color → next/previous → split
right/down → close.

## 2026-05-19 09:55 BST — poke

online, starting `fullstack-31` (recycled @@FullStackA session). Queue
in numerical order: `fullstack-31`, `-32`, `-33`, `-36`, `-37`, `-38`.

## 2026-05-19 10:08 BST — poke

`fullstack-31` is committed and pushed on `main`.

Commit:

* `e4b40ba` Drop inline X close on Graph + File Browser surfaces (fullstack-31)

Gate run: `npm run test -- revealBrowserActions`, `npm run check`,
`npm run build`, and `bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Re-audit of `fullstack-29`'s "Known concrete additions" complete:
no further inline close affordances on the Graph or File Browser
surfaces, no stray chrome leftovers from the Phase 1 migration.
