# Refuse `chan upgrade` on a Distro-Packaged Desktop

> Status: shipped in [v0.72.0](../../release/release-v0.72.0.md).

Status: accepted for v0.72.0. Grounded against `59acd07a` (`v0.71.0`) on 2026-07-19.

## Summary

`chan upgrade` refuses to self-upgrade when the build carries the distro-package marker, because the package manager owns the installed files. That refusal only covered the standalone CLI. The desktop personality routed straight to the desktop updater without ever reading the marker, so a chan-desktop installed from a distro package answered `chan upgrade` by routing into the desktop updater path instead of telling the user to update through their package manager.

chan-desktop already ships as a distro package, so this is reachable by users of published packages today: `packaging/distros/fedora/chan-desktop.spec` and `packaging/distros/debian/chan-desktop/debian/rules` have both set the marker since v0.67.0. Any packaging source added later only widens the audience.

## Evidence

The marker is `CHAN_PACKAGED`, read at build time in `crates/chan/src/update.rs`:

```rust
pub fn packaged_via() -> Option<&'static str> {
    option_env!("CHAN_PACKAGED")
}
```

Before this item, every `packaged_via()` call site was in `update.rs`: the startup banner (`maybe_print_banner`), the background probe (`run_probe`), and the CLI upgrade (`run_upgrade`). `crates/chan/src/lib.rs` dispatched `Command::Upgrade` on the personality alone, and `Personality::Desktop` reached `cmd_upgrade_desktop` -> `chan_server::handoff::try_upgrade` with no marker check anywhere on that path.

A packaged CLI build refuses with the message `upgrade_blocked_message` produces. For the Fedora COPR build, which exports `CHAN_PACKAGED=rpm`:

```text
Error: this build of chan is managed by the system package manager (rpm); self-upgrade is disabled. Update with sudo dnf upgrade.
```

The hint is per manager: `deb` reads `sudo apt upgrade`, and a marker value with no arm of its own reads `your system package manager`.

The packaged desktop produced no such message. `packaging/distros/fedora/chan-desktop.spec` exports `CHAN_PACKAGED=rpm`, builds `chan-desktop`, installs it into `%{_bindir}`, and symlinks `chan` and `cs` next to it, so the packaged desktop binary is exactly the one that answers `chan upgrade` as `Personality::Desktop`, with no marker check anywhere on that route.

Two facts bound how bad the observed behavior was, and neither makes the gap safe to leave:

- The desktop-side updater is macOS-only. `desktop_handle_upgrade` in `desktop/src-tauri/src/main.rs` is `#[cfg(not(target_os = "macos"))]`-stubbed to an error, and the on-launch check `spawn_launch_update_check` is a no-op off macOS. So on Linux, where the packaged builds live today, `chan upgrade` on a packaged desktop ended in `chan-desktop could not upgrade: desktop upgrade over hand-off is not supported on linux`, and with no desktop running it first launched the GUI (`launch_desktop_then_upgrade`) to get there. The user got an error that names the wrong reason instead of the one instruction that works, plus, in that no-desktop case, a window they did not ask for.
- The overwrite the marker exists to prevent is one `cfg` away. Any platform where the desktop updater does work, packaged by any recipe that sets `CHAN_PACKAGED`, would have had a packaged install download and replace package-manager-owned files.

## Desired Contract

- `chan upgrade` on a build with the marker set refuses, with `upgrade_blocked_message`, in every personality. No install path may reach an updater on a packaged build.
- The decision is made before the personality is consulted, so a personality or install path added later inherits the refusal instead of having to remember it.
- `--check` is refused with the same message. A packaged desktop does not report an available desktop-updater release: that release is not one this build can install, and the release the package manager will ship is a separate thing on its own schedule. The refusal already names the package manager, which is the command that does work. This also keeps the two personalities identical, since the CLI path has always refused `--check` the same way.
- An unpackaged build is unchanged: standalone still replaces the CLI tarball, desktop still drives `tauri-plugin-updater`.
- The refusal never tells the user to run a chan command that would fail.

## Boundaries

- `crates/chan/src/lib.rs`: an `UpgradeRoute` enum plus the pure `decide_upgrade_route(personality, packaged)`, and the `Command::Upgrade` arm matching on it. `cmd_upgrade_desktop`, `launch_desktop_then_upgrade`, and the handoff wire are untouched.
- `crates/chan/src/update.rs`: `packaged_upgrade_refusal(Option<&str>) -> Option<String>` wraps the existing `upgrade_blocked_message` so the packaged decision is one pure function taking the marker as an argument; `run_upgrade` keeps its own guard through the same helper, so the public function stays safe on its own.
- No new marker values and no new package-manager hints. The `aur` hint arm belongs to the AUR packaging item.
- The banner and probe guards are already correct and are not touched.
- Nothing changes inside the Tauri app. The desktop's own on-launch update check is macOS-only, and no macOS packaging sets `CHAN_PACKAGED` today; if one ever does, that check needs the same guard on the desktop side, where the CLI-side route cannot reach it.
- No new dependency, no CLI flag, no config key.

## Acceptance Checks

Automated, in `cargo test -p chan`:

- `tests::upgrade_route_refuses_a_packaged_build_in_every_personality`: both personalities return `Refuse` for a set marker, the message names the manager and says self-upgrade is disabled, and it does not point back at `chan upgrade`. The route is resolved before `--check` is read, so check-only refuses too.
- `tests::upgrade_route_installs_on_an_unpackaged_build`: with no marker, standalone routes to the CLI tarball replace and desktop routes to the desktop updater.
- `update::tests::test_packaged_upgrade_refusal_only_fires_for_packaged_builds`: no marker yields no refusal; a set marker yields the manager-specific message. `packaged_via()` is a compile-time `option_env!`, so the marker is passed as an argument rather than flipped at runtime.

Manual, on a packaged build:

- Build chan-desktop with `CHAN_PACKAGED=rpm` (or install the COPR/PPA package), then run `chan upgrade` and `chan upgrade --check`. Both exit non-zero with the package-manager refusal, no GUI window is launched, and no file under the package manager's ownership is written.
- The same binary built without the marker still reaches the desktop updater path.

Scoped commands:

```sh
cargo fmt --check
cargo clippy -p chan --all-targets -- -D warnings
cargo test -p chan
```
