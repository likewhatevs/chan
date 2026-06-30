# v0.58.0 draft: systemd PTY restore polish and desktop reconnect follow-up

Cut from `main` after `v0.57.0`. This draft starts with the Linux fdstore restart follow-up that now passes the real systemd PTY e2e in Lima. The remaining release work is the chan-desktop integration that refreshes already-open devserver windows after a restart rotates tenant tokens.

## Theme

Make systemd-managed devserver restarts feel like a reconnect, not a terminal loss. The backend now preserves the PTY master, validates that the slave side is still live, and carries enough replay tail for the browser to redraw without a false missed-bytes banner.

## Landed

### Systemd fdstore PTY restore

- PTY masters are duplicated as file descriptors instead of reopened through `/proc/self/fd`, so the restored descriptor still points at the live slave-side process.
- Restart manifests carry a bounded replay tail alongside each stored PTY fd. Imported sessions seed their replay ring from that tail while keeping the original sequence coordinates.
- Startup restore skips fdstore entries whose PTY slave no longer has a live process, avoiding a restored session that cannot actually speak to a terminal.
- `--force` on a systemd restart is now explicitly destructive; the default restart path prepares fdstore preservation when the service was already running.
- `chan-systemd` includes an opt-in real systemd e2e: the main transient unit owns the fdstore entry, a helper transient unit keeps the PTY slave alive through `cat`, and the restarted service proves IO still works after inheriting the fd.

## Validation

- `cargo test -p chan-library --locked restored_ring`
- `limactl shell default bash -lc 'cd /Users/fiorix/dev/github.com/fiorix/chan && CARGO_TARGET_DIR=$HOME/chan-target cargo test -p chan-systemd --locked'`
- `limactl shell default bash -lc 'cd /Users/fiorix/dev/github.com/fiorix/chan && CARGO_TARGET_DIR=$HOME/chan-target CHAN_SYSTEMD_FDSTORE_E2E=1 cargo test -p chan-systemd systemd_fdstore_e2e --locked -- --nocapture'`

## Pending Desktop Integration

The server side is working, but an already-open chan-desktop devserver window can keep its old per-window tenant token after `chan devserver --system --restart`. That leaves the window on the reconnect overlay, and Cmd+R reloads the stale URL with `?t=<old-token>`, producing `restore failed: missing or invalid token`. Hiding and showing the same window from the launcher works because that path goes through the devserver window watcher and gets the fresh `WindowRecord` token.

The integration fix should stay in the existing desktop model:

- keep `web/packages/workspace-app/src/api/desktop.ts` using the existing `reloadWindow()` helper and `reload_window` IPC;
- make the Tauri window watcher refresh already-open devserver webviews when the authoritative launch fields change;
- make native `reload_window` rebuild a `lib-*::w-*` devserver window from the latest devserver feed before falling back to `window.location.reload()`;
- do not add a new browser-side token fetch or reconnect command.
