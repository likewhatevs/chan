# Chan's UI exploration
Writing down ideas for us to explore how to experience using Chan, and especially how to maximise space based on previous experiences.
First of all, I think the current application we have is extremely useful with this tab management and tabs. My next take is to make the other overlays, except for Settings, first-class tabs, and no longer overlays - well later we can make tabs and panes fullscreen anyway.
## Phase 1
So the first steps:
1. Move Graph and File Browser to tabs just like Editor and Terminal
3. These overlays->tabs migration bring their own inspector
  4. We do not change the left-right optional file browsers, they remain
5. From here on, I think only the Search and Settings are on OverlayShell (check?)

Standard verification: no hang ups on regular browing, on the various 'Graph from here' and 'Show File' etc across various surfaces and inspectors.
## Phase 2
We are coming to a point in which our application is this hybrid-tab system and we're now going to support Editor, Terminal, Graph, and File Browser.
I'm going to refer to this multi-pane, multi-app window as Hybrid from now on - going to try to use capitalised form to bring clarity.

Originally I was thinking about Hybrid as a window we could spawn multiple of and tile across the viewport (looked at https://nextapps-de.github.io/winbox/ as a windowing reference and https://hypr.land/ for the tiling style). After thinking it through, the model simplifies: panes are the tile atoms, there is one Hybrid, and "make another Hybrid" collapses into "add another pane to the tile tree." One viewport, one tree, panes as leaves. Side-by-side comparison falls out for free because that's just two leaves in a horizontal split.

Floating windows and minimise are out. Tiling-first,and we could no z-order, no free-position/size to track.

### Model
- Binary tree, not grid. Every split has two children, horizontal or vertical, nested arbitrarily. Dragging a divider redistributes between two siblings only, never reflows the whole layout.
- Pane = leaf, tab = content inside the leaf. Tabs stay scoped per-pane like they already are.
- Detach tab to new pane. Drag a tab out, the target leaf splits in the direction of the drop edge, the tab becomes the new sibling. Reverse: drag a tab into another pane's tab bar; the source pane collapses if it was its last tab.
- Resize is local. Dividers act on two siblings only, no row/column spans, no global recalc.

### Keyboard: pane mode (Cmd+K)
Pane mode is **transactional**. Cmd+K snapshots the current layout, every op inside the mode runs against a draft, Enter commits the draft, Esc discards it. No conflation of "exit" and "undo" because each Cmd+K session is exactly one transaction.

Entering the mode flips the chrome with a slight overlay fx: thin tint on unfocused panes, brighter border on the focused one, a small pane-mode pill in the status bar. Enough to make the mode unmistakable without burying the layout underneath.

After Cmd+K, Cmd is released and the keys below are unprefixed:

```
W A S D              move focus (up / left / down / right)
↑ ← ↓ →              swap focused tile with neighbour in that direction
[                    shrink focused tile horizontally
]                    grow focused tile horizontally
-                    shrink focused tile vertically
=                    grow focused tile vertically
Shift + [ ] - =      larger nudge (e.g. 10% vs 2%)
0                    equalise siblings at the current split level
Enter                commit transaction and exit
Esc                  discard transaction and exit
```

Semantics:
- Focus moves and swaps are no-ops when there is no neighbour in that direction.
- Resize operates on the focused tile inside its parent split. If the parent split is on the wrong axis (e.g. `]` while the parent is a vertical split), walk up the tree to the nearest ancestor on the right axis and resize there. This matches the Hyprland "make me wider always works" feel.
- Resize clamps to a sensible minimum so a pane cannot collapse to zero.
- Transactional state is a tree snapshot taken on Cmd+K (shape + ratios + focus pointer). The draft tree drives rendering; commit replaces the live tree with the draft, abort drops the draft. Snapshot is small and cheap.

### Must: persistence
Layout state survives reload and reopen: tree shape, split ratios, per-pane tab order, focused pane. Belongs alongside the existing per-window state keyed by w=<label>. Without this, tiling is a toy.

### Verification
Open various tab/pane configurations. Drag tabs to detach and re-attach. Resize dividers in nested splits. Close panes back to a single leaf. Reload mid-layout and confirm the tree comes back the same. Validate "Graph from here" / "Show File" cross-nav from any pane lands somewhere sensible.

They (hyprland) seem to use the same shake fx + css hover wobble that we do.
