// New slide deck command (Apps category): creates a slides-seeded draft
// through the create-draft endpoint's `kind` body and opens it in the
// active pane, mirroring New diagram (createDiagramAndOpen). The seed's
// `chan: kind: slides` frontmatter routes the editor straight into the
// slides layout, and the caret lands at the end of the seed's "# Slide 1"
// heading so the deck is ready to type. Availability follows the
// workspace gate. See state/commands.ts for the Command shape and the
// workspaceOnly helper.

import { registerCommands, workspaceOnly } from "../commands";
import { api } from "../../api/client";
import { firstSlideHeadingCaret } from "../../editor/slides";
import { noteDraftCreated, setTransientStatus } from "../store.svelte";
import {
  activeTabInPane,
  issueCaretCommand,
  layout,
  openInActivePane,
} from "../tabs.svelte";

/// Land the caret at the end of the fresh deck's first heading line,
/// computed from the loaded document (never a hard-coded offset). Without
/// an explicit caret request the open falls back to the editor's
/// document-start default, which parks the caret inside the frontmatter
/// block, and the post-load saved-caret restore can land a stale per-path
/// offset when a deleted draft's untitled-N name is reused. Runs after
/// openInActivePane resolves so this command is the last caret intent and
/// wins over that restore.
function landCaretAtFirstHeading(path: string): void {
  const node = layout.nodes[layout.activePaneId];
  if (!node || node.kind !== "leaf") return;
  const tab = activeTabInPane(node);
  if (!tab || tab.kind !== "file" || tab.path !== path) return;
  const at = firstSlideHeadingCaret(tab.content);
  if (at === null) return;
  issueCaretCommand(tab, at, at);
}

/// Create a slides-seeded draft (a real, promotable draft) and open it
/// in the active pane with the caret at the end of "# Slide 1". Mirrors
/// createDiagramAndOpen: surface it in the tree and refresh
/// graph/workspace before opening. Exported so App.svelte's runCommand
/// can route the `app.slides.new` dispatch (pane hamburger Apps rows,
/// host bridge) through the same handler.
export async function createSlidesAndOpen(): Promise<void> {
  try {
    const { path } = await api.createDraft("slides");
    await noteDraftCreated(path);
    await openInActivePane(path);
    landCaretAtFirstHeading(path);
  } catch (err) {
    console.warn("[chan] createSlides failed", err);
    setTransientStatus(`New slide deck failed: ${(err as Error).message}`);
  }
}

registerCommands([
  {
    id: "app.slides.new",
    title: "New slide deck",
    category: "Apps",
    keywords: ["slides", "presentation", "deck", "present"],
    available: (ctx) => workspaceOnly(ctx),
    run: () => {
      void createSlidesAndOpen();
    },
  },
]);
