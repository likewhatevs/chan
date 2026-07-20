# Make The Arch aarch64 AUR Validation Gate Publication

Status: accepted scope for v0.74.0. Small and well understood; it is the remaining work of the v0.73.0 `packaging-aarch64-validation` item, which closes at v0.73.0 GA with the AUR aarch64 leg wired but not gating.

## Problem

`aur-validate-arm` in `.github/workflows/publish-downstream.yml:442` runs the full AUR recipe validation natively on aarch64 for both pkgbases, on every GA release, and its result cannot stop a bad package from reaching the AUR.

The job is scheduled by the same condition as the x86_64 validator: a successful `workflow_run` of the Release workflow on a GA `v*` tag with no `-` in it (`.github/workflows/publish-downstream.yml:443-451`), plus an opt-in `inputs.aur_validate_arm` branch for manual dispatch. It runs `runs-on: ubuntu-24.04-arm` (`.github/workflows/publish-downstream.yml:459`) over `matrix: pkgbase: [chan, chan-desktop]` with `fail-fast: false` (`.github/workflows/publish-downstream.yml:456-458`). It imports and signature-verifies the Arch Linux ARM rootfs (`.github/workflows/publish-downstream.yml:470-483`) and then runs the same entry point the x86_64 job runs, `packaging/distros/arch/build-in-ci.sh` (`.github/workflows/publish-downstream.yml:485-488` against `.github/workflows/publish-downstream.yml:414-417`).

Two lines make it advisory. It carries `continue-on-error: true` (`.github/workflows/publish-downstream.yml:454`), and `aur-publish` depends only on the x86_64 leg: `needs: [aur-auth, aur-validate]` (`.github/workflows/publish-downstream.yml:499`). The AUR recipes declare `arch=('x86_64' 'aarch64')`, so the AUR serves aarch64 users a recipe whose native build is observed but never enforced. `packaging/distros/arch/README.md:63` and `packaging/distros/arch/README.md:65` state this arrangement as the v0.73.0 position and name v0.74.0 as the release that adds the dependency.

The advisory posture is deliberate and time-limited. The Arch Linux ARM leg had never executed end to end, and v0.73.0 is also the first release that can reach the AUR at all: v0.72.0 published nothing there because the post-install verification of the shipped systemd user unit ran without the privileges it needs and failed inside the build container (CHANGELOG.md, Unreleased/Fixed, "The Arch AUR packages can publish again"). Making an unproven job a hard dependency would have risked blocking a first AUR publication for a second consecutive release. Once the leg has passed once, that reason is spent and the advisory job is pure risk: a broken aarch64 recipe publishes silently.

## Desired contract

After one real GA release in which both `aur-validate-arm` cells passed, aarch64 becomes a publication gate:

- `continue-on-error: true` is removed from `aur-validate-arm`.
- `aur-validate-arm` joins `aur-publish`'s `needs`, alongside `aur-auth` and `aur-validate`.

A failing aarch64 build, namcap error, install, or smoke then holds back both AUR pushes, exactly as an x86_64 failure already does per `packaging/distros/arch/README.md:65`. `aur-validate-arm` still uploads no `aur-metadata-*` artifact; the x86_64 matrix remains the sole producer of the published `PKGBUILD` and `.SRCINFO` (`.github/workflows/publish-downstream.yml:419-427`).

The manual-dispatch opt-in interacts with the new dependency and must be resolved in the same edit. On `workflow_dispatch`, `aur-validate-arm` runs only when `inputs.aur_validate_arm` is true (`.github/workflows/publish-downstream.yml:444-447`), while `aur-publish` has no such condition (`.github/workflows/publish-downstream.yml:491-498`). A skipped dependency skips its dependents, so a plain `targets=aur` dispatch without the ARM switch would stop publishing. The item must pick one of: make `aur-publish` tolerate a skipped ARM dependency with an explicit `if` that still fails on a failed ARM result, or drop the dispatch-time opt-in so ARM always runs on an AUR dispatch. Whichever is chosen, a dispatch that skips ARM must never be able to publish while an ARM failure silently disappears.

