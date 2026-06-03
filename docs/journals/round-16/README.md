# Round 16 - chan-desktop launcher redesign (archive)

The launcher redesign carried over from phase-16 and finished after the
v0.24.0 cut by the new-team-1 four-lane team (@@LaneA lead + @@LaneB/C/D).
The separate [Open workspace] and [Attach] header buttons were merged into
one [New] modal (Local / Remote outbound / Remote inbound); rows became
On|Where with directional icons + a connection dot; the per-row gear was
removed; the header tagline was dropped. Built and committed as fd27d29d.

@@Alex hand-smoked it and left 3 change requests on the smoke checklist
(header icon/[New] order swap, Remote-outbound code-block example,
Remote-inbound copy rewrite). Those follow-ups are carried into phase-17
round-1, not redone here.

## Design + recon docs

- desktop-redesign-design.md    the locked design (the D1-D4 decisions)
- launcher-inventory-LaneC.md   recon of the pre-redesign launcher code
- spa-settings-gap-LaneD.md     gear-removal settings-gap analysis (no gap)
- smoke-checklist-LaneD.md      the WKWebView hand-smoke; carries @@Alex's
                                3 inline round-1 change requests
- canary-reconcile-prep-LaneC.md  serve.rs frontend-canary reconciliation

## Team-work record

- bootstrap.md / re-bootstrap.md   the new-team-1 team process + resume doc
- journals/                        per-lane append-only logs (A/B/C/D)
- tasks/                           the task-{from}-{to}-{n} dispatch trail
