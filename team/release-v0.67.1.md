# Release v0.67.1

Patch release: the chan-desktop gateway OAuth handoff fix, the id.chan.app consent restyle, the `cs session self` whoami query, and a repo-wide writing-rules sweep. One candidate branch (`handoff-and-session-self`), accepted on rc1.

## What shipped

1. **Desktop OAuth handoff (gateway).** Chrome enforces the consent page's `form-action` CSP across the form POST's redirect chain, so `/desktop/authorize/confirm`'s 303 to `chan://auth/callback` was blocked and Authorize did nothing. Confirm now answers 200 with a handoff page (zero-delay meta refresh, manual "Open chan-desktop" fallback link, close-tab note); allow, deny, and blocked-on-confirm all render it. The chan:// fragment contract is unchanged, so pre-0.67.1 desktops are fixed by the gateway deploy alone. New shared `pages.rs` shell styles the consent and handoff pages to the SPA look; CSP gains `img-src 'self'`; responses gain `x-content-type-options: nosniff`.
2. **Desktop duplicate-callback dedup.** The handoff page's fallback link outlives the auto-continue; a re-click used to banner "no sign-in in progress" over a completed sign-in. Well-formed callbacks with nothing pending are now ignored (`CallbackOutcome::Ignored`); malformed URLs still surface and never consume a live browser leg.
3. **`cs session self` whoami query.** Bare invocation reports `{window, name, role, status, leader, identity?}` as a markdown field table with `--json [--pretty]`; reuses the previously-refused `SessionSelf { name: None, reset: false }` wire shape, so roster surfaces and old/new version mixes are unaffected.
4. **Writing-rules sweep.** Repo-wide em-dash conversion (comments and docs to `--`, user-visible copy rephrased per instance) and hard-wrapped markdown reflow; 195 files. Exempt: `team/`, `CHANGELOG.md`, one applied sqlx migration (checksum integrity), two em-dash-probing test assertions.

## Validation

- Full `make pre-push` gate green after every commit and on the rc and GA pin states (root fmt/clippy/tests, no-default-features build, gateway build, web-check, marketing checks).
- Gateway `cargo test` green including the `identity` integration suites against Postgres 16 (`desktop_authorize`: 7 tests, 3 new: 200 handoff shape and headers, deny copy, blocked-on-confirm, URL-embedded-exactly-twice contract).
- Multi-agent adversarial review of the diff: 13 findings, 4 refuted, 9 confirmed (all minor), all fixed pre-merge.
- Live `cs session self` smoke on an rc test server: bare table, `--json`/`--pretty`, rename/reset round-trip, clap conflicts, loopback identity omission, grace-clock status.
- Dry run `29193684079` (`publish=false` on `0.67.1-rc1`): all jobs green including macOS sign/notarize and updater signing. Artifacts validated locally: `Chan_0.67.1-rc1.dmg`, static musl CLI for x86_64 (runs, reports `chan 0.67.1-rc1`, ships the new `session self` surface) and aarch64 (format-verified).

## Post-release actions

- Deploy the GA `chan-gateway-identity` deb to the id.chan.app host and restart the service (owner, interactive). The live Authorize bug persists until this lands.
- Owner hand-smoke after deploy: chan-desktop -> Add dev server -> `https://id.chan.app` -> Authorize -> PAT stored and devserver connects; Cancel clears the awaiting row; consent and handoff pages render the SPA look.
- Verify the first-ever live `distros-publish` run (landed after v0.67.0's release run): COPR `fiorix/chan` all chroots, `ppa:fiorix/chan` noble + resolute.

## Known limitations / follow-ups

- The handoff page keeps the chan:// URL (PAT secret) in the tab DOM until closed; bounded by no-store/no-referrer/nosniff and documented in the module doc. Follow-up: one-time redemption code handoff.
- rc candidate report: `team/release-v0.67.1-rc1-handoff-and-session-self.md`.