## Acceptance

1. Read the real v0.73.0 GA `publish-downstream` run and record both `aur-validate-arm` cells, `chan` and `chan-desktop`, by run URL and conclusion. This is the gating evidence and it is read, not assumed. Only the owner can read it, since it requires the authenticated GitHub run history for `fiorix/chan`.
2. If both cells passed: apply the two-line change, and prove it with `actionlint` over `.github/workflows/publish-downstream.yml` plus a reading of the resulting job graph that shows `aur-publish` blocked by a failed ARM cell and unblocked by a passing one.
3. The next GA release after the change shows `aur-publish` running after four green validation cells (two architectures times two pkgbases) and both AUR repositories at the new `X.Y.Z-1`. The owner reads this from the run and from the AUR RPC; there is no local substitute.
4. If either cell failed, this item becomes fixing the Arch Linux ARM recipe path, and gating follows the fix. The untried surface, all of it inside `packaging/distros/arch/build-in-container.sh`, is:
   - the ALARM rootfs import and keyring bootstrap: the `archlinuxarm-keyring` branch that runs `pacman-key --init` and `pacman-key --populate archlinuxarm` (`packaging/distros/arch/build-in-container.sh:23-26`), reached only from the imported rootfs at `.github/workflows/publish-downstream.yml:470-483`;
   - the rolling `pacman -Syu --needed --noconfirm base-devel curl namcap sudo` on that branch (`packaging/distros/arch/build-in-container.sh:26`), which upgrades the whole rootfs rather than the split `-Sy`/`-Su` the upstream Arch image takes (`packaging/distros/arch/build-in-container.sh:28-29`);
   - dependency resolution through `makepkg --cleanbuild --force --syncdeps --noconfirm` (`packaging/distros/arch/build-in-container.sh:96`), against `makedepends` that Arch Linux ARM must satisfy for both recipes (`packaging/distros/arch/aur/chan/PKGBUILD.in:19`, `packaging/distros/arch/aur/chan-desktop/PKGBUILD.in:30`);
   - the native aarch64 Tauri build for `chan-desktop`, `npm --prefix web ci` then `cargo build --frozen --release -p chan-desktop` against the host WebKitGTK stack (`packaging/distros/arch/aur/chan-desktop/PKGBUILD.in:45`, `packaging/distros/arch/aur/chan-desktop/PKGBUILD.in:55`);
   - the namcap error gate, including whether the x86_64 waiver set still fits an aarch64 package (`packaging/distros/arch/build-in-container.sh:70-121`).
   The install and smoke steps that follow, `pacman -U`, `chan --version`, `cs --help`, `systemd-analyze verify` on the user unit, the packaged-upgrade refusal, and the `chan-desktop` stamp, desktop entry and five icon sizes (`packaging/distros/arch/build-in-container.sh:125-160`), are proven on x86_64 and unproven on aarch64.

## Boundaries

- Removing `continue-on-error` and adding the `needs` entry happen in the same edit and the same commit. The workflow must never contain a job that both gates publication and is allowed to fail, in either order of application.
- The change is confined to `.github/workflows/publish-downstream.yml` and the prose in `packaging/distros/arch/README.md:63` and `packaging/distros/arch/README.md:65` that describes the ARM leg as observed rather than gating. No recipe, script, or `build-in-ci.sh` change belongs in this item unless acceptance step 4 turns it into a recipe fix.
- No other publication path is touched. COPR, Launchpad, and the Docker jobs are separate jobs in the same workflow and separate from the Release workflow entirely; aarch64 on COPR is already green and closed by the v0.73.0 item's evidence.
- The x86_64 leg stays the sole source of published AUR metadata. Adding aarch64 as a gate must not make it a producer, or the two matrices could race to define what gets pushed.
