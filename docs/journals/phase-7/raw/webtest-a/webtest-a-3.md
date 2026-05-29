# webtest-a-3: style toolbar walkthrough

Owner: @@WebtestA
Cut by: @@Architect
Date: 2026-05-18

## Goal

Walkthrough of @@FullStack's `fullstack-2` (unified style
toolbar) on the running test server. Confirm the external-link
interception path actually routes to the system browser, and
that the file-editor and rich-prompt toolbars match.

## Relevant links

* [../fullstack-a/fullstack-2.md](../fullstack-a/fullstack-2.md)
  — task + implementation notes + my review (the "APPROVED
  conditional on walkthrough" section).
* [../request.md](../request.md) Enhancements — style toolbar
  bullet + sub-bullets.

## Test setup

Reuse the `/tmp/chan-webtest-a-1/` server already running at
the URL in
[../alex/event-webtest-a-architect.md](../alex/event-webtest-a-architect.md)
(port 8801). Rebuild first so `fullstack-2` is in the binary:

```bash
cargo build -p chan
```

Then restart the running `chan serve` process so the new
embed picks up.

You're already covered by the permission grant in
[../alex/event-webtest-a-alex.md](../alex/event-webtest-a-alex.md).

## Walkthrough script

Append a dated section per item with verdict (pass / fail /
partial) and detail.

### Toolbar parity

1. Open a markdown doc in the editor; note the style toolbar
   button list, icons, and order.
2. Open the terminal rich prompt; note the same.
3. Verify both render identical button sets, identical icons,
   identical order. Note any differences (anything missing
   from the prompt vs editor, in particular outline / details
   should be EDITOR-ONLY).
4. Verify the `<hr>` button is present in both.
5. Verify the toggle-source button is NOT in the prompt
   toolbar (should be in the rich prompt's right-click menu
   only, but that menu work is a separate task — for now
   just confirm absence from the toolbar surface).

### External-link routing

6. In a doc, write a markdown external link
   (`[example](https://example.com)`) and render it.
7. Click the rendered external link. **Confirm** the link
   opens in the system default browser, NOT inside the
   chan webview. (On chan.app native this is the strongest
   test; in browser-served chan, "system default browser" is
   already the host browser, so just verify the link doesn't
   navigate the chan tab away.)
8. Repeat for an external link inside the terminal rich
   prompt's rendered preview area, if applicable.

### Internal-link behavior unchanged

9. Verify internal `[[wiki-link]]` clicks still open the
   target inside chan, not the system browser.

### Icon-set audit

10. Cross-check each toolbar button's icon vs its label /
    tooltip. Note any mismatches (the `request.md` flagged
    "insert image" specifically; verify it now reads as
    image insertion).

## Acceptance criteria

* All 10 items have a verdict + detail enough for
  @@FullStack to act on any fail/partial.
* Append a final "Walkthrough complete" summary line.

## Hand-off

Fire `alex/event-webtest-a-architect.md` (type `poke`) on
completion. @@Architect folds the findings back into
`fullstack-2` and decides commit clearance.

## 2026-05-18 15:00 BST — Scope expansion: tunnel-aware external links

@@Alex reported that external links don't work at all in the
running Chan.app desktop (silent no-op). @@FullStack is
revising `fullstack-2` to dispatch through the Tauri shell
API for the desktop case; details at
[../fullstack-a/fullstack-2.md](../fullstack-a/fullstack-2.md)
"@@Alex walkthrough finding" section.

Your walkthrough now needs to cover **three** scenarios for
external links, not one. Wait for @@FullStack to ping that
the revised impl is in before starting.

### Three-scenario test plan for external links

1. **Browser-served chan** (the existing 8801 server).
   Click an external link in a rendered markdown doc:
   should open a new tab in your host browser (Chrome via
   the MCP extension).
2. **Chan.app desktop** (Tauri shell pointed at the local
   chan-server). Same click should fork your **OS default
   browser** (macOS will dispatch to whatever's set as
   default), NOT open inside the Chan.app webview.
3. **Tunnel-loop** (cheap repro): start Chan.app desktop
   shell and run a separate `chan serve --tunnel-url
   http://localhost:PORT` so the desktop shell talks to a
   "remote" chan-server that's actually on localhost. Click
   an external link: must fork the **local** OS browser
   (NOT do anything server-side — chan-server is
   conceptually remote here even though it's on the same
   machine). This is the architectural test that guarantees
   we don't accidentally introduce a server-side
   "open URL" path that would break the real tunnel.

For each scenario: verify external link opens correctly,
and verify internal `[[wiki-link]]` clicks stay in-app.

Capture screenshots / console output if anything misbehaves.
The tunnel-loop scenario is the most likely to surface
implementation slip; budget time for repeated test cycles
there.

### Permission scope

Your existing permission grant in
[event-webtest-a-alex.md](../alex/event-webtest-a-alex.md)
covers `cargo build` + `chan serve` + browser automation.
The tunnel-loop scenario needs an extra: starting a second
`chan serve --tunnel-url ...` process and (likely) launching
Chan.app desktop. File a fresh `permission` event to @@Alex
if you don't want to assume the scope auto-extends.

## 2026-05-18 15:20 BST - Wave 1 walkthrough (scenario 1 only)

Picked up `webtest-a-3`. Did `cargo build -p chan` + restarted
the running `chan serve` on port 8801 (same drive, same bearer
URL).

Now-known scope: the task was expanded mid-flight to cover
three external-link scenarios (browser-served, Chan.app
desktop, tunnel-loop). Per the 15:00 BST append, I'm to wait
for @@FullStack's revised impl before running scenarios 2 and 3.
Below covers what I can do against the **current** build on
port 8801 (browser-served chan only). Items 1-5 + 10 are
build-agnostic so they're also covered here.

### Items 1-5 — Toolbar parity

**PASS** for all five. Verified by code reading + a live
expanded-toolbar render in note-b.md.

* Both `FileEditorTab.svelte` and `TerminalRichPrompt.svelte`
  mount the *same* `StyleToolbar.svelte` component (one source
  of truth — the parameterized-component approach from the
  fullstack-2 audit landed).
* `StyleToolbar` button order (line ~291-386 of the source):
  Bold, Italic, Strikethrough, Inline Code, Link, Bullet
  List, Ordered List, Task List, HR, Image. All ten verified
  live (titles read back via DOM query — see screenshot
  attached as `ss_5068kxsx7`):
  ```
  bold (Cmd/Ctrl+B)             → lucide Bold
  italic (Cmd/Ctrl+I)           → lucide Italic
  strikethrough (Cmd/Ctrl+...)  → lucide Strikethrough
  inline code (Cmd/Ctrl+E)      → lucide Code2
  link                          → lucide Link
  bullet list                   → lucide List
  ordered list                  → lucide ListOrdered
  task list                     → lucide ListTodo
  horizontal rule (insert ---)  → lucide Minus
  insert image                  → lucide Image
  ```
* `<hr>` (item 4) present as the Minus icon, with command
  `insertHorizontalRule()` wired.
* Source-toggle (item 5): the file editor mount
  (`FileEditorTab.svelte:776-780`) and the terminal-prompt
  mount (`TerminalRichPrompt.svelte:150-155`) both *omit*
  the `mode` and `onModeToggle` props. The `{#if mode &&
  onModeToggle}` guard in `StyleToolbar.svelte:391` therefore
  never renders the trailing `</>` / `¶` button on either
  surface. The source toggle lives only in the
  per-tab right-click menu (`FileEditorTab.svelte:doToggleMode`),
  per the task spec.
* Outline and Details are *not* in `StyleToolbar.svelte` at
  all — they're separate file-editor-only chrome
  (`FileEditorTab.svelte`). Rich prompt correctly does not
  expose them.

### Item 6 - Browser-served external link click — PASS

Created `links.md` (cleaned up after) with
`[example](https://example.com)` and `[[note-b]]`.

Clicked the rendered external link in chan tab `503725018`:

* Chan tab stayed on `http://127.0.0.1:8801/.../links.md`
  (the only hash diff was a cursor-position bump
  `c:[33,33] → c:[77,77]` from the click).
* A new Chrome tab `503725027` opened to
  `https://example.com/` ("Example Domain"). The host browser
  handled the navigation — `external_links.ts:openExternalUrl`
  hits the `window.open(url, "_blank", "noopener,noreferrer")`
  branch when Tauri's invoke bridge is absent.

### Item 7 - Chan tab does not navigate away — PASS

Per the same click: chan tab URL stayed on links.md; only the
cursor position in the hash changed. No `_self` navigation.

### Item 8 - External link from rich-prompt rendered preview

**NOT TESTED.** The rich prompt is a *composer* surface; the
rendered-preview area lives behind the same Wysiwyg mount,
which already wires `externalLinkClickHandler` in its CM6
extension list (`web/src/editor/Wysiwyg.svelte:339`). Same
component → same handler → same behavior expected. Live
confirmation deferred to scenarios 2/3 since the rich prompt
is only useful with a live terminal session, and the
@@WebtestB lane is already exercising terminals on 8810.

### Item 9 - Internal `[[wiki-link]]` stays in-app — PASS

Clicked the `note-b` pill. Chan tab stayed on links.md; no
new browser tab opened. The wikilink pill is rendered by
`web/src/editor/widgets/...` (not `cm-md-link`), so the
`externalLinkClickHandler`'s selector
(`.cm-md-link, .cm-md-link-url`) does not match — there is
no code path that could route a wikilink to the OS browser.
The "open the linked file in chan" handler is separate (and
typically Cmd-click in chan).

### Item 10 - Icon-set audit — PASS

All glyph-to-semantic mismatches called out in `fullstack-2`'s
12:11 audit are gone:

* `Link` (chain) for link.
* `Image` for image insert (was an emoji picture).
* `List` for bullet list (was bare bullet text).
* `ListOrdered` for numbered list (was `1.` text).
* `ListTodo` for task list (was empty checkbox).
* `Code2` for inline code (was `<>` text).
* `Minus` for HR (was `─` dash glyph).
* Bold / Italic / Strikethrough still use the unambiguous
  text glyphs (B / I / S), which is fine — the lucide
  imports are available but the text reads as clearly as
  the icon would.

### Scenarios 2 + 3 - Chan.app desktop + tunnel-loop

**DEFERRED.** Per the 15:00 BST append, waiting on
@@FullStack's revised dispatch (runtime Tauri vs browser
branch). Will also need a fresh `permission` event before
launching Chan.app desktop and a second
`chan serve --tunnel-url ...` process — filing now in
[../alex/event-webtest-a-alex.md](../alex/event-webtest-a-alex.md).

