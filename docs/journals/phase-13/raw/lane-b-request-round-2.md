# @@LaneB request - Phase 13 round 2

You are @@LaneB, the round-2 lead for **editor lists + Bold/Italic
chords + desktop Cmd+Shift+N + hamburger split labels**, AND the
**merge-gate orchestrator**: re-gate the combined tree, serialize merges
to main, cut v0.18.0. You MAY spawn 2-3 in-session subagents. You do NOT
push to origin without an explicit @@Alex ask.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-2.md`
  (images: `image-1.png` list style, `image-13.png` current hamburger)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap-round-2.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/README.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-b/journal.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-b.md` (inbox)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-a-lane-b.md` (cross-lane from @@LaneA; may not exist yet - the Team Work label string lands here)

## Worktree + branch

Source ONLY in `../chan-lane-b`. The dir exists from round 1 on a stale
branch; on your FIRST turn bring it to main with the round-2 branch:

```
git -C ../chan-lane-b status            # confirm clean
git -C ../chan-lane-b checkout -B phase-13-r2-lane-b main
```

Journals + channels + this request file live in the MAIN checkout at
`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/` and
are edited by ABSOLUTE PATH (never the worktree copy).

## Scope

### B-slice 1 - Editor list glyphs (`image-1.png`)

`web/src/editor/decorations/blocks.ts` (~437-520) already distinguishes
`-` vs `*` vs ordered via class decoration (round-1 source-marker
preservation - keep it intact). `web/src/editor/Wysiwyg.svelte`
(~981-1046) holds the marker CSS + depth-aware indent + guide stripes.

Render to match `image-1.png` (adapt to our fonts/themes, same glyphs +
spacing):
- Enumerated/ordered: `1.` with nested levels restarting numbering.
- Hyphenated (source `-`): a dash/en-dash "-" glyph at every level.
- Bullet (source `*`): filled "*" at top level, hollow nested.

Apply via the existing marker decoration classes + `::marker` /
pseudo-element CSS. Don't break source-marker round-trip.
**Browser-smoke** all three kinds at multiple nesting depths.

### B-slice 2 - Bold (Cmd+B) + Italic (Cmd+I) chords

Commands already exist, just unbound: `web/src/editor/commands/format.ts`
(~94-105 `toggleBold`/`toggleItalic`), exported on `Wysiwyg.svelte`
(~302-305); `StyleToolbar.svelte` (~290-307) buttons already advertise
the chords in tooltips.

- Bind `Mod+B` / `Mod+I` in the editor's `Prec.high(keymap.of([...]))`
  block in `Wysiwyg.svelte` (~439-530).
- **@@Alex decision - move Dashboard off Cmd+I**: in
  `web/src/state/shortcuts.ts` (~366-374, `app.dashboard.open`), drop
  the `web: "Mod+I"` / `native: "Mod+I"` bindings; keep its Hybrid Nav
  `Mod+. i` (update the `note`). Dashboard stays reachable via Hybrid
  Nav + the Dashboard hamburger item; no `App.svelte` change needed.
- Add editor formatting entries to `shortcuts.ts` for discoverability
  (the comment at ~448-451 already anticipates this; Cmd+B is free).
- Confirm CodeMirror intercepts Cmd+I/B before the global handler when
  the editor is focused. **Browser-smoke**: italic/bold toggle in the
  editor AND Dashboard still opens via `Cmd+. i`.

### B-slice 3 - Desktop Cmd+Shift+N -> CURRENT workspace (Tauri only)

`desktop/src-tauri/src/main.rs` (~1868-1871) routes the
`CmdOrCtrl+Shift+N` accelerator to `open_new_launcher_window()` (the
Workspaces picker). Change it to spawn a new window of the FOCUSED
window's workspace:
- Resolve the focused window's workspace key from its label
  (`workspace-<key>-<seq>`).
- Reuse `serve::spawn_local_workspace_window()` /
  `build_workspace_window()` (`serve.rs` ~195-221, ~328-392) - the same
  path `open_local_workspace` (`main.rs` ~1201-1216) already uses for
  the launcher's "open workspace".
- **Verify in chan-desktop** (WKWebView), not Chrome (per
  `reference_terminal_webgl_wkwebview`): this is desktop-only.

### B-slice 4 - Hamburger split labels -> Cmd+/ and Cmd+?

The desktop already FIRES `app.pane.splitRight` on Cmd+/ and
`app.pane.splitDown` on Cmd+Shift+/ (= Cmd+?) via KEY_BRIDGE_JS, and
`shortcuts.ts` (~313-324) already binds `Mod+/` / `Mod+Shift+/`. The
visible gap is the hamburger menu STILL SHOWS the Pane-Mode chord
`Cmd+. /` / `Cmd+. ?` (`Pane.svelte` ~243-246, ~508-522, via
`paneModeChordLabel`).
- Change those two rows to display the direct `Cmd+/` and `Cmd+?` (use
  the formatted chord from the shortcuts entry, not `paneModeChordLabel`).
- Optional: add web bindings (Cmd+/ is not browser-reserved). Default:
  display + keep the desktop bindings; add web entries only if clean.
- Update `Pane.test.ts` (~209-212).

### Shared files Lane B owns (Lane A supplies strings only)

- `web/src/state/shortcuts.ts` - you own ALL edits. Lane A sends the
  Team Work *label* string for `app.terminal.richPrompt` on
  `event-lane-a-lane-b.md`; apply the label (keep the id stable).
- `web/src/components/Pane.svelte` + `web/src/components/EmptyPaneWelcome.svelte`
  - apply the "Rich Prompt" -> "Team Work" menu/label rename (string
  from Lane A) alongside your split-label edit.

Declare any unexpected overlap with Lane A on
`event-lane-b-lane-a.md` BEFORE editing.

### B-slice 5 - Merge gate + v0.18.0 (round close)

Re-gate the COMBINED tree before merging either lane to main. Lanes hand
merge-ready slices to you on `event-lane-{a,b}-alex.md`; they do not
self-merge. At round close, on a clean main, follow
`bootstrap-round-2.md` "Merge + release": bump versions, `Cargo.lock`,
dry-run `release.yml` (publish=false), tag `v0.18.0`. **Push the tag
ONLY on an explicit @@Alex ask.** Verify `/dl/latest.json` supersedes
0.17.0 + self-upgrade 0.17.0 -> 0.18.0 in chan-desktop. Commit phase-13
docs as `docs(phase-13): close round 2`.

## Suggested slicing (you own the call)

- Subagent 1: editor list glyphs (B-slice 1) - CodeMirror decorations +
  CSS.
- Subagent 2: bold/italic chords + Cmd+I/Dashboard remap (B-slice 2) +
  hamburger split labels (B-slice 4) - both `shortcuts.ts`-adjacent.
- Subagent 3 (or yourself): desktop Cmd+Shift+N (B-slice 3) - Tauri Rust.
- You: merge gate + release.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check  &&  npm run build  &&  npm test
```

