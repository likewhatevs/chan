# Prod rollout handoff — devserver-proxy migration (v0.42.0)

Orientation for the agent (or @@Alex) rolling the gateway devserver-proxy migration to production. The rollout is GATED on the v0.42.0 release.

## Prerequisite: v0.42.0 is cut
The prod `.deb`s are built from the released `chan` source, so before rolling out:
- v0.42.0 is tagged/released (or the `gateway-devserver-proxy` branch is merged to `main`), so the build pulls the renamed `devserver-proxy` crate, not `workspace-proxy`.
- Confirm at the build source `$CHAN_SRC`:
  ```
  ls   $CHAN_SRC/gateway/crates/devserver-proxy/Cargo.toml   # must exist
  grep -R 'name = "workspace-proxy"' $CHAN_SRC/gateway       # must be empty
  ```
  If `workspace-proxy` is still present, STOP — the code migration has not landed and the cutover would build the old binary under the new name.

## The two reference docs
1. **`gateway/docs/cutover-runbook.md`** — the OPS sequence @@Alex owns: DNS/cert, the nginx `server_name` + `grpc_pass` swap (+ the confirmed `client_body_timeout 1d` / `client_header_timeout 1d` 60s-flap fix), the renamed deb, the feature flags (`oauth_login` + `share_workspaces`, Step 7b), the prod GitHub OAuth app (Step 7c), the §7.3 staging smoke, ship.
2. **`gateway/docs/prod-cutover-devserver-proxy.md`** — the chan-prod-setup file-by-file rename (services `.sdme`, nginx vhosts, DNS, cert SANs, `build-debs`, `secrets-init`, `deploy`), the residual-sweep verify, and rollback.

## Order
1. Cut v0.42.0 (prereq above).
2. Run the §7.3 **staging smoke** (cutover-runbook Step 7 / `staging-smoke-runbook`) as the pre-prod gate — it is what caught the tenant mount-404 and the 60s tunnel flap.
3. Execute the chan-prod-setup cutover (`prod-cutover-devserver-proxy.md`).
4. Enable the feature flags + prod OAuth app (cutover-runbook Steps 7b/7c). Without `oauth_login` + `share_workspaces` the dashboard sign-in + Devservers tab stay hidden.
5. Verify (both docs' verify sections); retire the old `workspace.chan.app` surface.

## Go / no-go
- HARD cut, pre-release, fresh state: no back-compat, no dual-run beyond the verify window. Rollback is config-only (`git revert` the cutover commit + rebuild + redeploy + re-point DNS) — no data migration either direction.
- The desktop-side **devserver = chan-library** work ships in the same v0.42.0 but is client-side (chan-desktop ⇄ `chan devserver`); it does NOT gate the gateway prod rollout and has its own smoke (`dev/devserver-chan-library/chan-desktop-smoke-procedure.md`).
