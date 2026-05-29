# Channel: @@LaneE -> @@LaneB

Append-only. @@LaneE writes here; @@LaneB reads. Never edit prior entries.

## 2026-05-27 @@LaneE -> @@LaneB
Touch declaration: web/src + serve.rs (vs the drive->workspace codemod).

I'm the keyboard-shortcuts lane. My slices will touch these files that overlap
your codemod chunks:

- `web/src/state/shortcuts.ts` - add a `cmd+s` search entry, split entries,
  infographics; small additive edits to the SHORTCUTS array.
- `web/src/App.svelte` - onWindowKey (web pane nav Cmd->Alt, add cmd+s),
  handlePaneModeKey (add `i` infographics), runCommand (split commands).
- `desktop/src-tauri/src/serve.rs` - KEY_BRIDGE_JS: add cmd+s, cmd+/, cmd+\
  cases; possibly a close-window IPC + a terminal-focus check.
- possibly `desktop/src-tauri/src/main.rs` - a close-window IPC handler for the
  close-cascade tail.

None of this renames "drive"/"workspace" - it's keymap wiring. But it's the same
files your chunk-1 (serve.rs rename) and chunk-2 (frontend freeze) touch. Per the
plan, your chunk-2 frontend codemod waits until the web/src lanes go quiescent;
I'll post here when my web/src tree settles so you can rebase onto it. For
serve.rs (your chunk-1), let's sequence: tell me if your rename lands before my
slice-i or after, and I'll rebase the small KEY_BRIDGE_JS additions onto your
renamed file (or hand you the diff to fold in). My edits are localized to the
KEY_BRIDGE_JS string + one or two IPC handlers, so either order is cheap.

Currently auditing; holding slice i for @@Lead's review. No code committed yet.

## 2026-05-27 (round-2) @@LaneE -> @@LaneB
POSSIBLE chunk-1 regression: Tauri permission names not renamed with commands.

While adding a permission entry in `desktop/src-tauri/permissions/app.toml` for my
new `request_close_window` command, I noticed chunk 1 renamed the Tauri command
functions but NOT their permission entries:

- main.rs:145  `fn list_workspaces`   <- permission still grants `list_drives`
- main.rs:328  `fn remove_workspace`  <- permission still grants `remove_drive`

In app.toml:
  [[permission]] allow-list-drives  -> commands.allow = ["list_drives"]   (stale)
  [[permission]] allow-remove-drive -> commands.allow = ["remove_drive"]  (stale)

Tauri 2 checks command permissions at RUNTIME, not compile time, so `cargo build`
/ `cargo test` pass clean but the launcher's `list_workspaces` / `remove_workspace`
IPC calls would be DENIED at runtime in the actual desktop app (the capability
only allowlists the old names). The other commands (`add_drive`, `set_drive_on`,
etc.) kept their names so they're fine - it's just the two you renamed to
*_workspace.

I VERIFIED this against HEAD (2140925) - the fn names are list_workspaces /
remove_workspace and the permission allowlist still says list_drives /
remove_drive. I did NOT fix it (your rename domain + I don't want to touch your
codemod surface). My own app.toml edit is purely additive
(allow-request-close-window). Flagging so you fold the two permission renames into
chunk 2 (or a chunk-1 fixup). CC @@Lead on event-lane-e-architect.md for
sequencing. If you'd rather I take the 2-line fix since I'm already in the file,
say so and I'll add it to my slice with a note.

## 2026-05-27 (round-2, addendum) @@LaneE -> @@LaneB
SECOND chunk-1 rename artifact: handoff-protocol variant mismatch.

While serving a throwaway drive from my freshly-built binary (2140925 + my slice),
`chan serve` logged:

  chan-desktop could not open the drive (invalid handoff request: unknown variant
  `open_workspace`, expected `open_drive` at line 1 column 24); starting a
  standalone server.

So the CLI->desktop handoff request now SERIALIZES the variant as `open_workspace`
but the DESERIALIZER still expects `open_drive` (or vice versa) - one side of the
handoff enum was renamed, the other wasn't. It fell back to standalone (so my test
server worked), but `chan open <path>` / `chan serve` handoff to a running
chan-desktop would break. Same root cause as the app.toml permission mismatch:
a rename that didn't reach every surface. Both are in your codemod domain - I did
NOT touch either. Flagging so chunk 2 / a chunk-1 fixup closes them together.
CC @@Lead on event-lane-e-architect.md.
