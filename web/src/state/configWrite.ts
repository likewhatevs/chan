import { api } from "../api/client";
import type { Preferences } from "../api/types";

// Every PATCH /api/config is a whole-block replacement, so two back-of-card
// surfaces doing read-modify-write concurrently can clobber a field neither
// meant to touch — e.g. the terminal-config autosave reads config before a
// just-fired hybrid_surface_themes override PATCH lands, then writes the block
// back without the override. Funnel all config writes through one chain so each
// task re-reads the latest config, applies its mutation, and writes with no
// interleaving. A mutation that returns null skips a redundant PATCH.
//
// This is a leaf module (depends only on the api client) so both
// store.svelte.ts and editorTools.svelte.ts can import the SAME chain without
// forming an import cycle — store.svelte.ts already imports editorTools, so
// editorTools must not import store back. Sharing one module-level chain is
// what makes the serialization work across all writers.
let configWriteInflight: Promise<void> = Promise.resolve();

export function updateGlobalConfigSerial(
  mutate: (prefs: Preferences) => Preferences | null,
): Promise<void> {
  configWriteInflight = configWriteInflight
    .catch(() => {})
    .then(async () => {
      const cfg = await api.config();
      const nextPrefs = mutate(cfg.preferences);
      if (!nextPrefs) return;
      await api.updateConfig({ ...cfg, preferences: nextPrefs });
    });
  return configWriteInflight;
}
