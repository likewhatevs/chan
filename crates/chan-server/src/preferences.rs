//! Editor preferences: the fields that don't have another home.
//!
//! Most "preferences" the Settings UI surfaces already live in
//! existing config files:
//!
//! - `attachments_dir`: ServerConfig
//! - `default_drive_root`: chan-drive's Registry (config.toml)
//! - `drives`: chan-drive's Registry
//!
//! What's left lives here, persisted to
//! `<config>/chan/preferences.toml`:
//!
//!   - editor_theme (github / google_docs / word)
//!   - theme  (system / light / dark)
//!   - pane_widths (inspector / graph / file-browser sidebars)
//!   - browser_side_panes (left / right docked file-browser state)
//!   - line_spacing (standard / compact; legacy `tight` deserializes as
//!     compact)
//!   - date_format (id; UI-side mapping in dateFormats.ts)
//!   - strip_trailing_whitespace_on_save
//!
//! The Preferences view returned over /api/drive and /api/config is
//! assembled in lib.rs by joining EditorPrefs with ServerConfig.
//! PATCH /api/config splits the incoming body the same way: edits land
//! in whichever store owns the field.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

/// Fields persisted to `<config>/chan/preferences.toml`. A fresh
/// install defaults to the GitHub editor theme so the renderer
/// matches what most users expect from a markdown editor without
/// any further configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorPrefs {
    #[serde(default)]
    pub editor_theme: EditorTheme,
    #[serde(default)]
    pub theme: ThemeChoice,
    #[serde(default)]
    pub pane_widths: PaneWidths,
    #[serde(default)]
    pub browser_side_panes: BrowserSidePanes,
    #[serde(default)]
    pub line_spacing: LineSpacing,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default)]
    pub strip_trailing_whitespace_on_save: bool,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            editor_theme: EditorTheme::default(),
            theme: ThemeChoice::default(),
            pane_widths: PaneWidths::default(),
            browser_side_panes: BrowserSidePanes::default(),
            line_spacing: LineSpacing::default(),
            date_format: default_date_format(),
            strip_trailing_whitespace_on_save: false,
        }
    }
}

/// Editor theme. Drives the markdown renderer + source view
/// typography and chrome (headings, body, code blocks, blockquotes,
/// tables). Light/dark variants are picked from the active
/// `ThemeChoice`; density from `LineSpacing`. App chrome
/// (toolbar, panes, status bar) is not affected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorTheme {
    #[default]
    Github,
    GoogleDocs,
    Word,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneWidths {
    pub inspector: u32,
    pub graph: u32,
    pub browser: u32,
    // Per-field default so older preferences.toml (written before the
    // search inspector got its own width slot) load cleanly.
    #[serde(default = "default_search_width")]
    pub search: u32,
    // Per-field default so older preferences.toml (written before the
    // outline pane was split out of the right-side inspector) load
    // cleanly. Width of the left-side outline pane in the file editor.
    #[serde(default = "default_outline_width")]
    pub outline: u32,
}

impl Default for PaneWidths {
    // Mirrors web/src/state/store.svelte.ts DEFAULT_PANE_WIDTHS.
    fn default() -> Self {
        Self {
            inspector: 220,
            graph: 260,
            browser: 240,
            search: default_search_width(),
            outline: default_outline_width(),
        }
    }
}

fn default_search_width() -> u32 {
    280
}

fn default_outline_width() -> u32 {
    220
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSidePanes {
    pub left: bool,
    pub right: bool,
}

/// Editor density. `Standard` is the roomier default Google Docs /
/// Word-style spacing; `Compact` tightens prose + list line-height
/// for the Google Docs "single" look. The legacy `tight` value that
/// pre-phase-3 drives wrote to preferences.toml deserializes as
/// `Compact` so existing config files load without manual migration;
/// the next save flushes the canonical `compact` token to disk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineSpacing {
    #[default]
    Standard,
    #[serde(alias = "tight")]
    Compact,
}

fn default_date_format() -> String {
    // Matches the editor's `dateFormats.ts` default.
    "iso".into()
}

