# systacean-3: Round-1 close — patch version bump + tag + push

Owner: @@Systacean
Date: 2026-05-19

## Goal

Once Round 1's bug fixes have all landed on `main` (per
@@Architect's commit-grouping plan), cut the patch release.

* Bump `Cargo.toml` workspace version from `0.11.0` →
  `0.11.1` (or whatever @@Architect picks at close).
* Tag `chan-v0.11.1` on the release commit.
* Push branch + tag to the remote.
* If @@CI has the release workflow wired by close, the tag
  push triggers an artifact build; otherwise this is a local-
  only release for now and the artifact wiring carries to
  Round 2.

## Background

Mirrors phase-7's wave-1 closeout flow. Source in
[`../request.md`](../request.md) under "Round 1 — bug sweep +
new build".

## Acceptance criteria

* All wave-1 fixes landed on `main`.
* Pre-push gate green: `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test`, `web/npm run
  check`, `web/npm run build`, `scripts/pre-push`.
* Version bumped in every `Cargo.toml` that pins a workspace-
  member version.
* Tag created with the standard release-commit message shape.
* Push completed; commit + tag confirmed on the remote.
* @@Architect notified via poke event.

## How to start

Wait until @@Architect appends the commit-grouping plan to this
task file (or a sibling `architect-N.md`) listing the order in
which the fixes should land. Do not version-bump or tag until
@@Architect signals Round-1 close.

## 2026-05-20 — task re-activated as v0.11.1 (rich-prompt mini-wave patch)

Per the round restructure trail:

* `systacean-3` cancelled for original Round-1 close
  (request.md detour pulled BGE-small model gating
  forward; no binary cut at Round-1 close).
* Re-activated for the patch-release wave: Round-1
  closeout + rich-prompt mini-wave (13 commits on top
  of the Round-1 closeout set already in HEAD).
* Pre-authorization + tag-body draft published in
  [`../architect/commit-plan-v0.11.1.md`](../architect/commit-plan-v0.11.1.md)
  ("RE-ACTIVATED 2026-05-20" section).
* Gate-3 cleared by @@Alex's verbatim "ok let's do
  it" (transcribed by @@Architect in the inbound
  event log).
* Gate-1 cleared in the GO poke (`0525ae5`): all 13
  mini-wave commits confirmed in HEAD.

## 2026-05-20 — v0.11.1 cut + push complete

Executed the full version-bump + tag + push sequence.

### Pre-push gate (all green)

| Check                                          | Result          |
|------------------------------------------------|-----------------|
| `cargo fmt --check`                            | clean           |
| `cargo clippy --all-targets -- -D warnings`    | clean           |
| `cargo test --workspace`                       | all passing     |
| `RUSTFLAGS=-D warnings cargo build --no-default-features` | clean (systacean-8 follow-up `c1e9c41` unblocked this) |
| `cd web && npm run check` (svelte-check)       | 0e 0w           |
| `cd web && npm test` (vitest)                  | 544/544         |
| `cd web && npm run build`                      | built           |

### Version bump

5 files flipped `0.11.0` → `0.11.1`:

* `Cargo.toml` (workspace.package.version)
* `Cargo.lock` (refreshed by `cargo build -p chan`)
* `desktop/src-tauri/tauri.conf.json`
* `web/package.json`
* `web/package-lock.json` (chan-web entry only;
  third-party `@xterm/addon-fit ^0.11.0` left
  untouched as it's an upstream version that
  coincidentally matched our pre-bump value)

Runtime confirmation: `./target/debug/chan --version`
→ `chan 0.11.1`.

### Commit + tag

Commit `2c6680b` — `chan v0.11.1` (5 files,
+15 / -15). Pre-commit + post-commit audits clean,
no multi-agent stowaways.

Annotated tag `chan-v0.11.1` at `33dfd63` pointing
to commit `2c6680b`. Body verbatim from the commit-
plan's "Tag draft (v0.11.1)" section
(rich-prompt mini-wave bullets + Round-1 closeout +
Claude Code chord-encoding known-known).

### Push

```
git push origin main --follow-tags
```

Result:

```
To github.com:fiorix/chan.git
   18bdb34..2c6680b  main -> main
 * [new tag]         chan-v0.11.1 -> chan-v0.11.1
```

Remote verification (`git ls-remote origin`):

```
2c6680b6a6ff1e77abeee0f0ebca18641a030237  refs/heads/main
33dfd63da0f720587f1e4a329d2f10643e6a8dae  refs/tags/chan-v0.11.1
```

### Tag-triggered CI

`release-desktop.yml` fires on `chan-v*` tag — unsigned
matrix entry produces the binaries that @@WebtestA /
@@WebtestB will dogfood against per the post-tag
verification queues in their inbound channels.
Apple Developer ID notarization remains Round-2 work
per the request.md restructure.

### Recorded SHAs (subject lines are durable; SHAs may
drift on rebase but the tag is now signed/pushed so
these are stable):

* Release commit: `2c6680b` (`chan v0.11.1`).
* Tag object: `33dfd63` (`chan-v0.11.1`).
* Prior release commit: `18bdb34` (`Release v0.11.0`).

### Acceptance criteria — all met

* ✓ All wave-1 + mini-wave fixes landed on `main`.
* ✓ Pre-push gate green across all listed checks.
* ✓ Version bumped in every relevant manifest.
* ✓ Tag created with the canonical release-message
  shape from the commit-plan.
* ✓ Push completed; commit + tag confirmed on the
  remote.
* @@Architect notification poke fires next.

### What this commit ships

Per the commit-plan: 62 commits pushed in total
(Round-1 closeout + detour + rich-prompt mini-wave).
First proper unsigned `chan-v0.11.1` artifacts get
produced by `release-desktop.yml`'s tag handler.
Signed-DMG pipeline with real Apple Developer ID
keys is Round-2 work (per request.md).
