# aarch64 Packaging Validation

> Status: delivered in part in [v0.73.0](../../release/release-v0.73.0.md); the remainder is tracked as [aur-aarch64-publication-gate](../v0.74.0/aur-aarch64-publication-gate.md).

The COPR half is closed by evidence: aarch64 was already green there and the harvested per-chroot results are recorded below, which retires this item's original premise. The AUR half is wired but unproven. `aur-validate-arm` now runs on every GA release instead of only on manual dispatch, so v0.73.0 produces the first real Arch Linux ARM result; by owner ruling it is observed evidence and does **not** gate `aur-publish`, because an unproven job must not block a first AUR publication. Once one ARM cell has passed, adding it to `aur-publish`'s `needs` closes the item.

## Corrected baseline

The original claim that neither ecosystem had ever built aarch64 was false for COPR. Fedora aarch64 builds existed for both packages from the project's first versioned build, v0.67.0, and remained green through v0.71.0. What v0.72.0 added, and what had never run before, was the CentOS matrix on either architecture. Arch Linux ARM remains unproven.

The AUR recipes still declare `arch=('x86_64' 'aarch64')`. COPR build 10749265 proves the standalone package on both CentOS aarch64 targets, and build 10749266 proves the desktop package on EL10 aarch64. Withdrawal is therefore not the appropriate outcome.

## Fedora aarch64 history

The COPR build-list API reports these versioned builds with both `fedora-44-aarch64` and `fedora-rawhide-aarch64` in their chroot sets. Each listed build succeeded; duplicate retry builds for v0.67.1 and v0.67.3 are omitted here.

| Version | `chan` build | `chan-desktop` build | Fedora aarch64 |
|---|---:|---:|---|
| 0.67.0 | 10708248 | 10708249 | passed |
| 0.67.1 | 10711275 | 10711276 | passed |
| 0.67.2 | 10711451 | 10711452 | passed |
| 0.67.3 | 10718699 | 10718700 | passed |
| 0.68.0 | 10719634 | 10719635 | passed |
| 0.69.0 | 10721657 | 10721658 | passed |
| 0.70.0 | 10735575 | 10735576 | passed |
| 0.70.1 | 10737654 | 10737655 | passed |
| 0.70.2 | 10740599 | 10740600 | passed |
| 0.70.3 | 10740906 | 10740907 | passed |
| 0.71.0 | 10742767 | 10742768 | passed |
| 0.72.0 | 10749265 | 10749266 | passed |

