# Make `chan open` Route Correctly When Several Local Instances Are Running

Status: accepted scope for v0.74.0. A confirmed defect, traced from a user report of `chan open` reaching the wrong devserver, plus the routing gap the same report exposed when chan-desktop and a devserver run side by side. Every finding below is established by code trace and a host permission check; none of it has been reproduced live, and the live repro is acceptance item 11.

## Problem

A user who runs more than one chan instance on a box cannot predict which one `chan open` will reach, and the message it prints when it goes wrong blames the wrong component.

### The reported symptom, and what it actually is

The report was: another user's devserver holds 8787, my devserver holds 9999, and `chan open` lands on 8787.

Cross-user capture is NOT what happens, and this is recorded as a negative result so the next reader does not re-investigate it. `well_known_devserver_socket_path` (`crates/chan-server/src/devserver_handoff.rs:128-165`) resolves `$XDG_RUNTIME_DIR/chan-devserver.sock` when that variable is set, and `<tmp>/chan-devserver-<uid>.sock` otherwise. On a systemd Linux box the runtime dir is `/run/user/<uid>` at mode 0700 owned by that uid, so no other user can create or read a socket in it; verified on the reporting host, where `/run/user/1001` is `drwx------ fiorix fiorix`. The no-XDG arm carries the uid in the filename, so two users cannot collide in a shared `/tmp` either. Both arms hold. The same reasoning covers the desktop handoff's twin path (`crates/chan-server/src/handoff.rs:211-250`).

What produces the reported symptom is the fall-through. When no devserver of yours is reachable, `chan open` binds a standalone server on `DEFAULT_PORT` 8787 (`crates/chan/src/lib.rs:98`, `crates/chan/src/lib.rs:2850-2862`). If anything else already holds that port -- including another user's devserver, which defaults to the same number -- the bind fails `AddrInUse` and `devserver_port_collision_hint` (`crates/chan/src/lib.rs:2871-2887`) prints that the port is "most likely held by a running `chan devserver`" and that "the handoff did not mount this workspace there". That wording asserts the holder is YOUR devserver and that YOUR handoff failed. Neither is true in the cross-user case, and the user reasonably reads it as chan having chosen 8787 over their own 9999.

### The real defect: one discovery socket per uid

Every devserver a single user runs resolves the same socket filename. The path carries no port, no `CHAN_HOME` and no instance discriminator (`crates/chan-server/src/devserver_handoff.rs:128-165`), and `start_listener` takes it by force: an unconditional `std::fs::remove_file(&socket_path)` followed by `UnixListener::bind` (`crates/chan-server/src/devserver_handoff.rs:242-243`). The comment there explains the unlink as stale-socket cleanup, which is correct for the single-devserver world it was written for and wrong for a second live one.

So the second devserver you start silently steals discovery from the first. `chan open` then registers with whichever instance last won that race, on whatever port that instance happens to hold. There is no error, no warning, and no way to tell from the CLI which devserver answered: on success it prints only `chan: registered <path> with the local devserver` (`crates/chan/src/lib.rs:2760-2766`).

It degrades further on exit. `ListenerHandle::drop` unlinks the shared path (`crates/chan-server/src/devserver_handoff.rs:216`), so when the thief stops, the first devserver is left listening on an unlinked inode and the socket file is gone. Discovery is then dead for both, and `try_register_devserver` short-circuits on the missing file (`crates/chan-server/src/devserver_handoff.rs:419-422`) so every subsequent `chan open` falls through to a standalone bind on 8787 -- straight into the collision hint above.

`CHAN_HOME` does not help. It scopes the devserver's whole state, including its config and token at `<CHAN_HOME>/devserver/config.json` (`crates/chan-server/src/devserver.rs:189-198`, `crates/chan-workspace/src/paths.rs:28-58`), but not its discovery socket. That asymmetry is why the standing workaround for isolated test servers is `CHAN_NO_DEVSERVER_HANDOFF=1` rather than simply a distinct `CHAN_HOME`.

There is also no explicit escape hatch: `--devserver` is a boolean that means "force the registration" (`crates/chan/src/lib.rs:533-542`), never "register with the one on this port".

### The second defect: a desktop and a devserver together

Reported independently on macOS and on Windows: with chan-desktop running and a devserver running, `chan` misbehaves.

`decide_open_route` (`crates/chan/src/lib.rs:2451-2491`) resolves the target from explicit flags, then the shell's parentage read off `$CHAN_CONTROL_SOCKET`, then `forced_desktop`. From a plain shell -- macOS Terminal, Windows PowerShell, any terminal chan did not spawn -- parentage is `None` and the decision collapses to a static `forced_desktop ? Desktop : Standalone` (`crates/chan/src/lib.rs:2483-2489`). Nothing in that arm probes what is actually live.

The consequences split by platform. On Windows the bundle's console `chan.exe` is `Personality::Desktop`, so `forced_desktop` is true (`crates/chan/src/lib.rs:2703-2704`) and `chan open` always hands off to the desktop, ignoring a running devserver entirely. On macOS and Linux the plain `chan` binary makes `forced_desktop` false, so `chan open` always binds standalone on 8787 and collides with the user's own devserver, producing the misleading hint again. In neither arm is `chan-devserver.sock` consulted.

