# v0.26.0 smoke checklist (for @@Alex)

What's already been smoked vs what's still YOURS to check by hand. Agents can
drive Chrome (Blink) but NOT chan-desktop (WKWebView) or a real trackpad, so
those land on you.

## CHECKED - agent/Lead verified (Chrome-driven or lane-smoked)
You can re-confirm any of these, but they were validated on a real build:

- [x] Editor - hyphen lists render DISTINCT dashes (not bullets); star/plus use
      depth glyphs; ordered uses numbers. (Lead, Chrome)
- [x] Editor - bullet CLEANUP: click MID-TEXT of a nested bullet lands the caret
      where clicked; EOL-click -> line end; arrow up/down -> goal column. All at
      depth 1 AND 2, bullet/hyphen/ordered. (LaneA matrix + Lead spot-check on
      rebuilt :8787: nested mid-text click -> offset 55 = where clicked.)
- [x] Editor - `[[` completes workspace paths + keeps filename/heading targets.
      (LaneA; you also completed a real link live in notes.md.)
- [x] File Browser - tab menu: "Reload" gone; New file or Directory / New
      Terminal / New Graph (workspace root) present. (Lead, Chrome)
- [x] File Browser - selection-menu shortcut hints (New Terminal Cmd+Alt+T, New
      Graph Cmd+Shift+M, Delete Backspace, Settings Cmd+,). (Lead, Chrome)
- [x] File Browser - expand a directory: NO "Loading" hang, NO replaceState
      SecurityError. (Lead, Chrome - rapid expand, zero console errors)
- [x] Inspector - pill + dropdown per category: Directory / File / Media /
      Binary. (Lead, Chrome) NOTE: the editor "Show Details" (5th) category uses
      the same component path; not separately clicked - eyeball it if you like.
- [x] Graph - select-on-"graph from here"; dir nodes have a root edge; binary/
      symlink renders as a file node (not contact); NO spurious reload on editing
      an out-of-scope file; Copy link to graph + click-to-open from markdown.
      (LaneB, definitive same-edit reload smoke + click-to-open)
- [x] Graph - selected node PERSISTS across window reload. (LaneB smoke: select
      -> reload -> restored)
- [x] Terminal - UTF-8 in less AND vim (em dash, accents, CJK, emoji as glyphs,
      not raw bytes). (Lead, Chrome - it's a PTY-locale fix, renderer-agnostic)

## YET TO CHECK - your hand-smoke (WKWebView / real trackpad / desktop)
These cannot be agent-driven:

- [ ] Terminal - hide the rich prompt (menu or Cmd+Shift+P): focus returns to
      the terminal (xterm), cursor active. [chan-desktop / WKWebView]
- [ ] Terminal - Cmd+C copies the selection, Cmd+V pastes (no stray SIGINT, no
      double-paste). [chan-desktop / WKWebView]
- [ ] chan-desktop - local-disk [New] workspace flow shows NO old pre-flight
      dialog (no double dialog over the SPA boot menu). [desktop, image-1/2]
- [ ] Editor - trackpad FREE-SCROLL: no stall/jump when the cursor is far from
      the scroll target. Check BOTH Hybrid editor AND Source mode. [real trackpad
      - Blink can't reproduce momentum, so this is yours]
- [ ] chan-desktop - NEW bug fix (toggle race; bug captured in
      team/desktop-off-toggle-bug.md): turning a workspace OFF no longer flips
      the toggle ahead of server shutdown, and a quick OFF->ON no longer
      strands the row ON-with-no-Open. Also retry a few rapid OFF/ON cycles.
      [chan-desktop / WKWebView; patch landed in fix(desktop) 20526d0c, gated
      green, hand-smoke is the only thing left to verify it]

## Note
Your own server at :8791 (lists.md drive) is the PRE-FIX bundle. Rebuild it
(`npm run build` in web/ -> `cargo build -p chan` -> restart) to see the fixes,
or use the Lead's rebuilt :8787 (chan-smoke-p18 drive, all fixes baked).