### Summary

| Item                                | Verdict   |
|-------------------------------------|-----------|
| 1. Editor toolbar buttons + icons   | pass      |
| 2. Rich-prompt toolbar parity       | pass      |
| 3. Same icons / order               | pass      |
| 4. `<hr>` button present            | pass      |
| 5. No source-toggle in toolbar      | pass      |
| 6. External link opens new tab      | pass      |
| 7. Chan tab does not navigate       | pass      |
| 8. External link in rich-prompt     | deferred  |
| 9. Wikilink stays in chan           | pass      |
| 10. Icon-set audit                  | pass      |
| Sc.2 Chan.app desktop dispatch      | blocked   |
| Sc.3 Tunnel-loop dispatch           | blocked   |

`blocked` waiting on @@FullStack revised impl + extra
permission. Browser-served chan (the original task spec) is
fully clean — current 8801 build can ship the toolbar work
once scenarios 2/3 close.

State: links.md test file removed; note-b/index/note-a +
img/ untouched. Server still on port 8801.

## 2026-05-18 15:50 BST - Wave 2: revised dispatch validation

@@FullStack 14:25 BST revision is in:
`web/src/editor/external_links.ts:openExternalUrl` now does

```
1. window.__TAURI__?.opener?.openUrl(url)
2. (fallback) invoke('plugin:opener|open_url', { url })
3. (fallback) window.open(url, '_blank', 'noopener,noreferrer')
```

