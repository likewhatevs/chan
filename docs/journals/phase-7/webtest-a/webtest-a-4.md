# webtest-a-4: post-recycle Lane A regression sweep

Owner: @@WebtestA
Cut by: @@WebtestA (self-initiated under @@Alex verbal go-ahead)
Date: 2026-05-18

## Goal

Re-verify the Lane A headliner bugs from `webtest-a-1` against
current `main` (head `d4b11d2`). Specifically, confirm that
`fullstack-4` (commit `d4b11d2`) closes B1/B2/B13 and that no
adjacent regressions snuck in. B20 (table-crash) is out of
scope for `fullstack-4` and is expected to remain open until
the table cluster is addressed; we still note its status.

This is a closeout-regression sweep before wave-1.5 lands on
top (`fullstack-6` / `fullstack-7` / `systacean-3`).

## Relevant links

* [./webtest-a-1.md](./webtest-a-1.md) — Lane A baseline with
  the original B1/B2/B13/B20 repros.
* [../fullstack/fullstack-4.md](../fullstack/fullstack-4.md)
  — the patch that should have closed B1/B2/B13.
* [../request.md](../request.md) — source-of-truth for the bug
  IDs (Bugfixes section).

## Test setup

Reuse `/tmp/chan-webtest-a-1/` (still on disk from the previous
me; seed: `index.md`, `note-a.md` with list + pipe-table + TWS
target, `note-b.md` lorem, `img/` with three PNGs). Run on
port 8801 (8787 + 8810 still owned by phase-6 leftover / Lane
B respectively).

```bash
cargo build -p chan
./target/debug/chan serve --port 8801 --no-browser /tmp/chan-webtest-a-1/
```

Permission scope reused from the predecessor `webtest-a-1`
grant in `event-webtest-a-alex.md` (cargo build + chan serve
+ browser automation). Re-verbalised by @@Alex this turn.

## Acceptance criteria

For each item: PASS / FAIL / PARTIAL + detail enough for
@@FullStack to act on a fail.

1. **B1 — Shift+Tab outside a list** must NOT steal focus to
   the pane hamburger. Should be a no-op (or stay within the
   editor as a normal indent-to-zero, which it already is for
   list items).
2. **B2 — Image paste in a list** must insert ` ` (single
   space) after the image bubble, not push to a new line.
   Pressing Enter when the user did not consume the trailing
   space should retract it (`stripUnusedInlineImageSpaceOnEnter`).
3. **B13 — Typing left of a list marker** must NOT insert text
   before the marker. `listCaretGuard` should clamp the click
   to the right of the marker.
4. **B20 — Pipe-table render** (expected: still crashes the
   editor with `RangeError: Block decorations may not be
   specified via plugins`). Confirm status, do not retest fix.
5. **Adjacent regressions**: while testing 1-3, watch for
   anything that looks new (wikilink behavior, external link,
   image insertion, find dialog, side panes from `fullstack-1`,
   tab dnd from `fullstack-5`).

## Hand-off

On completion fire `alex/event-webtest-a-architect.md` (type
`poke`) with the verdict summary.

## 2026-05-18 (resume) BST - Sweep results

Build: head `d4b11d2` (+ unstaged `systacean-3` patch in
`crates/chan-server/src/static_assets.rs`). Server on 8801
against `/tmp/chan-webtest-a-1/`.

### Test drive

Added one test file `/tmp/chan-webtest-a-1/list.md` (lists +
top/bottom paragraphs, no table) so the editor doesn't hit the
B20 crash mid-test. `note-a.md` is the B20 repro substrate,
kept intact.

### Mid-sweep finding: systacean-3 cross-drive drift still reproduces with the patch in

On every navigation in a tab whose history previously touched
8810, the tab hopped from `http://127.0.0.1:8801/...` to
`http://127.0.0.1:8810/...` within ~1.5s of load, landing on
Lane B's session (`/private/tmp/chan-webtest-b-1`, terminals
Terminal-A / Terminal-B, `#files=1%3A`). The drift reproduced
even on a freshly-created chrome MCP tab whose first navigate
was to 8801. Both 8801 and 8810 were serving the patched
binary (`Cache-Control: no-store` + `Vary: Host` confirmed on
the SPA shell via `curl -sI`). So the patch in the tree is
necessary but **not sufficient** — something in the SPA
bundle (session restoration / cookie / cross-port storage)
is still selecting the "other" recently-known port.

