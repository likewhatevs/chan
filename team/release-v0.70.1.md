# Release v0.70.1

Patch round run 2026-07-17 off the v0.70.0 tag: nine items implemented by orchestrated agent waves on one shared worktree (branch `v0701-fixes`), verified headlessly, and shipped without an rc cycle by the owner's call. Coordination artifacts live in the untracked `dev/v0.70.1/` tree of the round host's checkout.

## Scope

All nine planned items. Tunneled (gateway) devservers: multipart uploads and file replaces mirror the gateway CSRF cookie (fixes the 403 on `cs upload`, drag-drop, and the export write-back); `cs export` verified end to end locally and through the tunnel, with its renderer-requirement errors reworded; window close for gateway-rostered devservers resolves the owning connection through the window feed instead of the persisted config (the discard was never sent, so the feed reopened every closed window), with bury-before-destroy and a launcher notice on failure; rostered connects seed the OS logo from the devserver's self-report. Naming: `--tunnel-devserver-name` with hostname default rides an additive tunnel-hello field (no protocol bump), persists through the identity validate exchange with owner-scoped `-2`/`-3` dedup. Desktop: gateways are renamable from the Gateways screen (label only, URL immutable). Infra: the sdme recipe installs `adduser`; tunnel-mode devservers under systemd bind an OS-assigned port by default with loud named-address bind failures; the `cs` symlink parses through chan-shell's own parser so help renders `cs <cmd>`.

## Branch And Commits

`v0701-fixes` cut from v0.70.0 (`e70b4d4c`); 11 commits to the tip: nine task commits, one export-wording commit, one review-hardening commit (tunnel-name control-char/percent escaping, gateway-side name sanitization, label-dedup advisory lock, sweeper doc refresh). The GA commit bumps the pins straight to 0.70.1, dates the CHANGELOG, pins the fedora specs, and adds this document.

## Validation

Implementation ran as three waves with disjoint file ownership, each commit own-gated (fmt after last edit, per-crate clippy `-D warnings`, focused tests, svelte-check plus package vitest for web). The T3 root cause was proven by live repro before the fix: a gateway-zone stack exonerated the CSRF and role guards (mirrored DELETE answers 204) and pinned the persisted-config iteration as the failing leg.

Verification: gateway-zone e2e all scenarios green, 72/72 assertions, including the new `upload` and `windowclose` scenarios and the same-name dedup poll; a fresh sdme rootfs built from the fixed recipe provisioned to completion, and the worktree binary swapped into its flagless systemd unit journaled `binding 127.0.0.1:0` with a different assigned port across restarts; the `cs export` matrix passed all four legs (local, tunneled, headless refusal, standalone-terminal refusal) with the new wording verified live. A five-lens adversarial review (correctness, frozen-contract audit, security, regressions, tests/hygiene; every finding re-traced by two independent refuters) confirmed four fixable findings, all fixed and re-gated in one commit; one pre-existing out-of-scope gap is recorded in the round journal (the Window-menu reopen poll skips rostered devservers). Full `make pre-push` green on the tip, both workspaces, with gateway cargo tests against Postgres.

## Release Workflow

No rc pin cycle for this patch. GA validation via a `release.yml publish=false` dispatch on the GA-pinned branch (the only macOS compile, sign, and notarize signal), artifacts checked before the tag; GA is the standard tag-push publish, distros-publish verified after.

## Operators

See the CHANGELOG Operators section: a name-announcing redial recreates a swept registry row labeled; owner-scoped label dedup is serialized and announced names are sanitized server-side; flagless tunnel units bind an OS-assigned port and the management verbs resolve the running port; gateway-zone grows `upload` and `windowclose` scenarios.
