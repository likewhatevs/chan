# fullstack-a-10: Chrome-style tab-name fade-out + full-path tooltip on file tabs

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Replace today's tab-name truncation (ellipsis or premature cut)
with Chrome's fade-out style: the tab title text fades into
transparency at the right edge of the tab when it doesn't fit.
No trailing ellipsis character.

Additionally, file tabs MUST show the **full file path** as a
hover tooltip so the user can see exactly which file the tab
represents when the visible name is truncated.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): "Tab name
abbreviation looks ugly — copy Chrome's fade-out style".
Reference screenshot: Alex shared a Chrome tab strip showing
the gradient fade.

## Acceptance criteria

* All tab strips (document tabs, terminal tabs, file-browser
  tabs, graph tabs, search tabs) use the CSS mask-image fade
  technique instead of `text-overflow: ellipsis`.
* The fade kicks in only when the text would otherwise
  overflow; short titles render fully without a fade.
* File tabs: hover shows the full file path (not just the
  basename) as a `<title>` tooltip or equivalent.
* File browser tree items (file AND directory rows): hover
  shows the full absolute path the entry resolves to.
* Other tabs (terminal, Graph, Search): hover tooltip not
  required. If trivially cheap to add for consistency, use
  the full tab title.
* Works in both light and dark mode (the mask gradient end
  colour stays transparent regardless of theme).

## How to start

1. Find the shared tab-name rendering helper (likely
   `web/src/components/PaneTabStrip.svelte` or a
   `TabTitle.svelte`).
2. Replace `text-overflow: ellipsis` + `white-space: nowrap`
   with the mask approach:

```css
mask-image: linear-gradient(to right, black calc(100% - 1.5rem), transparent);
-webkit-mask-image: linear-gradient(to right, black calc(100% - 1.5rem), transparent);
```

   Pick the fade-out width that visually matches Chrome
   (~1rem to 1.5rem of fade at the right edge tends to read
   well). The `text-overflow` rule can stay as a fallback for
   browsers without mask support; on chan-desktop's WebKit/
   WebView2 the mask version is the visible one.
3. Wire the file-tab full-path tooltip through whatever shape
   the rest of chan uses (probably `title` attribute on the
   tab element, or a small custom hover-card if title attrs
   feel cheap).

## 2026-05-19 — implementation note

Two edits:

1. **Pane.svelte tab strip** — dropped the `truncateTabTitle()`
   middle-elision wrapper at both call sites (tab strip + pane-
   mode preview title) and replaced the `.path { white-space:
   nowrap }` CSS with a `display: inline-block; max-width:
   22ch; overflow: hidden; white-space: nowrap; mask-image:
   linear-gradient(to right, black calc(100% - 1.25rem),
   transparent)` (plus the `-webkit-mask-image` mirror for
   WebKit). The full title renders into the constrained box;
   the gradient mask fades it to transparency at the right
   edge instead of inserting `[..]`. The 22ch cap reads close
   to the prior 15-char visible window without making short
   titles feel padded; the 1.25rem fade band matches Chrome's
   visual feel. `truncateTabTitle()` stays exported (tests
   still cover its behaviour) — just unused here.

2. **FileTree.svelte rows** — added a `fullPath(node.path)`
   helper that joins the drive root with the drive-relative
   path. Wired the result into the `title` attribute on both
   directory and file row roots. The file branch composes
   `fullPath` with the existing contact / view-only annotations
   so hover still surfaces those signals alongside the path.
   Tunnel-public mode returns the drive-relative path
   verbatim (root is intentionally blank there).

The tab strip's parent `<button>` already carried
`title={tabTooltip(t)}`, which returns the full file path for
file tabs (and useful contextual strings for terminal / graph /
browser tabs); kept as-is. The mask fades only when text would
overflow the 22ch cap; shorter titles render flush without a
visible mask edge.

Files touched:

* `web/src/components/Pane.svelte` — drop two
  `truncateTabTitle()` calls, drop the import, swap `.path`
  CSS for the mask-image fade.
* `web/src/components/FileTree.svelte` — `fullPath` helper +
  `title=` on directory and file row roots.

Pre-push gate (SPA portion): vitest 474/474 green;
`npm run check` 0 errors / 0 warnings.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Mask-image gradient is the right technique — `text-overflow:
ellipsis` was always going to look harsh once Alex flagged
the Chrome reference. The `22ch` cap + `1.25rem` fade band
reads well across both light and dark (transparent endpoint
is theme-agnostic by definition). Leaving `truncateTabTitle`
exported with its tests intact is correct — covered by other
call sites; the unused-but-not-dead pattern matches our
"VCS is the archive" rule (drop once we audit remaining
usages in a separate pass).

FileTree `fullPath` helper + `title=` wire-up handles both
directory and file rows; the composition with contact /
view-only annotations preserves the existing hover signal.
Tunnel-public mode returns the drive-relative path verbatim
which matches the existing root-blank intent.

The parent `<button>`'s existing `title={tabTooltip(t)}` for
file tabs already surfaces the full path; not double-wrapping
the span is right.

Gate green.

**Commit clearance**: approved. Suggested subject:

```
Tab strip + FB tree: Chrome-style fade-out + full-path hover (fullstack-a-10)
```

Push waits for Round-1 close.
