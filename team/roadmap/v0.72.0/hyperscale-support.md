# CentOS Stream COPR Support for Hyperscale Deployments

Status: implemented; validated locally on x86_64 only. The live chroots and repositories are configured, no COPR build has run, and nothing has exercised aarch64. Grounded against `59acd07a` (`v0.71.0`) on 2026-07-19.

## Summary

Expand the `fiorix/chan` COPR project from Fedora-only buildroots to the CentOS Stream buildroots used by Hyperscale deployments. Hyperscale is the deployment context, not a distinct COPR buildroot or a new package dependency.

The intended matrix, and what has actually been built, are two different things. The `intent` columns are the configuration; the `proof` column is the evidence:

| COPR chroot | `chan` | `chan-desktop` | proof |
| --- | --- | --- | --- |
| `centos-stream+epel-next-9-x86_64` | build | exclude | local sdme rebuild |
| `centos-stream+epel-next-9-aarch64` | build | exclude | none |
| `centos-stream-10-x86_64` | build | build | local sdme rebuild |
| `centos-stream-10-aarch64` | build | build | none |

Only the x86_64 rows have been built anywhere. The aarch64 rows are enabled configuration with no build behind them: the validation host is x86_64 with no aarch64 binfmt/QEMU registration, so this item claims aarch64 support nowhere until a native build proves it.

Keep every currently enabled Fedora chroot. The package matrix, rather than a spec-level architecture exclusion, owns the EL9 desktop restriction.

## Dependency Findings

Disposable CentOS image probes resolved the current spec requirements before any COPR setting changed:

- Stream 9 with CRB, EPEL, and EPEL Next provides Rust/Cargo 1.96, GCC/G++ 11.5, `systemd-rpm-macros`, and every current `chan` `BuildRequires`.
- Stream 10 with CRB and EPEL provides Rust/Cargo 1.96, GCC/G++ 14.4, `systemd-rpm-macros`, GTK3 3.24, libsoup3 3.6, and WebKitGTK 4.1 2.48.
- EPEL Next 9 does not provide `libsoup3-devel` or `webkit2gtk4.1-devel`. The current Tauri/Wry dependency graph is tied to WebKitGTK 4.1 and libsoup3, so `chan-desktop` cannot build there from the target repositories.
- Chan does not use `cargo-rpm-macros`, the dependency that required EPEL for sdme's Stream 10 build. The existing specs invoke Cargo directly.
- Both source packages already compile from the vendored Cargo and prebuilt web closure produced by `packaging/distros/mkdist`. No new source fetch is required in the RPM buildroot.

The desktop exclusion avoids introducing a privately maintained WebKitGTK backport or downgrading the desktop dependency stack solely for EL9.

## COPR Configuration

Enable these project chroots:

```text
centos-stream+epel-next-9-aarch64
centos-stream+epel-next-9-x86_64
centos-stream-10-aarch64
centos-stream-10-x86_64
```

Set this chroot-specific additional repository on each of the four CentOS chroots exactly as written:

```text
https://dl.fedoraproject.org/pub/epel/$releasever/Everything/$basearch/
```

The variables must remain literal. A hardcoded `/10/` works for Stream 10 but makes the EL9 buildroot consume incompatible EPEL 10 release packages. Do not set this repository project-wide: the Fedora chroots expand `$releasever` to a Fedora release with no matching EPEL repository, and COPR renders additional build repositories with `skip_if_unavailable=0`.

Set the `chan-desktop` SCM package chroot denylist to:

```text
centos-stream+epel-next-9-*
```

Keep `chan` unrestricted. The custom release webhooks continue to rebuild both SCM packages; the package denylist prevents the two unsupported desktop jobs.

Raw SRPM submissions are independent of the SCM package settings. `packaging/distros/copr/build-srpm.sh --submit` therefore passes two explicit `--exclude-chroot` values for `chan-desktop`, one per EL9 architecture. It does not enumerate allowed chroots, so future Fedora and Stream 10 targets remain automatic.

The release path never runs that script: `distros-publish` POSTs the custom webhook, which rebuilds the SCM packages from Git. At release time the console denylist is therefore the only thing that keeps the two EL9 desktop jobs from being scheduled, and no artifact in this repo can assert or verify it.

## Local Validation Contract

`make copr-check` builds the supported matrix in disposable sdme containers before submission:

1. Produce the selected self-contained SRPMs through the existing Fedora container path.
2. Start one clean sdme container per release and package, avoiding the intentional `chan`/`chan-desktop` RPM conflict.
3. Mirror the COPR repositories: CRB on both releases, `epel-release` plus `epel-next-release` on EL9, and the literal generic external EPEL repository on both releases, exactly as COPR renders a chroot additional repository.
4. Resolve the SRPM `BuildRequires` with DNF.
5. Rebuild as an unprivileged `builder` user with `CARGO_NET_OFFLINE=true`.
6. Install the binary RPM and smoke its packaged behavior.
7. Store the RPM, upgrade output, guest exit status, and build log under `target/distros/copr-check/el<9|10>/<arch>/<package>/`.
8. Remove each container as its target finishes, then report a per-target PASS/FAIL summary and exit non-zero if any target failed.

Every target runs even after an earlier one fails, so one cycle reports the whole matrix. `KEEP_CONTAINER=1` keeps every container instead, including a failed target's: the guest command always exits 0 and reports its real status through a file on the writable `/out` bind, because `sdme new` deletes the container when its guest command fails.

