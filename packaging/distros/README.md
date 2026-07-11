# distros

Source packaging for Fedora COPR and Ubuntu Launchpad (PPA): standalone `chan` and `chan-desktop`, built by the services from a vendored source tarball so their offline builders need no network. The public command surface is the root Makefile (`make distros-tarball`, `make copr-srpm`, `make copr-build`, `make ppa-source`, `make ppa-upload`).

| Path | Concern |
|---|---|
| `mkdist` | Vendored source tarball builder shared by both ecosystems: git archive + prebuilt web bundles + `cargo vendor` + in-tarball `.cargo/config.toml` offline redirect. |
| `shared/` | `chan-devserver.service` (systemd user unit) and `chan-desktop.desktop`, installed by both the rpm and deb packages. |
| `fedora/` | `chan.spec`, `chan-desktop.spec`. The build tooling rewrites `%upstream_version` from the workspace Cargo.toml, so the committed value is a fallback. |
| `copr/` | `make-srpm.sh` (tarball + spec sync + `rpmbuild -bs`; runs in the COPR SRPM chroot or a local Fedora container) and `build-srpm.sh` (local container driver, `--submit` for copr-cli). |
| `debian/` | `chan/` + `chan-desktop/` debian source dirs, `build-source.sh` (per-series `debuild -S`), `upload.sh` (dput). |

`.copr/Makefile` at the repo root is COPR's "make srpm" entry point and delegates to `copr/make-srpm.sh`.

## Package shape

- `chan`: `/usr/bin/chan` + `/usr/bin/cs` symlink (argv0 dispatch) + the devserver user unit. Runtime deps: glibc and systemd (the unit and `chan devserver --service=systemd`); everything native is statically linked (ring, bundled SQLite, zstd; rustls, no OpenSSL).
- `chan-desktop`: `/usr/bin/chan-desktop` + `chan`/`cs` symlinks to it (the desktop binary IS the CLI via argv0 dispatch), `.desktop` entry with the `chan://` scheme handler, hicolor icons, and the same devserver unit. Conflicts with/replaces/provides `chan`. Runtime deps add the WebKitGTK 4.1/GTK3/libsoup3 stack (auto-derived from sonames).
- Both builds export `CHAN_PACKAGED=rpm|deb`, which bakes the self-update surface off (`crates/chan/src/update.rs`): the probe/banner stay silent and `chan upgrade` points at the package manager. The desktop app still writes its self-healing `~/.local/bin/{chan,cs}` shims on boot; they point at the packaged binary and coexist with the `/usr/bin` symlinks.
- The packaged user unit lives in `/usr/lib/systemd/user/`; a unit written by `chan devserver --service=systemd` into `~/.config/systemd/user/` overrides it per the systemd user search order, so both flows coexist.

## Service-side setup (one-time)

COPR (`https://copr.fedorainfracloud.org`, project `fiorix/chan`, ID 245281):

1. Create the project; enabled chroots: the latest stable Fedora + rawhide, x86_64 + aarch64.
2. Add two packages, `chan` and `chan-desktop`, source type "SCM" pointed at the GitHub repo, build method "make srpm", Spec File `packaging/distros/fedora/<pkg>.spec` (COPR forwards it as `spec=` to `.copr/Makefile`), committish empty (builds main's HEAD).
3. Release automation: the `distros-publish` workflow (below) POSTs the project's **custom** webhook (Settings > Integrations, `webhooks/custom/<id>/<token>/<pkg>/`) for both packages; the base URL lives in the `COPR_WEBHOOK` repo secret. Do not enable COPR's GitHub integration: it fires on every push, not just tags.
4. Local iteration without a tag: `make copr-srpm`, then `copr-cli build fiorix/chan target/distros/srpm/<pkg>-*.src.rpm` (token from the COPR API page in `~/.config/copr`), or `make copr-build`, or curl the custom webhook manually.
5. If a stable chroot's rust trails the workspace MSRV, enable an external rust repo for that chroot in the project settings, or wait for the distro update; rawhide tracks upstream closely.

Launchpad (`ppa:fiorix/chan`):

1. Host prerequisites for building/signing uploads: `devscripts`, `debhelper`, `dpkg-dev`, `dput`, and a GPG key (`debuild -S` runs `debian/rules clean`, which needs debhelper's `dh`).
2. Create the PPA on the Launchpad account; enable arm64 in the PPA's processor settings if wanted (amd64 is on by default).
3. Register the GPG key used by debsign with the Launchpad account, and set `DEBSIGN_KEY` (or rely on the debsign default key). Local signing needs a tty pinentry: prime gpg-agent from a real terminal first (any one-off `gpg --clearsign`) when driving the build from an agent shell, or `debsign` the staged `*_source.changes` manually.
4. Local flow (re-uploads, pre-release testing): `make distros-tarball && make ppa-source PPA_SERIES="noble resolute"` then `make ppa-upload`. Source-only uploads; each series gets `X.Y.Z-1~<series>1` while the orig tarball stays byte-identical across series (Launchpad requires this for one upstream version). A packaging-only re-upload bumps `SERIESREV`, which switches debuild to `-sd` (Launchpad already has the orig).
5. Default PPA quota is ~2 GiB; each upstream version stores the ~63 MB orig twice (one per source package). Delete superseded versions or request a quota bump as needed.

## Release automation

`.github/workflows/distros-publish.yml` runs when the Release workflow completes for a `vX.Y.Z` tag (workflow_run; branch dry runs are filtered out) and is deliberately separate from release.yml: a COPR or Launchpad failure can never block or fail the GitHub release. Retry with `workflow_dispatch` and the tag. Two independent jobs, each a no-op when its secret is absent:

- `copr` curls the custom webhook for both packages.
- `launchpad` rebuilds the vendored tarball at the released commit, imports `LAUNCHPAD_GPG_PRIVATE_KEY`/`LAUNCHPAD_GPG_PASSPHRASE` into an ephemeral loopback-pinentry keyring, and runs the same build-source.sh + upload.sh as the local flow.

## Toolchain note

The workspace MSRV (`rust-toolchain.toml`) can run ahead of the distro archives. Fedora ships current rust in stable releases; Ubuntu ships versioned `rustc-1.XX`/`cargo-1.XX` packages that trail by a release or two, so `debian/*/rules` prefers the versioned toolchain when present and passes `--ignore-rust-version` to tolerate the gap. When `rustc-1.95` (or newer) reaches the target series, drop the flag and tighten the `Build-Depends` alternation in `debian/*/control`.