The routing only works today when the shell was spawned BY one of the two instances, because that is the only case where parentage answers.

### The primitive already exists in-tree

The devserver handoff predates two mechanisms that solve exactly its problem, and uses neither.

`control_socket::start_stable` and `take_stable_lock` (`crates/chan-server/src/control_socket.rs:672-740`) are the "take this path only when no live server owns it" discipline: a flock on a `.lock` sibling held for the handle's lifetime, so a dead server's node is reclaimed and a live server's socket is never clobbered. Its own doc comment states the contrast explicitly -- the plain bind unlinks unconditionally, which is only safe for pid-unique paths -- and the devserver discovery path is precisely a non-pid-unique path doing the unsafe thing.

`stable_control_socket_candidates` and `socket_identity` (`crates/chan/src/lib.rs:2163-2210`), driven by `control_socket_for_pid_in_dirs` (`crates/chan/src/lib.rs:2090-2124`), are the "enumerate candidate sockets in the runtime dir, ask each one who it is, match" discovery pass. The devserver handoff has no `Identify` verb at all; its protocol carries a single `RegisterWorkspace` request (`crates/chan-server/src/devserver_handoff.rs:51-64`).

Interim workaround for users on a shipped build: run one devserver per box, or set `CHAN_NO_DEVSERVER_HANDOFF=1` and pass an explicit `--port` to keep a second instance out of the way. If `chan open` reports a bind collision on 8787, do not trust its claim about which devserver holds the port; check with `ss -ltnp` or the equivalent.

## Desired contract

Nine changes. Items 1 to 4 are the confirmed defect, 5 to 7 are the routing and diagnosis the same report exposed, 8 and 9 are the hardening that makes the module's stated same-user property true rather than assumed.

1. **Per-instance discovery sockets.** The single well-known path becomes a per-user DIRECTORY of instance sockets: `$XDG_RUNTIME_DIR/chan-devserver/<16 hex>.sock`, else `<tmp>/chan-devserver-<uid>/<16 hex>.sock`. The hash folds the devserver's stable library id and its bound port, built with the same FNV-1a construction as `stable_socket_name` (`crates/chan-server/src/control_socket.rs:300-316`) and for the same reason: the name must be stable across chan builds so a restarted devserver rebinds its own path, and a user-editable identity must never reach the filename verbatim. The directory arm keeps the macOS `sun_path` 104-byte budget the current naming was designed around.

2. **Never clobber a live owner.** Bind through the existing `take_stable_lock` flock (`crates/chan-server/src/control_socket.rs:709-740`) rather than `remove_file` + bind, and make `Drop` unlink only a path this process actually owns. A stale node from a dead devserver is still reclaimed; a live one is not.

3. **An `Identify` verb on the handoff protocol.** Add a request and response carrying `{pid, library_root, port, version}`, and bump `PROTOCOL_VERSION` (`crates/chan-server/src/devserver_handoff.rs:41`). Discovery enumerates the instance directory and probes each candidate, following the shape of `control_socket_for_pid_in_dirs` (`crates/chan/src/lib.rs:2090-2124`) including its bounded probe timeout, so one wedged devserver cannot hang `chan open`.

4. **Deterministic selection, never a guess.** With no explicit target: zero live candidates falls through to standalone exactly as today; one candidate registers exactly as today; several candidates prefer the one whose library root equals this CLI's resolved `CHAN_HOME`, and if that matches none or more than one, `chan open` REFUSES and lists the live candidates by port, library root and version alongside the flag that disambiguates. Refusing is the point: a wrong guess mounts a workspace on a server the user did not mean, and the flock then makes that hard to undo.

5. **An explicit target selector.** `chan open --devserver[=<port|url>]`, widening today's boolean (`crates/chan/src/lib.rs:533-542`). A bare `--devserver` keeps its current meaning of forcing the registration. A value names the instance, and naming an instance that is not live is a clean refusal, not a silent standalone fall-through -- the user asked for a specific server. The nested-devserver refusal in `decide_open_route` is unchanged.

6. **Live-instance-aware routing.** The `Parentage::None` arm of `decide_open_route` (`crates/chan/src/lib.rs:2483-2489`) takes the set of live local instances as an input and routes on it: exactly one live instance wins regardless of personality, and with both a desktop and one or more devservers live, `forced_desktop` still elects the desktop -- the Windows console `chan.exe` contract must not change silently -- but the handoff note names the devserver that was not chosen and the flag that would choose it. `decide_open_route` stays PURE, as its header already promises: the liveness probe is resolved by the caller and passed in, exactly as `parentage` is today.

7. **Honest collision diagnosis.** `devserver_port_collision_hint` (`crates/chan/src/lib.rs:2871-2887`) stops asserting that the holder is your devserver and that your handoff failed. With the discovery result now in hand, the CLI can distinguish "a devserver of yours is live but this workspace was not mounted there" from "this port is held by something that is not a devserver of yours, possibly another user's", and say which.

