import { api } from "../api/client";

export const editorToolsPrefs = $state({
  stripTrailingWhitespaceOnSave: false,
});

let stripWhitespacePersistInflight: Promise<void> = Promise.resolve();

export function applyEditorToolPreferences(prefs: {
  strip_trailing_whitespace_on_save?: boolean;
}): void {
  editorToolsPrefs.stripTrailingWhitespaceOnSave =
    prefs.strip_trailing_whitespace_on_save ?? false;
}

export function persistStripTrailingWhitespaceOnSave(value: boolean): Promise<void> {
  editorToolsPrefs.stripTrailingWhitespaceOnSave = value;
  stripWhitespacePersistInflight = stripWhitespacePersistInflight
    .catch(() => {})
    .then(async () => {
      const cfg = await api.config();
      if (cfg.preferences.strip_trailing_whitespace_on_save === value) return;
      await api.updateConfig({
        ...cfg,
        preferences: {
          ...cfg.preferences,
          strip_trailing_whitespace_on_save: value,
        },
      });
    });
  return stripWhitespacePersistInflight;
}
