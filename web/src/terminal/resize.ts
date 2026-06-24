export type FitLike = {
  fit(): void;
};

export type SizedTerminal = {
  cols: number;
  rows: number;
};

export function runTerminalFit(
  fit: FitLike | null,
  term: SizedTerminal | null,
  onStatusDetail: (detail: string) => void,
): boolean {
  try {
    fit?.fit();
    if (term) onStatusDetail(`${term.cols}x${term.rows}`);
    return true;
  } catch {
    return false;
  }
}

export function createTrailingFitScheduler(runFit: () => void, delayMs = 120): {
  schedule(): void;
  clear(): void;
} {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return {
    schedule() {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = null;
        runFit();
      }, delayMs);
    },
    clear() {
      if (!timer) return;
      clearTimeout(timer);
      timer = null;
    },
  };
}
