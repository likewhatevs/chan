# Items 1+4 — editor tab keep-alive + tab-click focus

Lane: @@Editor. Order is mandatory: **item 4 first, then item 1**
(keep-alive removes the remount-rAF focus that currently masks the
click-focus bug for file tabs; landing 1 without 4 regresses file-tab
clicks into the same steal terminals have today).

Line numbers are from main @ 3ebee587; verify before editing.

## Symptoms

- Item 1: switch away from an editor tab and back (same pane) → the
  whole document shows raw un-decorated markdown until a click; scroll
  resets to the top. Reproduced by @@Alex on the desktop app
  (WKWebView), screenshots on phase-23.md.
- Item 4: clicking a terminal tab activates it but keyboard focus does
  not land in xterm; Cmd+Shift+[/] (pane prev/next) focuses correctly.

## Root causes

- **Item 1:** `FileEditorTab.svelte` wraps its editor body in
  `{#key tab.id}` (~1202-1429) and Pane.svelte renders only the active
  file tab (`{:else if active?.kind === "file"}` ~1387-1396) → every
  switch destroys and recreates the EditorView (Wysiwyg.svelte onMount
  ~377-583 / onDestroy ~616-619). The decoration walker
  (`web/src/editor/decorations/walker.ts` ~76-113) computes from the
  visible viewport; on remount the pre-layout viewport covers only the
  top. A `geometryChanged` recompute trigger was already added (comment
  ~88-99, pinned by walker.test.ts) and does NOT hold on WKWebView —
  patching this symptom again is the wrong move. scrollTop is persisted
  nowhere (FileTab type, state/tabs.svelte.ts ~142-251, has only
  `caret`); caret restore uses scrollIntoView(nearest) which does not
  restore scroll context.
- **Item 4:** the `.tab` div has `tabindex="0"`. The tab-strip
  mousedown handler (Pane.svelte ~1100-1110) bumps `tabFocusPulse`,
  and TerminalTab's focus effect (~263-288) calls `term?.focus()` in a
  queueMicrotask — but the browser's mousedown DEFAULT ACTION focuses
  the tab element AFTER that microtask (microtask checkpoints run
  between listeners, before the default action). The keyboard path has
  no competing default action. File tabs are immune today only because
  the remount path ends in a rAF focus (Wysiwyg ~577-582, 608-613),
  which runs after the event task.
- Vestigial: `tabMouseDownPrevActive` (Pane.svelte 581, 1107, 1113,
  1182) is written but never read; ignore or remove.

## Decision: keep-alive (not another save/restore patch)

Render file tabs the way terminals already are: kept mounted, hidden
with `visibility:hidden` (terminal precedent: Pane.svelte ~1444-1460,
TerminalTab.svelte ~1866-1881). Rationale:

1. A scroll/decoration patch would be the THIRD patch on this symptom
   on a surface (WKWebView) the team can't iterate on quickly.
   Keep-alive removes the race category: no remount → DecorationSets,
   scroll, caret, undo history, FindBar state all persist.
2. The costs are already paid: session restore eagerly loads every
   file tab's content (tabs.svelte.ts ~4333); autosave (App.svelte
   ~138-199) and file-watch reload (flagExternalChange /
   refreshTabFromDisk) are store-level, mount-independent. The only
   new cost is a live CM6 EditorView DOM per open file tab — fine for
   a solo-user notes app.
3. `visibility:hidden` (never `display:none`) keeps real layout
   geometry, so CM6 measures correctly while hidden — no pre-layout
   viewport exists to mis-decorate.

## Phase 0 — reproduce (30 min)

- `make desktop-dev` (WKWebView) + `cd web && npm run dev` (Chrome).
- Item 4 probe: click a terminal tab, run `document.activeElement` in
  the inspector. Expected: the `.tab` div holds focus. If focus sits
  elsewhere, re-adjudicate before coding (the mouseup fix below is
  correct for any stolen-during-mousedown variant).
- Item 1 probe: long doc, scroll mid, switch away/back on WKWebView.

## Phase 1 — item 4: pulse on mouseup (Pane.svelte ~1096-1137)

- Keep the existing `onmousedown` handler EXACTLY as-is (pinned by
  tabSwitchFocusFollow.test.ts:53-57; its early blur is desirable).
- Add on the `.tab` div:
  `onmouseup={(e) => { if (e.button !== 0) return; if (t.kind === "terminal" || t.kind === "file") bumpTabFocusPulse(); }}`

Why mouseup and not:
- `preventDefault` on mousedown — suppresses HTML5 `dragstart` on the
  `draggable="true"` tab in WebKit/Firefox → breaks tab reorder/DnD.
- `onclick` — the `.path` span's onclick calls stopPropagation()
  (~1180-1183), so label clicks never reach the tab's onclick.

Mouseup runs after the default action placed focus on the tab, so the
second pulse's microtask focus is the last word; it also covers any
pane-activation ordering for free (pane-root mousedown ~1053-1056 has
already run setActivePane by mouseup). A completed drag fires dragend,
not mouseup → no spurious bump after reorders. Mouseup on the close
button bubbling to the tab mirrors today's mousedown behavior. The
rich-prompt guard in TerminalTab's focus effect (~281-287) still
prevents stealing the bubble's caret. Double pulse (down+up) is
idempotent.

## Phase 2 — item 1: keep-alive

### 2a. Pane.svelte
- Delete the `{:else if active?.kind === "file"}` branch (~1387-1396).
- Add adjacent to the terminal `{#each}` (~1453-1460), inside
  `.face.front`:

```svelte
{#each pane.tabs.filter((t) => t.kind === "file") as t (t.id)}
  <FileEditorTab
    tab={t}
    active={!paneMode.active && !pane.showingBack && t.id === pane.activeTabId}
    focused={!paneMode.active && !pane.showingBack && t.id === pane.activeTabId && viewLayout.activePaneId === pane.id}
  />
{/each}
```

