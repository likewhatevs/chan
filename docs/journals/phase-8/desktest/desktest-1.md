# desktest-1 — updater bridge walkthrough (held)

Owner: @@Desktest
Phase: 8, Round 3
Date cut: 2026-05-23
Status: held pending @@Alex runtime permission and
`desktacean-3` runbook.

## Goal

Verify the chan-desktop updater bridge path empirically:
an old-key install accepts a bridge update that embeds the new
production pubkey, then the bridged install accepts a later update
signed with the production key.

## Background

`desktacean-2` rotated the updater pubkey in
`desktop/src-tauri/tauri.conf.json`. `desktacean-3` is cutting the
runbook and command shape. This task waits for that runbook plus
runtime permission because it launches chan-desktop builds.

## Acceptance Criteria

1. Use throwaway app config / throwaway drives only. Do not touch
   @@Alex's running v0.12.0 chan.app session.
2. Install or run an old-key build that trusts the DEV pubkey.
3. Serve or point it at bridge update metadata and artifact signed
   with the old DEV key, embedding the production pubkey.
4. Verify the bridge update is accepted.
5. Verify the bridged app now trusts the production pubkey by
   accepting a follow-up update signed with the production key
   (local/mock is acceptable).
6. Append a walkthrough report with commands, artifact names,
   result, teardown, and any gaps.

## Coordination

Held until:

* @@Alex grants runtime-launch permission.
* `desktacean-3` provides the runbook and artifact command shape.

No product edits expected. Small patches require informing
@@Desktect and @@Desktacean first.

