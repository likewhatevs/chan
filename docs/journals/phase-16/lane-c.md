# @@LaneC — Pre-flight + Desktop launcher

Read `round-1-plan.md` first. You own the desktop<->SPA settings move
end-to-end, so the DT1/P2 coupling stays inside your lane. Round-1: LAND P1;
DESIGN P2/DT1/DT2 (build lands round-2 unless a slice is ready sooner).

## Round-1 tasks

1. **P1 — `cs` symlink boot/pre-flight check (NON-BLOCKING).** During
   workspace boot + pre-flight in chan-server, detect whether `cs` is on
   `$PATH`. If missing, offer to create/fix it as a sibling symlink to the
   running `chan` binary (`std::env::current_exe()`; pattern at
   `crates/chan/src/update.rs:549-554`) via a NEW server route. If creation
   fails (read-only dir / off PATH), surface it and let the user
   accept/continue — never block boot. Lands on the SPA for BOTH `chan` and
   desktop. Surface it on the pre-flight surface
   (`web/src/components/PreflightOverlay.svelte` + server
   `crates/chan-server/src/routes/preflight.rs`).

2. **P2 (design + start) — relocate the desktop onboarding modal to the
   SPA.** Phase-14 already moved the readiness overlay
   (`PreflightOverlay.svelte` + `/api/preflight`). Still in the desktop
   launcher: the onboarding modal (`desktop/src/main.js:440-594`
   `showPreflightDialog` — workspace summary + Semantic/Reports toggles +
   warnings; backend `compute_workspace_preflight`,
   `desktop/src-tauri/src/main.rs:708-741`). Move it OPEN-THEN-CONFIGURE:
   server opens the workspace -> SPA shows summary+toggles on first load ->
   server reconfigures semantic/reports. ONE shared serve+desktop flow.

3. **DT1/DT2 (design + start) — chan-desktop launcher redesign.** Spec +
   screenshots: `desktop-redesign-draft/draft.md` (+ image.png/-1/-2).
   - DT1: header -> `[chan icon] Workspaces ... [New] [theme]`; columns
     `ON | WHERE`; row -> `[on/off] [computer|home|network icon] [path or
     URL] ... [Open ▾]`; REMOVE the per-row settings gear (config lives in
     the SPA now — this is the other half of P2); add an INBOUND vs OUTBOUND
     indicator for remote/URL workspaces.
   - DT2: `[New]` opens a new window with 3 choices (Local dir / Outbound
     remote / Inbound remote), each swapping layout+options like the Team
     Work dialog's real-estate selector.

## Files you OWN

`crates/chan-server/src/routes/preflight.rs` + a new symlink route,
`web/src/components/PreflightOverlay.svelte`,
`desktop/src/{main.js,index.html}`, `desktop/src-tauri/src/*`.

## Coordination

- DT1 removes the gear; P2 must move those settings into the SPA first (or
  together) so no setting vanishes. You own both — sequence P2 ahead.
- `PreflightOverlay.svelte` mounts in `App.svelte`; if you add a mount
  point, poke @@Lead so it doesn't collide with @@LaneD's frontend work.
- P1's new server route must not touch @@LaneA's control_socket/wire.

## Verify

`make pre-push` green. P1: smoke on a real `chan serve` where `cs` is absent
from PATH (offer + create + the non-blocking continue path). Desktop changes
are WKWebView-only — gate green + flag empirically-unverified for @@Host to
confirm. Post the commit sha to `event-lane-c.md` and poke @@Lead.