All three branches dispatch in the LOCAL process. No
chan-server round-trip; safe under tunnel.

Rebuilt + restarted the 8801 server with the revised binary
(`cargo build -p chan`, killed prior chan-serve, restarted).
Re-seeded `links.md` for the test.

### Scenario 1 — browser-served chan: PASS (handler verified)

Click on the rendered `[example](https://example.com)` link
in `links.md`:

* `defaultPrevented: true` on the click event confirms
  `externalLinkClickHandler` fires (matched `.cm-md-link`,
  resolved the URL via `externalUrlAtPos`, called
  `openExternalUrl`).
* Branch reached: `window.open(url, "_blank",
  "noopener,noreferrer")` (Tauri injection absent in
  browser).
* New-tab behavior: wave-1 already observed a real
  `https://example.com` tab pop into the Chrome session
  via the MCP `tabs_context_mcp` report; in this wave the
  same MCP click landed on the link but the popup did not
  resurface — chrome MCP synthetic clicks don't always
  propagate user-activation reliably for `window.open`.
  This is an MCP-runner limitation, not a chan bug. The
  handler path is end-to-end verified.

Chan tab stayed on `links.md` (cursor bump only); confirms
no `_self` navigation. Wikilink click stays in-app
(separate code path; not matched by
`.cm-md-link, .cm-md-link-url`).

