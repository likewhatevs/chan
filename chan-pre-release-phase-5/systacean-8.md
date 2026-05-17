# @@Systacean task 8: BUG-WT5-C round-2 — hydrate session ids BEFORE Svelte mounts terminal tabs

Owner: @@Systacean (Alex routed away from @@Frontend this round to
balance capacity)
Status: REVIEW
Severity: HIGH — still blocks the headline terminal-persistence
contract.
Source: [webtest-1](./webtest-1.md) round-6 re-smoke on the
post-frontend-6 binary.

## Context for @@Systacean

This is a frontend/store fix even though the work is being routed
to @@Systacean this round. The change lives in
`web/src/state/store.svelte.ts` and `web/src/state/tabs.svelte.ts`
(plus possibly `web/src/components/TerminalTab.svelte`). The Rust
side is unaffected — the server-side
`crates/chan-server/src/terminal_sessions.rs` already does the
right thing per [systacean-5](./systacean-5.md). The bug is purely
in the client's bootstrap ordering between hash-restore, session-
blob fetch, Svelte mount, and the store's auto-save.

## What's wrong

[frontend-6](./frontend-6.md) added
`hydrateTerminalSessionsFromLayout()` and a bootstrap call to copy
`tsid` / `tseq` from the session blob onto the hash-restored
layout. That's the right intent, but the call lands too late:

* `restoreLayout(fromHash)` synchronously mounts `TerminalTab`.
* `TerminalTab.svelte`'s `$effect → start() → connect()` fires on
  mount, **before** the bootstrap reaches
  `await api.getSession()` → `applySessionSidecars()` →
  `hydrateTerminalSessionsFromLayout()`.
* `connect()` therefore opens the WebSocket with
  `session=undefined`, the server allocates a fresh PTY, and the
  client persists the new `tsid` over the old one.

A second compounding race: the store's auto-save fires a PUT with
the in-memory layout (still tsid-less) before hydration grafts the
old id, clobbering the persisted blob too.

Network trace from Webtest A (PID 78898 on /private/tmp/chan-test-phase5):

```
GET  /api/session?w=c38a0791    # body still has old tsid
GET  /api/index/status
PUT  /api/session?w=c38a0791    # writes layout WITHOUT tsid <- race 2
GET  /api/session?w=c38a0791    # confirms blob is now tsid-free
```

Live shell PID measured across four reloads: 79138 → 79861 → 80347
→ 8163055e6df6…  (fresh every time).

## Fix shape (Webtest A's option (a), recommended for @@Systacean)

Invert the bootstrap order so the session blob is fetched first,
then `restoreLayout` runs with both inputs and initialises the
hash-restored terminal tabs with their persisted `tsid` / `tseq`
**before** any Svelte mount.

Sketch:

```ts
// store.svelte.ts bootstrap path
const fromHash = fresh ? null : readLayoutHash();
const remote = fresh ? null : await api.getSession();
const sessionLayout = remote && !isLegacyLayoutPayload(remote)
  ? (remote as SessionPayload).layout
  : null;

try {
  if (fromHash) {
    // Pass the session-blob layout in so restoreLayout can graft
    // tsid/tseq onto matching terminal tab descriptors before
    // Svelte ever sees them.
    await restoreLayout(fromHash, sessionLayout);
    if (remote && !isLegacyLayoutPayload(remote)) {
      applySessionSidecars(remote as SessionPayload);
    }
  } else if (!fresh && remote) {
    // existing no-hash branch already works; keep as-is.
    ...
  }
}
```

`restoreLayout` grows a second parameter (`sessionLayout: Layout | null`)
and walks both in lockstep when building the in-memory layout —
copying `terminalSessionId` and `lastSeq` from the session-blob
terminal tabs onto the matching hash-restored ones during
construction. `hydrateTerminalSessionsFromLayout()` from
frontend-6 can either be folded into the new `restoreLayout`
signature or kept as a small helper called from inside.

## Race-2 belt-and-suspenders (Webtest A's option (c))

Also suppress the store's auto-save PUT until bootstrap has marked
the layout as "hydrated". Cheapest path: a `bootstrapHydrated`
rune in the store, gated to `false` during bootstrap, flipped to
`true` after the hydrate pass. Auto-save reads the rune and
no-ops while it's false.

Without this, even with fix (a) in place a slow GET could leave
the auto-save firing during the window where the layout is built
but `applySessionSidecars` hasn't run yet. Cheap insurance.

## Acceptance criteria

* On a chan-test-phase5 drive with one terminal tab, four
  successive `location.reload()` calls produce the same shell PID
  (`echo $$`) all four times. `tsid` in the session blob is
  unchanged across reloads.
* Server logs show **attach** (not create) on the WS upgrade for
  every reload after the first.
* The PUT-before-hydrate race no longer appears in the network
  trace; PUTs to `/api/session?w=<key>` always carry the current
  `tsid` once the terminal has reported its id.
* `chan-desktop` reload of a single drive window also preserves
  the shell. (Webtest A flagged this was likely broken in the
  same way; fix is the same code path.)
* `npm --prefix web run check` + `npm --prefix web test -- --run`
  + `npm --prefix web run build` all green.
* Extend the regression test added in frontend-6
  (`web/src/state/tabs.test.ts`) so it asserts:
  * The hash-restored tab descriptor has `terminalSessionId` set
    BEFORE the first observable mount-time read (assert ordering,
    not just final state).
  * Auto-save PUT bodies during the hydration window do not blank
    `terminalSessionId`.

## Hardening expectations

* `restoreLayout` signature change has no other callers besides
  bootstrap (verify via grep before landing); if it does, decide
  whether to default the new arg to `null` or update all callers.
* The hydration logic must remain a no-op when the session blob
  carries no layout (fresh launches), so the fresh-launch path
  in [webtest-1.md](./webtest-1.md) still works.

## Coordination

* @@Webtest A re-runs the BUG-WT5-C round-3 repro (four reloads,
  same shell PID) on the fixed bundle. They also finally get to
  exercise the two-attach + idle-close cases that BUG-WT5-C
  blocked.
* If multi-attach in a plain browser stays unreachable because of
  the per-tab `w=` key (frontend-7 working as intended), file a
  webtest-N follow-up that drives multi-attach from a raw
  WebSocket client (`websocat` or a Node script) so the registry's
  multi-attach contract gets one live exercise pass.

## Progress

* 2026-05-17 @@Systacean: picked up after task check found this
  unblocked and re-routed to Systacean. Inspecting store bootstrap
  and terminal layout hydration before changing ordering.
* 2026-05-17 @@Systacean: implemented pre-mount terminal-session
  hydration and the bootstrap save gate. Focused tests and full web
  gate are green.

## Completion notes

Implemented Webtest A option (a): `bootstrap()` now fetches the
session blob before hash layout restore, derives the session layout,
and calls `restoreLayout(fromHash, sessionLayout)`. `restoreLayout`
copies `tsid` / `tseq` onto terminal tab descriptors while building
the in-memory layout, before Svelte can mount `TerminalTab` and run
its connect effect.

Implemented option (c): `scheduleSessionSave()` and pagehide flushes
are gated by `bootstrapHydrated`, so the app cannot write a
tsid-less hash-restored layout back to `/api/session` during the
bootstrap window.

New / extended tests:

* `terminal session serialization > hydrates terminal session ids
  during restore before mount-time reads`
* `session persistence bootstrap guard > does not save a tsid-less
  layout while bootstrap hydration is pending`

Verified:

* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build`

Live shell-PID-across-reloads transcript is not run in this lane;
@@Webtest A owns the browser re-smoke per coordination above.
