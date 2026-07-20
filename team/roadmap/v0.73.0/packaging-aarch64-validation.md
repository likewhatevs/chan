# aarch64 Packaging Validation

Status: accepted scope for v0.73.0. Carried forward from v0.72.0, which shipped both distro packaging ecosystems validated on x86_64 only. Nothing about aarch64 is proven in either one.

## Problem

v0.72.0 shipped CentOS Stream COPR packaging ([hyperscale-support](../done/hyperscale-support.md)) and Arch Linux AUR packaging ([aur-support](../done/aur-support.md)). Both declare aarch64. Neither has ever built it.

- The AUR recipes `packaging/distros/arch/aur/chan/PKGBUILD.in` and `packaging/distros/arch/aur/chan-desktop/PKGBUILD.in` declare `arch=('x86_64' 'aarch64')`.
- The `fiorix/chan` COPR project has `centos-stream+epel-next-9-aarch64` and `centos-stream-10-aarch64` enabled, with `chan-desktop` denied on both EL9 chroots. The intended aarch64 targets are therefore EL9 `chan`, EL10 `chan`, and EL10 `chan-desktop`.
- No aarch64 build has run in either ecosystem, on any host or any runner. The declaration and the chroot configuration are the only things that exist.
- The validation host is x86_64 with no binfmt or QEMU registration, so no local run can produce aarch64 evidence. `make copr-check` and `make aur-check` both build for the host architecture.

The gap this leaves is that two published packaging surfaces advertise an architecture whose build has never been observed to succeed or fail. A user on aarch64 is the first party to run it.

## Desired outcome

Turn "declared but never built" into one of two settled states, per ecosystem:

- proven: a native aarch64 build ran, passed, and its evidence is recorded in this item, with the aarch64 leg wired into the path that runs at GA; or
- withdrawn: the aarch64 declaration is removed from the recipes and the chroot configuration, so no packaging surface claims an architecture nothing builds.

Either outcome closes this item. Leaving the declaration in place with no build behind it does not.

## What each ecosystem needs

### COPR

Native COPR builds are the aarch64 acceptance gate. COPR builds each enabled chroot on its own architecture, so submitting the existing SRPMs to the enabled aarch64 chroots is the whole mechanism; no new tooling is required to get the first result. The same `make copr-check` matrix can also run on a native aarch64 sdme host, with that host's Stream 9 and Stream 10 rootfs names passed through `COPR_EL9_ROOTFS` and `COPR_EL10_ROOTFS`, as a faster local reproduction than a service round trip.

Both routes exercise the same spec files, the same vendored `packaging/distros/mkdist` source, and the same literal generic EPEL repository. What is unknown is whether every declared `BuildRequires` resolves on aarch64 buildroots and whether the `chan-desktop` GTK3, libsoup3, and WebKitGTK 4.1 closure is complete there.

### AUR

`.github/workflows/distros-publish.yml` already carries `aur-validate-arm`. It runs on a native `ubuntu-24.04-arm` runner, imports the signed Arch Linux ARM rootfs, and runs `packaging/distros/arch/build-in-ci.sh` for both package bases. It is opt-in on manual dispatch behind `aur_validate_arm`, and it is absent from `aur-publish`'s `needs`, so a GA run schedules no ARM cell and publication never waits on it.

Working the leg is a dispatch with `targets=aur` and `aur_validate_arm=true` against a GA tag. The unproven steps inside it are the rootfs import and keyring bootstrap, the rolling `-Syu`, dependency resolution through `makepkg --syncdeps` on ALARM, the native ARM Tauri build for `chan-desktop`, and namcap review of the resulting aarch64 packages.

## Acceptance

Proving it requires, per ecosystem:

- COPR: `chan` succeeds on `centos-stream+epel-next-9-aarch64` and `centos-stream-10-aarch64`; `chan-desktop` succeeds on `centos-stream-10-aarch64`; no `chan-desktop` EL9 aarch64 job is scheduled; the EL9 log resolves the generic external repository as EPEL 9 and the EL10 logs resolve it as EPEL 10; and a fresh aarch64 container installs the published packages and repeats the existing package smokes, including `systemd-analyze verify` on the user unit and the packaged-upgrade refusal.
- AUR: one `distros-publish` dispatch with `targets=aur` and `aur_validate_arm=true` is green for both `chan` and `chan-desktop`, including their in-container test suites, pacman install, post-install smokes, and namcap error-class review. After that run passes, `aur-validate-arm` gains the `workflow_run` trigger the other AUR jobs carry and a place in `aur-publish`'s `needs`, so GA covers aarch64 instead of skipping it.

Withdrawing it requires:

- removing `aarch64` from the `arch=()` array in both `PKGBUILD.in` files;
- disabling `centos-stream+epel-next-9-aarch64` and `centos-stream-10-aarch64` in the COPR project, and dropping the `--exclude-chroot centos-stream+epel-next-9-aarch64` argument in `packaging/distros/copr/build-srpm.sh`, which would then name a chroot that no longer exists;
- removing the `aur_validate_arm` input and the `aur-validate-arm` job, since nothing would be left to validate;
- updating [hyperscale-support](../done/hyperscale-support.md) and [aur-support](../done/aur-support.md) so no shipped item's text declares an architecture the packaging no longer offers.

Whichever outcome lands, this item records the evidence for it: the commands run, the host or runner, and the per-target result.

## Boundaries

- A native aarch64 build is the evidence. A binfmt or QEMU build is acceptable for early diagnosis only, because COPR and Arch Linux ARM users both build natively and an emulated buildroot does not prove either environment.
- Fix whatever the aarch64 build reveals in the recipes and the packaging scripts. Do not change application APIs, Rust types, or the package contract to manufacture a passing build.
- Do not add `-bin` AUR variants, backport WebKitGTK 4.1 or libsoup3 to EL9, or change Tauri and Wry versions in this item.
- The EL9 `chan-desktop` exclusion stays as it is on both architectures. It is a dependency-availability decision, not an architecture decision.
