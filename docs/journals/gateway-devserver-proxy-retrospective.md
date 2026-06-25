# Retrospective — gateway devserver-proxy migration (Track B, branch gateway-devserver-proxy)

Round opened 2026-06-21 off v0.41.0. Team gw: host @@Alex, @@Lead, and workers @@Proxy / @@Profile / @@Client.

## Delivered (done + validated)
- **workspace-proxy -> devserver-proxy** migration: crate/binary/unit/env rename; per-devserver gate (`drv`=devserver_id, `Path=/`); segment-preserving forward; 404 the management API on the wildcard; registry re-keyed on the token-resolved `devserver_id` (hex SHA-256 of the PAT); the whole `public` path dropped (wire + proxy + tunnel.public scope, always-authenticated).
- **profile** reshaped to per-devserver grant + `devserver_access`; **identity** open-routes mint `drv`=devserver_id + produce `devserver_id`; the devserver IS the registered token.
- **chan devserver** gains `--tunnel-url`/`--tunnel-token`; `--tunnel-name`/ `--tunnel-public` dropped; `chan serve --tunnel-*` removed; tenants mount at the public slug.
- **id dashboard SPA** rewired to /api/devservers/* (sharing-only: devserver list + email-grant + the working /s/:owner/:workspace).
- VALIDATION: comprehensive gate green (both workspaces + web + marketing); §7.3 wire verify 7/7 on a live TLS stack; 3b dashboard proven live (devserver_id chain identical across identity/DB/proxy on a real PAT).

## Findings the smoke earned (would not have surfaced from unit tests)
- `/blog/` tenant-root trailing-slash 404 (axum nest gap) -> FIXED (e7bd1da8).
- 60s tunnel flap = nginx `client_body_timeout` default on the long-lived POST, PRE-EXISTING -> one-line fix, in the cutover runbook.
- `--to` never overrode `--tab-name` for [F] followups -> FIXED (ca27b5e3) + the [F]-creates-an-empty-file doc/template note + the --to-host bootstrap guidance.

## Carryover / pending
- Bug fixes (drag-drop 2599a6b9, rich-prompt 423279ab) + the survey/team fixes (d644fd06, ca27b5e3, 14d11ca4): await @@Alex desktop verify -> deliver to main.
- NEXT PHASE: devserver = chan-library (dev/devserver-chan-library/plan.md) - rustacean audit + grill-with-docs + e2e, fresh-config. + --followup-dir default to the workspace .Drafts, with the resolved path printed to the caller. (The host->manager terminology idea was retracted by @@Alex.)
- @@Alex-owned cutover ops (cutover-runbook.md): DNS/cert/nginx (+client_body_timeout)/ chan-prod-setup/deb/oauth_login+share_workspaces flags/prod-OAuth-app/staging-smoke.

## Highlights
- Clean cross-workspace sequencing held: the public-drop straddled both workspaces (gateway path-deps on chan-tunnel-*); the consume-first/field-second ordering avoided a red window. The W1/W5/W6 devserver_id contract, pinned up front, held end-to-end (proven identical across 3 surfaces live).
- The §7.3 smoke + the live hypothesis-confirm paid for themselves: caught the mount 404, root-caused + FIXED the 60s flap, proved the :8443 id is cutover-safe.
- Lanes self-coordinated by the end (peer-to-peer re-verify, escalations with options) and reported honestly (browser-smoke-blocked flags, step-5 unverified correction, staleness-reconcile-not-redo).

## Lowlights / honest feedback
- @@Lead (me): I OVER-POKED during the convergence + 3b phases. Many pokes cited stale HEADs (the fast convergence moved past them), so the lanes spent cycles reconciling "already done" pokes. Lesson (now my discipline): verify HEAD fresh immediately before each poke, and minimize volume - let lanes drain + report + self-coordinate. Also: my first round-close gate had a malformed GW_FMT step (false-red; the virtual gateway workspace needs `cargo fmt --all`).
- @@Proxy: exemplary - F1 foundation, the gate, drove the §7.3 smoke + the hypothesis-confirm, the straggler sweep (caught the release-breaking web-marketing deb-name drift), the :8443 root-cause, the oauth_login hidden-blocker catch. No substantive miss; a couple of crossed reconciliations were downstream of my over-poking, not their fault.
- @@Profile: strong - per-devserver grant/identity, the SPA rewire, the proactive stale-README catch, the sharp "Open devserver" escalation with options, honest 3b corroboration (incl. correcting step-5 to unverified). Minor: the first 3b report over-claimed step-5 PASS; self-corrected.
- @@Client: strong - the critical-path re-key (the unblocker), both bug fixes with a reactivity analysis, the honest browser-smoke-blocked flag (no claimed smoke), and disciplined staleness-reconcile (verified HEAD, never re-did committed work).
- @@Alex (host): the settled grill-with-docs design (ADR-0001) made the build unambiguous; decisive steering (full-bundle, provide-creds) kept momentum; sharp catches (the followup from/to bug, F-empty-file, prod-OAuth-app, the devserver= chan-library vision). Constructive: the optional 3b chase consumed real cycles on GitHub-side friction (app suspension + a callback typo); a quick up-front check that the dev OAuth app was healthy would have streamlined it - but the payoff (the flap + mount findings) justified running the smoke.

## Process notes worth keeping
- The isolated/own-gate model + explicit-pathspec commits kept a busy 3-lane shared worktree clean (no contamination across the convergence).
- dev/ is gitignored: round docs live there as the live bus; promote the persist-worthy ones (ADR, CONTEXT, this retrospective) at the wrap.
