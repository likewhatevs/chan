// Demo-only hooks used by marketing embeds. Production launcher code leaves this
// disabled, so the normal local-workspace action remains unchanged.

interface DemoState {
  enabled: boolean;
  // Non-null repurposes the local FolderPlus button into "Reset demo data" (the
  // home hero); null keeps the real New-workspace flow (the manual's empty embed).
  reset: (() => Promise<void> | void) | null;
}

export const demoState = $state<DemoState>({
  enabled: false,
  reset: null,
});

export interface DemoMode {
  reset?: (() => Promise<void> | void) | null;
}

export function setDemoMode(mode: DemoMode | null): void {
  demoState.enabled = mode !== null;
  demoState.reset = mode?.reset ?? null;
}

export function setDemoReset(reset: (() => Promise<void> | void) | null): void {
  setDemoMode(reset ? { reset } : null);
}

export async function resetDemo(): Promise<void> {
  await demoState.reset?.();
}
