# task-Lead-ChanDesktop-2 — addendum: extended patterns, workspace correction, bundle-id flag

From: @@Lead. To: @@ChanDesktop. Extends task-Lead-ChanDesktop-1
(do both in one pass).

## Correction: you are IN the root workspace

round-1-plan.md said desktop/ is a separate cargo workspace — wrong;
`desktop/src-tauri` (crate `chan-desktop`) is a root-workspace
member (only gateway/ is separate). Your own-gate is therefore
`cargo clippy -p chan-desktop --all-targets` / `cargo test -p
chan-desktop` from the repo root. @@Chan has been told to run their
workspace-wide sweeps with `--exclude chan-desktop` so desktop
warnings stay yours.

## Extended archaeology patterns (recon under-counted)

Sweep desktop/ again with the wider net:

```
grep -rniE '(systacean|desktacean|desktest|desktect|@@(Host|CI|Architect|Lane|FullStack|Webtest)|round-[0-9]+|wave-[0-9]+|slice [a-z0-9]+|track [ab]\b)'
```

The old desktop-side team used @@Desktect/@@Desktacean/@@Desktest
handles — expect them in comments.

## Bundle identifier — REPORT, do not change

`desktop/src-tauri/tauri.conf.json` has
`"identifier": "com.chanwriter.desktop"` — the chan-writer naming
survives in the bundle id. Changing it affects updater continuity,
notarization history, and macOS app identity, so it is a host
decision: note it in your completion task (with any other
identifier-shaped leftovers you find, e.g. in Info.plist values or
the updater endpoint config) and I will survey @@Alex. Do NOT
rename it unilaterally.

Completion: fold into your task-1 completion file.
