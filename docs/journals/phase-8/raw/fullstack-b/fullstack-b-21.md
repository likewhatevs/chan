# fullstack-b-21: codesign bundled chan sidecar (notarization fix)

Owner: @@FullStackB
Date: 2026-05-21

## Goal

Make the bundled chan sidecar inside `Chan.app/Contents/MacOS/chan`
satisfy Apple notarization: Developer ID signature, hardened
runtime enabled, secure timestamp. Unblock `ci-8` dry-run #4
+ the v0.11.2 cut path.

Single commit. Smallest viable fix preferred — the universal2 +
externalBin re-architecture is a separate post-v0.11.2 task.

## Background

### `ci-8` dry-run #3 result

`chan-v0.11.99-dryrun.3` (HEAD `2c9ff0e`, includes `-b-20`):

* **Linux**: green. First-ever working Linux desktop bundle.
* **macOS**: Apple notarization REJECTED (fast-reject ~20s).
  Submission `7f327f46-8c5a-430d-80fb-95d174109d50`.

`xcrun notarytool log` returned:

```json
{
  "status": "Invalid",
  "statusSummary": "Archive contains critical validation errors",
  "archiveFilename": "Chan_0.11.1_x64.dmg",
  "issues": [
    {
      "path": "Chan_0.11.1_x64.dmg/Chan.app/Contents/MacOS/chan",
      "message": "The binary is not signed with a valid Developer ID certificate.",
      "architecture": "arm64"
    },
    {
      "path": "Chan_0.11.1_x64.dmg/Chan.app/Contents/MacOS/chan",
      "message": "The signature does not include a secure timestamp.",
      "architecture": "arm64"
    },
    {
      "path": "Chan_0.11.1_x64.dmg/Chan.app/Contents/MacOS/chan",
      "message": "The executable does not have the hardened runtime enabled.",
      "architecture": "arm64"
    }
  ]
}
```

All three errors are on the SAME path: the bundled chan
sidecar. `chan-desktop` itself + the `.app` outer signature
+ the `.dmg` itself all pass.

### Root cause

`-b-20` swapped `bundle.externalBin = ["binaries/chan"]` for
`bundle.macOS.files = { "MacOS/chan": "binaries/chan-aarch64-apple-darwin" }`.

Tauri's signing pass walks binaries it knows about:
`chan-desktop` (its own bin), `externalBin` entries (declared
sidecars), and the `.app` wrapper. `bundle.macOS.files` is a
"copy these files in" primitive — it does NOT route the file
through the signing pass.

The cargo-built `target/release/chan` that the `chan-bin`
Makefile recipe stages carries an ad-hoc signature from rustc
/ macOS, which is why `-b-20`'s local `codesign --verify` passed
(`--verify` accepts any valid signature; doesn't check WHO
signed). Apple's notary correctly rejects it as not Developer
ID, not hardened-runtime, not timestamped.

### `-b-20` lineage

`-b-20` empirically ruled out option (iii) (literal triple in
`externalBin`) because Tauri 2's tauri-build appends the
target triple unconditionally. Option (ii) (per-platform
`bundle.macOS.externalBin`) wasn't conclusively tested — see
"Fix options" below.

## Authorization

**Authorization: yes**, covers any combination of:

* `desktop/src-tauri/tauri.conf.json` (bundle.* edits).
* `desktop/Makefile` (chan-bin recipe, app-notarized recipe,
  any new codesign step).
* `desktop/CLAUDE.md` (documentation refresh).

If the fix needs to extend into `.github/workflows/release-desktop.yml`,
coordinate with @@CI before touching workflow YAML.

@@FullStackB may proceed without further @@Alex confirmation.

## Fix options (implementer picks based on what actually works)

### Option A — sign chan inside chan-bin recipe (recommended)

Add a `codesign --force --options=runtime --timestamp --sign
"$APPLE_SIGNING_IDENTITY"` step to `desktop/Makefile`'s
`chan-bin` recipe, AFTER staging the binary to
`src-tauri/binaries/chan-<host-triple>` and BEFORE Tauri's
bundle pass runs. Gate on `APPLE_SIGNING_IDENTITY` being set
(skip the sign if empty; local dev without an identity still
gets an unsigned-chan staging, which is fine because the
signed paths — `app-signed` / `app-notarized` — gate on
`sign-prereqs` which enforces the identity exists).

Tauri's subsequent bundle-signing pass should accept an
already-validly-signed chan (Tauri's bundler typically only
RE-signs files it owns; the chan binary is just a payload
file from its perspective).

