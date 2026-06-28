# v0.55.0: editor polish, devserver hardening, and docs consolidation

Branch `round-v0.55.0`, cut from the v0.54.0 GA `main`. A lean EXECUTION round run as one parallel team: a lead plus four file-local lanes for Editor, Devserver, Docs, and Packaging. The round intentionally stayed out of the deferred gateway/devserver backend work and focused on local devserver correctness, the markdown editor surface, packaging/docs follow-through from v0.54.0, and a small marketing-site tail that landed mid-round. The rc was cut as `0.55.0-rc1`; host smoke ran in parallel, and GA followed after the accepted fixes and carryovers were dispositioned.

## Theme

Tighten the editor's everyday affordances, make the local devserver more legible and less brittle, and move the project's historical release memory into the repo-root `team/` layout. The round also connected the v0.54.0 Docker publishing work to self-hosting docs and Kubernetes manifests.

## Design gate

One consolidated survey covered all worker lanes. The load-bearing rulings were: keep the gateway/devserver backend deferred to v0.56.0; handle ENH1, B2, and B5 as devserver-local work; harden and clarify the existing model-download proxy path instead of embedding a separate download stack; put the settable local-workspace display name in the Devserver lane because it already owned the launcher and registry files; and move signing procedures out of the public repo, leaving only stubs that point agents and CI errors at the private team tree.

## What landed (by lane)

### Editor -- list, table, mermaid, selection, and rich-prompt work

- Ordered-list insertion renumbers the tail, including the loose blank-line-separated case (`030f4ef9`, `c35656d7`).
- Wide tables stay readable by scrolling horizontally instead of wrapping cell text character by character, in both editor and rendered/printed output (`e56a055f`).
- List-line selection was narrowed in the editor CSS (`4c825bd2`), though host smoke later showed the visual gutter bug was not fully solved and carried to v0.56.0.
- Mermaid diagrams gained a click-to-zoom/pan view with keyboard controls (`3d6cd326`), but host validation found the dark-mode overlay and page-width side effects unacceptable. v0.56.0 removed this feature and restored the simpler pre-zoom behavior.
- Rich-prompt image paste started rewriting delivery paths relative to the terminal CWD (`6df5b216`). Host smoke changed the product requirement: the eventual v0.56.0 contract is a bare absolute drafts path, with display text equal to terminal wire text.

### Devserver -- OS identity, display names, proxy errors, and Windows keying

- The devserver self-reports its OS, with Linux distro data where available; the launcher renders OS icons on local and remote machine cards (`fbeb477e`, `b0833d8`).
- Local workspaces accept an optional display name end to end, from launcher dialog through registry storage and launcher render (`fbeb477e`, `b0833d8`).
- Model-download failures behind unusable proxy environment variables now surface a clear error; the investigation confirmed that the upstream hf-hub client already honors standard `HTTP(S)_PROXY`, `ALL_PROXY`, and SOCKS proxy settings, while `NO_PROXY` and https-scheme proxies remain unsupported for that download path (`7226fc51`).
- Windows devserver path handling normalized the `\\?\` verbatim-prefix keying in workspace metadata (`e1c6a26c`) and quieted the stale local-toggle toast by persisting the bound port (`a683e35d`). Host smoke still found served-workspace lookup mismatches in `chan ps` / `chan close`; that became the v0.56.0 Windows path-keying fix.

### Docs -- release history and private signing procedures

- The old development log moved out of `docs/phases/` into the repo-root `team/` release-history layout, with one front-door report per release era (`05aeca56`).
- The tracked journals tree was removed from the public docs surface (`cd7ed4be`).
- Agent `@@mention` vocabulary was narrowed to the reusable skill identities plus generic `@@agent`, with the tracked docs made allowlist-clean (`617f3628`, `9607dd07`).
- Release-signing procedures moved out of the public repo; CI and agent references now point at the stub/private-procedure boundary (`79ab46ef`, `bf4b0aae`).

### Packaging and marketing -- Docker references and site tail

- Self-hosting docs gained the Docker Hub pull path for the public images published in v0.54.0, and the Kubernetes manifests now point at the `fiorix/<service>` images (`38190d81`, `ef72c3d0`).
- The marketing redesign landed during the round, then follow-ups made the launcher demo taller, shortened the hero and meta description, removed a stale signing-doc comment, moved the search shortcut off bare Ctrl+S, and added the free-software footer line (`e79e36cd`, `3ca5a50c`, `fa5a508a`, `e7bd5362`, `386e46e4`, `9dcc788e`, `d38a8f93`).

### Lead -- release pins, changelog, gate, and rc

- The release pins moved `0.54.0 -> 0.55.0-rc1`, then GA moved `0.55.0-rc1 -> 0.55.0` (`e370b941`, `7c83f70c`).
- The v0.55.0 changelog and first new-layout team report were added (`2a495f7c`).
- The integrated gate ran the host-runnable pre-push path, with desktop and cross-OS artifacts covered by the non-publishing release dry-run and host devices.

## Validation and carryover

Host smoke confirmed install/launch on macOS, the table scroll fix, top-level ordered-list renumbering, the search-chord change, launcher OS icons, local display names, and the constrained-env model-download behavior. It also found the important carryovers: mermaid zoom regressed the diagram experience, rich-prompt image paste should emit a bare absolute path instead of Markdown/relative delivery, list selection still bled visually, and Windows `chan ps` / `chan close` still failed to resolve a devserver-served `\\?\` workspace. Those became the core of v0.56.0's bug-fix wave.

## Notes

- The gateway/devserver backend was deliberately deferred again: tunnel offline status, desktop OAuth onboarding, multi-devserver, and `cs window new` from a devserver all move to a later round.
- The round's adversarial review found six confirmed issues before close: the loose-list renumber gap, an inaccurate OS-icon comment, a stale `.gitignore` pointer, and three historical-doc nits. All were fixed before the rc cut.
- Validation stayed split by environment: local web/cargo checks on the Linux host, release dry-run for desktop/container builds, and host-owned device smoke for macOS/Windows.
