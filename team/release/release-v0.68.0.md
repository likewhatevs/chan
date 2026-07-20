# Release v0.68.0

Delivery round run 2026-07-14..15: four implementation lanes plus an integrator on one shared worktree (branch `0.68.0-rc1`), with a mid-round host smoke and a second wave of host-reported items. Coordination artifacts live in the untracked `dev/v0.68.0/` tree of the round host's checkout.

## Scope

Eleven items. Gateway: multiple devservers per account (`{user}--{disc}` hosts, `?d=` share selector, per-user cap env), a consent-page devserver picker recorded by chan-desktop, a one-time redemption code replacing the PAT secret in the desktop sign-in callback (BREAKING for pre-0.68 desktops), and `chan-gateway-admin token create` over a new identity operator surface. Workspace app: Export to PDF (Inspector action + `cs export` over a server-brokered window-bus job, client-side raster engine), live-collaborative Excalidraw boards on a new scene-session authority, a pane-menu Close pane row, tag pills excluded from link labels, and a Hide window command. Desktop: per-window-kind native menus off macOS with label-addressed menu routing (fixes a Wayland focus-steal misroute). Server: doc/scene session reconcilers no longer trust mtime-only echo detection or single uncorroborated reads (data-loss fix for cloud-FUSE-mounted workspaces). CI: the PPA publish path is retry-idempotent (Launchpad accepted-check + bounded dput retries, optional sftp transport).

## Branch And Commits

`0.68.0-rc1` cut from v0.67.3 (`58616cf8`); 28 commits to the rc tip `a7adadba`; rc pins at `c1af36ef`. The GA commit strips the rc pins, dates the changelog, pins the fedora specs, and adds this document.

## Validation

Every commit was own-gated by its lane and independently re-verified by the integrator from an isolated worktree as it landed. Committed e2e harnesses under `scripts/e2e/` carry the load-bearing checks: `gateway-zone.sh` (28 asserts against real identity/profile/devserver-proxy services and real `chan devserver` tunnels, including a headless-Chrome consent flow over a stub OAuth endpoint) and `browser-smoke/` (real server + headless Chrome: pane menu, PDF export via both surfaces with byte-level page assertions including a boundary-duplication detector, two-client Excalidraw convergence, editor collab regression). Final composite gate on the tip: full `make pre-push` green, desktop scope 199/199, gateway-zone 28/28, browser-smoke all green. The `release.yml publish=false` dry run passed on all platforms including macOS sign/notarize (one known-flaky timing test passed on re-run).

The harnesses caught three real bugs pre-ship (fresh-out upload rejection in the export job, an appState echo loop between live Excalidraw clients, PDF page-boundary duplication on long documents); the host smoke caught the pagination bug's severity and two UX items that shipped in the same cycle.

## Release Workflow

rc validation via `publish=false` dispatch (run 29363056622). GA is the standard tag-push publish. The downstream publication workflow runs the new skip/retry logic live for the first time at this GA (its pre-GA dispatch validation is structurally impossible: a tag dispatch builds the tag's tree); `LAUNCHPAD_SSH_PRIVATE_KEY` is provisioned, so PPA uploads ride sftp.

## Operators

See the CHANGELOG Operators section: pre-deploy username check (`--` now reserved), `MAX_DEVSERVERS_PER_USER` rename and default flip (unset now 100), optional `IDENTITY_ADMIN_TOKEN`, desktops older than 0.68 must upgrade to sign in.

## Known Limitations

PDF export is raster (text not selectable); heading-orphan pagination can emit a near-empty page before an oversized block. Excalidraw boards ride the 2 MiB scene cap; image-heavy boards need the planned asset side-channel. The Hide window command is desktop-only (the bury IPC has no web analogue; blocker documented in the round journals). Live-session fold-ins of genuine external edits into a dirty session still discard unflushed edits by design (convergence semantics; a preserve-local variant is an open design question). PAT revocation drops all of a user's tunnels (non-revoked devservers reconnect).
