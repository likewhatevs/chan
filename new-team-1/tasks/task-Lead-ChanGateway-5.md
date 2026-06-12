# task-Lead-ChanGateway-5 — second-pass review: @@Chan's landed core commits

From: @@Lead. To: @@ChanGateway. Your task-4 is ACCEPTED (2d13684a
verified; the deep-link .service URL is BETTER than my spec — keep
it; F1/F2 routed to @@ChanDesktop as task-Lead-ChanDesktop-7).
Noting your tests/-dir file-set gap find for the round close: that's
a second sweep trap distinct from the size trap.

Next assignment, same shape as task-4 §2: adversarial review-only
pass over @@Chan's six landed core commits. They ran a
subagent-heavy fan-out, which is exactly where a second pass earns
its keep. Review, don't edit crates/** or web/** — findings come to
me.

Commits (oldest first):

- d7d0a7e0 — chan-shell fixture/help neutralization. Check: help
  text still self-consistent; fixtures semantically identical.
- bb049d6c — chan-server scrub, 31 files. Includes ONE non-comment
  change: a dropped allow(dead_code) on survey_bus — verify the
  symbol really is referenced now (that allow existed for a reason
  once).
- 53fe79d3 — core crates scrub + the reports-help truth fix. Check
  the new `chan add --reports` / Reports help text against
  IndexConfig::default() (reports ON for new workspaces, legacy
  files stay false).
- 01d0cba6 — param refactors: cmd_serve 15-arg tail → ServeArgs,
  control_socket start/handle_request → ControlSocketCtx. The
  task-4-§2 treatment: field-by-field mapping at every call site, no
  swapped/defaulted values, and confirm their "no cross-workspace
  callers" claim with your own rg (they overturned my recon's
  13-call-site count — verify the overturn).
- fbeb5c13 — 8 design.md rewrites + chan-llm README. Sample 3-4
  load-bearing claims per your method (they cite tunnel HelloAck
  enum, graph schema v6, nine StandardTools as corrections — verify
  those three at minimum).
- dc94b16e — chanwriter purge in the chan-workspace crate
  description. Check Cargo.lock/metadata didn't need a matching
  touch.

Their web/ commits and the shortcut normalization are still in
flight — NOT in scope; I'll route that review when their completion
lands.

Completion: task-ChanGateway-Lead-N.md + poke, findings only (no
edits).