Pro: most targeted; chan is signed at the moment it enters
the bundling chain.
Pro: no Tauri config change; preserves `-b-20`'s working
`bundle.macOS.files` shape.
Con: adds a codesign call to the Makefile. Verify Tauri's
later bundle-sign doesn't clobber the chan signature.

### Option B — re-sign chan AFTER tauri bundle, before notarytool

Add a `codesign --force --options=runtime --timestamp --sign
"$APPLE_SIGNING_IDENTITY" "$APP/Contents/MacOS/chan"` step in
`desktop/Makefile`'s `app-notarized` recipe between
`cargo tauri build` and `xcrun notarytool submit`. Then
re-sign the `.app` deeply because mutating any file inside a
signed bundle invalidates the outer signature.

Pro: locally scoped to the notarization path.
Con: fragile — re-signing sequence has to be exactly right
or the outer signature stays invalid. Option A is cleaner.

### Option C — switch to bundle.macOS.externalBin (the proper Tauri primitive)

Move the chan sidecar declaration to a per-platform override:
`bundle.macOS.externalBin = ["binaries/chan"]` (instead of
the cross-platform `bundle.externalBin`). Verify whether
Tauri 2's per-platform `externalBin` override behaves
differently from the top-level key WRT target-triple
auto-expansion. If it does the right thing (expands once,
finds the staged `binaries/chan-aarch64-apple-darwin`),
Tauri's signing pass picks chan up automatically — no
custom codesign step needed.

Pro: cleanest; uses the documented Tauri primitive that the
signing pass was designed around.
Con: not guaranteed to work without empirical test;
`-b-20`'s top-level externalBin had the unconditional
triple-append bug. Per-platform may inherit the same
behaviour.

### Recommendation

Try Option C first (one-line tauri.conf.json change to test
hypothesis). If it works, ship that — it's the right Tauri
primitive. If per-platform externalBin has the same triple-
append bug as the top-level, fall back to Option A.

## Acceptance criteria

* Locally: `cargo tauri build --bundles app,dmg` from
  `desktop/` produces a `.dmg` whose
  `Chan.app/Contents/MacOS/chan` passes
  `codesign --verify --strict --deep` against the
  Developer ID Application identity AND `codesign -dv
  --verbose=2` shows `flags=0x10000(runtime)` +
  `Timestamp=...` (not "Timestamp=none").
* Optionally: `xcrun notarytool submit` against the new
  `.dmg` locally returns `status: Accepted` (skip if quota
  / API constraints make this expensive; the CI dry-run #4
  is the authoritative test).
* Pre-push gate clean.
* `desktop/CLAUDE.md` "Bundled chan sidecar" section
  updated to document whatever fix shape lands (Option A
  → note the chan-bin codesign step; Option C → note the
  externalBin per-platform shape that works).

## How to start

1. Reproduce locally: run `make app-notarized` from
   `desktop/` against the current HEAD. Confirm
   `codesign -dv --verbose=2 .../Chan.app/Contents/MacOS/chan`
   shows ad-hoc / no-timestamp / no-runtime — matches the
   notarization rejection.
2. Try Option C (one-line tauri.conf.json change to
   per-platform externalBin). Run `cargo tauri build`. If
   the chan-target-triple file resolves and the resulting
   `.app/Contents/MacOS/chan` carries Developer ID +
   hardened runtime + timestamp, ship that.
3. If Option C fails, fall back to Option A: add the
   codesign step to the chan-bin recipe.
4. Append "approved + commit readiness" once verified
   locally.

## Coordination

* **Blocks v0.11.2 tag-cut** (still). The cut path stays:
  -b-21 commit → @@CI cuts `chan-v0.11.99-dryrun.4` →
  green DMG → @@WebtestB second-Mac verify → @@Alex
  "cut it" → @@Systacean cuts `chan-v0.11.2`.
* **No overlap** with @@FullStackA's wave-2 frontend work.
* **No overlap** with @@CI's lane unless the fix needs
  workflow-YAML help (Option D, not currently recommended).
* **Composes with -b-15 + -b-16** (chan binary resolution
  + version probe) — the resolver path is unchanged;
  this task only changes how the bundled chan is signed.

## Suggested commit subject

```
chan-desktop: codesign bundled chan sidecar for notarization (fullstack-b-21)
```

