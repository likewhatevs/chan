# architect-syseng-1

Historical syseng prep note. At the time, syseng-1 was blocked on
rustacean-1/-2/-3; later phase work moved all of those tasks and
syseng-1 to REVIEW. Prep notes, the reusable fixture-drive build
script, the hardening checklist, and the chan-drive survey remain in
`syseng-1.md` under "2026-05-16 Prep phase".

## Advisory for rustacean-2 (filesystem-graph design)

Three constraints from the chan-drive survey directly shape the
fs-graph walker. Folding these into the rustacean-2 brief before
rustacean starts will save a round-trip later.

1. **Cannot reuse `chan-drive::fs_ops::walk_drive` for the fs-
   graph.** That walker DROPS symlinks/FIFOs/sockets/devices by
   design (correct for the content index, wrong for a graph that
   needs ghost/symlink/hardlink nodes). The fs-graph walker
   belongs in chan-core (likely a `walk_drive_with_specials` next
   to `walk_drive` in `fs_ops.rs`) and must keep lstat semantics.

2. **Symlink target classification must NOT use
   `resolve_safe_strict`.** That helper rejects mid-path symlinks
   pointing outside the drive, which is right for read/write
   but wrong for a graph that wants to surface those as ghost
   nodes labeled `outside-drive`. The walker should `readlink`,
   classify the literal target, and emit:
   - in-drive + exists -> edge to that node
   - in-drive + missing -> ghost
   - outside-drive    -> ghost, NEVER traversed
   - readlink fails   -> ghost

3. **Hardlink identity is `(st_dev, st_ino)`, not path.**
   `nlink>1` is a hint, the dedup key is the tuple. The fixture
   in `syseng-1.md` exercises this with
   `top.md` <-> `hardlink-to-top.md` sharing inode 72108170.

4. **Loop guard.** `readlink` chains can loop (`a -> b -> a`).
   The walker needs a visited-`(dev, ino)` set per traversal
   plus a hop cap (suggest 40, matching `MAXSYMLINKS`).

## Advisory for rustacean-1 (migration purge)

The pre-v3 contact-email backfill has a two-side surface:

- Consumer (in scope for rustacean-1):
  `crates/chan-server/src/indexer.rs:142-155`, the
  `emails_need_backfill` branch and the one-shot rebuild it
  queues.
- Producer (chan-core, NOT in scope here):
  `chan-drive::graph.rs::contacts_need_email_backfill`
  (plus tests around lines 2143-2210 of that file).

The rustacean-1 brief already says chan-core cleanup is filed
as a separate task to architect; this confirms the path.

## Follow-up status

Superseded by later phase work: syseng-1 reached REVIEW, and the
blockers/residuals it found were dispatched and resolved through
`architect-syseng-2`, `rustacean-4`, `rustacean-5`, and `rustacean-6`.