The command accepts `PKG=all|chan|chan-desktop`, `COPR_RELEASE=all|9|10`, `KEEP_CONTAINER=0|1`, and `REUSE_SRPM=0|1`, each with a Makefile default. An explicit `PKG=chan-desktop COPR_RELEASE=9` is rejected as unsupported.

This is not a mock buildroot. `dnf builddep` installs the spec's declared `BuildRequires` into a full CentOS container rootfs, so anything the build needs that the rootfs already carries but the spec does not declare passes here and fails in COPR's minimal buildroot. The check catches missing repositories, unresolvable declared dependencies, offline-vendoring breaks, and packaging or install regressions; it does not prove the `BuildRequires` list is complete.

Rootfs names are per host, not portable. The Makefile defaults are `centos-stream-9` and `centos-stream-10`, imported with:

```sh
sudo sdme fs import centos-stream-9 quay.io/centos/centos:stream9 --install-packages=yes -v
sudo sdme fs import centos-stream-10 quay.io/centos/centos:stream10 --install-packages=yes -v
```

A missing rootfs preflight prints the import command and the names this host does have. The validation host's entries are `cs9` (Stream 9) and `vfy-centos` (Stream 10), so its command is:

```sh
make copr-check DOCKER='sudo docker' \
  COPR_EL9_ROOTFS=cs9 \
  COPR_EL10_ROOTFS=vfy-centos
```

## Implementation Evidence

Local x86_64 validation completed on 2026-07-19 with the host's `cs9` and `vfy-centos` sdme rootfs entries. The vendored SRPMs were rebuilt as an unprivileged user with Cargo offline, then installed and checked in clean containers:

- Stream 9 `chan` resolved CRB, EPEL, and EPEL Next dependencies, built `chan-0.71.0-1.el9.x86_64.rpm`, installed it, and passed its CLI, packaged-upgrade, and systemd unit checks.
- Stream 10 `chan` resolved dependencies through CRB plus the generic external EPEL URL, built `chan-0.71.0-1.el10.x86_64.rpm`, installed it, and passed the same checks.
- Stream 10 `chan-desktop` resolved the complete GTK3, libsoup3, and WebKitGTK 4.1 closure, including `javascriptcoregtk4.1-devel` and `webkit2gtk4.1-devel` from EPEL. It built `chan-desktop-0.71.0-1.el10.x86_64.rpm`, installed it, and passed its CLI entry point, package marker, desktop entry, icon, shared-library, conflict, and systemd unit checks.
- The literal generic repository selected EPEL 9 in the Stream 9 container and EPEL 10 in the Stream 10 containers. Cargo made no network fetch during any RPM rebuild.

The first desktop post-install attempt exposed a validator error rather than a package error: desktop `chan upgrade` delegates to a running GUI and times out in a headless container. The corrected validator checks the embedded RPM package-manager marker directly, and a new end-to-end desktop rebuild passed.

The live `fiorix/chan` COPR configuration was applied by the project owner and verified through COPR's public API on 2026-07-19:

- All four CentOS chroots are enabled alongside the existing Fedora 44 and Rawhide chroots.
- The project-wide additional repository list is empty.
- Each CentOS chroot has the literal generic EPEL repository, while the rendered Fedora 44 and Rawhide build configurations do not.
- The owner configured the `chan-desktop` EL9 denylist. COPR's public package endpoint does not expose this field, so the first submission must still confirm that no EL9 desktop job is scheduled.

The configuration is in place for COPR validation builds, none of which has run yet. This x86_64 host lacks aarch64 emulation, so native COPR builds remain the aarch64 acceptance gate.

## Acceptance

Local x86_64 acceptance requires all three supported builds to pass:

- EL9 `chan` builds with an `.el9` release, installs, and runs `chan --version` and `cs --help`.
- EL10 `chan` builds with an `.el10` release and passes the same smoke.
- EL10 `chan-desktop` builds with an `.el10` release, installs, validates its desktop entry and icons, has no missing shared libraries, and retains its `Conflicts: chan` metadata.
- Every package verifies the installed user unit with `systemd-analyze verify`.
- The standalone `chan` package's `chan upgrade` exits unsuccessfully and names `dnf upgrade`.
- The desktop binary contains the packaged `sudo dnf upgrade` refusal marker. Its `chan upgrade` personality delegates to a running GUI, so that path is not a valid headless-container smoke.

After local x86_64 acceptance, COPR acceptance requires:

- `chan` succeeds on all enabled Fedora chroots and all four new CentOS chroots.
- `chan-desktop` succeeds on all enabled Fedora chroots and both Stream 10 chroots.
- No `chan-desktop` EL9 job is scheduled.
- EL9 build logs resolve the generic external repository as EPEL 9; EL10 logs resolve it as EPEL 10.
- Fresh Stream 9 and Stream 10 sdme containers can install the published COPR packages and repeat the package smokes.

This x86_64 host has no aarch64 binfmt/QEMU registration. The first native COPR aarch64 builds are therefore the required aarch64 acceptance gate. The same sdme check can run on a native aarch64 host when one is available.

## Boundaries

- Do not add a Hyperscale repository to the buildroot or add a Hyperscale runtime dependency. Chan's RPM requires systemd but does not require sdme's systemd 255 minimum.
- Do not backport WebKitGTK 4.1 or libsoup3 to EL9 in this item.
- Do not change Tauri/Wry versions to manufacture EL9 desktop support.
- Do not change application APIs, schemas, or Rust types. The public additions are the COPR package matrix and `make copr-check`.
