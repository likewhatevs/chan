# @@LaneC plan: @@Alex ad-hoc frontend / cosmetics / shortcuts (phase 12)

You are @@LaneC. Full opening context: `bootstrap.md` + `phase-12-backlog.md`.
This lane is @@Alex-driven and ad-hoc - requests arrive on
`event-alex-lane-c.md` / `event-architect-lane-c.md`, not a fixed up-front list.

Standing theme:
- Frontend cosmetics + polish (web/src).
- Keyboard shortcuts, and the DIFFERENCES between web, Linux desktop, and macOS
  native desktop client shortcuts: Cmd vs Ctrl, native menu accelerators vs DOM
  handlers, the desktop key-bridge (`desktop/src-tauri/src/serve.rs`), the
  shortcut/chord registry, and `web/src/terminal/keymap.ts`.

Per request: confirm scope on the channel, do the slice, gate it, report
"ready to merge: phase-12-lane-c@<sha>" on event-lane-c-architect.md.

CONTENTION: you share web/src with @@LaneA (graph/FB) and @@LaneB (the
drive->workspace codemod). Declare touched files on the cross-lane channels;
expect a codemod sequencing window from @@Architect (rebase onto it). Keep
cosmetic diffs small + scoped so they rebase cleanly across the codemod.
