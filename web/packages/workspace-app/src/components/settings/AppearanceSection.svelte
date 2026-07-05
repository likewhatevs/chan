<script lang="ts">
  // Appearance settings: the editor-prefs slice that skins the app and
  // editor. App theme reuses `setThemeChoice` so the chrome re-skins the
  // instant it is picked; the rest write single fields through the
  // parent's commit.

  import type {
    BubbleOverlayMode,
    EditorTheme,
    LineSpacing,
    Preferences,
    ThemeChoice,
  } from "../../api/types";
  import { setThemeChoice } from "../../state/store.svelte";
  import type { CommitFn } from "./commit";
  import SettingField from "./SettingField.svelte";
  import PillRadio from "./PillRadio.svelte";

  let { prefs, commit }: { prefs: Preferences; commit: CommitFn } = $props();

  const THEMES = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ] as const;
  const EDITOR_THEMES = [
    { value: "github", label: "GitHub" },
    { value: "google_docs", label: "Google Docs" },
    { value: "word", label: "Microsoft Word" },
  ] as const;
  const SPACING = [
    { value: "standard", label: "Standard" },
    { value: "compact", label: "Compact" },
  ] as const;
  const BUBBLES = [
    { value: "stack", label: "Inline" },
    { value: "tray", label: "Tray" },
  ] as const;
</script>

<SettingField label="Theme" hint="App-wide colour theme. System follows your OS setting.">
  <PillRadio
    name="settings-theme"
    ariaLabel="App theme"
    value={prefs.theme}
    options={THEMES}
    onselect={(v) =>
      commit(
        (p) => ({ ...p, theme: v as ThemeChoice }),
        () => {
          setThemeChoice(v as ThemeChoice);
          return Promise.resolve();
        },
      )}
  />
</SettingField>

<SettingField
  label="Editor theme"
  hint="Typography and chrome of the markdown editor only."
>
  <PillRadio
    name="settings-editor-theme"
    ariaLabel="Editor theme"
    value={prefs.editor_theme}
    options={EDITOR_THEMES}
    onselect={(v) => commit((p) => ({ ...p, editor_theme: v as EditorTheme }))}
  />
</SettingField>

<SettingField
  label="Line spacing"
  hint="Reading density for paragraphs and lists in the editor."
>
  <PillRadio
    name="settings-line-spacing"
    ariaLabel="Line spacing"
    value={prefs.line_spacing}
    options={SPACING}
    onselect={(v) => commit((p) => ({ ...p, line_spacing: v as LineSpacing }))}
  />
</SettingField>

<SettingField
  label="Watcher bubbles"
  hint="Show filesystem-watch notices inline, or collapse them to a count tray until expanded."
>
  <PillRadio
    name="settings-bubbles"
    ariaLabel="Watcher bubbles"
    value={prefs.bubble_overlay_mode}
    options={BUBBLES}
    onselect={(v) =>
      commit((p) => ({ ...p, bubble_overlay_mode: v as BubbleOverlayMode }))}
  />
</SettingField>
