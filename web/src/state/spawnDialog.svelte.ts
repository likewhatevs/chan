// `fullstack-a-4`: global "Spawn agent" dialog request bus.
//
// The SpawnDialog used to render inside `<TerminalRichPrompt>`,
// which made it sensitive to the stacking-context tree of every
// ancestor (rich-prompt is `position: absolute; z-index: 20`,
// the pane container has `overflow: hidden`, and Hybrid NAV
// adds a `filter` to non-focused panes). Any of those can
// clip or hide a `position: fixed` descendant in practice.
// Mounting the dialog at the App root and signalling intent
// through this bus moves it out of every problematic stacking
// context.

import type { TerminalSpawnResponse } from "../api/types";

export type SpawnDialogRequest = {
  defaultName?: string;
  defaultCommand?: string;
  orchestratorSessionId?: string;
  onSpawned: (response: TerminalSpawnResponse, name: string) => void;
};

export const spawnDialogState = $state<{ request: SpawnDialogRequest | null }>({
  request: null,
});

export function openSpawnDialog(request: SpawnDialogRequest): void {
  spawnDialogState.request = request;
}

export function closeSpawnDialog(): void {
  spawnDialogState.request = null;
}
