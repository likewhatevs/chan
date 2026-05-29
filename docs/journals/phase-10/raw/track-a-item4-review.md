# Phase 10 Track A Item 4 - Architect Review

Reviewer: @@Architect. Date: 2026-05-26. Baseline reviewed: working tree on
top of `23fa3aa` (icons + `desktop/README.md`, uncommitted).

Verdict: CHANGES REQUESTED. Icons and README are approved. `desktop/design.md`
(a named audit target) is untouched and now contradicts `desktop/CLAUDE.md`;
fix that and add the journal note before close-out.

## Passes

- Icons (Subtask A): regenerated set is correct on the master PNG - dark
  `#101112` ground, orange `#ef8f58` enso, macOS squircle with transparent
  margins, enso centered on its fitted circle (not the left-shifted
  bbox-centering trap). Pending only the recorded Cmd+Tab / Dock check on the
  built `.app`.
- `desktop/README.md`: Download section is now fresh-state and accurate
  (macOS plus Linux `.deb` / `.AppImage` published, Windows not yet).

## Blocking gaps: desktop/design.md

`desktop/design.md` was a named audit target in both the bootstrap and the
source handoff, is currently untouched, and contradicts `desktop/CLAUDE.md`:

1. Section 5 bundled-binary architecture (`Contents/Resources/bin/chan`,
   `/usr/lib/chan-desktop/chan`, "The bundled `chan` binary lives at...").
   CLAUDE.md states there is NO bundled binary; the app is self-contained
   (`5d2e299`). This is the most stale section. Must be corrected.
2. Section 8 self-upgrade: `https://chan.app/dl/latest/VERSION` plus
   "generalise the upgrade module out of `chan` into chan-core" is superseded
   by tauri-plugin-updater (the minisign / manifest path documented in
   CLAUDE.md). Rewrite to fresh-state.
3. Sections 6 and 7 Windows MSI: Windows desktop is paused, not killed
   (roadmap: "when Windows desktop support returns"; README says "not
   published yet"). KEEP Windows as forward-looking design, but mark it
   clearly deferred / not-yet-shipped rather than a current distribution
   channel. This is @@Architect's call on the one judgment item.

## Process gaps

- `tauri.conf.json` updater URL: leaving it is the CORRECT outcome (it is the
  chan-prod-setup-owned desktop manifest, not the removed CLI `/dl/latest`),
  but there is no record of the decision. Record it in the journal so the keep
  is auditable.
- No journal note exists. Add a focused note under `docs/journals/phase-10/`
  recording what the docs audit changed vs. left intentionally (incl. the
  tauri.conf.json updater-URL keep-decision) and how the icon was verified on
  the built `.app`.

## Task back to @@IconDocs (append as a NEW task; do not amend the finished one)

Item 4 review: icons approved, README approved. Before close-out, address
`desktop/design.md` (named audit target, untouched, contradicting
`desktop/CLAUDE.md`):

1. Section 5 - remove the bundled-`chan`-binary description; the app is
   self-contained, no binary shipped.
2. Section 8 - replace the `/dl/latest/VERSION` plus chan-core-extraction
   upgrade plan with the current tauri-plugin-updater reality.
3. Sections 6/7 - keep Windows as forward-looking / deferred design, not a
   current distribution channel.

Then add a phase-10 journal note recording the docs audit (changed vs.
intentionally-kept, incl. the tauri.conf.json updater-URL keep-decision) and
the icon Cmd+Tab / Dock verification on the built `.app`. Re-send for review
when done. Do not commit or tear down yet.

---

## Re-review: APPROVED (2026-05-26)

The change-request items are all addressed and verified.

- `desktop/design.md` reconciled against `desktop/CLAUDE.md`: §5
  self-contained runtime, §8 tauri-plugin-updater, §6/§7 Windows deferred.
  Coherence fixes in §1/§3.2/§3.5/§4 + the `ChanDesktop.app` -> `Chan.app`
  bundle name were flagged, not silent.
- @@Architect independently grounded the two new claims against source:
  `release-desktop.yml` builds only `linux-x86_64` + `macos-aarch64` (the old
  "Linux amd64/arm64" was the stale claim), and `productName` is `Chan` so
  `bundle/macos/Chan.app` is correct. Not invented.
- tauri.conf.json updater-URL keep-decision recorded.
- Two out-of-scope items correctly flagged, not absorbed.

### One small addition before commit (no re-review needed)

- design.md §10.2 "chan-tunnel-server from chan-core": Phase 5 collapsed
  chan-core into this workspace. Authorized to fix as a one-line factual
  correction in this same pass (you are already in the file). Include it in
  the item-4 commit.

### Not item 4

- `desktop/CLAUDE.md` `Chan_0.14.0` worked example: fold into the next
  version-bump chore, not this commit.

### Known-state, not a blocker

- Live Cmd+Tab switcher re-confirm is pending Alex's relaunch + `killall
  Dock`. The on-disk `Chan.icns` is byte-verified correct (sha `3c736e03`);
  the off-center switcher render was a stale launch-time icon, not a file
  defect. Note this explicitly in your summary.

### Close-out sequence

1. Make the §10.2 one-line fix above.
2. Write your summary and post your FINAL task update; close the task.
3. Commit atomically (path-scoped `git add` for README + design.md + the 17
   icons + the journal note; chained `git add` + `git diff --staged --stat`
   audit + `git commit`; verify `git show --stat HEAD`). Do not stage the
   unrelated dirty files (`phase-11*`, `attachments/`). Do not push.
4. Send a task back confirming the loop is closed and you are ready to tear
   down. Wait for @@Architect ack before tearing down.

## Standing process note for @@IconDocs (and all agents)

Write all review-loop information into the TASK and the JOURNAL SUMMARY
directly. Do not send it as a chat message for Alex to copy/paste to the
architect. The task update + journal note are the canonical record the
architect reads; Alex is not a courier.
