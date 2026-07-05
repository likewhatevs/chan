// Open-count of pane-LOCAL modals (ones mounted inside a pane surface,
// not at the App root). Examples: the MCP-env info dialog in
// TerminalTab, the import-contacts wizard in FileBrowserSurface. Their
// open-state lives in component-local `$state`, so App.svelte's
// paneChordBlocked() cannot see it directly the way it sees the
// app-root modals (prompt / confirm / conflict / team / warnings).
//
// Each such modal registers via markPaneModalOpen() while visible and
// calls the returned releaser when it closes (drive both from one
// $effect with a cleanup return). paneChordBlocked() then bails on
// `openCount > 0` so the pane-flip command never flips the pane hidden
// behind the dialog, matching the guard the app-root modals already get.

export const paneModalGuard = $state<{ openCount: number }>({ openCount: 0 });

/// Mark one pane-local modal as open. Returns an idempotent releaser;
/// call it (or return it from a `$effect`) when the modal closes or the
/// host unmounts. Clamped at zero so a double-release can't drive the
/// count negative and wedge the guard off.
export function markPaneModalOpen(): () => void {
  paneModalGuard.openCount += 1;
  let released = false;
  return () => {
    if (released) return;
    released = true;
    paneModalGuard.openCount = Math.max(0, paneModalGuard.openCount - 1);
  };
}
