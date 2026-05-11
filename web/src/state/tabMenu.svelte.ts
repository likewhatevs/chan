// Per-tab popover ("..." bubble) state. Lives outside the FileTab
// schema because it is purely transient UI: no need to persist or to
// survive serialization. Anchored to the trigger button's bounding
// rect so the bubble can position itself in a portal-like layer.

export type AnchorRect = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};

export const tabMenu = $state<{
  openForTabId: string | null;
  anchor: AnchorRect | null;
}>({ openForTabId: null, anchor: null });

export function openTabMenu(tabId: string, anchor: AnchorRect): void {
  tabMenu.openForTabId = tabId;
  tabMenu.anchor = anchor;
}

export function closeTabMenu(): void {
  tabMenu.openForTabId = null;
  tabMenu.anchor = null;
}

export function toggleTabMenu(tabId: string, anchor: AnchorRect): void {
  if (tabMenu.openForTabId === tabId) closeTabMenu();
  else openTabMenu(tabId, anchor);
}
