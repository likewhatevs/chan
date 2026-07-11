# v0.67.0 Release

v0.67.0 closes the live co-editing round plus the distro source-packaging work, both landing on `main` from `origin/main` at `5b2e0008`. Two release candidates were validated as pin states; no rc tag was pushed.

## Scope

- Live co-editing: opening the same file in two clients (a second window, a gateway browser session, or a split pane) edits one shared document through a per-document server authority, keystrokes converge live instead of last-save-wins, the dirty dot means only "keystrokes not yet confirmed", and saving is a flush the server acknowledges (no conflict modal while attached). External writes (an agent `echo >>`, a `git checkout`) merge into open editors in place, `/api/files` reads and writes stay coherent with editors, and undo only rewinds your own edits. Editable text under 2 MiB attaches; on an old server or a network drop past a short grace the editor falls back to classic autosave + CAS, and `chan.docsync=0` opts a browser out.
- Peer cursors with names: every collaborator caret and selection renders live in a stable per-person color with a name flag that fades when idle; file tabs grow a count pill while others hold the same file; a peer's split panes read as one person.
- Live layout sync for co-viewed windows: two clients holding the same window id converge within about a second (splits, closes, resizes, tab moves, A/B flip, hybrid themes, terminal titles apply in place, no reload); unsaved editors survive a peer closing the tab, terminals reattach by session id, each client keeps its own focus and scroll. The server broadcasts `session_changed` after every session blob write; receivers refetch and reconcile structurally.
- Session participants always have a name: `cs session list` and the roster never render an empty name; gateway-tunnel participants show `Display Name <email>`, every participant gets a stable generated default, `cs session self --name` still wins, and the new `cs session self --reset` clears the override.
- Gateway sign-in narration in the launcher: a devserver connecting through a pasted gateway URL shows "Waiting for sign-in in your browser..." while OAuth runs, and failures explain themselves (denied, cancelled, timed out, no devserver registered, registered-but-offline named by label); a revoked token self-heals into a fresh sign-in. The desktop entry 404 body carries machine-readable reasons; mixed old/new versions degrade to the generic message.
- Terminal find works: Cmd+F on a focused desktop terminal opens the find bar and matches highlight on every surface (the search addon previously threw on its first decoration and the desktop chord never reached the terminal).
- Distro source packaging: Fedora COPR specs + srpm entry points, Ubuntu PPA debian source packaging, a vendored distro source tarball tool (`mkdist`), distros targets wired into the root Makefile, and self-update deferral to the package manager when the build is packaged.

## Branch And Commits

- Base: `5b2e0008` (`feat(web): reposition the home hero`, prior `main` tip).
- Branches: `0.67.0-rc2` (the co-editing round, in `../chan-v0670`) and `distros` (packaging, stacked on the rc2 pin, in `../chan-distros`).
- rc1 pin: `5766ca49`. rc2 pin: `7212681f`.
- Packaging commits: `485c431d..f92f160a` (nine), stacked on the rc2 pin: vendored tarball tooling, Fedora COPR, Ubuntu PPA, package-manager self-update deferral (`b20f7928`), Makefile wiring, and four packaging fixes.
- Merge: `main` fast-forwarded `5b2e0008 -> f92f160a` (both branches, one linear ff, no merge commit).
- GA pin commit: the commit that adds this report, strips `-rc2` from every pin, regenerates the three lockfiles, and cuts the CHANGELOG `[v0.67.0]` section.

## Validation

- Per-lane gates each round: full-crate clippy + tests + fmt for the Rust lanes (chan-server integration and the doc-session registry, chan-library participant names + presence, gateway identity/desktop-entry), `svelte-check` plus the full workspace-app vitest for the web lanes (2660 tests at the last full run), launcher tests for the sign-in narration.
- Adversarial review on every lane diff; findings fixed before commit (nanosecond mtime overflow of JS numbers pinned as string wire tokens, a coarse-fs-clock durability heuristic replaced with a real flush signal, two editor-remount bugs, one server reconcile bug, and a post-crash recovery gap fixed under an Option-B ruling).
- Full `make pre-push` green from an isolated worktree at `f19d7c12` (rc2), with the one later web-only commit (`2aa66d24`) covered by a full `make web-check` re-run; the rc1 gate ran green at `cdb5bd3c` with a `make web-check` re-run at `91cdb49b`. The gate is required again on the GA commit before tagging.
- Headless-Chrome browser verification against real test-server builds: rc1 covered terminal find and the co-view basics; rc2 passed all 16 co-editing + peer-cursor checks (convergence with no lost keystrokes, live agent-write merge with no banner, own-only undo, server-kill reconnect resync, no ConflictModal), evidence in `dev/v0.67.0/browser-smoke-rc1/` and `browser-smoke-rc2/`.
- Host smoke by the release owner: the browser-reachable surface was driven end to end by a headless-Chrome agent; the macOS desktop, WKWebView theming/IME, and real `id.chan.app` tunnel rows are owner-only (`dev/v0.67.0/host-smoke-rc1.md`, `host-smoke-rc2.md`) and validated on the owner's host.
- Packaging: the source-packaging tooling does not touch the `make pre-push` gate (no packaging steps in it) and is separate from the release.yml deb/rpm matrix; the COPR/PPA source builds are exercised out of band from the `../chan-distros` worktree.

## Release Workflow

- rc1 (`5766ca49`): `publish=false` dry run `29074833072`, SUCCESS on every build job (macOS sign/notarize/staple included; publish/Pages correctly skipped). Artifacts archived (`Chan_0.67.0-rc1.dmg` codesign + staple + spctl verified, both linux musl arches).
- rc2 (`7212681f`): `publish=false` dry run `29103350390`, SUCCESS on every build job (macOS + Windows signed, both linux arches, gateway packages, docker images; publish/Pages skipped). Artifacts archived (`Chan_0.67.0-rc2.dmg`, both linux musl arches).
- GA pins (`f1a9964f`): `make pre-push` green locally; `publish=false` dry run `29141148575` on the exact GA state (packaging included), SUCCESS on every build job (macOS sign/notarize/staple, Windows signed, both linux arches, gateway packages, docker images); publish/Pages correctly skipped. Artifacts stamped `0.67.0` (`Chan_0.67.0.dmg`, CLI reports `chan 0.67.0`).
- GA publish: annotated tag `v0.67.0` on `f1a9964f`, pushed after the pre-tag gate and the GA dry run passed; publish run `29142592804`, SUCCESS (GitHub Release uploaded, `/dl` metadata regenerated to 0.67.0, Pages deployed, all four docker manifests pushed). `/dl/{cli,desktop}/latest.json` verified at 0.67.0.

## Known Limitations

- Reconnect-settle window: typing in the roughly one-second window while a doc session reconnects can drop a few keystrokes; the document converges with no corruption.
- Attach-vs-reset is a millisecond-wide race documented as an accepted residual; a failed forced flush answers `/api/files` PUT with a static 503 text rather than an inline io error.
- Doc sync attaches only editable text under 2 MiB in source or WYSIWYG mode; drafts, larger files, and read-only tabs stay classic (read-only attaches receive-only).
- One-time upgrade effect: a pre-rc2 server serving this SPA build latches doc sync off after one failed dial and behaves exactly classic; the two ship together in a real release.
- Owner-only rows deferred to host smoke: macOS peer-caret flag clipping on line 1, WKWebView selection tint on older WebKit, Japanese IME composition across a peer edit, and the real `id.chan.app` tunnel hop.
- Packaged self-update deferral (`b20f7928`) applies only to distro-packaged builds; the GitHub release build is non-packaged and self-updates as before.
