# task-Lead-ChanGateway-4 — sweep fixup + second-pass review of the desktop lane

From: @@Lead. To: @@ChanGateway. Your task-1 (+2 addendum) is
ACCEPTED — commit 5c44bf00 verified, my independent rg sweep confirms
your surfaces clean except the fixup below. Trail note: your addendum
cites commit 8f1aef62 for the build-gateway.sh fix, but main has it
as 26f72350 — append a one-line correction to your journal so the
audit trail resolves.

## 1. Fixup: three .service files my no-filter rg caught

All three systemd packaging units still point Documentation at the
dead org (these ship inside the deb/rpm artifacts):

- gateway/crates/profile/packaging/chan-gateway-profile.service:3
- gateway/crates/identity/packaging/chan-gateway-identity.service:3
- gateway/crates/workspace-proxy/packaging/chan-gateway-workspace-proxy.service:3

`Documentation=https://github.com/chan-writer/chan-gateway` →
`https://github.com/fiorix/chan` (the canonical repo; the gateway
lives under gateway/ there). Lesson per the round-plan addendum:
packaging/config files hide from file-type-filtered sweeps — same
trap that bit my *.md-filtered docs sweep.

## 2. Second-pass review: @@ChanDesktop's landed tidy commits

Adversarial read of ad6d5c2c (scrub + hygiene) and e8b4356a
(design.md rewrite) — fresh eyes from outside the lane:

- Behavior preservation: the WindowSpec param-struct refactor (5 call
  sites) and the unbury_or_restore dedup — check arg order/defaults
  didn't silently swap at any call site, and the three spawn paths
  (local/tunnel/outbound) still differ only where they should.
- Comment rewrites: spot-check that rewritten "constraint" comments
  state constraints that are actually still true in the code.
- design.md vs source: pick 3-4 load-bearing claims from the new
  desktop/design.md (window label scheme, bury/restore LRU,
  standalone-terminal control socket, remote-window reopen) and
  verify each against the source. You have fresh cross-workspace
  context from your own design.md pass.
- You are review-only in desktop/**: report findings, don't edit.
  Real findings → completion file to me; I route fixes.

## Context

Postgres container + ssh bridge: leave them UP as you offered — my
isolated integrated gate will want gateway tests; I'll tear down at
round close. gateway/package.json 0.0.0: noted as a release-cut item
(my ledger), no action for you.

Completion: task-ChanGateway-Lead-N.md + poke, as usual.
