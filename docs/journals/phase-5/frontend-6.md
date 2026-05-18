# @@Frontend task 6: BUG-WT5-C — terminal reload spawns a new PTY when the URL hash is present

Owner: @@Frontend
Status: REVIEW
Severity: HIGH — breaks the headline contract of
[systacean-5](./systacean-5.md) + [frontend-4](./frontend-4.md)
("terminal reload survives").
Source: [webtest-1](./webtest-1.md) round-5 smoke, BUG-WT5-C.

## Symptom

* Server side is correct: the PTY stays in
  `terminal_sessions::Registry`, the ring is intact, attach works
  when given the right `session=<id>`.
* Frontend persists `terminalSessionId` (`tsid`) and `lastSeq`
  (`tseq`) in the per-window session blob.
* But on a plain browser reload the bootstrap discards them. The
  next TerminalTab mount opens a WebSocket with no `session=`
  param, the server allocates a fresh id, and the client writes
  the new id back over the persisted one. The old session goes
  orphan and idle-prunes; user-visible behaviour is "every reload
  is a clean shell".

## Root cause (single-file diagnosis from Webtest A)

`web/src/state/store.svelte.ts` (around line 316-329):

```ts
const fromHash = fresh ? null : readLayoutHash();
try {
  if (fromHash) {
    // URL hash wins on layout (copy-pasted links must reproduce
    // tabs verbatim), but personal UI prefs like tree expansion
    // still come from session.json.
    await restoreLayout(fromHash);
    if (!fresh) {
      const remote = await api.getSession();
      if (remote && !isLegacyLayoutPayload(remote)) {
        applySessionSidecars(remote as SessionPayload);
      }
    }
```

1. `restoreLayout(fromHash)` reconstructs the tab layout from the
   hash. The hash intentionally strips `tsid`/`tseq` (see
   `web/src/state/tabs.svelte.ts:1124` — "Only emitted in the
   per-window session payload, never in the shareable URL hash";
   that's the right call so a shared link does not leak the
   recipient's PTY ids back to the sender).
2. The follow-up `applySessionSidecars(remote)` only applies
   sidecars (tree expansion etc.); it does **not** merge the
   blob's layout's per-tab `tsid` / `tseq` onto the hash-restored
   layout.
3. TerminalTab mounts seeing `terminalSessionId === undefined`,
   creates a fresh server session, persists the new id, and the
   old PTY orphans.

Webtest A's confirming experiment: stripping the hash before
reload (`history.replaceState(null,'','/?t=<TOKEN>'); location.reload()`)
takes the bootstrap down the `!fresh` no-hash branch which DOES
restore from the session blob and DOES preserve `tsid`. So the
fix is to copy the persisted ids onto the hash-restored tabs.

## Goal

After `restoreLayout(fromHash)` completes, hydrate the
hash-restored terminal tabs with the persisted
`terminalSessionId` + `lastSeq` from the session blob whenever the
blob has matching entries.

## Proposed fix

After `restoreLayout(fromHash)`, fetch the session blob (or reuse
the existing `applySessionSidecars` call) and walk both layouts in
lockstep:

* For each pane in the hash-restored layout, walk the same pane in
  the session-blob layout (match by pane index for now).
* For each `kind: "terminal"` tab, match by position within the
  pane's tab list (single-window single-pane is the common case;
  multi-pane reorderings are out of scope for this fix).
* If a matching session-blob terminal tab carries `tsid` / `tseq`,
  copy them onto the hash-restored tab descriptor before the
  TerminalTab component mounts and opens its WebSocket.

Position-matching is the smallest defensible heuristic; a stable
per-tab id is the next-level fix (out of scope here — file as
follow-up if it bites).

Alternative considered (suggested in webtest-1): delay
TerminalTab WS open by one tick and wait for sidecars. Rejected
for this task — the sidecar path doesn't currently carry layout,
and growing it to do so is a larger contract change. The
position-match graft is local to bootstrap and unblocks the
feature today.

## Acceptance criteria

* On a chan-test-phase5 drive with one terminal tab open in a
  single window, a normal browser reload (hash present)
  reattaches to the same PTY session id, and the server logs
  show attach (not create) on the WebSocket upgrade. The PTY
  process id reported by `ps` inside the shell stays constant
  across reload.
* The ring's `missed_bytes` banner appears when the user was
  off long enough for output to roll the ring.
* `chan-desktop` reload of a single drive window keeps the
  terminal tab alive (the same bootstrap path runs in chan-
  desktop's webview).
* `npm --prefix web run check` + `npm --prefix web test -- --run`
  + `npm --prefix web run build` all green.
* Add a frontend test asserting: a session-blob layout with
  `tsid: "abc"` on its terminal tab, after running bootstrap
  with a hash that produced the same tab without `tsid`, yields
  a final tab descriptor with `tsid: "abc"`.

## Hardening expectations

* Multi-pane and reordered cases: explicitly out of scope; if the
  position-match heuristic miscarries (e.g. user reordered
  terminals between sessions), the worst case is a fresh PTY,
  which matches today's broken behaviour anyway. File a follow-up
  task if Alex hits it.
* Confirm the no-hash branch still works — there's a
  test/baseline already in `webtest-1.md` ("hash-clearing reload
  preserves the registry attachment"). Don't regress it.

## Coordination

* @@Webtest A re-runs the BUG-WT5-C repro on the fixed bundle
  and flips webtest-1's frontend-4 acceptance line from FAIL to
  PASS. They also pick up the two-attach and idle-close cases
  that BUG-WT5-C was blocking.

## Progress

* 2026-05-17 @@Frontend started after the round-5 webtest poke.
* Added `hydrateTerminalSessionsFromLayout()` in
  `web/src/state/tabs.svelte.ts`. It walks the live hash-restored layout and
  the per-window session layout by pane/terminal position, then copies
  `tsid` / `tseq` onto matching terminal tabs before `TerminalTab` mounts.
* Updated `bootstrap()` in `web/src/state/store.svelte.ts` so the URL-hash
  layout branch still applies session sidecars and now hydrates terminal
  session metadata from wrapped or legacy session payloads.
* Added a regression test in `web/src/state/tabs.test.ts`:
  `hydrates terminal session ids onto hash-restored terminal tabs`.

## Completion notes

* Verification:
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
* Build completed with existing Vite chunk-size / ineffective dynamic-import
  warnings, but no errors.
* Diff locations:
  * `web/src/state/tabs.svelte.ts`: terminal session graft helper.
  * `web/src/state/store.svelte.ts`: hash-restore bootstrap calls the graft.
  * `web/src/state/tabs.test.ts`: regression coverage.
* Webtest A should rerun BUG-WT5-C against the rebuilt bundle.
