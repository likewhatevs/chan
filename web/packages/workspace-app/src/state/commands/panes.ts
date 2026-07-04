// Net-new Panes commands: the focus-border colour picks (one flat row per
// colour). The reuse-existing Panes entries (splits, prev / next, close,
// flip) live in core.ts; these wrap setWindowFocusColor, which the
// runCommand switch does not expose. Register with registerCommands. See
// state/commands.ts for the Command shape and helpers.

import { registerCommands, type Command } from "../commands";
import { setWindowFocusColor, type FocusColor } from "../tabs.svelte";

function focusColor(color: FocusColor): Command {
  return {
    id: `app.pane.focusColor.${color}`,
    title: `Focus border: ${color}`,
    category: "Panes",
    keywords: ["focus", "border", "colour", "color", "pane"],
    available: () => true,
    run: () => setWindowFocusColor(color),
  };
}

registerCommands([
  focusColor("blue"),
  focusColor("orange"),
  focusColor("green"),
  focusColor("pink"),
]);
