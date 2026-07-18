# Release v0.67.2

Same-day patch on v0.67.1, cut straight to GA (owner call: one focused fix, macOS signing proven twice on the v0.67.1 runs the same day, so no rc branch and no dry run).

## What shipped

**Gateway devserver windows open natively (chan-desktop).** First real-world use of the gateway devserver surface (unblocked hours earlier by v0.67.1's OAuth fix) hit a latent v0.65.0 bug: the desktop built each window's entry path as `/{prefix}/index.html` with an already-absolute `WindowRecord.prefix`, producing a `//`-prefixed path that identity's entry validator correctly rejects (400). The failed mint tore down the whole devserver window feed via `?`, looping a 2s reconnect logged only at DEBUG: windows created on the devserver (visible from the browser launcher and `cs w l`) never materialized on the desktop, with zero visible errors.

Shipped changes, all chan-desktop plus identity-side tests:

- `window_entry_path` normalizes the prefix to exactly one leading slash (handles both prefix shapes in the codebase).
- A per-row entry-mint failure clears that row's token and keeps the feed pass alive (the watcher holds back exactly that window; a warning names it). An all-rows failure still aborts before the snapshot commit so open webviews ride out an identity outage on last-good tokens and the reconnect loop retries; this preserves the pre-fix outage semantics that the naive skip-row version would have regressed (caught in adversarial review).
- The feed-disconnected reconnect log is WARN, rate-limited to one per 5 minutes per devserver (a dead feed means that devserver's whole window surface is dark).
- Identity gains unit tests pinning `validate_desktop_entry_path`'s accept/reject contract; the validator itself is unchanged (it was correct).
- `.agents/desktop.md` gains a Debugging section (`CHAN_LOG` env var, stderr-only logs, launcher webview inspection) after those gaps cost real time during the incident.

## Validation

- New tests: path normalization, all-mints-fail returns Err, partial failure keeps the pass alive (one-shot mock entry endpoint), identity validator contract. Full desktop suite 186 green; identity 25 green.
- Two-lens adversarial review of the diff caught one real regression in the initial hardening (transient identity outage would have closed open windows with no re-mint path) and the unthrottled WARN loop; both fixed before commit.
- Full `make pre-push` green on the fix and on the GA pin state.

## Post-release

- Owner E2E on macOS after auto-update: connect devserver, new terminal opens natively, launcher list live, no feed reconnect loop under `CHAN_LOG=chan_desktop=debug`.
- No gateway deploy needed (identity untouched).
- Incident timeline and diagnosis: `dev/v0.67.1/journal.md`.