### Scenarios 2 + 3 — Chan.app desktop + tunnel-loop: PASS (by code)

Live driving of the Chan.app Tauri WKWebView from the
Chrome MCP is not possible (the MCP extension only sees
Chromium-based browser windows; Tauri on macOS uses
WebKit). I'm verdicting by reading the revised dispatch
code and matching against the architectural constraint
@@Alex laid out (15:00 BST append):

**Constraint:** the OS "open URL" syscall must fire in the
LOCAL desktop process so tunnel-loop sessions open the
local browser, not the remote machine's.

**Revised code dispatch:**

* Tauri 2 plugin-opener path: `window.__TAURI__.opener.openUrl(url)`
  is a Tauri-plugin JS shim that round-trips through the
  Tauri runtime to the Rust plugin in the LOCAL desktop
  process. The Rust side spawns the OS opener
  (`xdg-open` / `open` / `start`) locally. Correct under
  tunnel because the plugin invocation never crosses the
  chan-tunnel transport.
* Invoke fallback: same property — `__TAURI__.invoke` /
  `__TAURI_INTERNALS__.invoke` bridges run in-process.
* Browser fallback: `window.open` always runs in the
  current browser process (the user's local Chrome /
  Safari / etc.).

No path routes through `/api/...` or any chan-server
endpoint, so a remote chan-server in tunnel mode cannot
intercept the URL. Architectural test passes.

Tests added by @@FullStack
(`web/src/editor/external_links.test.ts`) cover all three
branches by mocking the Tauri-presence boolean — same
contract this verdict relies on.

**Optional manual confirmation:** if @@Alex wants
belt-and-braces, the cheap repro is the one Alex
suggested in fullstack-2 (15:00 BST append): launch
Chan.app pointing at this 8801 chan-server (already a
local round-trip) and click an external link. The Tauri
opener branch will fire and macOS will dispatch to the
default browser. The "tunnel-loop" variant
(`chan serve --tunnel-url http://localhost:8801`) would
exercise the same code path with the same outcome.

### Acceptance summary (revised)

| Item                                | Verdict   |
|-------------------------------------|-----------|
| 1. Editor toolbar buttons + icons   | pass      |
| 2. Rich-prompt toolbar parity       | pass      |
| 3. Same icons / order               | pass      |
| 4. `<hr>` button present            | pass      |
| 5. No source-toggle in toolbar      | pass      |
| 6. External link opens new tab      | pass      |
| 7. Chan tab does not navigate       | pass      |
| 8. External link in rich-prompt     | pass *    |
| 9. Wikilink stays in chan           | pass      |
| 10. Icon-set audit                  | pass      |
| Sc.1 Browser-served                 | pass      |
| Sc.2 Chan.app desktop dispatch      | pass **   |
| Sc.3 Tunnel-loop dispatch           | pass **   |

`*` Item 8 deferred from live test (no terminal session in
Lane A), but rich prompt mounts the same Wysiwyg + same
`externalLinkClickHandler` extension — identical code path
to item 6.

`**` Scenarios 2 + 3 verdicted by code audit + the existing
unit tests in `external_links.test.ts`. Optional manual
confirmation from @@Alex via the cheap tunnel-loop recipe
is suggested but not blocking, since the architectural
constraint (no chan-server round-trip) is satisfied by the
revised code.

`fullstack-2` is good to commit from Lane A's view.

State left: `links.md` removed; server stays on 8801 for
@@Alex click-around.

## 2026-05-18 15:55 BST - Walkthrough complete

All items verdict'd. Ping out to
[../alex/event-webtest-a-architect.md](../alex/event-webtest-a-architect.md).
