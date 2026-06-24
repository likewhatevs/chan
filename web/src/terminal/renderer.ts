export type RefreshableTerminal = {
  rows: number;
  refresh?: (start: number, end: number) => void;
};

export function refreshTerminalRows(term: RefreshableTerminal | null): void {
  if (!term) return;
  term.refresh?.(0, Math.max(0, term.rows - 1));
}

export function shouldUseWebglRenderer(isDesktop: boolean, os: string): boolean {
  return !(isDesktop && os === "linux");
}
