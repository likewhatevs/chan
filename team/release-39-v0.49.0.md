# Phase 39 — v0.49.0: UI responsiveness, desktop cosmetics, tunnel e2e, container packaging

Round 2026-06-24. Team of 6, disjoint trees, single cut on @@Alex's go: @@Lead
(architect/gate/release), @@Workspace (`crates/`), @@Launcher (`web-launcher/`),
@@Desktop (`desktop/src-tauri/`), @@Gateway (`gateway/` + tunnel), @@Packaging (new
`docker/` + `kube/`). Five work areas from `dev/request.md`, cut as a test release
`v0.49.0-rc1` (macOS / Linux / Windows; RC artifacts on GitHub, not chan.app).

## Theme

Make the chan-launcher UI track what the backend is actually doing, fix three desktop
cosmetics/ACL papercuts, prove the `chan devserver --tunnel-url` path end to end against
a real proxy, and produce the first container-distribution artifacts (Dockerfiles +
kube yaml). The round's real story was environment: a bare host with no rust, node, or
GUI toolchain, validated through sdme.

## What landed (by lane / commit)

- **@@Workspace — lifecycle status + idempotent turn-on** (`2d465f1b`): a
  `WorkspaceStatus` (`stopped|starting|running|error` + `error`) on `LauncherWorkspace`
  and a `DevserverStatus` (`disconnected|connecting|connected`) replacing
  `DevserverEntry.connected`. A transient `mount_state` overlay marks `starting`/`error`
  inside the SHARED inner mount path, so every entry point (the `open_or_get` wrapper,
  the desktop boot-restore, the devserver `mount_at`) reports it with no per-caller
  routing, and the window-watch feed fires on mount START and FAILURE too. The
  false-`WorkspaceLocked` fix: a contended flock whose record names our own pid now
  returns `WorkspaceAlreadyOpen`, so `WorkspaceLocked` fires ONLY for a genuinely-foreign
  live holder.
- **@@Launcher — status-driven spinners** (`223fa9c3`): the on/off + connect spinners
  read the backend `status` instead of the blind 45s optimistic timer (localStorage
  marker demoted to a short in-memory click->refetch bridge). `starting` spins + disables
  the toggle (kills the click-races-own-mount path), `error` clears the spinner + shows
  the reason, a devserver `disconnected` clears the connect spinner with no manual reload.
- **@@Desktop — cosmetics + ACL + reload** (`4d627108`): a custom CENTERED "Window
  Hidden" webview notice replacing the native left-aligned alert; the home glyph on every
  local window title (dropped the desktop-icon fallback); `pick_upload_files` granted to
  tunnel (`lib-*`) windows so `cs upload` over a tunnel opens the picker (`outbound-*`
  stays denied); `cmd+r`/`ctrl+r` reload on the launcher window. Plus the devserver
  `status` producer in its `DevserverRegistry::list()`, the boot-restore mount funneled
  through the idempotent wrapper, and the dead `list_devservers`/`DevserverView` command
  removed.
- **@@Gateway — cross-container tunnel e2e** (`c098c4b3`): a real `devserver-proxy` +
  real `chan devserver --tunnel-url` in separate containers, a 200 routed end to end
  through the h2c tunnel (`gateway/scripts/dev/sdme/devserver-tunnel-e2e/`). No
  tunnel-path bug surfaced.
- **@@Packaging — container distribution** (`56cc8bbc`): multi-stage Dockerfiles for
  `chan` and the three gateway services (identity, profile, devserver-proxy) running the
  real `make` recipes, kube yaml (Deployments/Services/ConfigMap/Secret + Postgres + an
  sdme single-pod variant), and a headless-Chrome browser-upload test harness, under new
  `docker/` + `kube/` trees.

## Cross-lane contract

@@Workspace published the workspace/devserver `status` enum + transition-event guarantee
in `dev/v0.49.0/team/journals/journal-Workspace.md` BEFORE either side built; @@Launcher
consumed it. The only wire change was two new fields on the two list rows — the
watch->refetch loop was untouched, which kept the launcher change minimal. The one
coupling that crossed lanes was `DevserverEntry`'s field flip (chan-library type, one
impl in chan-desktop's `config.rs`); @@Lead sequenced it as a lockstep so the shared tree
stayed coherent.

## Highlights

- **The sdme compile-verify earned its keep.** chan-desktop cannot compile on this host
  (no GTK/webkit), so @@Desktop's static pass could not catch type errors. The sdme
  `cargo clippy -p chan-desktop` run caught two real cross-lane breakages a grep could not
  (a test on the removed `connected` field, and a `LauncherWorkspace` built without the
  new `status`/`error`). Prove, do not eyeball.
- **Publish-the-contract-first held.** The frozen `§STATUS CONTRACT` let @@Launcher build
  the full consumer side against mocks while @@Workspace built the producer, with zero
  rework — "the proposed and frozen shapes were identical."
- **Correct authorization boundaries.** A self-signed/insecure registry to demonstrate
  `sdme kube apply` is an owner decision; the security classifier correctly refused it on
  a peer-survey answer alone and required direct user intent. The round proceeded on the
  no-TLS-bypass path (option 4) for live evidence with the kube-apply demo documented as a
  follow-up.

## Lowlights + lessons

- **The host had no toolchains.** No rust (central `rustup` 1.95.0 install, HOLD'd lanes
  off a concurrent `~/.rustup` race), no node (@@Launcher installed the tarball to
  `~/.local`), no Linux GUI toolchain for chan-desktop (gated in sdme/CI, excluded from
  the bare-host gate per @@Alex's keep-it-in-containers constraint). Each was a round-wide
  stall resolved once, centrally.
- **`sudo sdme` is passwordless here; general `sudo` is not.** @@Packaging's "no
  privileged runner" escalation was a misread of that split; naming it unblocked the
  sdme-based validation for two lanes at once.
- **Lean pokes, re-ratified.** @@Lead drifted into multi-clause pokes that inlined whole
  decisions; @@Alex called it out. Corrected to one-line pointers with the substance in
  the append-only task files.

## Follow-ups (→ next round / @@Alex)

- Production `chan devserver --tunnel-url` e2e against the real chan-gateway (@@Alex runs).
- True TWO-zone tunnel routing (needs host inter-zone `iptables`/forwarding; this round
  proved it one-zone/two-container).
- `sdme kube apply` end-to-end via a trusted local registry (the option-4 runtime proof
  stands; kube-apply image resolution documented, gated on the registry decision).
- Dynamic/handshake-negotiated upload ACL — spec'd (a 30s single-use upload token bound
  to window+dir); not built this round.
- Remote devserver-hosted workspace rows carry `on` only (mapped `Running`/`Stopped`);
  remote `starting`/`error` fidelity needs a deeper pass-through.
- Suppressing a genuinely-foreign-lock thrown error from the launcher banner in favor of
  only the row affordance (needs a foreign-lock wire shape not in the contract).
- Launcher right-click "Reload" shows no `⌘R` label: the launcher falls through to
  WebKit's native context menu (workspace panes show the chord because they render their
  own custom menu). Matching it needs a custom web-launcher context menu; the chord itself
  works. @@Alex to decide whether to add the label. (Smoke note 8.1.)

## The cut

Five lane commits on local `main` (`2d465f1b`, `223fa9c3`, `4d627108`, `56cc8bbc`,
`c098c4b3`), full `make pre-push` from an isolated worktree (chan-desktop excluded on the
bare host, verified in sdme), version bump `0.48.0 → 0.49.0`, then `v0.49.0-rc1` via the
release workflow. RC artifacts land on GitHub; the Windows `.msi` is pulled with `gh`.
