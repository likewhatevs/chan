# v0.55.0: editor polish, devserver hardening, and docs consolidation

## Theme

A focused execution round off the v0.54.0 GA tree: tighten the markdown editor, harden the local dev server, and consolidate the project's docs. Four file-local lanes (editor, devserver, docs, packaging) built in parallel under a single consolidated design gate, plus a lead seam for the release pins and the integrated gate. Mid-round the public marketing site landed a redesign with an embedded live launcher; the round rebased onto it cleanly and carried a few launcher/devserver-aware follow-ups on top.

## What landed (by lane)

### Editor

- Ordered lists renumber the tail on a mid-list insert, including loose (blank-line-separated) lists, instead of leaving a duplicate number.
- Wide tables scroll horizontally instead of wrapping each cell character-by-character, in the editor and in the rendered/printed output.
- List-line selection highlights just the line; the marker keeps its hanging-left look without the selection bleeding into the margin.
- Mermaid diagrams open a click-to-zoom view with pan and keyboard control (zoom, reset, arrows + WASD, wheel, Escape), on web and desktop.
- A rich-prompt pasted image is delivered as a path relative to the terminal's working directory (absolute on-disk when that directory is unknown or outside the workspace); the composer preview matches what is sent.

### Devserver

- A dev server self-reports its OS (and Linux distribution where available) on its direct-dial info; the launcher shows an OS icon on the local machine card and on each remote dev server.
- A local workspace accepts an optional display name end to end: launcher dialog -> registry -> rendered name in place of the folder basename.
- The model download fails with a clear, actionable error when a proxy environment variable is set but unusable. Grounding the actual client showed hf-hub already honors standard `HTTP(S)_PROXY` / `ALL_PROXY` / SOCKS proxies; `NO_PROXY` and https-scheme proxies are documented as unsupported for the model download (the originally-planned ureq hand-configuration was infeasible -- hf-hub owns its agent).
- Windows: `chan open` no longer prints the stale-port error toast (the dev server persists its bound port and the local on-toggle is best-effort), and `chan ps` resolves a server's PID and kind under the `\\?\` verbatim path prefix (the workspace root is keyed identically across the prefix via normalize-on-read).

### Docs

- The dev-log moved to a repo-root `team/` release-history layout (the old `docs/phases/` retired, lossless, with history preserved), and `docs/journals/` was removed.
- Agent `@@mention` vocabulary was narrowed to the five reusable skill identities (`@@architect`, `@@fabler`, `@@rustacean`, `@@syseng`, `@@webdev`) plus the generic `@@agent`; every tracked `.md` is now allowlist-clean and the mention index is fully narrowed.
- Release-signing procedures moved out of the public repo into the team's private tree, leaving a stub that the CI error strings and agent docs point at.

### Packaging

- Self-hosting docs document the Docker Hub pull path, and the Kubernetes cluster manifests point at the published `fiorix/<service>` images (operator-set version placeholder); the path is exercised by the non-publishing CI dry-run.

### Lead + marketing follow-ups

- The release pins moved to `0.55.0-rc1` across every manifest and lockfile; the round rebased onto the landed marketing redesign.
- On the redesigned marketing site: the embedded launcher demo was made taller so the add-devserver form clears the fold, the hero feature line and meta description were shortened, and a stale signing-doc reference in the asset script was dropped.

## Notes

- One consolidated design gate: all four lanes batched their open questions into a single host survey; the rulings landed in the round's `followups/`. The headline override was B2 (no embedding; harden/clarify the proxy path instead).
- An adversarial review of the committed work (per-lane finders with refute-verification) surfaced six confirmed issues -- a loose-list renumber gap, an inaccurate OS-icon comment, a stale `.gitignore` pointer, and three historical-doc nits -- all fixed before the close; one false positive was refuted.
- The integrated gate ran `make pre-push` minus the desktop crate (the dev host has no GTK); the desktop and container builds are validated by the non-publishing CI dry-run. Cross-OS desktop/browser smoke is on-device by the host.
- Validation was decoupled from the rc cut: the rc builds while the host validates on-device, and anything found becomes a follow-up. The Windows `chan ps` keying fix landed with unit tests proving the keyed root matches across the `\\?\` prefix, with on-device Windows confirmation deferred to the host.
- Deferred to v0.56.0: the architecture gap-fill (Windows arm64, macOS Intel), a design-docs sweep that replaces filesystem-layout and pasted-help prose with layered architecture diagrams, the related `workspace` -> `devserver` domain cleanup across the gateway docs, and the Windows terminal-idle-reconnect bug.
