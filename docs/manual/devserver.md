# Devserver

A devserver is one `chan devserver` process that hosts many workspaces behind a single port. On the box, you start a devserver and then register workspaces into it with `chan open PATH`: each `chan open` registers its workspace and exits instead of binding its own port, so one process owns each workspace. Chan Desktop attaches to the devserver and lists its workspaces in their own launcher group. See [workspaces](workspaces.md) for the serve side and [Chan Desktop](desktop.md) for the attach lifecycle.

```sh
chan devserver --bind 127.0.0.1 --port 8787
```

There is no TLS and only a bearer-token gate, so keep the bind on loopback and reach the devserver from another machine over a forwarded localhost port (below).

## Keep it running: `--service`

`chan devserver --service` runs the devserver under the platform service manager instead of in the foreground, so it survives the terminal that launched it. This is what lets you start a devserver on a box, disconnect, and resume your sessions later: the service keeps the workspaces mounted, and re-running `chan devserver --service` re-attaches to the already-running instance rather than starting a second one.

The backend is chosen per OS — a systemd user service on Linux, a launchd LaunchAgent on macOS, a detached background process tracked by a PID file on Windows — but the flags are the same everywhere:

```sh
chan devserver --service                # start (or re-attach), then surface the bearer token
chan devserver --service --stop         # stop the supervised devserver
chan devserver --service --restart      # restart (or start if stopped) on the current --bind/--port
```

`--stop`/`--restart` require `--service`; a plain foreground `chan devserver` is stopped with Ctrl-C.

### Linux: systemd user service

`--service` runs the devserver under a systemd user service named `chan-devserver.service`, which survives the terminal **and** survives logout. On first run it:

- Ensures user lingering is on, so the service runs without an active login session. If lingering is off and chan cannot enable it, it stops with a hint to run it once as root:

  ```sh
  sudo loginctl enable-linger USER
  ```

- Writes `~/.config/systemd/user/chan-devserver.service` (its `ExecStart` runs `chan devserver --bind=... --port=...`), then enables and starts it with `systemctl --user enable --now` and follows its journal.

You can also manage it with the usual user-scoped systemd commands:

```sh
systemctl --user status chan-devserver.service
systemctl --user restart chan-devserver.service
systemctl --user stop chan-devserver.service
journalctl --user -u chan-devserver.service -f
```

### macOS: launchd LaunchAgent

`--service` runs the devserver under a per-user launchd **LaunchAgent** named `app.chan.devserver`. It writes `~/Library/LaunchAgents/app.chan.devserver.plist` (whose `ProgramArguments` run `chan devserver --bind=… --port=…`), loads it into your `gui/$UID` login session with `launchctl bootstrap`, and starts it.

A LaunchAgent outlives the terminal that launched it and your GUI login session, but — unlike systemd's linger — it does **not** survive a full logout (that would need a root LaunchDaemon, out of scope for a per-user dev tool). launchd has no journal, so the agent's output goes to `~/.chan/devserver/devserver.log`.

You can also manage it with `launchctl`:

```sh
launchctl print gui/$(id -u)/app.chan.devserver        # status (state = running, pid, last exit code)
launchctl kickstart -k gui/$(id -u)/app.chan.devserver # restart
launchctl bootout gui/$(id -u)/app.chan.devserver      # stop and unload
tail -f ~/.chan/devserver/devserver.log                # follow its log
```

### Windows: detached background process

`--service` spawns the devserver as a detached background process and records its pid in `~/.chan/devserver/service.json`. Unlike systemd/launchd there is no OS supervisor: the process is **per-login** — it does not survive logout and does not auto-restart on crash — but it does survive the terminal that launched it, which is enough to start a devserver and walk away for the session. Its output goes to `~/.chan/devserver/devserver.log`.

`--service --stop` reads the pid (guarded against pid reuse by the recorded process creation time) and terminates it; `--service --restart` stops and respawns. There is no console signal to a detached process, so `--stop` is a hard terminate — safe because the devserver drains each HTTP request synchronously and releases its per-workspace locks on exit.

## Reach it from Chan Desktop at localhost

Chan Desktop attaches to a devserver at a host and port (New, then Devserver). The convenient case is `localhost`: when the Linux devserver runs in a host-network VM or container on your Mac, a devserver bound to `127.0.0.1` inside it surfaces on the Mac's own `localhost`, so the desktop reaches it with no public bind and no port juggling. On macOS you can also run the devserver natively with `--launchd` and attach at `localhost:PORT` directly — no VM needed.

| Case                  | Mac localhost | Extra setup on the box        |
|-----------------------|---------------|-------------------------------|
| lima VM               | automatic     | none                          |
| ssh -L (remote box)   | via forward   | ssh, a running devserver      |

To confirm reachability from a Mac shell, the unauthenticated health route answers 200 when the devserver is up (the other `/api/devserver/*` routes need the bearer token):

```sh
curl -sS http://localhost:PORT/api/devserver/info
```

### In a lima VM

lima runs the Linux VM and forwards the guest's `127.0.0.1:PORT` listener out to the Mac's `localhost:PORT`, so a devserver bound to loopback inside the VM is reachable from the Mac directly. The standard cloud image already has the systemd user-session pieces and lingering on, so no extra setup is needed.

```sh
# Enter the VM:
limactl shell default

# Inside the VM (make sure $PATH contains ~/.local/bin):
curl https://chan.app/install.sh | bash
chan devserver --systemd --port 8791
```

Then in Chan Desktop: New, then Devserver, host `localhost`, port `8791`.


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
