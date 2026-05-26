// Per-File-Browser-instance scoped /ws subscription manager (phase-11
// Slice E).
//
// Slice A built the client-side registry (`fbTreeInstances`,
// `fbDirSubscriberCount`) and the transport (`watchSubscription()` with
// `subscribeDir` / `unsubscribeDir`). Slice C built the server-side
// per-socket `ScopeRegistry` with its own authoritative refcount. This
// module is the glue: it lets each File Browser / Graph surface drive
// scope subscriptions through the instance registry so that
//
//   * on open, an instance subscribes to the drive root (`""`) and the
//     server broadcasts root-level fs changes to it,
//   * on directory expand, the instance subscribes to that dir's scope;
//     a second instance expanding the same dir REUSES the subscription
//     (no redundant `sub` frame), and
//   * on collapse / dispose, the instance unsubscribes; the LAST
//     instance to drop a dir sends the `unsub` that lets the server tear
//     the scope down.
//
// The cross-instance refcount lives in `fbDirSubscriberCount` (the union
// of every live instance's `subscribedDirs`). We send a wire `sub` only
// on the 0 -> 1 transition and a wire `unsub` only on the 1 -> 0
// transition; the server keeps its own per-socket refcount as the
// authority, so this is purely a client-side dedupe of redundant frames.
//
// Why a separate module and not methods on the store: the store owns the
// socket handle + the registry data; this module owns the lifecycle
// POLICY (when to emit sub/unsub, how to re-sync after a reconnect). One
// reason to change each.

import {
  ensureFbTreeInstance,
  fbDirSubscriberCount,
  fbTreeInstance,
  fbTreeInstances,
  disposeFbTreeInstance,
  watchSubscription,
} from "./store.svelte";
import type { WatchScopeDir } from "../api/types";

/// The drive root scope. Always implicitly watched by the server; we
/// send a `sub` for it once (on the first instance) for symmetry but
/// never `unsub` it, since every File Browser conceptually watches the
/// root for its whole lifetime.
const ROOT: WatchScopeDir = "";

/// Register a File Browser / Graph instance and subscribe it to the
/// drive root. Idempotent: safe to call on every (re)mount. Returns the
/// instance's metadata record (the `$state` proxy) so the caller can read
/// its expand/selection bookkeeping if needed.
export function fbWatchRegister(instanceId: string) {
  const inst = ensureFbTreeInstance(instanceId);
  // `ensureFbTreeInstance` seeds `subscribedDirs[""] = true`, so the root
  // is already in the instance's set. Make sure the wire `sub` has gone
  // out at least once (the server is idempotent on the root, so a repeat
  // is harmless; sending unconditionally here covers the case where this
  // is the very first instance and the socket is already open).
  watchSubscription()?.subscribeDir(ROOT);
  return inst;
}

/// Subscribe an instance to a directory scope (on expand). Records the
/// dir in the instance's `subscribedDirs` and emits a wire `sub` only if
/// this is the first instance to watch `dir` (the 0 -> 1 cross-instance
/// transition). The root is handled by `fbWatchRegister`; calling this
/// with `""` is a harmless idempotent no-op.
export function fbWatchSubscribe(instanceId: string, dir: WatchScopeDir): void {
  const inst = ensureFbTreeInstance(instanceId);
  if (inst.subscribedDirs[dir]) return; // already subscribed by this instance
  inst.subscribedDirs[dir] = true;
  // After the record, the count includes this instance. A count of 1
  // means we are the first; emit the wire frame. The root scope is
  // always considered active, so we let the server dedupe it.
  if (dir === ROOT || fbDirSubscriberCount(dir) === 1) {
    watchSubscription()?.subscribeDir(dir);
  }
}

/// Unsubscribe an instance from a directory scope (on collapse). Clears
/// the dir from the instance's `subscribedDirs` and emits a wire `unsub`
/// only on the LAST instance to drop it (the 1 -> 0 cross-instance
/// transition), which lets the server tear the scope down. The root scope
/// is never torn down from here.
export function fbWatchUnsubscribe(instanceId: string, dir: WatchScopeDir): void {
  if (dir === ROOT) return; // root stays subscribed for the instance's life
  const inst = fbTreeInstance(instanceId);
  if (!inst || !inst.subscribedDirs[dir]) return;
  delete inst.subscribedDirs[dir];
  // After the removal, a count of 0 means no live instance watches the
  // dir anymore; emit the wire `unsub` so the server drops the scope.
  if (fbDirSubscriberCount(dir) === 0) {
    watchSubscription()?.unsubscribeDir(dir);
  }
}

/// Dispose an instance: unsubscribe every dir it held (driving the
/// refcount transitions, so a dir watched by no other instance is torn
/// down) and then drop its registry record. Call on unmount / pane close.
/// Unsubscribing BEFORE disposing is required: `fbDirSubscriberCount`
/// walks the registry, so the record must still be present while we
/// decide which dirs reached 0.
export function fbWatchDispose(instanceId: string): void {
  const inst = fbTreeInstance(instanceId);
  if (!inst) return;
  // Snapshot the keys: `fbWatchUnsubscribe` mutates `subscribedDirs`.
  for (const dir of Object.keys(inst.subscribedDirs)) {
    if (dir === ROOT) continue;
    fbWatchUnsubscribe(instanceId, dir);
  }
  disposeFbTreeInstance(instanceId);
}

/// Reconcile an instance's subscriptions against a target set of dirs
/// (the dirs it currently has expanded, root excluded). Subscribes any
/// newly-expanded dir and unsubscribes any collapsed one. Idempotent;
/// safe to call from a reactive effect that recomputes the expanded set.
/// The root is always kept subscribed and is not part of `targetDirs`.
export function fbWatchReconcile(
  instanceId: string,
  targetDirs: Iterable<WatchScopeDir>,
): void {
  const inst = ensureFbTreeInstance(instanceId);
  const want = new Set<WatchScopeDir>();
  for (const dir of targetDirs) {
    if (dir === ROOT) continue;
    want.add(dir);
  }
  // Subscribe newly-wanted dirs.
  for (const dir of want) {
    if (!inst.subscribedDirs[dir]) fbWatchSubscribe(instanceId, dir);
  }
  // Unsubscribe dirs this instance no longer wants.
  for (const dir of Object.keys(inst.subscribedDirs)) {
    if (dir === ROOT) continue;
    if (!want.has(dir)) fbWatchUnsubscribe(instanceId, dir);
  }
}

/// Re-establish every live instance's scope subscriptions on the current
/// socket. The server's `ScopeRegistry` is per-socket, so a reconnect
/// starts with an empty subscription set; wiring this as the watcher
/// socket's `onReady` callback replays the union of all instances'
/// desired scopes so the tree keeps receiving scoped `fs` frames after a
/// transient disconnect. Re-sends the root too (idempotent server-side).
export function fbWatchResyncAll(): void {
  const sub = watchSubscription();
  if (!sub) return;
  sub.subscribeDir(ROOT);
  const sent = new Set<WatchScopeDir>([ROOT]);
  for (const inst of Object.values(fbTreeInstances.byId)) {
    for (const dir of Object.keys(inst.subscribedDirs)) {
      if (sent.has(dir)) continue;
      sent.add(dir);
      sub.subscribeDir(dir);
    }
  }
}