impl EditorPrefs {
    pub fn load() -> Result<Self, Error> {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Result<Self, Error> {
        crate::store::load_toml(path)
    }

    pub fn save(&self) -> Result<(), Error> {
        self.save_to(&default_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<(), Error> {
        crate::store::save_toml(path, self)
    }
}

/// `~/.chan/preferences.toml` on desktop. iOS / Android pass an
/// explicit path via `load_from` / `save_to` since their sandbox
/// dir isn't `chan_drive::paths::config_dir`.
pub fn default_path() -> PathBuf {
    chan_drive::paths::config_dir().join("preferences.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        let prefs = EditorPrefs::default();
        prefs.save_to(&p).unwrap();
        let loaded = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs, loaded);
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let prefs = EditorPrefs::load_from(&tmp.path().join("nope.toml")).unwrap();
        assert_eq!(prefs, EditorPrefs::default());
    }

    #[test]
    fn partial_file_fills_defaults() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        std::fs::write(&p, "date_format = \"long\"\n").unwrap();
        let prefs = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs.date_format, "long");
        assert_eq!(prefs.theme, ThemeChoice::System);
    }

    #[test]
    fn pane_widths_partial_fills_missing_defaults() {
        // A preferences.toml that omits some pane-width slots must
        // still load, with the missing slots resolved to their
        // current defaults. Guards against the partial-config
        // resilience contract for `[pane_widths]`.
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        std::fs::write(
            &p,
            "[pane_widths]\ninspector = 240\ngraph = 300\nbrowser = 250\n",
        )
        .unwrap();
        let prefs = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs.pane_widths.inspector, 240);
        assert_eq!(prefs.pane_widths.graph, 300);
        assert_eq!(prefs.pane_widths.browser, 250);
        assert_eq!(prefs.pane_widths.search, default_search_width());
        assert_eq!(prefs.pane_widths.outline, default_outline_width());
    }

    #[test]
    fn theme_serializes_lowercase() {
        let prefs = EditorPrefs {
            theme: ThemeChoice::Dark,
            ..Default::default()
        };
        let s = toml::to_string(&prefs).unwrap();
        assert!(s.contains("theme = \"dark\""), "got: {s}");
    }

    #[test]
    fn editor_theme_defaults_to_github_and_serializes_snake_case() {
        let prefs = EditorPrefs::default();
        assert_eq!(prefs.editor_theme, EditorTheme::Github);
        let prefs = EditorPrefs {
            editor_theme: EditorTheme::GoogleDocs,
            ..Default::default()
        };
        let s = toml::to_string(&prefs).unwrap();
        assert!(s.contains("editor_theme = \"google_docs\""), "got: {s}");
    }

    #[test]
    fn line_spacing_default_is_standard() {
        // Phase-3 flipped the default from `tight` to `standard`.
        // The default is observable on a fresh drive that never wrote
        // a preferences.toml; lock it down so a future refactor that
        // re-orders the variants doesn't silently change behavior.
        let prefs = EditorPrefs::default();
        assert_eq!(prefs.line_spacing, LineSpacing::Standard);
    }

    #[test]
    fn line_spacing_serializes_canonical_tokens() {
        // Wire form must use the variant's lowercase name; the
        // legacy `tight` token is read-only via #[serde(alias)] and
        // must never be emitted on save.
        let standard = toml::to_string(&EditorPrefs {
            line_spacing: LineSpacing::Standard,
            ..Default::default()
        })
        .unwrap();
        assert!(
            standard.contains("line_spacing = \"standard\""),
            "got: {standard}"
        );
        let compact = toml::to_string(&EditorPrefs {
            line_spacing: LineSpacing::Compact,
            ..Default::default()
        })
        .unwrap();
        assert!(
            compact.contains("line_spacing = \"compact\""),
            "got: {compact}"
        );
        assert!(
            !compact.contains("\"tight\""),
            "compact serializes as `tight`: {compact}"
        );
    }

    #[test]
    fn line_spacing_legacy_tight_loads_as_compact() {
        // Pre-phase-3 drives have `line_spacing = "tight"` on disk.
        // The serde alias lets those files load without manual
        // migration; the next save flushes the canonical `compact`
        // token, so this compatibility shim self-erodes over time.
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        std::fs::write(&p, "line_spacing = \"tight\"\n").unwrap();
        let prefs = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs.line_spacing, LineSpacing::Compact);
        // Sanity: re-saving emits the canonical token.
        prefs.save_to(&p).unwrap();
        let saved = std::fs::read_to_string(&p).unwrap();
        assert!(saved.contains("line_spacing = \"compact\""), "got: {saved}");
    }

    #[test]
    fn line_spacing_compact_loads_as_compact() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("preferences.toml");
        std::fs::write(&p, "line_spacing = \"compact\"\n").unwrap();
        let prefs = EditorPrefs::load_from(&p).unwrap();
        assert_eq!(prefs.line_spacing, LineSpacing::Compact);
    }
}
