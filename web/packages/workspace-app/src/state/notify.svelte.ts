// Tiny status-message bus. Lets leaf state modules (tabs.svelte.ts,
// editor extensions) surface a transient status string without taking
// a hard import on the store, which would create a cycle.
//
// At app boot, `store.svelte.ts` registers a handler that maps notify()
// calls to `ui.status`. Modules that import store can set ui.status
// directly; this bus is for the ones below it.

let handler: ((msg: string) => void) | null = null;

export function setNotifyHandler(fn: (msg: string) => void): void {
  handler = fn;
}

export function notify(msg: string): void {
  if (handler) handler(msg);
  else console.warn(msg);
}