- Add a rationale comment mirroring the terminal one (~1444-1451),
  citing the WKWebView raw-flash bug. Do NOT reorder or wrap the
  terminal block (paneTerminalMount.test.ts pins it). The
  `{:else if !active}` placeholder and graph/browser/dashboard
  branches stay unchanged.

### 2b. FileEditorTab.svelte
1. Props: `let { tab, active = false, focused = false } = $props()`
   (extend ~147-148).
2. Remove `{#key tab.id}` (~1202) + `{/key}` (~1429) — one instance
   per tab id from the keyed each-block makes it dead weight.
3. Root `.editor-tab`: `class:active`, `aria-hidden={!active}`,
   `role="tabpanel"`. CSS copying the terminal contract:
   `.editor-tab { position: absolute; inset: 0; visibility: hidden; pointer-events: none; }`
   `.editor-tab.active { visibility: visible; pointer-events: auto; }`
   (keep existing flex column props; visibility, never display:none).
4. **Gate mount autofocus** — the one real background hazard:
   Wysiwyg.svelte:85 and Source.svelte:57 default `autoFocus = true`;
   at session restore N background editors would each focus at
   mount+rAF (caret lands in a random hidden tab). Pass
   `autoFocus={focused}` to `<Wysiwyg>` (~1270) and `<Source>`
   (~1361). Newly opened tabs are active+focused at mount → new-draft
   focus flow preserved.
5. The pulse/focus effect (~168-176) needs no change.
6. Cheap insurance, mirroring TerminalTab's active-flip recovery
   (~340-344): `$effect(() => { if (active) { wysiwygRef?.remeasure?.(); sourceRef?.remeasure?.(); } })`
   where `remeasure()` is a tiny new export on Wysiwyg/Source wrapping
   `view.requestMeasure()`. Covers tab-becomes-active without focus
   (flip-back, pane-mode exit).
7. Behavior shift to FLAG not fix: the onDestroy (~670-674) clearing
   the "Choose the moved file…" status now fires on tab close, not
   switch (arguably better). Verify toastAutoDismissSweep.test.ts.

Verified safe ungated in background tabs: editorBuffer
recovery/persistence effects (~188-236, path-keyed, idempotent);
svelte:window keydown/pointerdown (~746, gated on per-tab menuOpen);
Outline/Inspector/FindBar/StyleToolbar mount only via per-tab flags
(their persistence across switches is a feature).

### 2c. walker.ts — unchanged
Keep `geometryChanged` and its test: still correct, still covers
mode-toggle remounts (Wysiwyg↔Source within a tab).

## Tests (vitest, repo's ?raw source-pin style)

1. New `web/src/components/paneFileTabKeepAlive.test.ts` (model:
   paneTerminalMount.test.ts, TerminalTab.scrollback.test.ts): file
   each-block exists and is NOT under the active-tab if-chain;
   active/focused props pinned with all four gates; FileEditorTab has
   class:active + visibility:hidden pair; `not.toMatch(/\{#key tab\.id\}/)`;
   `autoFocus={focused}` threaded to both editors.
2. `paneFocusFollowFlip.test.ts:40-48` pins the OLD markup — re-pin to
   the each-block form (it will fail otherwise; expected).
3. Item-4 pin in `tabSwitchFocusFollow.test.ts`: mouseup handler regex
   + a comment documenting the default-action steal.
4. Optional jsdom mount test (Pane.test.ts style): two file tabs, flip
   activeTabId, both hosts stay in DOM, one `.active`. If CM6-in-jsdom
   needs excessive shims, fall back to source pins (repo convention:
   browser is the source of truth, tests pin wiring).
5. Full: `cd web && npm test && npm run check`. Expect
   FileEditorTab.recovery / editorBuffer / perTabInspectorWidth /
   editorRightClickRevamp / tabMenuReloadInspector to pass; scan
   failures for unlisted markup pins.

## Verification

Chrome (`npm run dev`): two long docs in one pane — switch away/back →
instant decoration, scroll/caret/undo/FindBar preserved; tab DnD
reorder + cross-pane drag (mouseup-fix risk surface); OS-file drop
into active editor works, non-zones blocked (hidden editors are
pointer-events:none → can never become drop targets); terminal click →
type immediately; rich-prompt-open terminal click → caret stays in
bubble; flip + Hybrid Nav with mixed tabs; empty-pane welcome; session
restore ~5 tabs/2 panes → caret lands once, in the active pane's
active tab.

WKWebView (the real gate, via @@Desktop's build): the item-1 repro
(scroll preserved, no raw flash, also after Hybrid-Nav exit and
flip-back); the item-4 repro (document.activeElement = xterm
textarea); ~20 file tabs memory sanity in Activity Monitor.

## Regression risks

- paneFocusFollowFlip pin (known, re-pinned).
- Session-restore focus fights — highest-risk new behavior; the
  autoFocus={focused} gate is the mitigation; test explicitly.
- fileDropGuard (web/src/state/fileDropGuard.ts): more .cm-editor
  nodes in DOM, but hidden ones are pointer-events:none and the guard
  is target-based (closest) — semantics unchanged; smoke covers.
- Flip-card/paneMode: new absolute hosts sit in .face.front whose
  visibility flip (~1594-1629, incl. the WebKitGTK workaround) hides
  children; per-tab active gate covers pane mode. Verify flip on
  desktop.
- Follow-ups (out of scope, note at round close): FileTab.scrollTop
  session field for scroll-across-RELOAD (FileBrowser-style,
  store.svelte.ts ~2969-2983); optional LRU eviction if tab counts
  ever hurt memory.
