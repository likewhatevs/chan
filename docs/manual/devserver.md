# Devserver

A devserver is one `chan devserver` process that hosts many workspaces behind a single port. On the box, you start a devserver and then register workspaces into it with `chan serve PATH`: each `chan serve` registers its workspace and exits instead of binding its own port, so one process owns each workspace. Chan Desktop attaches to the devserver and lists its workspaces in their own launcher group. See [workspaces](workspaces.md) for the serve side and [Chan Desktop](desktop.md) for the attach lifecycle.

```sh
chan devserver --bind 127.0.0.1 --port 8787
```

There is no TLS and only a bearer-token gate, so keep the bind on loopback and reach the devserver from another machine over a forwarded localhost port (below).

## Keep it running: `--systemd` (Linux), `--launchd` (macOS)

On Linux, `chan devserver --systemd` runs the devserver under a systemd user service named `chan-devserver.service` instead of in the foreground, so it survives the terminal that launched it and survives logout. This is what lets you start a devserver on a box, disconnect, and resume your sessions later: the service keeps the workspaces mounted, and re-running `chan devserver --systemd` re-attaches to the already-running service (it prints "re-attaching to the running systemd user service chan-devserver.service" and does not rewrite the unit) rather than starting a second one.

On first run it:

- Ensures user lingering is on, so the service runs without an active login session. If lingering is off and chan cannot enable it, it stops with a hint to run it once as root:

  ```sh
  sudo loginctl enable-linger USER
  ```

- Writes `~/.config/systemd/user/chan-devserver.service` (its `ExecStart` runs `chan devserver --bind=... --port=...`), then enables and starts it with `systemctl --user enable --now` and follows its journal.

`--systemd` is Linux-only — on macOS use `--launchd` (below); on other platforms it prints a note and runs in the foreground.

Manage the service with the usual user-scoped systemd commands:

```sh
systemctl --user status chan-devserver.service
systemctl --user restart chan-devserver.service
systemctl --user stop chan-devserver.service
journalctl --user -u chan-devserver.service -f
```

### macOS: `--launchd`

On macOS, `chan devserver --launchd` runs the devserver under a per-user launchd **LaunchAgent** named `app.chan.devserver`, the counterpart to `--systemd`. It writes `~/Library/LaunchAgents/app.chan.devserver.plist` (whose `ProgramArguments` run `chan devserver --bind=… --port=…`), loads it into your `gui/$UID` login session with `launchctl bootstrap`, and starts it. Re-running re-attaches to the already-running agent (it prints "re-attaching to the running launchd agent app.chan.devserver") rather than starting a second one.

A LaunchAgent outlives the terminal that launched it and your GUI login session, but — unlike systemd's linger — it does **not** survive a full logout (that would need a root LaunchDaemon, out of scope for a per-user dev tool). launchd has no journal, so the agent's output goes to `~/.chan/devserver/devserver.log`, which the foreground supervisor follows.

Manage the agent with `launchctl`:

```sh
launchctl print gui/$(id -u)/app.chan.devserver        # status (state = running, pid, last exit code)
launchctl kickstart -k gui/$(id -u)/app.chan.devserver # restart
launchctl bootout gui/$(id -u)/app.chan.devserver      # stop and unload
tail -f ~/.chan/devserver/devserver.log                # follow its log
```

## Reach it from Chan Desktop at localhost

Chan Desktop attaches to a devserver at a host and port (New, then Devserver). The convenient case is `localhost`: when the Linux devserver runs in a host-network VM or container on your Mac, a devserver bound to `127.0.0.1` inside it surfaces on the Mac's own `localhost`, so the desktop reaches it with no public bind and no port juggling. On macOS you can also run the devserver natively with `--launchd` and attach at `localhost:PORT` directly — no VM needed.

| Case                  | Mac localhost | Extra setup on the box        |
|-----------------------|---------------|-------------------------------|
| lima VM               | automatic     | none                          |
| sdme container (host) | automatic     | systemd PAM packages, linger  |
| ssh -L (remote box)   | via forward   | ssh, a running devserver      |

