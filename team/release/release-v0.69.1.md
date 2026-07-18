# Release v0.69.1 - tunnel-mode systemd restart and a rootless devserver image

A small patch: a tunnel-mode devserver can restart gracefully under systemd (fd-preserving, as the local path already did), and the chan-devserver container image switches to a rootless, PPA-free chan install. Coordination artifacts live in the untracked `dev/v0.69.1/` tree of the round host's checkout.

## What shipped

- **`chan devserver --restart` works in tunnel mode under systemd.** Setting `CHAN_TUNNEL_TOKEN` (env or `--tunnel-token`) with `--service=systemd` now configures the service in tunnel mode instead of being refused: the generated unit carries the PAT via `Environment=` (written 0600) and dials the gateway via `--tunnel-url`, reusing the first-run endpoint on a plain restart and refreshing it on `--force`. Restart preserves live PTYs across the bounce through the systemd fd store, exactly as the non-tunnel path does; under systemd the tunnel devserver also binds its loopback management API so the fd-park handshake can reach it. launchd still refuses tunnel mode (its plist would persist the token 0644).
- **The chan-devserver container image installs chan per-user, without the PPA.** `chan-devserver.sdme` no longer enables `ppa:fiorix/chan` or bakes the `chan` package into the rootfs; `chan-devserver-provision` installs the released `chan` as the target user via `https://chan.app/install.sh` into `~/.local/bin`, so the user can `chan upgrade` without root, honoring `http(s)_proxy`. The systemd user unit runs the absolute `~/.local/bin/chan`.
