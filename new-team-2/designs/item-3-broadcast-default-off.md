# Item 3 — teams start with broadcast OFF

Lane: @@TeamFlow. Tiny. Line numbers from main @ 3ebee587.

## Today

Broadcast is OFF by default at every layer — server session spawn
(`crates/chan-server/src/terminal_sessions.rs:1128`,
`broadcast: AtomicBool::new(false)`; CreateOptions has no broadcast
field) and SPA tab creation (`web/src/state/tabs.svelte.ts` ~1162,
~1231, `broadcastEnabled: false`). There is NO broadcast field in the
team config (`crates/chan-workspace/src/teams.rs` TeamConfig) and no
serde default involved.

The ONLY enabler is the SPA team bootstrap orchestrator,
`web/src/state/teamOrchestrator.svelte.ts` ~461-473 (runs for BOTH
team new and team load, after tabs surface):

```typescript
for (const tab of allTerminalTabs()) {
  setTerminalBroadcastEnabled(tab, false);   // clear-all sweep
}
setTerminalBroadcastEnabled(leadTab, true);  // ← delete
for (const tab of workerTabs) {
  setTerminalBroadcastTarget(leadTab, tab.id, true);  // ← delete
}
```

Safe to remove: bootstrap identity prompts are delivered SERVER-side
by `spawn_and_poke_team` (control_socket.rs ~813-862) through the
write queue — not via SPA broadcast.

## Change

1. KEEP the clear-all sweep (good hygiene: a team load should not
   inherit a stale broadcast group); DELETE the lead-enable line and
   the worker-target loop.
2. Update `web/src/components/teamBootstrapOrchestrator.test.ts`:
   ~line 191 pins `expect(tab.broadcastEnabled).toBe(true)` → false
   for all tabs; ~176 pins "final broadcast membership == {lead,
   workers} exactly" → membership empty; keep the ~195-228 test that
   PRE-EXISTING broadcast groups are cleared (that behavior stays).

## Verification

Throwaway team on a standalone test server: after bootstrap completes,
every tab's broadcast toggle is OFF; manually enabling broadcast still
works (toggle + cross-window fan respected — server tests
terminal_sessions.rs ~2476-2595 unchanged).