To confirm reachability from a Mac shell, the unauthenticated health route answers 200 when the devserver is up (the other `/api/devserver/*` routes need the bearer token):

```sh
curl -sS http://localhost:PORT/api/devserver/info
```

### In a lima VM

lima runs the Linux VM and forwards the guest's `127.0.0.1:PORT` listener out to the Mac's `localhost:PORT`, so a devserver bound to loopback inside the VM is reachable from the Mac directly. The standard cloud image already has the systemd user-session pieces and lingering on, so no extra setup is needed.

```sh
# Put the chan binary in the VM (or use a shared $HOME mount where it already is):
limactl cp ./chan default:/usr/local/bin/chan
limactl shell default

# Inside the VM:
chan devserver --systemd --port 8791

# From the Mac (still reachable after you exit the VM shell; the service stays up):
curl -sS http://localhost:8791/api/devserver/info
```

Then in Chan Desktop: New, then Devserver, host `localhost`, port `8791`.

### In an sdme container

An sdme container on the VM's default (host) network shares the VM's network namespace, so a devserver bound to `127.0.0.1:PORT` inside the container rides the same lima auto-forward out to the Mac. Unlike the lima cloud image, the minimal container rootfs lacks the systemd user-session pieces, so install them once and enable lingering for the user that runs the devserver (skip this and the devserver fails at `systemctl --user`):

```sh
# In the lima VM:
sudo sdme create chan-dev -r ubuntu --started
sudo sdme cp ./chan chan-dev:/usr/local/bin/chan
sudo sdme exec chan-dev chmod 0755 /usr/local/bin/chan

# One-time prereqs the minimal rootfs needs (the lima cloud image already has them):
sudo sdme exec chan-dev bash -c 'apt-get update && apt-get install -y libpam-systemd dbus-user-session'
sudo sdme exec chan-dev useradd -m -s /bin/bash chuser
sudo sdme exec chan-dev loginctl enable-linger chuser

# Run the devserver as that non-root user with its session environment:
sudo sdme exec chan-dev bash -c '
  U=$(id -u chuser)
  runuser -u chuser -- env HOME=/home/chuser USER=chuser \
    XDG_RUNTIME_DIR=/run/user/$U DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$U/bus \
    chan devserver --systemd --port 8792'

# From the Mac:
curl -sS http://localhost:8792/api/devserver/info
```

Then attach from Chan Desktop at `localhost:8792` as above.

## A remote box: forward the port with ssh

When the devserver is not on a host-network VM or container (a box on your LAN, a VPS, or a container that publishes a port), reach it by forwarding its loopback port to your Mac's localhost. Chan Desktop's devserver form has a connect-script field for this; the recommended one-liner forwards the port and starts (or reattaches to) the devserver on the box in a single command:

```sh
ssh [user@]HOST -L PORT:127.0.0.1:PORT "chan devserver --port=PORT --systemd"
```

The quoted `chan devserver ...` is the command ssh runs on the box (ssh's positional remote command). `--systemd` starts the service on the first connect and reattaches to it on later ones, and ssh holds the `-L` forward open while it runs, so the desktop attaches at `localhost:PORT`. A scripted connect that fails keeps its terminal open and offers retry, edit, or abandon. See [Chan Desktop](desktop.md) for the attach lifecycle.

If a devserver is already running on the box, a plain forward is enough:

```sh
ssh -f -N -L 8893:localhost:8791 user@host
curl -sS http://localhost:8893/api/devserver/info
```

## Notes

- The bearer token persists at `~/.chan/devserver/config.json` (mode 0600) and is reused across restarts.
- In a lima VM, the `journalctl --user` follow may warn about insufficient permissions (a lima user-group quirk) and the launcher may briefly say the service is no longer active; the service itself stays up and serving.

## Security

The devserver logs its bearer token at startup, so under `--systemd` the token lands in the persistent journal (readable by root and by members of the `systemd-journal` and `adm` groups) and under `--launchd` in `~/.chan/devserver/devserver.log`. Run a devserver only on a trusted, single-user box, keep it bound to loopback, and reach it over `localhost` or `ssh -L` rather than a public bind.