8. **Enforce same-user rather than assume it.** A peer-credential check on accept in `start_listener` (`SO_PEERCRED` on Linux, `getpeereid` / `LOCAL_PEERCRED` on macOS), refusing a peer whose uid differs from the process's own. Defense in depth behind the directory permissions, and it makes the module header's same-user claim (`crates/chan-server/src/devserver_handoff.rs:13-16`) true by construction rather than by an assumption about how the runtime dir is provisioned.

9. **Desktop handoff parity.** Apply 2 and 8 to `crates/chan-server/src/handoff.rs` (the unconditional unlink at `crates/chan-server/src/handoff.rs:398`, and its accept loop). Its path stays one-per-uid: a desktop is a singleton per GUI session, so it needs the live-owner lock and the peer check, not per-instance naming.

## Acceptance

Every check names the mutation that must turn it red; the standing rule is that a new check proven only green is not a check.

1. Two same-user devservers on different ports and different `CHAN_HOME`s each bind their own discovery socket, and both remain reachable while the other runs. Extend the integration coverage in `crates/chan/tests/devserver_resilience.rs`, whose sandbox already isolates `CHAN_HOME` and the per-uid discovery socket. Red mutation: restore the single well-known path and the test must fail on a collision.

2. A second devserver cannot take a live one's socket, and a stale node from a dead one is still reclaimed. Red mutation: replace the `take_stable_lock` bind with `remove_file` + bind; the steal case must fail. Companion red mutation for the reclaim half: make the lock unconditional so a dead owner's path is never reused.

3. Stopping one of two live devservers leaves the other's discovery socket intact and still registering. Red mutation: unlink a path this process does not own in `Drop` and the test must fail.

4. Selection, unit level on the pure resolver: two live candidates with a `CHAN_HOME` match registers with the match; two live with no match refuses and names both; one live registers; zero live returns the standalone fall-through. Red mutations: drop the `CHAN_HOME` preference, and separately turn the refusal into a first-candidate pick. Each must fail its case.

5. `--devserver=<port>` registers with that instance while another is live, and refuses cleanly when nothing is listening there. Red mutation: ignore the value and fall back to discovery.

6. Peer check: a connection whose reported uid differs is refused before dispatch. Inject the uid resolver so this runs without a second account on the gate host. Red mutation: accept any uid.

7. Route arbitration, unit level on `decide_open_route`, covering desktop-only, devserver-only, both, and neither, for both personalities and for each parentage. Red mutation: revert the `Parentage::None` arm to the static `forced_desktop ? Desktop : Standalone` form.

8. The collision hint no longer claims your handoff failed when no devserver of yours is live, and still says so when one is. Red mutation: restore the current unconditional wording.

9. Wire skew in both directions: an old CLI against a new devserver and a new CLI against an old one each resolve to a clean documented fallback, not a hang and not a misparse. Extends `dispatch_rejects_protocol_skew` (`crates/chan-server/src/devserver_handoff.rs:601`). Red mutation: drop the version gate in `dispatch`.

10. Whole-repo gate: `make pre-push`, run on a detached worktree rather than a shared tree.

11. Live proof, owner-only, on real hardware. On macOS and on Windows, run chan-desktop and a devserver at the same time and confirm a plain-shell `chan open` does the documented thing in each case, including the note naming the devserver that was not chosen. Then, on one box, run two devservers on different ports and confirm the selection and refusal behaviour. This cannot be run here: the development host has no display server, no macOS and no Windows, so neither the desktop handoff nor the packaged console `chan.exe` personality is reproducible locally. The tests above prove the resolvers, not the platform behaviour.

## Boundaries

Do not change `CHAN_NO_DEVSERVER_HANDOFF` or `CHAN_NO_DESKTOP_HANDOFF` semantics (`crates/chan-server/src/devserver_handoff.rs:176-181`). Isolated test servers depend on them and must keep working unchanged; this item removes the need to reach for them, it does not retire them.

Do not change `DEFAULT_PORT` or `resolve_devserver_port` (`crates/chan/src/lib.rs:98`, `crates/chan/src/lib.rs:3530-3536`). The equality between `chan open`'s default and `chan devserver`'s is load-bearing for the collision hint and is documented user surface; item 7 fixes the message, not the number.

Do not touch the per-tenant control sockets or their discovery (`crates/chan/src/lib.rs:2015-2249`, `crates/chan-server/src/control_socket.rs`). They are cited here as the pattern to reuse, not as a repair target, and the `$CHAN_CONTROL_SOCKET` baked into already-open shells must keep resolving exactly as it does.

Do not add cross-user devserver sharing. The contract stays same-user. This item makes that boundary explicit and enforced; it does not widen it, and a request to reach another user's devserver is separate work with its own security review.

Do not fold the desktop and devserver handoff modules into one abstraction. They stay two modules with two independent protocols and two version constants; only the bind discipline and the peer check are shared, and each is a small helper rather than a merge.

Do not make the devserver refuse to start when another one is already running. Running several is the point of this item; only discovery has to become unambiguous.
