// Curated font catalog used by the Settings tab font picker.
//
// Each entry pairs a short display label with the actual CSS
// font-family value we persist. Stack-style strings (with multiple
// fallbacks) are used for the platform-default options so the
// picker degrades gracefully on systems missing a specific face.
//
// Keep this list intentionally small. The aim is "obvious choices,
// universally available" rather than "every font on the system":
// a true installed-fonts enumeration needs Chromium's
// `queryLocalFonts()` which prompts the user and isn't available
// in our Tauri shell.

export type FontFamilyOption = {
  /// Display label shown in the dropdown.
  label: string;
  /// CSS `font-family` value persisted in the global config.
  value: string;
  /// "sans" / "serif" / "mono" hint so callers can split the
  /// catalog by role (the `code` role lists mono fonts first).
  category: "sans" | "serif" | "mono";
};

export const FONT_FAMILIES: FontFamilyOption[] = [
  // Platform defaults (resolve to whatever the OS exposes).
  {
    label: "System UI (default)",
    value:
      "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif",
    category: "sans",
  },
  { label: "ui-sans-serif", value: "ui-sans-serif, system-ui, sans-serif", category: "sans" },
  { label: "ui-serif", value: "ui-serif, Georgia, serif", category: "serif" },
  { label: "ui-monospace", value: "ui-monospace, SFMono-Regular, monospace", category: "mono" },

  // Common sans-serif faces with sensible fallbacks.
  { label: "Helvetica", value: "Helvetica, Arial, sans-serif", category: "sans" },
  { label: "Arial", value: "Arial, sans-serif", category: "sans" },
  { label: "Inter", value: "Inter, system-ui, sans-serif", category: "sans" },
  { label: "Verdana", value: "Verdana, sans-serif", category: "sans" },

  // Serif faces.
  { label: "Georgia", value: "Georgia, serif", category: "serif" },
  { label: "Times New Roman", value: "\"Times New Roman\", Times, serif", category: "serif" },
  { label: "Palatino", value: "Palatino, \"Palatino Linotype\", serif", category: "serif" },

  // Monospace faces (code editors).
  { label: "Menlo", value: "Menlo, Monaco, monospace", category: "mono" },
  { label: "Monaco", value: "Monaco, Menlo, monospace", category: "mono" },
  { label: "Consolas", value: "Consolas, \"Courier New\", monospace", category: "mono" },
  { label: "Courier New", value: "\"Courier New\", Courier, monospace", category: "mono" },
  { label: "JetBrains Mono", value: "\"JetBrains Mono\", ui-monospace, monospace", category: "mono" },
  { label: "Fira Code", value: "\"Fira Code\", ui-monospace, monospace", category: "mono" },
  { label: "Cascadia Code", value: "\"Cascadia Code\", ui-monospace, monospace", category: "mono" },
];

/// Find the catalog entry whose stored value matches `family`.
/// Comparison is exact: stored values come from this very list (or
/// were typed in by hand on a prior install), so anything we
/// recognize is a verbatim match. Mismatches return `null` so the
/// caller can render a synthetic "(custom)" entry instead of
/// silently coercing the saved value to one of the listed options.
export function findFontOption(family: string): FontFamilyOption | null {
  return FONT_FAMILIES.find((o) => o.value === family) ?? null;
}
