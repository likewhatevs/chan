# Inbox: @@LaneC -> @@LaneB

Append-only. Newest entry at the bottom.

---

## 2026-05-29 - stale `--tunnel-workspace` flag in the identity SPA (your tree)

Routed from @@LaneA's copy-correctness flag (see
`event-lane-a-lane-c.md`). The real `chan serve` clap flag is
`--tunnel-workspace-name` (`crates/chan/src/main.rs` `tunnel_workspace_name`,
no alias); `--tunnel-workspace=` is rejected outright, so the
copy/paste snippet fails to start and the remote window opens blank.

Two user-facing occurrences live in YOUR tree
(`gateway/crates/identity/web`), so they belong in your round-2 gateway
cleanup, not mine:

- `gateway/crates/identity/web/src/views/Workspaces.svelte:326`
  ```
  <code>chan serve --tunnel-workspace={newWorkspace.trim().toLowerCase() || "<name>"}</code>.
  ```
  -> `--tunnel-workspace-name=...`
- `gateway/crates/identity/web/src/views/Workspaces.svelte:356`
  ```
  Offline &middot; start <code>chan serve --tunnel-workspace={d.name}</code>
  ```
  -> `--tunnel-workspace-name=...`

I have already fixed the two non-frontend gateway docs in main
(`gateway/crates/profile/design.md`, `gateway/scripts/dev/README.md`).
@@LaneA fixed the desktop listen-panel snippet on their branch. Two more
stale references sit in applied SQL migration COMMENTS
(`gateway/migrations/0009_workspace_grants.sql:6`,
`0010_workspaces.sql:5`); I left those untouched on purpose because
editing an applied sqlx migration file changes its checksum and breaks
migration validation on existing databases. Flagging for awareness, not
for a fix in this round.

---

## 2026-05-29 - stand down: @@Alex landed the whole sweep on main

Disregard the routing above. @@Alex committed the full fix directly to
main as `00465a33 fix(gateway): correct chan serve tunnel flag in
user-facing copy` (the two `Workspaces.svelte` lines plus the two
migration comments). Both `Workspaces.svelte` occurrences read
`--tunnel-workspace-name` on main now. Nothing for you to do here; just
pull/rebase on main so you do not reintroduce the old flag in your
round-2 gateway cleanup.
