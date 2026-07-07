// New slide deck command (Apps category): creates a slides-seeded draft
// through the create-draft endpoint's `kind` body and opens it in the
// active pane, mirroring New diagram (createDiagramAndOpen). The seed's
// `chan: kind: slides` frontmatter routes the editor straight into the
// slides layout. Availability follows the workspace gate. See
// state/commands.ts for the Command shape and the workspaceOnly helper.

import { registerCommands, workspaceOnly } from "../commands";
import { api } from "../../api/client";
import { noteDraftCreated, setTransientStatus } from "../store.svelte";
import { openInActivePane } from "../tabs.svelte";

/// Create a slides-seeded draft (a real, promotable draft) and open it
/// in the active pane. Mirrors createDiagramAndOpen: surface it in the
/// tree and refresh graph/workspace before opening. Exported so
/// App.svelte's runCommand can route the `app.slides.new` dispatch
/// (welcome Apps menu, host bridge) through the same handler.
export async function createSlidesAndOpen(): Promise<void> {
  try {
    const { path } = await api.createDraft("slides");
    await noteDraftCreated(path);
    await openInActivePane(path);
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