Source: the unauthenticated COPR [`build/list`](https://copr.fedorainfracloud.org/api_3/build/list?ownername=fiorix&projectname=chan&packagename=chan&limit=50) endpoint, cross-checked against the corresponding `chan-desktop` query and the per-chroot API.

## v0.72.0 CentOS results

Both webhook builds finished on 2026-07-20. [`chan` build 10749265](https://copr.fedorainfracloud.org/coprs/fiorix/chan/build/10749265/) succeeded on all eight enabled chroots. [`chan-desktop` build 10749266](https://copr.fedorainfracloud.org/coprs/fiorix/chan/build/10749266/) succeeded on Fedora and EL10 for both architectures, but failed on both EL9 chroots.

| Package | Chroot | Result |
|---|---|---|
| `chan` | `centos-stream+epel-next-9-aarch64` | passed |
| `chan` | `centos-stream+epel-next-9-x86_64` | passed |
| `chan` | `centos-stream-10-aarch64` | passed |
| `chan` | `centos-stream-10-x86_64` | passed |
| `chan-desktop` | `centos-stream+epel-next-9-aarch64` | failed |
| `chan-desktop` | `centos-stream+epel-next-9-x86_64` | failed |
| `chan-desktop` | `centos-stream-10-aarch64` | passed |
| `chan-desktop` | `centos-stream-10-x86_64` | passed |

The per-chroot API records are available from `/api_3/build-chroot` with the build id and chroot name. Direct builder logs: [`chan` EL9 aarch64](https://download.copr.fedorainfracloud.org/results/fiorix/chan/centos-stream+epel-next-9-aarch64/10749265-chan/builder-live.log.gz), [`chan` EL10 aarch64](https://download.copr.fedorainfracloud.org/results/fiorix/chan/centos-stream-10-aarch64/10749265-chan/builder-live.log.gz), and [`chan-desktop` EL10 aarch64](https://download.copr.fedorainfracloud.org/results/fiorix/chan/centos-stream-10-aarch64/10749266-chan-desktop/builder-live.log.gz).

### External EPEL repository

The EL9 aarch64 `chan` log proves that the literal generic repository selected EPEL 9 content:

```text
epel-rpm-macros  noarch  9-18.el9  https_dl_fedoraproject_org_pub_epel_releasever_Everything_basearch
```

The EL10 aarch64 desktop log proves the same repository selected EPEL 10 content:

```text
webkit2gtk4.1-devel  aarch64  2.48.3-1.el10_1  https_dl_fedoraproject_org_pub_epel_releasever_Everything_basearch
```

The x86_64 logs carry the matching `9-18.el9` and `2.48.3-1.el10_1` rows. The standalone EL10 build consumed no package from the external repository; its builder config retains the literal `$releasever` URL and its root log invokes DNF with `--releasever 10`, while the desktop dependency row above proves an actual EPEL 10 package resolution.

### EL9 desktop scheduling failure

The acceptance requirement that no EL9 desktop aarch64 job be scheduled failed for v0.72.0: both EL9 jobs ran, so the COPR console denylist was not active for this build. Both logs fail dependency resolution with:

```text
No matching package to install: 'libsoup3-devel'
No matching package to install: 'webkit2gtk4.1-devel'
```

The repository-side guard now in `packaging/distros/fedora/chan-desktop.spec` rejects EL9 during spec evaluation and names those packages. It does not prove or replace the console denylist. The public COPR API does not expose that setting, so only the next build's chroot count can prove whether the denylist is active.

## AUR GA observation

`aur-validate-arm` uses a native `ubuntu-24.04-arm` runner, verifies and imports the signed Arch Linux ARM rootfs, and invokes the same `build-in-ci.sh` path as x86_64 for both package bases. The workflow now:

1. runs the ARM matrix after every successful GA Release workflow;
2. retains `aur_validate_arm=true` as the manual-dispatch opt-in;
3. selects the release tag and checkout SHA on both event paths; and
4. keeps `aur-publish` waiting on `aur-auth` and x86_64 `aur-validate` only for v0.73.0.

The ARM job is observed-but-not-gating for v0.73.0. It has never executed end to end and cannot be honestly proven before a tag carrying the `systemd-analyze` fix. v0.72.0 never published to the AUR, so making this unproven job a dependency would risk blocking the first AUR publication for a second consecutive release. v0.73.0 runs ARM at GA to produce the evidence while the established x86_64 job remains the publication gate; v0.74.0 adds `aur-validate-arm` to `aur-publish`'s `needs` after it has passed once.

The x86_64 matrix remains the sole producer of `aur-metadata-*`; the ARM matrix uploads nothing. No workflow was dispatched while making this change. The first native ARM run must still prove the rootfs import and keyring bootstrap, rolling `-Syu`, `makepkg --syncdeps`, the native Tauri build, the namcap error gate, pacman install, `systemd-analyze` smoke, packaged-upgrade refusal, desktop stamp and refusal hint, desktop entry, and five icon sizes. There is no `ldd` gate.

## Remaining unproven acceptance

- **Fresh aarch64 COPR install smoke:** not runnable on this host. It is x86_64, has no QEMU binary or QEMU binfmt registration, and all 15 sdme rootfs are x86_64. A native aarch64 sdme host with `COPR_EL9_ROOTFS` and `COPR_EL10_ROOTFS`, or a new native ARM Actions job, must install the published RPMs and repeat the package smokes.
- **AUR native build:** not yet run. The v0.73.0 GA workflow automatically runs both ARM matrix cells as observed evidence. Only the owner may additionally dispatch `targets=aur aur_validate_arm=true`. Until both cells pass, the AUR aarch64 declaration remains wired but unproven and does not gate publication.
- **COPR console denylist:** not observable through the public API. The v0.72.0 evidence proves it was ineffective for that build; current console state remains unknown until inspected in the UI or inferred from a later build's absent EL9 desktop jobs.

`packaging/distros/copr/build-in-container.sh` cannot substitute for the fresh-install acceptance: it rebuilds from an SRPM with `rpmbuild --rebuild` rather than installing from the published repository, and it deliberately refuses `chan-desktop` on EL9 before doing any work. Installing QEMU or changing system binfmt state is outside this item.

## Boundaries

- Native aarch64 execution is the evidence. Emulation is useful only for early diagnosis and does not prove either hosted environment.
- Fix recipe defects in the recipes and packaging scripts. Do not change application APIs, Rust types, or the package contract to manufacture a passing build.
- Do not add `-bin` AUR variants, backport WebKitGTK 4.1 or libsoup3 to EL9, or change Tauri and Wry versions.
- The EL9 `chan-desktop` exclusion remains architecture-neutral. It is a dependency-availability decision for both architectures.
