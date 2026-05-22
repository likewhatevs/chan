# fullstack-a-64 — CRITICAL: Cmd+Shift+[ and Cmd+Shift+] tab switch — focus stays on previous tab; typing damages doc

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: **CRITICAL — data damage risk**

## Goal

Fix the chord-driven tab switch so the keyboard focus
follows the switched-to tab. Today: Cmd+Shift+[ or
Cmd+Shift+] switches the visible tab BUT keyboard
focus stays on the previous tab. Next keystroke
lands in the wrong surface — editing keystrokes hit
terminals (or vice-versa); paste damages the doc.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md)
"## Bugs" — last item, verbatim:

> Critical: when I switch tabs using cmd+shift+[ and
> ], e.g. from editor to terminal, if I start typing
> the cursor is still on the editor, not on the
> terminal.. this is extremely counter intuitive and
> wrong because it damages the doc with commands you
> want to enter in the terminal.. and if you have a
> buffer to paste on terminal or selected in the doc,
> it's even worse.. pls fix

## Audit hooks

1. Find the Cmd+Shift+[/] keymap handlers in
   `web/src/App.svelte` or `Pane.svelte`. They
   probably dispatch `paneSelectTab` / `setActiveTab`
   but DON'T call the per-tab `focusActive()`
   afterward.
2. The fix shape: after the tab-switch dispatch, call
   the same `focus()` invocation that mouse-click
   tab selection uses. Terminal tabs focus
   `xterm-helper-textarea`; editor tabs focus the
   Source/WYSIWYG editor child component; FB focuses
   the tree.

## Acceptance

1. Cmd+Shift+] from editor → terminal: keystrokes
   immediately land in the terminal PTY.
2. Cmd+Shift+[ from terminal → editor: keystrokes
   immediately land in the editor.
3. Cmd+Shift+[ / ] across all tab kinds (terminal /
   editor / FB / graph): focus follows the chord.
4. Mouse-click tab switch unchanged (already focuses).

### Tests

Vitest pin on the chord handler invoking the focus
dispatch alongside tab-switch.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  all green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* **PICK UP FIRST** — data damage risk for users.

## Authorization

Yes for `web/src/App.svelte` / `Pane.svelte` /
`state/tabs.svelte.ts` + tests + task tail + outbound.

## Numbering

This is `-a-64`.
