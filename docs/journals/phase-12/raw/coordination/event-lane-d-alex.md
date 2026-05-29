# Channel: @@LaneD -> @@Alex

Append-only. @@LaneD writes here; @@Alex reads. Never edit prior entries.
Escalate human-decision blockers: release scope (e.g. fold in slice 5/6 or
defer), new secrets to provision, version/semver calls.

## 2026-05-27 @@LaneD -> @@Alex
Two gates for the next release: a missing signing secret + a desktop scope call.

1. **Provision `TAURI_SIGNING_PRIVATE_KEY`** (GitHub Actions repo secret on
   fiorix/chan). `gh secret list` shows the 6 `APPLE_*` secrets but NOT this one.
   `release.yml`'s macOS desktop job hard-requires it and exits 1 if absent, so
   the next tag's Release workflow will fail there even after I fix the RPM bug.
   (Optional companion `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` only if your updater
   key has a passphrase.) Secret VALUE never goes in chat/journal/commit - set it
   via the GitHub UI / `gh secret set`. The key is the Tauri updater signing key
   generated per desktop/CLAUDE.md. If you can't find it, you may need to
   regenerate (which changes the updater pubkey baked into the app).

2. **Desktop scope for this patch.** The Release workflow ALWAYS builds + signs +
   notarizes the macOS desktop and signs an updater payload on a tag (the build
   jobs aren't publish-gated). That whole path has NEVER run (it was gated behind
   the linux job that failed on v0.15.5), so it's unproven. Two options:
   (a) ship signed desktop + updater this patch - provision the secret above, and
   accept the first real notarization run is on the cut (I'll dry-run it first);
   (b) defer desktop - I'd make the macOS desktop job opt-in so a CLI-only patch
   can ship without the signing path. Slice 5 (desktop updater UX) in the
   carryover suggests (a) is the intent - confirm?

No rush - the cut is gated on lanes A/B/C landing anyway. Detail:
docs/journals/phase-12/lane-d/journal.md. Full findings to @@Lead on
event-lane-d-architect.md.