# Release v0.67.3

Next-morning patch on v0.67.2, straight to GA (owner call, same rationale as v0.67.2). Third and final layer of the gateway-devserver onion: v0.67.1 made sign-in complete, v0.67.2 made windows open, v0.67.3 makes them stay open and attach shells.

## What shipped

**Gateway devserver windows reload-looped (chan-desktop).** `rewrite_gateway_window_tokens` minted a fresh 30-second entry token into every feed push and `RemoteLaunchKey` keyed on the token, so every push retargeted every open window; each reload flipped the window's connected state, which pushed the feed again -- a self-sustaining ~1-2s loop that killed shell tabs before a PTY could attach (WebKit surfaced the killed session fetch as an "access control checks" error). Raw-tunnel devservers never hit it (stable tokens); gateway devservers had a dead feed until v0.67.2, so this was the first cycle the churn could manifest.

Shipped changes (desktop + workspace-app):

- Mint-on-navigate: rows keep devserver-local tokens; `devserver::window_navigation_url` resolves the target at open/retarget/reload time (gateway: fresh entry mint; raw: stable tenant token). `rewrite_gateway_window_tokens` deleted. Gateway launch keys ignore token churn; raw keys keep retargeting on rotation.
- Lifecycle hardening around the now-async open/retarget gap, all from the adversarial review: dispatch-time launch-key remember (no duplicate navigate tasks), in-flight marker as cancellation token (no resurrecting windows closed or disconnected during a mint), vanished-retarget bails instead of rebuilding, ownership-correct failure bookkeeping, and settled-task nudges (success: immediate validating reconcile; failure: 15s bounded retry, replacing the retry loop the old feed-teardown accidentally provided).
- Cmd+R / tab-Reload on devserver windows migrated to the same URL resolution. The review caught this as a compile-clean functional break: the re-signed serve.rs helpers would have received a bare origin.
- Side finding: terminal-only windows no longer call `/api/preflight` or `/api/screensaver/state` (both workspace concepts; the slim terminal tenant mounts neither; the 404s were benign but logged on every terminal window everywhere, local included).

## Validation

- New/updated tests: `window_navigation_url` (gateway one-shot mock, raw assembly, failure), launch-key gating both connection kinds, source pins for every hardening invariant plus the migrated reload path, SPA source pins for both terminal-only gates. Desktop suite 188 green; web svelte-check + vitest green.
- Two-lens adversarial review (lifecycle races; correctness/parity): 5 confirmed findings, all fixed pre-commit, including the reload-path break and a zombie-window class the naive async gap would have shipped.
- Full `make pre-push` green on the fix and on the GA pin state.

## Post-release

- Owner E2E on macOS after auto-update: connect gateway devserver, new terminal opens ONCE and stays; shell attaches; no reload cadence; Cmd+R reloads into the tenant, not the bare origin; browser-created terminals appear live in the desktop launcher.
- No gateway deploy needed (gateway untouched this cycle).
- Diagnosis + incident trail: `dev/v0.67.1/journal.md` (v0.67.2 section) and this cycle's trace in the session record.
