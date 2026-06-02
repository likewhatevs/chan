// Rich Prompt: window-global VISIBILITY for the floating markdown bubble that
// overlays the active terminal's bottom. One bubble per window; it renders only
// inside the ACTIVE terminal (TerminalTab gates on its own `active` prop + this
// `visible` flag), so the global toggle shows the bubble on whichever terminal
// is active.
//
// The prompt TEXT is no longer held here: each terminal's bubble is backed by a
// per-terminal chan-workspace DRAFT (`tab.richPromptDraftPath` -> a real
// `Drafts/<name>/draft.md`), so the text + any pasted images live on disk and
// any MCP/disk agent can read the media as files. RichPrompt.svelte owns that
// draft binding; this module owns only the show/hide flag.
//
// Toggled by Cmd+Shift+P (App.svelte onWindowKey) and the terminal right-click
// "Show/Hide Rich Prompt" entry. Submit routes through the terminal WS `prompt`
// frame into the per-session write queue (see tabs.svelte.ts
// sendPromptToActiveTerminal).

export const richPrompt = $state<{ visible: boolean }>({
  visible: false,
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