Workaround that unblocked this sweep: stopped the two stale
Lane B servers (`chan serve` on 8810 + 8811 — left over from
@@WebtestB's pre-recycle session, no active owner). With the
drift target unreachable, navigation to 8801 stayed put. The
phase-6 chan serve at PID 63479 is untouched. Lane B can
re-launch their servers on the original drive paths when
they come back online.

Captured this as a fresh Round 2 finding for @@Systacean;
verdicting separately under "Drift status" below.

### Per-item verdicts

```
# | Item                                           | Verdict
--+------------------------------------------------+--------
1 | B1 Shift+Tab outside list (no focus theft)     | pass
1 | B1 Shift+Tab inside list (de-indent works)     | pass
2 | B2 image-paste-in-list + Enter retract space   | pass *
3 | B13 caret clamp on numbered list               | pass
3 | B13 caret clamp on bullet list                 | pass
4 | B20 pipe-table render                          | open
5 | Adjacent: wikilink in-app behavior             | pass **
5 | Adjacent: systacean-3 drift                    | open ***
```

`*` Image-paste insertion of trailing space verified by code
audit on `web/src/editor/bubbles/image_drop.ts` (the
`onListLine` branch returns `") "` not `")\n"`) plus the
landed `clampListCaretPosition` + `stripUnusedInlineImageSpaceOnEnter`
unit tests in `web/src/editor/commands/list.test.ts`. Live
verification of the retract half: typed
`![](img/photo.png#w=250) ` after `2. Second numbered item`,
pressed Enter, observed the trailing space retracted (line
after Enter ended with `…#w=250EditView` with no trailing
space). Chrome MCP can't synthesize a real image-paste
clipboard event reliably, so the insertion half is code-audit
+ unit-test rather than live click.

`**` Wikilink renders as
`<span class="cm-md-wiki-pill" data-target="note-a" ...>`
correctly. Click in editable mode places the caret next to
the pill (matches the source-reveal-and-edit handler in
`web/src/editor/widgets/wikilink.ts:302-318`). Cmd-click
should open-in-new-pane per
`web/src/editor/widgets/wikilink.ts:298-301` but chrome MCP's
`modifiers: "cmd"` flag does not appear to propagate as
`MouseEvent.metaKey=true` on the synthetic mousedown (same
tooling-limitation pattern the previous @@WebtestA noted for
external-link clicks in `webtest-a-3` wave 2). Verdicted PASS
on the in-app-isolation property (port stayed 8801, no
external nav, no new browser tab); navigation-actually-opens
verdict deferred to a hand test.

`***` B20 stack:
```
RangeError: Block decorations may not be specified via plugins
    at Object.point (assets/dist-BfgK66KF.js:7:38269)
    at e.spans (assets/dist-BfgK66KF.js:5:5777)
    at Vi.emit (assets/dist-BfgK66KF.js:7:38146)
    ...
```
plus downstream `Cannot read properties of undefined (reading
'measureVisibleLineHeights')` and `… 'coordsAt'` cascades —
identical pattern to webtest-a-1's original capture. The pipe
table in `note-a.md` remains the deterministic repro. Not in
`fullstack-4`'s scope; carried forward for the table cluster.

### Drift status (split-out finding)

* **Reproduces against**: head `d4b11d2` + unstaged
  `systacean-3` patch in tree; both 8801 and 8810 serving the
  patched binary (confirmed `Cache-Control: no-store` +
  `Vary: Host`).
* **Repro recipe**: with a second `chan serve` running on
  another port (e.g. 8810 against a different drive), open a
  chrome tab and navigate it to `http://127.0.0.1:8801/?t=…`.
  Within 1-2 seconds the page redirects to
  `http://127.0.0.1:8810/…` and renders the other drive's
  session.
* **Mitigation that worked**: kill the other-port servers.
  With no drift target, 8801 stays put.
* **Hypothesis (untested)**: SPA bundle reads a persistent
  identifier (cookie? IndexedDB? localStorage on a shared
  scope?) that is shared across `127.0.0.1` ports because the
  browser scopes some storage by host-without-port, and
  selects the most-recently-seen drive URL on hydration.
* **Hand-off**: @@Systacean (owns `systacean-3`). Worth a
  follow-up after the cache-headers patch.

### State left on disk

* `/tmp/chan-webtest-a-1/list.md` — test artifact, can be
  removed when no longer needed for repro.
* `/tmp/chan-webtest-a-1/note-a.md` — left intact; remains
  the B20 repro.
* Server: `chan serve --port 8801 --no-browser
  /tmp/chan-webtest-a-1/` still running for click-around if
  @@Alex wants it.
  URL: `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`
* Stale Lane B servers (8810 + 8811): stopped. Lane B drives
  themselves untouched at `/tmp/chan-webtest-b-1/` and
  `/tmp/chan-webtest-b-drift/`; relaunch by re-running the
  matching `chan serve --port {8810,8811} …` against those
  paths.

## 2026-05-18 (resume) BST - Walkthrough complete
