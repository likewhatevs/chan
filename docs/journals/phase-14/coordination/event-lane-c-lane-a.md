# Inbox: @@LaneC -> @@LaneA

Append-only. Newest entry at the bottom.

---

## 2026-05-29 - ack: `--tunnel-workspace` -> `--tunnel-workspace-name` copy sweep

Got it, thanks for the root-cause and the file list. Triaged against
HEAD (`grep -rnE -- '--tunnel-workspace([^-]|$)' | grep -v
tunnel-workspace-name`, excluding journals). Disposition:

- FIXED by me in main (isolated docs, no lane conflict):
  - `gateway/crates/profile/design.md:207`
  - `gateway/scripts/dev/README.md:107`
- ROUTED to @@LaneB (their tree, `gateway/crates/identity/web`; their
  round-2 gateway cleanup owns it): the two
  `gateway/crates/identity/web/src/views/Workspaces.svelte` snippets
  (:326, :356). See `event-lane-c-lane-b.md`.
- LEFT to your branch: `desktop/src-tauri/src/main.rs:871` still shows
  the old flag in main, but that is your `build_snippets()` fix which
  lands when lane-a merges. No action from me.
- FLAGGED, deliberately NOT edited: two more stale references in applied
  SQL migration comments (`gateway/migrations/0009_workspace_grants.sql:6`,
  `0010_workspaces.sql:5`). Editing an applied sqlx migration file
  changes its checksum and breaks migration validation on existing
  databases, so I am not touching them as copy cleanup.

The rest of the C1 frontend comments/docs/copy pass still waits on A+B
merging (it edits the same code you are rewriting); this flag fix was
isolated enough to take now.

---

## 2026-05-29 - heads up: your desktop snippet fix is now on main too

@@Alex landed the desktop fix directly on main as `f2eb32a9
fix(chan-desktop): correct chan serve tunnel flag in tunnel listen-panel
snippet`, the same `build_snippets()` change you have on your branch. When
you rebase `phase-14-lane-a` onto main, expect your own snippet commit to
go empty or conflict on `desktop/src-tauri/src/main.rs`; drop it in favor
of main's. The gateway copy (`00465a33`) is also on main.

---

## 2026-05-30 - FYI: tunnel hostname renamed drive.chan.app -> workspace.chan.app

Touches your tunnel area, so flagging. Per @@Alex (canonical hostname =
`workspace.chan.app`; the online service is already deployed and tested
on it, sole user, no back-compat), I renamed `drive.chan.app` ->
`workspace.chan.app` across the chan client tunnel default
(`crates/chan/src/main.rs`), the `chan-tunnel-{proto,client,server}`
crates (defaults, the host-allow suffix match, design docs, and tests),
chan-server's host check + comments, the desktop tunnel analogue,
`docs/manual/tunnel.md`, and the marketing copy. The 75 `drive.chan.app`
mentions under `docs/journals` are history and were left as written.
`cargo test` for chan / chan-server / chan-tunnel-{proto,client,server}
is green (one mixed-case test input needed a manual fix the lowercase
sweep missed). It is a separate commit on `phase-14-lane-c`, not pushed.
