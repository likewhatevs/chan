# v0.56.0: design-doc cleanup, devserver env contract, and validation carryover fixes

Branch `round-v0.56.0`, cut from the v0.55.0 GA `main`. The round opened as a focused cleanup pass with a small delivery team, then turned into a validation-carryover bug-fix wave after the v0.55.0 host smoke. A second Codex handoff finished the last launcher edge, ran the integrated gate, cut `v0.56.0-rc1`, drove host validation, landed the final rc feedback, and published GA as `v0.56.0`.

## Theme

Turn the v0.55 smoke failures into shipped fixes, simplify the design docs, align the gateway's externally-visible names with the `devserver` domain, and harden the launcher/devserver disconnect lifecycle. This is mostly a correctness and documentation release: few new concepts, but several sharp user-facing edges removed.

## Round shape

The first phase landed four scoped commits: diagram-based design-doc cleanup, removal of completed prod-cutover migration runbooks, the gateway `WORKSPACE_* -> DEVSERVER_*` env-contract rename, and launcher hover polish. The v0.55 validation tail then supplied five core bugs: mermaid zoom, rich-prompt image paste, list-line selection, Cmd+E caret mapping, and Windows `chan ps` / `close` lookup under verbatim paths. Addendum work added `cs open` focus, devserver disconnect/Abandon lifecycle, service restart/status behavior, graph-from-here tab behavior, marketing mobile/footer polish, and control-terminal attention replay.

## Design gates

The delivery team ran one consolidated host survey. Approved rulings: keep the launcher wobble on the machine header and action buttons; keep a devserver's control terminal after disconnect while removing workspace/standalone windows; widen desktop Abandon narrowly for product devserver origins only, then confirm custom `--tunnel-url` origins stay denied; make image paste emit a bare absolute drafts path with display equal to wire text; make devserver `--service --restart` preserve the running bind/port unless explicitly overridden; center the middle marketing buttons on mobile only; and make "Graph from here" open a new focused tab.

Two rulings changed after implementation reality and host validation. The mermaid zoom dark-mode fix first tried a light-panel re-render, but the host rejected the whole zoom/view affordance; final GA removes the zoom feature and restores normal diagrams. The list-selection plan changed after the prescribed padding-to-margin swap proved to be a CM6 no-op; the shipped fix removes the empty gutter bleed as a best-effort visual correction.

## What landed

### Docs and gateway contract

- Design docs replaced stale filesystem-layout and pasted-help prose with architecture diagrams (`d1dfaad9`), then a later simplification pass removed oversized diagram/prose blocks and kept the docs more maintainable (`9ef6f619`).
- Completed prod-cutover migration runbooks were removed, leaving only steady-state docs such as the Postgres test guide, sharing-unit ADR, and dev setup (`0b12b97a`).
- The gateway contract renamed externally configured `WORKSPACE_*` variables to `DEVSERVER_*`: `DEVSERVER_GATE_SECRET`, `DEVSERVER_ADMIN_TOKEN`, `DEVSERVER_ADMIN_URL`, `DEVSERVER_PUBLIC_SCHEME`, and `DEVSERVER_PUBLIC_PORT` (`87f908de`). The admin CLI's `CHAN_ADMIN_WORKSPACE_URL` and internal Rust type names were intentionally left for a later coherence pass.

### Editor and graph carryovers

- Cmd+E now preserves the caret when toggling rendered Markdown and source mode, using the same source/rendered caret mapping as the context-menu toggle (`8411d19a`, `89537885`).
- `cs open` on a newly created empty file focuses the editor with the caret ready to type (`33e65213`).
- List selection no longer leaves the empty gutter highlight to the left of list markers (`5c04fbb7`).
- The v0.55 mermaid zoom/view feature was removed after host validation showed it broke the diagram experience; diagrams return to the pre-zoom rendering behavior (`e0026410`).
- "Graph from here" always opens and focuses a fresh graph tab instead of reusing or overwriting an existing graph tab (`65499f3a`, `bbdb95c1`).

### Composer and rich prompt

- Pasting an image into the rich prompt writes the file into drafts and inserts the bare absolute drafts path. The string shown in the prompt is now the exact string sent to the terminal: no `![](...)`, no `#w=`, no CWD-relative rewrite (`41ebce79`).

### CLI, host lookup, and service lifecycle

- Windows served-workspace lookups normalize `\\?\` verbatim paths consistently, fixing the v0.55 `chan ps` `served / - / -` case and making `chan close` tear down the devserver-served workspace (`b64657de`).
- `chan devserver --service --restart` preserves the running service's bind and port when no replacement flags are passed, and `--service --status` prints the managed command line across the service backends (`25127e1a`).

### Launcher, desktop, and marketing

- Launcher hover wobble extends to machine headers and action buttons without compounding on nested icon buttons (`f84a96f9`).
- Devserver disconnect lifecycle now clears workspace windows immediately, leaves the control terminal available for logs/re-run, makes the reconnect overlay Abandon-only with a quiet spinner, and routes Abandon through the desktop path so it works from product devserver windows (`7898988d`, `c6b49c3b`, `f7b6c540`).
- The launcher replays control-terminal attention state after reconnect/reload so a terminated control process does not leave the row green and quiet (`c0f6ff1c`).
- Marketing footer and mobile layout were corrected, and the launcher demo Windows path was normalized (`7840e22c`, `d317338c`, `9dd1d9fb`).
- Release and Pages metadata generation learned prerelease handling so rc metadata does not masquerade as GA and the site can preserve the right download state (`0f984f05`).

### Lead and release

- The rc bumped all pins to `0.56.0-rc1` and cut tag `v0.56.0-rc1` (`1f28387c`).
- After host validation, final fixes removed mermaid zoom, simplified design docs, replayed control-terminal attention, and bumped/published `0.56.0` (`e0026410`, `9ef6f619`, `c0f6ff1c`, `78762c2f`).

## Validation

Host validation covered chan-desktop, browser, marketing preview/site, service management, and the Windows box. Confirmed before GA: list selection, Cmd+E caret mapping, `cs open` focus, graph-from-here new tabs, image paste display/wire identity, devserver reconnect/Abandon lifecycle, `--service` restart/status behavior, marketing layout, and Windows `chan ps` / `chan close`. The failed mermaid zoom validation drove the final removal of the zoom/view feature instead of another visual patch.

## Notes

- Custom gateway `--tunnel-url` origins still do not get the privileged desktop Abandon IPC; the release keeps product tunnel origins as the narrow supported scope.
- The env-contract rename affects self-hosters: `/etc/chan-gateway/*.env` and orchestration secrets need the `DEVSERVER_*` names before deploying v0.56.0 gateway binaries.
- Remaining optional cleanup: internal `workspace_*` Rust names and `CHAN_ADMIN_WORKSPACE_URL`, plus broader configurable Abandon origins if custom gateway use needs that later.
