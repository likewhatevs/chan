// Rich Prompt: window-global visibility + draft for the floating markdown
// bubble that overlays the active terminal's bottom. One bubble per window;
// it renders only inside the ACTIVE terminal (TerminalTab gates on its own
// `active` prop + this `visible` flag), so switching the active terminal moves
// it. The draft is one shared logical input - it persists across toggles and
// active-terminal switches, and is cleared on submit.
//
// Toggled by Cmd+Shift+P (App.svelte onWindowKey) and the terminal right-click
// "Show/Hide Rich Prompt" entry. Submit routes through the terminal WS `prompt`
// frame into the per-session write queue (see tabs.svelte.ts
// sendPromptToActiveTerminal); this module owns only the UI state.

export const richPrompt = $state<{ visible: boolean; draft: string }>({
  visible: false,
  draft: "",
});

export function toggleRichPrompt(): void {
  richPrompt.visible = !richPrompt.visible;
}

export function showRichPrompt(): void {
  richPrompt.visible = true;
}

export function hideRichPrompt(): void {
  richPrompt.visible = false;
}
