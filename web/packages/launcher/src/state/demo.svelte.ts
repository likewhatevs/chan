// Demo-only hooks used by marketing embeds. Production launcher code leaves this
// disabled, so the normal local-workspace action remains unchanged.

interface DemoState {
  enabled: boolean;
  reset: (() => Promise<void> | void) | null;
}

export const demoState = $state<DemoState>({
  enabled: false,
  reset: null,
});

export function setDemoReset(reset: (() => Promise<void> | void) | null): void {
  demoState.enabled = reset !== null;
  demoState.reset = reset;
}

export async function resetDemo(): Promise<void> {
  await demoState.reset?.();
}