(Or for Option C: "chan-desktop: bundle.macOS.externalBin
for chan sidecar (fullstack-b-21)".)

## Open questions

(populated as you investigate)

## 2026-05-21 — implementation note

### Option C ruled out

Tested first per the task body's recommendation:
`bundle.macOS.externalBin = ["binaries/chan"]` (per-platform
override). Result: tauri-build rejects the field with

```
unknown field `externalBin`, expected one of `frameworks`,
`files`, `bundle-version`, ..., `signing-identity`,
`hardened-runtime`, ..., `dmg`
```

Tauri 2's per-platform `MacOSConfig` does NOT include an
`externalBin` key. Option C is fundamentally unavailable in
this Tauri version.

### Option A landed

Added a codesign step to `desktop/Makefile`'s `chan-bin`
recipe, gated on `APPLE_SIGNING_IDENTITY` being non-empty:

```make
@if [ -n "$$APPLE_SIGNING_IDENTITY" ]; then \
    codesign --force --options=runtime --timestamp \
        --sign "$$APPLE_SIGNING_IDENTITY" $(CHAN_BIN); \
fi
```

This runs AFTER the chan binary is staged to
`src-tauri/binaries/chan-<host-triple>` and BEFORE Tauri's
bundler picks it up via `bundle.macOS.files`. Tauri's later
bundle-signing pass on the .app preserves chan's already-valid
Developer ID signature (Tauri's signing only re-signs files it
owns; the chan binary is a payload-via-files entry from its
perspective).

### Verification

`make app-signed` from `desktop/` end-to-end on aarch64 Mac
with `APPLE_SIGNING_IDENTITY` auto-detected from keychain:

```
$ codesign -dv --verbose=2 .../Chan.app/Contents/MacOS/chan
Executable=.../Chan.app/Contents/MacOS/chan
Identifier=chan-aarch64-apple-darwin
Format=Mach-O thin (arm64)
CodeDirectory v=20500 size=51813 flags=0x10000(runtime) ...
Authority=Developer ID Application: Alexandre Fiori (W73XV5CK3N)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=21 May 2026 at 09:36:30
TeamIdentifier=W73XV5CK3N
Runtime Version=26.4.0
```

All three notarization-rejection criteria satisfied:
* Developer ID Application signature: ✓
* Hardened runtime (`flags=0x10000(runtime)`): ✓
* Secure timestamp (`Timestamp=...`, not "none"): ✓

`codesign --verify --strict --deep` on `Chan.app` exits 0.

### Changes landed

* **`desktop/Makefile`** — `chan-bin` recipe extended with the
  gated codesign step + a comment block explaining why
  (`bundle.macOS.files` is a copy-verbatim primitive that
  skips Tauri's signing pass; rustc's ad-hoc signature is
  insufficient for Apple notarization). Skipped when
  `APPLE_SIGNING_IDENTITY` is empty so local dev / unsigned
  builds aren't affected.
* **`desktop/CLAUDE.md`** — "v0.11.2 hotfix: aarch64-only DMG,
  no externalBin" section expanded with the `-b-21`
  notarization-fix rationale + the Makefile codesign snippet.
  Documents Option C's "not in Tauri 2's MacOSConfig" finding
  so the next implementer doesn't retry it.

### Acceptance criteria — verification

| Criterion                                                                                  | State                                                            |
|--------------------------------------------------------------------------------------------|------------------------------------------------------------------|
| `Chan.app/Contents/MacOS/chan` passes `codesign --verify --strict --deep` with Developer ID | Verified locally, exit 0.                                         |
| `codesign -dv --verbose=2` shows `flags=0x10000(runtime)` + `Timestamp=...`                | Verified locally (see snippet above).                             |
| Pre-push gate                                                                              | Workspace fmt + clippy `-D warnings` + test + no-default-features build all green. |
| `desktop/CLAUDE.md` updated with the chosen fix shape                                      | Landed in the "Bundled chan sidecar" section.                     |

`notarytool submit` skipped per the task body's optional-quota
clause; CI dry-run #4 is the authoritative test.

### Coordination footprint

* No file overlap with @@FullStackA / @@CI / @@Systacean.
* Makefile edit is in scope per the task body's authorization
  list ("`desktop/Makefile` (chan-bin recipe, app-notarized
  recipe, any new codesign step)").
* tauri.conf.json untouched (Option C ruled out before any
  shape change landed).

### Suggested commit subject

```
chan-desktop: codesign bundled chan sidecar for notarization (fullstack-b-21)
```

Touches:
* `desktop/Makefile`
* `desktop/CLAUDE.md`

Hotfix priority — committing per task body's "After -b-21
commits @@CI cuts dry-run #4" framing.
