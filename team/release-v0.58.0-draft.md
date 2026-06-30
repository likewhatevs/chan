# v0.58.0 draft: systemd PTY restore polish and desktop reconnect follow-up

Cut from `main` after `v0.57.0`. This draft starts with the Linux fdstore restart follow-up that now passes the real systemd PTY e2e in Lima, plus the chan-desktop integration that refreshes already-open devserver windows after a restart rotates tenant tokens.

## Theme

Make systemd-managed devserver restarts feel like a reconnect, not a terminal loss. The backend now preserves the PTY master, validates that the slave side is still live, and carries enough replay tail for the browser to redraw without a false missed-bytes banner.

## Landed

### Systemd fdstore PTY restore

- PTY masters are duplicated as file descriptors instead of reopened through `/proc/self/fd`, so the restored descriptor still points at the live slave-side process.
- Restart manifests carry a bounded replay tail alongside each stored PTY fd. Imported sessions seed their replay ring from that tail while keeping the original sequence coordinates.
- Startup restore skips fdstore entries whose PTY slave no longer has a live process, avoiding a restored session that cannot actually speak to a terminal.
- `--force` on a systemd restart is now explicitly destructive; the default restart path prepares fdstore preservation when the service was already running.
- `chan-systemd` includes an opt-in real systemd e2e: the main transient unit owns the fdstore entry, a helper transient unit keeps the PTY slave alive through `cat`, and the restarted service proves IO still works after inheriting the fd.

### Desktop reconnect integration

- The Tauri window watcher now refreshes already-open devserver webviews when the authoritative launch fields change, so a stable `{library_id}::{window_id}` label no longer hides a rotated per-window tenant token.
- Native `reload_window` rebuilds `lib-*::w-*` devserver windows from the latest devserver feed before falling back to `window.location.reload()`, so Cmd+R no longer recycles a stale `?t=<old-token>` URL.
- Same-component terminal socket reconnects resume from the mounted xterm's in-memory cursor when the server generation still matches, while fresh xterms still require a validated snapshot before sending a replay cursor.

## Validation

- `cargo test -p chan-library --locked restored_ring`
- `limactl shell default bash -lc 'cd /Users/fiorix/dev/github.com/fiorix/chan && CARGO_TARGET_DIR=$HOME/chan-target cargo test -p chan-systemd --locked'`
- `limactl shell default bash -lc 'cd /Users/fiorix/dev/github.com/fiorix/chan && CARGO_TARGET_DIR=$HOME/chan-target CHAN_SYSTEMD_FDSTORE_E2E=1 cargo test -p chan-systemd systemd_fdstore_e2e --locked -- --nocapture'`
- `cargo test -p chan-desktop --locked`
- `cd web && npm run test -w @chan/workspace-app -- desktop.test.ts cmdRWindowReload.test.ts terminalRemountReplay.test.ts protocol.test.ts`
- `cd web && npm run check -w @chan/workspace-app`

## Repro Fixed

Before this fix, an already-open chan-desktop devserver window could keep its old per-window tenant token after `chan devserver --system --restart`. That left the window on the reconnect overlay, and Cmd+R reloaded the stale URL with `?t=<old-token>`, producing `restore failed: missing or invalid token`. Hiding and showing the same window from the launcher worked because that path went through the devserver window watcher and got the fresh `WindowRecord` token; now the already-open window is refreshed through the same model.