Then append to `event-lane-b-alex.md`:

```
ready to merge: phase-13-r2-lane-b@<sha>  -  <one-line slice summary>
```

Browser-smoke (per `feedback_svelte_static_gate_misses_runtime`): the
list glyphs at depth and the bold/italic chords. chan-desktop smoke (per
`feedback_terminal_webgl_wkwebview`): the Cmd+Shift+N new-window
behavior.

## Coordination rules

- Append-only directional channels; never edit another agent's entries.
- Each turn, BEFORE acting, read `event-alex-lane-b.md` (inbox) and
  `event-lane-a-lane-b.md` (the Team Work label string lands here).
- Progress + merge-ready + merge-gate confirmations + release cut:
  append to `event-lane-b-alex.md`.
- Cross-lane to @@LaneA: append to `event-lane-b-lane-a.md`.
- Self-document in `lane-b/journal.md` (append a round-2 section).
- Subagents speak through you on the bus.

## First turn checklist

1. Bring the worktree to main on `phase-13-r2-lane-b` (above).
2. Read all recovery files.
3. Append an opening round-2 entry to `lane-b/journal.md`.
4. Kick your slices (they're file-disjoint - run in parallel).
5. Work each slice to the gate; report on `event-lane-b-alex.md`.

## Out of scope

Anything not in `roadmap-round-2.md`. Escalate scope creep on
`event-lane-b-alex.md`. Don't push to origin without an explicit
@@Alex ask.
