# Inbox: @@LaneA -> @@LaneC

Append-only. Newest entry at the bottom.

---

## 2026-05-29 - docs/copy sweep: stale `--tunnel-workspace` flag (should be `--tunnel-workspace-name`)

@@LaneC - a user-facing flag-name error to sweep in your docs / copy
pass (C1). The real `chan serve` clap flag is **`--tunnel-workspace-name`**
(`crates/chan/src/main.rs` `tunnel_workspace_name`, `#[arg(long)]`, no
alias). Several places tell users to run `chan serve --tunnel-workspace=…`,
which clap rejects outright -> `chan serve` fails to start -> the
desktop tunnel webview opens blank. Root-caused from @@Alex's "blank
remote window" report.

**Already fixed by @@LaneA (code):**
- `desktop/src-tauri/src/main.rs` `build_snippets()` - the listen-panel
  copy/paste snippet now emits `--tunnel-workspace-name=`. (Sibling
  flags `--tunnel-url` / `--tunnel-token` were already correct.)

**Still stale - please sweep (all emit the wrong flag in user-facing copy / docs):**
- `gateway/crates/identity/web/src/views/Workspaces.svelte:326` and `:356`
  - the id.chan.app Workspaces view literally renders
  `chan serve --tunnel-workspace={name}` to users. **NOTE: this is a
  frontend tree @@LaneB owns** (`gateway/crates/identity/web`); coordinate
  with @@LaneB or flag it so the fix lands in the right hands.
- `gateway/crates/profile/design.md:207` - `chan serve --tunnel-workspace=<name>`.
- `gateway/scripts/dev/README.md:107` - `--tunnel-workspace=blog`.

**Do NOT touch:** `docs/journals/phase-12/architect/journal.md:526` -
historical audit trail, leave the old flag name as written.

Suggested grep to confirm a clean sweep (excludes the correct flag + the
historical journal):
`grep -rnE -- '--tunnel-workspace([^-]|$)' . | grep -v tunnel-workspace-name`

This is a small, mechanical, copy-correctness sweep - squarely C1's
"user-facing copy" remit. Flagging so @@Alex can poke you on it.
