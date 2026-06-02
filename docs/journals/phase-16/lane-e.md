# @@LaneE — Docs + Build/CI + bootstrap

Read `round-1-plan.md` first. D1 is PRIORITY-0 and ships on its own track to
unblock @@Host's designer — do it FIRST. B1 is date-bound. D2/D3 are trivial.
TW2 comes after the cs-terminal patterns settle (@@LaneA).

## Round-1 tasks

1. **D1 — tunnel/gateway messaging reframe (CONTENT ONLY).** Reposition so
   the SELF-HOSTABLE `gateway/` components are the offering: users run their
   OWN "Google Drive/Docs equivalent, with chan's IDE on it" on their own
   infra. The tunnel = CORE chan capability that powers it (desktop attaches
   inbound tunnels from `chan serve --tunnel-url`; also plain h2 to a remote
   `chan serve`). Stop selling the chan-hosted online service as the
   headline; it is experimental + disabled by default (no auto-enrollment).
   Document the admin tooling (oauth config, user enrollment, workspace
   sharing) + DNS wildcard + Let's Encrypt setup (reference @@Host's private
   chan-prod-setup repo — ask @@Lead to get specifics from @@Host via
   survey). Touches `web-marketing/` COPY + `README.md` + `gateway/` docs.
   SCOPE LIMIT: content/messaging/docs ONLY. Do NOT do visual design — that
   is @@Host's designer's separate track.

2. **B1 — GitHub Actions Node-20 bump (HARD DATE 2026-06-16).** Bump
   `actions/checkout@v4` / `setup-node@v4` / `upload-artifact@v4` /
   `download-artifact@v4` to current major across the 5 workflows: `ci.yml`,
   `gateway-ci.yml`, `pages.yml`, `release-desktop.yml`, `release.yml`.
   Shared CI infra — state the task authorization inline in your commit so
   the classifier sees context; secret VALUES never appear, only workflow
   YAML.

3. **D2** Broken `[architect](skills/architect.md)` link in
   `docs/agents/architect.md:15` and `desktect.md:30` (+ related dead path
   `bootstrap.md:253`). Repoint or remove.

4. **D3** `docs/journals/README.md` (~:18-21) phase-8 staleness — raw/ was
   deleted (round-4 e747f1d2); phase-8 now holds only README. Fix the
   paragraph.

5. **TW2** Update the Team Work bootstrap process to teach `cs terminal`
   usage + 1-liner pokes (lean-poke-bus + CK-SUBMIT chord). Do this AFTER
   @@LaneA's cs-terminal patterns are merged so the doc matches reality.

## Files you OWN

`web-marketing/` (copy), `README.md`, `gateway/` docs, `.github/workflows/*`,
`docs/agents/*.md`, `docs/journals/README.md`, and the Team Work bootstrap
doc (TW2).

## Coordination

- F6 (@@LaneD) swaps the web-marketing theme toggle to an icon: agree the
  file split (you = copy/content, D = icon component) before editing
  `web-marketing/`.
- chan-prod-setup specifics + how much of the online-service admin flow to
  document are @@Host calls — route the question through @@Lead (survey),
  don't guess.

## Verify

`make pre-push` green (B1 changes CI YAML — also sanity-check the workflow
files parse). D1 can ship as its own commit/release. Post the commit sha to
`event-lane-e.md` and poke @@Lead.
