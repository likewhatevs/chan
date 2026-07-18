# v0.59.1: desktop excalidraw subgraphs and launcher icon column

Cut from `main` after `v0.59.0`. This is a patch that clears the v0.59.0 chan-desktop known limitation, reverts the v0.59.0 launcher column alignment, and swaps the remote window-title glyph.

## Theme

Make `mermaid-to-excalidraw` diagrams render everywhere the plain `mermaid` renderer does, and undo the launcher alignment regression that shipped alongside the v0.59.0 diagram work.

## Excalidraw subgraphs on chan-desktop

- A `mermaid-to-excalidraw` flowchart containing a `subgraph` failed to convert on chan-desktop (WKWebView), logging `SubGraph element not found` and leaving an error or a rasterized image in place of the diagram; this was the v0.59.0 known limitation. The v0.59.0 note guessed at a font or WASM cause; the real cause was a bug in `@excalidraw/mermaid-to-excalidraw`. Mermaid 11 renders subgraph cluster elements with a render-id prefix (`id="diagN-Machine"`), but the library looked them up by exact id (`[id='Machine']`) instead of the prefix-tolerant match its node and edge lookups already use, so the cluster was never found.
- The library's cluster lookup is patched via `patch-package`, so subgraph flowcharts now convert to real excalidraw shapes in both the browser and chan-desktop.
- As a safety net, the excalidraw block also degrades to the plain `mermaid` renderer whenever a conversion fails on source the plain renderer can still draw, so a diagram always shows and only genuinely broken source surfaces its error.

## Launcher icon column

- This reverts the v0.59.0 `--rail-step` button-column alignment, so launcher button groups return to their per-element spacing and the "Library" title sits flush-left again.
- Each devserver leads its two rows with a left icon column: the Globe kind mark on the name row and the OS mark directly under it on the `host:port` row, so they align as one column; the connected status dot stays on the name row.

## Remote window glyph

- chan-desktop remote and devserver window and terminal titles use an up-right arrow (`↗`) instead of `⊕`, which rendered as a plus in the macOS title-bar font. The glyph stays monochrome line-art; the launcher Globe and the local-window glyphs are unchanged.

## Validation

- Frontend own-gate after the last edit: `npm run check` plus the full `npm run test`, with `excalidraw_render.test.ts` covering the fallback path.
- The full `make pre-push` gate before push.
- On-device WKWebView verification (the subgraph flowchart rendering with the console clear of `SubGraph element not found`) is the maintainer's, since the development host has no WKWebView.

## Release

- GA bumps all release pins to `0.59.1`, updates the changelog and this release report, then tags `v0.59.1`. No rc was cut.
