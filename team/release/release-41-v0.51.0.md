# Phase 41 — v0.51.0: Windows desktop support, published (unsigned)

Feature line `windows-headless-devserver`, rebased onto v0.50.0 and merged to
`main`. A single development line (not a multi-lane round): eight commits that take
the experimental Windows desktop from a CI-only artifact to a published download.
The dev host can build neither the Tauri/GTK desktop nor Windows targets, so all
Windows compilation + smoke is validated by CI (the `windows-latest` runner in
`release-desktop.yml`, and now `release.yml`); local proofs covered the
cross-platform Rust and the web / marketing surfaces.

## Theme

Make Windows a real download. The terminal stops requiring Git BASH and defaults
to the user's shell, `chan open` integrates with a running devserver over a named
pipe and hands off to a running desktop, service install unifies under one
`--service` flag, and the release pipeline builds and publishes the (unsigned) NSIS
installer plus a standalone Windows CLI — surfaced on the install page. Signing is
deferred until the SSL.com Authenticode cert issues (procedure since moved
to the team's private dev/ tree).

## What landed (by commit)

- **`a0dde18b` fix(chan-server): force process exit on the shutdown deadline.** On
  Windows a task that outlives the graceful-shutdown deadline could keep the process
  alive; force the exit once the deadline lapses.
- **`15993577` feat(devserver): `--service` with a Windows backend.** Unifies the
  previous `--systemd` / `--launchd` flags into one cross-platform `--service` flag.
- **`4e8893ed` feat(terminal): default to the user's shell on Windows.** Drops the
  Git-BASH requirement (and the in-app install gate); a Windows-shell builder
  classifies PowerShell / cmd / POSIX with a `CHAN_SHELL` override.
- **`b4570f39` feat(devserver): Windows named-pipe discovery.** `chan open` finds
  and registers into a running devserver over a named pipe (the unix-socket analogue).
- **`16facc4a` build(windows): NSIS installer.** Bundles both SPA bundles plus the
  console `chan.exe` as a resource and brands the NSIS installer (icons, header /
  sidebar images), built via `tauri.windows.conf.json`.
- **`312345f1` fix(windows): `chan open` hands off to a running desktop** from the
  bundled `chan.exe`, so a CLI open focuses the existing window instead of starting
  a second server.
- **`ed6ea3e0` feat(web-marketing): Windows downloads on the install page.** Adds the
  installer + CLI cards as optional, manifest-derived downloads (like the gateway
  debs) so they light up once a release publishes them and fall back otherwise.
- **`ed270411` build(windows): publish the unsigned installer + CLI from CI.** A
  non-blocking `windows-artifacts` job in `release.yml` builds and uploads
  `Chan_<version>_x64-setup.exe` and `chan-x86_64-pc-windows-msvc.zip`.

## Notes

- **Unsigned, temporarily.** The installer ships unsigned (SmartScreen may warn on
  first run); Authenticode signing (W.1/W.2 of the Windows signing procedure,
  since moved to the team's private dev/ tree) lands when the SSL.com cert issues.
- **Best-effort Windows.** The release job runs `continue-on-error`; a Windows build
  failure still ships the Linux and macOS release and omits Windows until the next
  good build.
