// Per-tenant leadership derivation for the launcher's create-control gating.
//
// Leadership is per tenant (one tenant prefix = one leader window). The watch
// feed publishes `library.leaders[prefix] = <leader window_id>`; this launcher
// "owns" a tenant's leader when that window_id is one of its live window.open
// handles. A leader-only action on a tenant is allowed when the tenant is
// leaderless (creating establishes leadership at the window's first /ws connect)
// or this launcher owns its leader. This is the client half of the honest-client
// mint gate; the server re-checks the acting window_id against the tenant leader.
//
// Reactivity keys on `library.leaders` (reactive $state): a handle add/remove
// always coincides with a feed push (mint, discard, election), so a consumer in
// a reactive context re-evaluates whenever leadership actually moves.

import { library } from "./library.svelte";
import { hasWindowHandle } from "./windowManager.svelte";

/** The leader window_id of a tenant, or null when leaderless. */
export function tenantLeader(prefix: string): string | null {
  return library.leaders[prefix] ?? null;
}

/** Whether this launcher holds the live handle of a tenant's leader window. */
export function ownsTenantLeader(prefix: string): boolean {
  const leader = library.leaders[prefix];
  return leader !== undefined && hasWindowHandle(leader);
}

/** Whether a leader-only action on a tenant is allowed from this launcher:
 * leaderless (creating establishes leadership) or this launcher owns the
 * leader window. */
export function canActOnTenant(prefix: string): boolean {
  const leader = library.leaders[prefix];
  return leader === undefined || hasWindowHandle(leader);
}
